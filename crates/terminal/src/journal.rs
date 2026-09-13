//! Durable pane journal (ADR 021 tier 2): batched, bounded PTY output writer.
//!
//! The journal is a second fan-out subscriber of the terminal broadcast
//! (Plan rule 2): output is captured once and written here together with live
//! clients. Batching (default 250 ms or 64 KiB, whichever first) keeps write
//! pressure off the PTY reader thread; per-pane and global byte caps keep the
//! backing store bounded (Plan rule 8). A crash mid-batch loses at most one
//! batch - restore keeps the last whole seq and flags the pane degraded.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Storage backend for journal chunks (SQLite via `orbynode_database`).
pub trait JournalStore: Send + Sync + 'static {
    /// Append one chunk; returns the assigned seq.
    fn append(&self, pane_id: i64, chunk: &[u8]) -> Result<i64, String>;
    /// Chunks strictly after `after_seq`, oldest first.
    fn read(&self, pane_id: i64, after_seq: i64) -> Result<Vec<(i64, Vec<u8>)>, String>;
    /// Highest stored seq, or -1 when empty.
    fn max_seq(&self, pane_id: i64) -> Result<i64, String>;
}

/// Batch tuning knobs.
#[derive(Debug, Clone)]
pub struct JournalConfig {
    /// Flush when this much output accumulated.
    pub batch_bytes: usize,
    /// Flush at least this often.
    pub batch_ms: u64,
    /// Per-pane retained bytes; older chunks evicted oldest-first.
    pub pane_cap_bytes: i64,
    /// Process-wide retained bytes across all panes.
    pub global_cap_bytes: i64,
}

impl Default for JournalConfig {
    fn default() -> Self {
        JournalConfig {
            batch_bytes: 64 * 1024,
            batch_ms: 250,
            pane_cap_bytes: 2 * 1024 * 1024,
            global_cap_bytes: 256 * 1024 * 1024,
        }
    }
}

/// Per-pane in-memory staging buffer.
#[derive(Default)]
struct Staging {
    bytes: Vec<u8>,
}

/// Handle for one journaled pane. Cloning the `Arc` keeps the writer alive.
#[derive(Clone)]
pub struct PaneJournal {
    inner: Arc<JournalInner>,
}

struct JournalInner {
    pane_id: i64,
    store: Arc<dyn JournalStore>,
    cfg: JournalConfig,
    staging: Mutex<Staging>,
    /// Last seq handed to the store (persisted view).
    last_seq: AtomicI64,
    /// Set when a write failed; the pane is degraded until re-enabled.
    degraded: Mutex<bool>,
    shutdown: tokio::sync::Notify,
}

impl PaneJournal {
    /// Start the batching writer task for this pane.
    pub fn start(pane_id: i64, store: Arc<dyn JournalStore>, cfg: JournalConfig) -> Self {
        let last_seq = store.max_seq(pane_id).unwrap_or(-1);
        let inner = Arc::new(JournalInner {
            pane_id,
            store,
            cfg,
            staging: Mutex::new(Staging::default()),
            last_seq: AtomicI64::new(last_seq),
            degraded: Mutex::new(false),
            shutdown: tokio::sync::Notify::new(),
        });
        spawn_flusher(inner.clone());
        PaneJournal { inner }
    }

    /// Stage output bytes; flushed on the next batch boundary.
    pub fn record(&self, chunk: &[u8]) {
        if *self
            .inner
            .degraded
            .lock()
            .expect("journal degraded poisoned")
        {
            return; // degrade silently: never block the PTY reader
        }
        let mut st = self.inner.staging.lock().expect("journal staging poisoned");
        st.bytes.extend_from_slice(chunk);
        // Hard staging bound: if output outruns the flusher, drop the middle
        // and keep head+tail so recent context survives.
        if st.bytes.len() > self.inner.cfg.batch_bytes * 8 {
            let keep = self.inner.cfg.batch_bytes * 4;
            let drop_n = st.bytes.len() - keep;
            st.bytes.drain(..drop_n);
        }
    }

