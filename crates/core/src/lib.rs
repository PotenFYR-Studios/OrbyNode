//! OrbyNode core — shared types: version, config, paths.
//!
//! Kept intentionally small. Do not let this become a dumping ground (Plan §120).

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

pub const NAME: &str = "OrbyNode";

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Daemon configuration.
///
/// Secure defaults per Plan §16/§17: bind localhost only; `0.0.0.0` is opt-in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub bind: SocketAddr,
    pub data_dir: PathBuf,
    /// Serve the web UI from disk instead of the embedded assets (development).
    pub static_dir: Option<PathBuf>,
    /// Emit JSON logs instead of human-readable ones.
    pub log_json: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            bind: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 7676),
            data_dir: default_data_dir(),
            static_dir: None,
            log_json: false,
        }
    }
}

impl Config {
    /// Load configuration from `ORBYNODE_*` environment variables over secure defaults.
    pub fn from_env() -> Self {
        let mut cfg = Config::default();
        if let Some(bind) = env_parse::<SocketAddr>("ORBYNODE_BIND") {
            cfg.bind = bind;
        } else if let Some(port) = env_parse::<u16>("ORBYNODE_PORT") {
            cfg.bind.set_port(port);
        }
        if let Some(dir) = env_path("ORBYNODE_DATA_DIR") {
            cfg.data_dir = dir;
        }
        if let Some(dir) = env_path("ORBYNODE_STATIC_DIR") {
            cfg.static_dir = Some(dir);
        }
        if std::env::var("ORBYNODE_LOG_FORMAT").as_deref() == Ok("json") {
            cfg.log_json = true;
        }
        cfg
    }
}

fn env_parse<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::var(key).ok().and_then(|v| v.parse().ok())
}

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key).map(PathBuf::from)
}

/// SQLite URL for the daemon database, under the data dir.
pub fn db_url(data_dir: &std::path::Path) -> String {
    format!(
        "sqlite://{}?mode=rwc",
        data_dir.join("orbynode.db").display()
    )
}

/// `~/.orbynode` — falls back to a local dir when HOME is unknown (rare).
pub fn default_data_dir() -> PathBuf {
    home_dir()
        .map(|h| h.join(".orbynode"))
        .unwrap_or_else(|| PathBuf::from(".orbynode"))
}

fn home_dir() -> Option<PathBuf> {
    // ponytail: HOME/USERPROFILE is enough for M0; switch to the `dirs` crate if
    // platform-specific special dirs (XDG, macOS, known folders) become necessary.
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_localhost_only() {
        let cfg = Config::default();
        assert_eq!(cfg.bind.ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(cfg.bind.port(), 7676);
        assert!(cfg.static_dir.is_none());
        assert!(!cfg.log_json);
    }

    #[test]
    fn version_is_set() {
        assert!(!version().is_empty());
    }
}
