//! OrbyNode terminal runtime — real PTYs, captured once, fanned out (ADR 002).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::broadcast;

// ---------- Public API ----------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalConfig {
    pub cols: u16,
    pub rows: u16,
    pub scrollback_bytes: usize,
    pub cwd: Option<std::path::PathBuf>,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        TerminalConfig {
            cols: 80,
            rows: 24,
            scrollback_bytes: 1024 * 1024,
            cwd: None,
        }
    }
}

/// Static metadata about a terminal.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TerminalInfo {
    pub id: u64,
    pub cols: u16,
    pub rows: u16,
    pub exited: bool,
}

pub type TerminalId = u64;

#[derive(Debug)]
pub enum TerminalError {
    NotFound,
    Io(std::io::Error),
}

impl std::fmt::Display for TerminalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerminalError::NotFound => write!(f, "terminal not found"),
            TerminalError::Io(e) => write!(f, "terminal io error: {e}"),
        }
    }
}

impl std::error::Error for TerminalError {}

/// Shared internal state of one live PTY.
struct TerminalInner {
    info: Mutex<TerminalInfo>,
    writer: Mutex<Box<dyn std::io::Write + Send>>,
    master: Mutex<Box<dyn portable_pty::MasterPty + Send>>,
    tx: broadcast::Sender<Vec<u8>>,
    scrollback: Mutex<Scrollback>,
    exit_tx: tokio::sync::watch::Sender<bool>,
    killer: Mutex<Box<dyn portable_pty::ChildKiller + Send + Sync>>,
}

/// One live PTY. Cheap to clone (Arc inside).
#[derive(Clone)]
pub struct Terminal {
    id: TerminalId,
    inner: Arc<TerminalInner>,
}

/// Bounded byte ring buffer for scrollback (Plan §66).
struct Scrollback {
    buf: Vec<u8>,
    cap: usize,
}

impl Scrollback {
    fn new(cap: usize) -> Self {
        Scrollback {
            buf: Vec::with_capacity(cap.min(64 * 1024)),
            cap,
        }
    }
    fn push(&mut self, data: &[u8]) {
        if data.len() >= self.cap {
            // Keep only the tail.
            self.buf.clear();
            self.buf.extend_from_slice(&data[data.len() - self.cap..]);
            return;
        }
        let excess = (self.buf.len() + data.len()).saturating_sub(self.cap);
        if excess > 0 {
            self.buf.drain(..excess);
        }
        self.buf.extend_from_slice(data);
    }
}

impl Terminal {
    pub fn info(&self) -> TerminalInfo {
        self.inner
            .info
            .lock()
            .expect("terminal info poisoned")
            .clone()
    }

    pub fn id(&self) -> TerminalId {
        self.id
    }

    /// Subscribe to raw PTY output (captured once, fanned out — Plan §59).
    pub fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
        self.inner.tx.subscribe()
    }

    /// Replay of the bounded scrollback ring buffer (reconnect path).
    pub fn scrollback(&self) -> Vec<u8> {
        self.inner
            .scrollback
            .lock()
            .expect("scrollback poisoned")
            .buf
            .clone()
    }

    /// Write raw bytes to the PTY (stdin).
    pub fn write(&self, data: &[u8]) -> Result<(), TerminalError> {
        if self
            .inner
            .info
            .lock()
            .expect("terminal info poisoned")
            .exited
        {
            return Err(TerminalError::Io(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "terminal exited",
            )));
        }
        let mut w = self.inner.writer.lock().expect("writer poisoned");
        w.write_all(data)
            .and_then(|()| w.flush())
            .map_err(TerminalError::Io)
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), TerminalError> {
        {
            let mut info = self.inner.info.lock().expect("terminal info poisoned");
            info.cols = cols;
            info.rows = rows;
        }
        self.inner
            .master
            .lock()
            .expect("master poisoned")
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TerminalError::Io(std::io::Error::other(e.to_string())))
    }

    /// Resolves when the child exits.
    pub async fn wait_exit(&self) -> Option<()> {
        let mut rx = self.inner.exit_tx.subscribe();
        // Observe the current value first so already-exited terminals resolve.
        while !*rx.borrow_and_update() {
            if rx.changed().await.is_err() {
                return None;
            }
        }
        Some(())
    }

    /// Kill the child, then wait for the exit signal.
    pub async fn terminate(&self) -> Result<(), TerminalError> {
        self.inner
            .killer
            .lock()
            .expect("killer poisoned")
            .kill()
            .map_err(|e| TerminalError::Io(std::io::Error::other(e.to_string())))?;
        tokio::time::timeout(Duration::from_secs(5), self.wait_exit())
            .await
            .map_err(|_| {
                TerminalError::Io(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "terminal did not exit in time",
                ))
            })?;
        Ok(())
    }
}

/// Creates and tracks terminals for this daemon process.
pub struct TerminalManager {
    defaults: TerminalConfig,
    terminals: Mutex<HashMap<TerminalId, Arc<Terminal>>>,
    next_id: AtomicU64,
}