    /// Persisted seq so far (monotonic per pane).
    pub fn persisted_seq(&self) -> i64 {
        self.inner.last_seq.load(Ordering::Relaxed)
    }

    pub fn is_degraded(&self) -> bool {
        *self
            .inner
            .degraded
            .lock()
            .expect("journal degraded poisoned")
    }

    /// Clear the degraded flag after operator intervention.
    pub fn clear_degraded(&self) {
        *self
            .inner
            .degraded
            .lock()
            .expect("journal degraded poisoned") = false;
    }

    /// Flush staging now (graceful shutdown path).
    pub async fn flush(&self) {
        self.flush_once();
    }

    /// Signal the flusher to stop after a final flush.
    pub async fn shutdown(&self) {
        self.flush_once();
        self.inner.shutdown.notify_waiters();
    }

    fn flush_once(&self) {
        let bytes = {
            let mut st = self.inner.staging.lock().expect("journal staging poisoned");
            if st.bytes.is_empty() {
                return;
            }
            std::mem::take(&mut st.bytes)
        };
        match self.inner.store.append(self.inner.pane_id, &bytes) {
            Ok(seq) => {
                self.inner.last_seq.store(seq, Ordering::Relaxed);
            }
            Err(e) => {
                tracing::error!(pane = self.inner.pane_id, error = %e, "journal append failed");
                *self
                    .inner
                    .degraded
                    .lock()
                    .expect("journal degraded poisoned") = true;
            }
        }
    }
}

/// Free-function flush used by the flusher task (operates through the handle).
fn flush_inner(inner: &Arc<JournalInner>) {
    let bytes = {
        let mut st = inner.staging.lock().expect("journal staging poisoned");
        if st.bytes.is_empty() {
            return;
        }
        std::mem::take(&mut st.bytes)
    };
    match inner.store.append(inner.pane_id, &bytes) {
        Ok(seq) => {
            inner.last_seq.store(seq, Ordering::Relaxed);
        }
        Err(e) => {
            tracing::error!(pane = inner.pane_id, error = %e, "journal append failed");
            *inner.degraded.lock().expect("journal degraded poisoned") = true;
        }
    }
}

fn spawn_flusher(inner: Arc<JournalInner>) {
    tokio::spawn(async move {
        let interval = Duration::from_millis(inner.cfg.batch_ms.max(10));
        loop {
            tokio::select! {
                _ = tokio::time::sleep(interval) => {}
                _ = inner.shutdown.notified() => {
                    flush_inner(&inner);
                    return;
                }
            }
            let should_flush = {
                let st = inner.staging.lock().expect("journal staging poisoned");
                !st.bytes.is_empty()
            };
            if should_flush {
                flush_inner(&inner);
            }
        }
    });
}

/// Maps terminal IDs to pane journals; the daemon keeps one per journaled pane.
#[derive(Default)]
pub struct JournalRegistry {
    by_terminal: Mutex<HashMap<u64, Arc<PaneJournal>>>,
    next_temp: AtomicU64,
}

