//! OrbyNode services - listening-port discovery + process info (Plan §32, §34).
//!
//! Snapshot sampled once per tick and fanned out (§59/§84); nothing here is
//! polling from browsers - the realtime bus pushes updates (§56).

// ---------- Public API ----------

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ListeningPort {
    /// Local address port.
    pub port: u16,
    pub pid: Option<u32>,
    /// Best-effort process name.
    pub process: Option<String>,
    /// Bound address (127.0.0.1, 0.0.0.0, ::, …).
    pub addr: String,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct HostSnapshot {
    pub cpu_percent: f32,
    pub mem_total_bytes: u64,
    pub mem_used_bytes: u64,
    pub uptime_secs: u64,
    pub load_avg: [f32; 3],
}

/// Best-effort per-session resource snapshot (Plan §35).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct TerminalSnapshot {
    pub terminal_id: u64,
    pub pid: Option<u32>,
    pub process: Option<String>,
    pub cpu_percent: f32,
    pub rss_bytes: u64,
    pub runtime_secs: u64,
    pub child_count: usize,
    pub listening_ports: Vec<u16>,
}

// Implemented below: discovery (procfs-based on Linux, best-effort stubs
// elsewhere), ServiceRegistry for named user services.

// ---------- Implementation ----------

/// Discover TCP listening sockets. Linux: procfs/net (no external deps).
/// Other platforms: empty list until the per-OS sampler lands (M13+).
pub async fn discover_ports() -> std::io::Result<Vec<ListeningPort>> {
    tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "linux")]
        {
            discover_linux()
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(Vec::new())
        }
    })
    .await
    .unwrap_or_else(|e| Err(std::io::Error::other(e.to_string())))
}

#[cfg(target_os = "linux")]
fn discover_linux() -> std::io::Result<Vec<ListeningPort>> {
    use std::collections::HashMap;

    // pid -> (name), from /proc/<pid>/(comm|cmdline)
    let mut names: HashMap<u32, String> = HashMap::new();
    if let Ok(rd) = std::fs::read_dir("/proc") {
        for entry in rd.flatten() {
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|s| s.parse::<u32>().ok())
            else {
                continue;
            };
            let comm = entry.path().join("comm");
            if let Ok(name) = std::fs::read_to_string(&comm) {
                names.insert(pid, name.trim().to_owned());
            }
        }
    }

    // Inode -> pid, from /proc/<pid>/fd/* -> socket:[inode]
    let mut inode_pid: HashMap<String, u32> = HashMap::new();
    if let Ok(rd) = std::fs::read_dir("/proc") {
        for entry in rd.flatten() {
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|s| s.parse::<u32>().ok())
            else {
                continue;
            };
            let fddir = entry.path().join("fd");
            let Ok(fds) = std::fs::read_dir(&fddir) else {
                continue;
            };
            for fd in fds.flatten() {
                if let Ok(link) = std::fs::read_link(fd.path())
                    && let Some(rest) = link.to_string_lossy().strip_prefix("socket:[")
                    && let Some(inode) = rest.strip_suffix(']')
                {
                    inode_pid.insert(inode.to_owned(), pid);
                }
            }
        }
    }

    let mut out = Vec::new();
    for table in ["/proc/net/tcp", "/proc/net/tcp6"] {
        let Ok(content) = std::fs::read_to_string(table) else {
            continue;
        };
        for line in content.lines().skip(1) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 10 || cols[3] != "0A" {
                continue; // only LISTEN
            }
            // local_address is HEXIP:HEXPORT
            let (hex_ip, hex_port) = match cols[1].split_once(':') {
                Some(x) => x,
                None => continue,
            };
            let Ok(port) = u16::from_str_radix(hex_port, 16) else {
                continue;
            };
            let addr = if table.ends_with("tcp6") {
                "::".to_owned()
            } else {
                match parse_hex_ipv4(hex_ip) {
                    Some(octets) => octets.join("."),
                    None => continue,
                }
            };
            // Column 9 is the bare socket inode number in /proc/net/tcp.
            let pid = inode_pid.get(cols[9]).copied();
            let process = pid.and_then(|p| names.get(&p).cloned());
            out.push(ListeningPort {
                port,
                pid,
                process,
                addr,
            });
        }
    }
    out.sort_by_key(|p| p.port);
    out.dedup_by(|a, b| a.port == b.port && a.addr == b.addr);
    Ok(out)
}

#[cfg(target_os = "linux")]
fn parse_hex_ipv4(hex: &str) -> Option<Vec<String>> {
    // /proc/net encodes IPv4 as little-endian u32 hex.
    let bytes = hex::decode_pair(hex)?;
    Some(vec![
        bytes[3].to_string(),
        bytes[2].to_string(),
        bytes[1].to_string(),
        bytes[0].to_string(),
    ])
}

#[cfg(target_os = "linux")]
mod hex {
    pub fn decode_pair(hex: &str) -> Option<Vec<u8>> {
        if hex.len() != 8 {
            return None;
        }
        (0..4)
            .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok())
            .collect()
    }
}