impl TerminalManager {
    pub fn new(defaults: TerminalConfig) -> Arc<Self> {
        Arc::new(TerminalManager {
            defaults,
            terminals: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        })
    }

    /// Spawn a real PTY running the platform shell, start its reader thread.
    pub fn create(&self, cfg: TerminalConfig) -> Result<Arc<Terminal>, TerminalError> {
        // Zero/absent fields fall back to the manager defaults.
        let cfg = TerminalConfig {
            cols: if cfg.cols == 0 {
                self.defaults.cols
            } else {
                cfg.cols
            },
            rows: if cfg.rows == 0 {
                self.defaults.rows
            } else {
                cfg.rows
            },
            scrollback_bytes: if cfg.scrollback_bytes == 0 {
                self.defaults.scrollback_bytes
            } else {
                cfg.scrollback_bytes
            },
            cwd: cfg.cwd.or_else(|| self.defaults.cwd.clone()),
        };
        let cols = if cfg.cols == 0 { 80 } else { cfg.cols };
        let rows = if cfg.rows == 0 { 24 } else { cfg.rows };
        let pty_system = portable_pty::native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TerminalError::Io(std::io::Error::other(e.to_string())))?;

        let mut cmd = shell_command();
        if let Some(dir) = &cfg.cwd {
            cmd.cwd(dir);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| TerminalError::Io(std::io::Error::other(e.to_string())))?;
        drop(pair.slave); // master holds the other end

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, _) = broadcast::channel(4096);
        let (exit_tx, _exit_rx) = tokio::sync::watch::channel(false);

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| TerminalError::Io(std::io::Error::other(e.to_string())))?;
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| TerminalError::Io(std::io::Error::other(e.to_string())))?;

        let term = Arc::new(Terminal {
            id,
            inner: Arc::new(TerminalInner {
                info: Mutex::new(TerminalInfo {
                    id,
                    cols,
                    rows,
                    exited: false,
                }),
                writer: Mutex::new(writer),
                master: Mutex::new(pair.master),
                tx,
                scrollback: Mutex::new(Scrollback::new(cfg.scrollback_bytes)),
                exit_tx,
                killer: Mutex::new(child.clone_killer()),
            }),
        });

        spawn_reader(term.clone(), reader);
        spawn_reaper(term.clone(), child);
        self.terminals
            .lock()
            .expect("terminals poisoned")
            .insert(id, term.clone());
        Ok(term)
    }

    pub fn get(&self, id: TerminalId) -> Option<Arc<Terminal>> {
        self.terminals
            .lock()
            .expect("terminals poisoned")
            .get(&id)
            .cloned()
    }

    pub fn list(&self) -> Vec<TerminalInfo> {
        self.terminals
            .lock()
            .expect("terminals poisoned")
            .values()
            .map(|t| t.info())
            .collect()
    }

    pub async fn terminate(&self, id: TerminalId) -> Result<(), TerminalError> {
        let term = self.get(id).ok_or(TerminalError::NotFound)?;
        term.terminate().await?;
        self.terminals
            .lock()
            .expect("terminals poisoned")
            .remove(&id);
        Ok(())
    }
}

/// Dedicated blocking reader thread (ADR 002): reads the PTY once, fans out to
/// broadcast subscribers and the scrollback ring buffer, and watches for exit.
fn spawn_reader(term: Arc<Terminal>, mut reader: Box<dyn std::io::Read + Send>) {
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break, // EOF: child exited and stream closed
                Ok(n) => {
                    let _ = term.inner.tx.send(buf[..n].to_vec());
                    term.inner
                        .scrollback
                        .lock()
                        .expect("scrollback poisoned")
                        .push(&buf[..n]);
                }
                Err(e) => {
                    tracing::debug!(error = %e, terminal = term.id, "pty read error");
                    break;
                }
            }
        }
        tracing::debug!(terminal = term.id, "pty reader eof");
    });
}

/// Authoritative exit watcher: `child.wait()` reaps and signals, independent
/// of PTY EOF (grandchildren can keep the master readable after the shell dies).
fn spawn_reaper(term: Arc<Terminal>, mut child: Box<dyn portable_pty::Child + Send + Sync>) {
    std::thread::spawn(move || {
        let _ = child.wait();
        let mut info = term.inner.info.lock().expect("terminal info poisoned");
        info.exited = true;
        drop(info);
        let _ = term.inner.exit_tx.send(true);
        tracing::info!(terminal = term.id, "terminal exited");
    });
}

/// Platform shell for M1 (ADR 002: full detection lands with agent support).
fn shell_command() -> portable_pty::CommandBuilder {
    if cfg!(windows) {
        let cmd = portable_pty::CommandBuilder::new("powershell.exe");
        return cmd;
    }
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());
    let mut cmd = portable_pty::CommandBuilder::new(shell);
    cmd.arg("-l");
    cmd.env("TERM", "xterm-256color");
    cmd
}