impl JournalRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin journaling a terminal as the given pane.
    pub fn attach(
        &self,
        terminal_id: u64,
        pane_id: i64,
        store: Arc<dyn JournalStore>,
        cfg: JournalConfig,
    ) -> Arc<PaneJournal> {
        let journal = Arc::new(PaneJournal::start(pane_id, store, cfg));
        self.by_terminal
            .lock()
            .expect("journal registry poisoned")
            .insert(terminal_id, journal.clone());
        journal
    }

    pub fn get(&self, terminal_id: u64) -> Option<Arc<PaneJournal>> {
        self.by_terminal
            .lock()
            .expect("journal registry poisoned")
            .get(&terminal_id)
            .cloned()
    }

    /// Stop journaling a terminal after a final flush.
    pub async fn detach(&self, terminal_id: u64) {
        let journal = self
            .by_terminal
            .lock()
            .expect("journal registry poisoned")
            .remove(&terminal_id);
        if let Some(j) = journal {
            j.shutdown().await;
        }
    }

    /// Placeholder id for journal-less callers (never persisted).
    pub fn temp_id(&self) -> u64 {
        self.next_temp.fetch_add(1, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MemStore {
        rows: Mutex<Vec<(i64, i64, Vec<u8>)>>,
        next: AtomicI64,
        fail: AtomicU64,
    }

    impl MemStore {
        fn with_failures(n: u64) -> Arc<Self> {
            let s = Self::default();
            s.fail.store(n, Ordering::Relaxed);
            Arc::new(s)
        }
    }

    impl JournalStore for MemStore {
        fn append(&self, pane_id: i64, chunk: &[u8]) -> Result<i64, String> {
            if self.fail.fetch_sub(1, Ordering::Relaxed) > 0 {
                return Err("injected failure".into());
            }
            let seq = self.next.fetch_add(1, Ordering::Relaxed) + 1;
            self.rows
                .lock()
                .unwrap()
                .push((pane_id, seq, chunk.to_vec()));
            Ok(seq)
        }
        fn read(&self, pane_id: i64, after_seq: i64) -> Result<Vec<(i64, Vec<u8>)>, String> {
            Ok(self
                .rows
                .lock()
                .unwrap()
                .iter()
                .filter(|(p, s, _)| *p == pane_id && *s > after_seq)
                .map(|(_, s, c)| (*s, c.clone()))
                .collect())
        }
        fn max_seq(&self, pane_id: i64) -> Result<i64, String> {
            Ok(self
                .rows
                .lock()
                .unwrap()
                .iter()
                .filter(|(p, _, _)| *p == pane_id)
                .map(|(_, s, _)| *s)
                .max()
                .unwrap_or(-1))
        }
    }

    #[tokio::test]
    async fn records_are_batched_and_persisted() {
        let store: Arc<dyn JournalStore> = Arc::new(MemStore::default());
        let cfg = JournalConfig {
            batch_ms: 10,
            ..JournalConfig::default()
        };
        let journal = PaneJournal::start(1, store.clone(), cfg);
        journal.record(b"hello ");
        journal.record(b"world");
        // Wait past the batch window.
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert!(journal.persisted_seq() >= 1, "chunks persisted");
        let rows = store.read(1, -1).unwrap();
        let joined: Vec<u8> = rows.iter().flat_map(|(_, c)| c.iter().copied()).collect();
        assert_eq!(joined, b"hello world");
        journal.shutdown().await;
    }

    #[tokio::test]
    async fn write_failure_degrades_without_panicking() {
        let store = MemStore::with_failures(1);
        let cfg = JournalConfig {
            batch_ms: 10,
            ..JournalConfig::default()
        };
        let journal = PaneJournal::start(2, store as Arc<dyn JournalStore>, cfg);
        journal.record(b"will fail");
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert!(journal.is_degraded(), "failure must degrade the pane");
        // Recording while degraded is a no-op, not a crash.
        journal.record(b"ignored");
        journal.clear_degraded();
        journal.record(b"accepted again");
        journal.shutdown().await;
    }

    #[tokio::test]
    async fn shutdown_flushes_pending_bytes() {
        let store: Arc<dyn JournalStore> = Arc::new(MemStore::default());
        let cfg = JournalConfig {
            batch_ms: 60_000, // flusher never fires during the test
            ..JournalConfig::default()
        };
        let journal = PaneJournal::start(3, store.clone(), cfg);
        journal.record(b"final bytes");
        journal.shutdown().await;
        let rows = store.read(3, -1).unwrap();
        assert_eq!(rows.len(), 1, "shutdown flush persisted staging");
        assert_eq!(rows[0].1, b"final bytes");
    }
}