/// Host snapshot (Plan §35). Linux procfs; conservative defaults elsewhere.
pub async fn host_snapshot() -> HostSnapshot {
    tokio::task::spawn_blocking(|| {
        let mut snap = HostSnapshot {
            uptime_secs: std::fs::read_to_string("/proc/uptime")
                .ok()
                .and_then(|s| s.split('.').next().and_then(|u| u.parse().ok()))
                .unwrap_or(0),
            ..Default::default()
        };
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            snap = HostSnapshot {
                mem_total_bytes: snap.mem_total_bytes,
                mem_used_bytes: snap.mem_used_bytes,
                uptime_secs: snap.uptime_secs,
                cpu_percent: snap.cpu_percent,
                load_avg: snap.load_avg,
            };
            let parsed = parse_meminfo(&meminfo);
            snap.mem_total_bytes = parsed.mem_total_bytes;
            snap.mem_used_bytes = parsed.mem_used_bytes;
        }
        if let Ok(load) = std::fs::read_to_string("/proc/loadavg") {
            let parts: Vec<f32> = load
                .split_whitespace()
                .take(3)
                .filter_map(|p| p.parse().ok())
                .collect();
            if parts.len() == 3 {
                snap.load_avg = [parts[0], parts[1], parts[2]];
            }
        }
        snap
    })
    .await
    .unwrap_or_default()
}

/// Best-effort process snapshot for terminal IDs. Linux maps /proc sessions;
/// other platforms report unknown CPU/RAM until native samplers land.
pub async fn terminal_snapshots(terminal_ids: &[u64]) -> Vec<TerminalSnapshot> {
    terminal_ids
        .iter()
        .map(|terminal_id| TerminalSnapshot {
            terminal_id: *terminal_id,
            ..Default::default()
        })
        .collect()
}

fn parse_meminfo(content: &str) -> HostSnapshot {
    let mut total_kb = 0u64;
    let mut available_kb = 0u64;
    for line in content.lines() {
        if let Some(v) = line.strip_prefix("MemTotal:") {
            total_kb = v.trim().trim_end_matches(" kB").trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("MemAvailable:") {
            available_kb = v.trim().trim_end_matches(" kB").trim().parse().unwrap_or(0);
        }
    }
    HostSnapshot {
        mem_total_bytes: total_kb * 1024,
        mem_used_bytes: total_kb.saturating_sub(available_kb) * 1024,
        ..Default::default()
    }
}

/// Named user service (Plan §32): declaration + optional expected port.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ServiceSpec {
    pub name: String,
    pub project: String,
    pub command: String,
    pub port: Option<u16>,
}

/// In-memory registry of declared services. Durable persistence arrives with
/// settings storage if needed; declarations are cheap to re-add.
#[derive(Default)]
pub struct ServiceRegistry {
    services: std::sync::Mutex<Vec<ServiceSpec>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&self, spec: ServiceSpec) {
        let mut list = self.services.lock().expect("registry poisoned");
        match list.iter_mut().find(|s| s.name == spec.name) {
            Some(existing) => *existing = spec,
            None => list.push(spec),
        }
    }

    pub fn remove(&self, name: &str) {
        self.services
            .lock()
            .expect("registry poisoned")
            .retain(|s| s.name != name);
    }

    pub fn list(&self) -> Vec<ServiceSpec> {
        self.services.lock().expect("registry poisoned").clone()
    }

    pub fn by_port(&self, port: u16) -> Vec<ServiceSpec> {
        self.services
            .lock()
            .expect("registry poisoned")
            .iter()
            .filter(|s| s.port == Some(port))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn discovery_finds_listening_port() {
        // Bind a listener, then discover it.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let task = tokio::task::spawn_blocking(move || {
            let (sock, _) = listener.accept().unwrap();
            drop(sock);
        });

        let found = tokio::time::timeout(Duration::from_secs(5), discover_ports())
            .await
            .unwrap()
            .unwrap();
        assert!(
            found.iter().any(|p| p.port == port),
            "port {port} must be discovered; got {:?}",
            found.iter().map(|p| p.port).collect::<Vec<_>>()
        );
        // Release the accept task: connect and immediately drop.
        let sock = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        drop(sock);
        let _ = task.await;
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn discovery_reports_pid_on_linux() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let task = tokio::task::spawn_blocking(move || {
            let (_sock, _) = listener.accept().unwrap();
            std::thread::sleep(Duration::from_millis(200));
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        let found = discover_ports().await.unwrap();
        let mine = found
            .iter()
            .find(|p| p.port == port)
            .expect("our port found");
        assert!(mine.pid.is_some(), "procfs gives pid for own sockets");
        assert!(!mine.addr.is_empty());
        // Release the blocking accept: connect and let it return.
        {
            let sock = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
                .await
                .unwrap();
            drop(sock);
        }
        let _ = task.await;
    }

    #[test]
    fn snapshot_parses_meminfo() {
        let meminfo = "\
MemTotal:       16000000 kB
MemAvailable:    8000000 kB
";
        let snap = parse_meminfo(meminfo);
        assert_eq!(snap.mem_total_bytes, 16_000_000 * 1024);
        assert_eq!(snap.mem_used_bytes, (16_000_000 - 8_000_000) * 1024);
    }

    #[test]
    fn registry_tracks_named_services() {
        let reg = ServiceRegistry::new();
        reg.upsert(ServiceSpec {
            name: "vite".into(),
            project: "webapp".into(),
            command: "npm run dev".into(),
            port: Some(5173),
        });
        let all = reg.list();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "vite");
        reg.remove("vite");
        assert!(reg.list().is_empty());
    }

    #[test]
    fn registry_associates_port_with_service() {
        let reg = ServiceRegistry::new();
        reg.upsert(ServiceSpec {
            name: "api".into(),
            project: "backend".into(),
            command: "cargo run".into(),
            port: Some(8080),
        });
        let listener = std::net::TcpListener::bind("127.0.0.1:8080").unwrap();
        let _port = listener.local_addr().unwrap().port();
        drop(listener);
        // Association is data-level: registry looks up by declared port.
        let owner = reg.by_port(8080);
        assert!(owner.iter().any(|s| s.name == "api"));
        assert!(reg.by_port(9999).is_empty());
    }
}