use portable_pty::PtySize;

impl Drop for TerminalInner {
    fn drop(&mut self) {
        // Best-effort: the child must not outlive the terminal struct when the
        // manager drops it (daemon shutdown path refined in M3).
        if let Ok(mut killer) = self.killer.lock() {
            let _ = killer.kill();
        }
    }
}

// ---------- Tests (TDD: these define the behavior) ----------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    async fn recv_with_timeout(rx: &mut broadcast::Receiver<Vec<u8>>, needle: &[u8]) -> bool {
        let deadline = tokio::time::sleep(Duration::from_secs(10));
        tokio::pin!(deadline);
        loop {
            tokio::select! {
                _ = &mut deadline => return false,
                ev = rx.recv() => match ev {
                    Ok(bytes) => {
                        tracing::debug!(bytes = bytes.len(), "pty out");
                        if windows_subsequence(&bytes, needle) { return true; }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => return false,
                },
            }
        }
    }

    /// Subsequence match so interleaved PTY echo cannot break the assertion.
    fn windows_subsequence(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len().max(1)).any(|w| w == needle)
    }

    #[tokio::test]
    async fn creates_terminal_and_streams_output() {
        let mgr = TerminalManager::new(TerminalConfig::default());
        let term = mgr.create(TerminalConfig::default()).expect("create");
        let mut rx = term.subscribe();
        term.write(b"echo __orby_mark__\n").expect("write");
        assert!(
            recv_with_timeout(&mut rx, b"__orby_mark__").await,
            "marker not seen"
        );
    }

    #[tokio::test]
    async fn two_subscribers_both_receive_output() {
        let mgr = TerminalManager::new(TerminalConfig::default());
        let term = mgr.create(TerminalConfig::default()).expect("create");
        let mut rx1 = term.subscribe();
        let mut rx2 = term.subscribe();
        term.write(b"echo __two_marks__\n").expect("write");
        let (a, b) = tokio::join!(
            recv_with_timeout(&mut rx1, b"__two_marks__"),
            recv_with_timeout(&mut rx2, b"__two_marks__")
        );
        assert!(a && b, "both subscribers must see the output");
    }

    #[tokio::test]
    async fn scrollback_replays_recent_output() {
        let cfg = TerminalConfig {
            scrollback_bytes: 64 * 1024,
            ..Default::default()
        };
        let mgr = TerminalManager::new(cfg);
        let term = mgr.create(TerminalConfig::default()).expect("create");
        let mut rx = term.subscribe();
        term.write(b"echo __scroll_mark__\n").expect("write");
        assert!(recv_with_timeout(&mut rx, b"__scroll_mark__").await);
        // Allow the reader thread to land the bytes in the ring buffer.
        for _ in 0..50 {
            if windows_subsequence(&term.scrollback(), b"__scroll_mark__") {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("scrollback never contained the marker");
    }

    #[tokio::test]
    async fn scrollback_is_bounded() {
        let cfg = TerminalConfig {
            scrollback_bytes: 32,
            ..Default::default()
        };
        let mgr = TerminalManager::new(cfg);
        let term = mgr
            .create(TerminalConfig {
                scrollback_bytes: 32,
                ..Default::default()
            })
            .expect("create");
        term.write(&[b'x'; 4096]).expect("write");
        // Give the reader a moment, then the buffer must be capped.
        for _ in 0..20 {
            if term.scrollback().len() >= 32 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(
            term.scrollback().len() <= 32,
            "scrollback exceeded bound: {}",
            term.scrollback().len()
        );
    }

    #[tokio::test]
    async fn resize_is_accepted_and_metadata_tracks_size() {
        let mgr = TerminalManager::new(TerminalConfig::default());
        let term = mgr.create(TerminalConfig::default()).expect("create");
        term.resize(120, 40).expect("resize");
        assert_eq!(term.info().cols, 120);
        assert_eq!(term.info().rows, 40);
    }

    #[tokio::test]
    async fn terminate_ends_child_and_is_reported() {
        let mgr = TerminalManager::new(TerminalConfig::default());
        let term = mgr.create(TerminalConfig::default()).expect("create");
        term.terminate().await.expect("terminate");
        tokio::time::timeout(Duration::from_secs(10), term.wait_exit())
            .await
            .expect("exit timed out")
            .expect("exit signal");
        assert!(term.info().exited);
        // Subsequent writes fail cleanly.
        assert!(term.write(b"nope").is_err());
    }

    #[tokio::test]
    async fn manager_lists_and_gets_terminals() {
        let mgr = TerminalManager::new(TerminalConfig::default());
        let t1 = mgr.create(TerminalConfig::default()).expect("create");
        let infos = mgr.list();
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].id, t1.info().id);
        assert!(mgr.get(t1.info().id).is_some());
        assert!(mgr.get(999_999).is_none());
        assert!(matches!(
            mgr.terminate(999_999).await,
            Err(TerminalError::NotFound)
        ));
    }
}
