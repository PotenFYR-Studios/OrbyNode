//! OrbyNode integrations — safe install/update/rollback of agent hook
//! configuration (Plan §12): idempotent, backups, preview, no silent
//! overwrite of unknown user config.

// ---------- Public API ----------

/// Which agent an integration targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    Claude,
    Codex,
    Gemini,
    OpenCode,
    Hermes,
}

impl Target {
    /// Settings file the integration modifies, relative to the user home.
    pub fn settings_path(self) -> &'static str {
        match self {
            Target::Claude => ".claude/settings.json",
            Target::Codex => ".codex/config.toml",
            Target::Gemini => ".gemini/settings.json",
            Target::OpenCode => ".config/opencode/opencode.json",
            Target::Hermes => ".hermes/settings.json",
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct IntegrationStatus {
    pub target: Target,
    /// Config file exists on disk.
    pub detected: bool,
    /// OrbyNode hook section present.
    pub installed: bool,
}

#[derive(Debug)]
pub enum IntegrationError {
    Io(String),
    /// Refusing to clobber an unrecognized config shape.
    UnsupportedConfig,
}

impl std::fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegrationError::Io(e) => write!(f, "io error: {e}"),
            IntegrationError::UnsupportedConfig => write!(f, "unsupported config layout"),
        }
    }
}

impl std::error::Error for IntegrationError {}

/// Filesystem root (user home) — injected for testability.
pub trait ConfigStore: Send + Sync {
    fn read(&self, rel: &str) -> std::io::Result<Option<String>>;
    fn write(&self, rel: &str, contents: &str) -> std::io::Result<()>;
    fn backup(&self, rel: &str) -> std::io::Result<String>;
    fn list_backups(&self, rel: &str) -> std::io::Result<Vec<String>>;
    fn restore(&self, backup: &str) -> std::io::Result<()>;
}

/// Real store rooted at a directory (usually the user home).
pub struct FsStore {
    root: std::path::PathBuf,
}

impl FsStore {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        FsStore { root: root.into() }
    }
}

impl ConfigStore for FsStore {
    fn read(&self, rel: &str) -> std::io::Result<Option<String>> {
        match std::fs::read_to_string(self.root.join(rel)) {
            Ok(s) => Ok(Some(s)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn write(&self, rel: &str, contents: &str) -> std::io::Result<()> {
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, contents)
    }

    fn backup(&self, rel: &str) -> std::io::Result<String> {
        let src = self.root.join(rel);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let backup_rel = format!("{rel}.orbynode-bak-{stamp}");
        let dst = self.root.join(&backup_rel);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(src, dst)?;
        Ok(backup_rel)
    }

    fn list_backups(&self, rel: &str) -> std::io::Result<Vec<String>> {
        // Returns full store-relative backup paths (same contract as the
        // in-memory store), newest last.
        let mut out = Vec::new();
        let full = self.root.join(rel);
        let Some(dir) = full.parent() else {
            return Ok(out);
        };
        let base = std::path::Path::new(rel)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let parent_rel = std::path::Path::new(rel)
            .parent()
            .map(|p| p.to_string_lossy().to_string());
        for entry in std::fs::read_dir(dir)? {
            let name = entry?.file_name().to_string_lossy().to_string();
            if name.starts_with(&format!("{base}.orbynode-bak-")) {
                out.push(match &parent_rel {
                    Some(p) if !p.is_empty() => format!("{p}/{name}"),
                    _ => name,
                });
            }
        }
        out.sort();
        Ok(out)
    }

    fn restore(&self, backup: &str) -> std::io::Result<()> {
        // backup path is "<settings-path>.orbynode-bak-<stamp>"; derive the
        // original by stripping the suffix.
        let marker = ".orbynode-bak-";
        let Some(idx) = backup.rfind(marker) else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "not an orbynode backup",
            ));
        };
        let original = &backup[..idx];
        std::fs::copy(self.root.join(backup), self.root.join(original))?;
        Ok(())
    }
}

/// The marker key OrbyNode writes so installs are recognizable and removable.
pub const HOOK_KEY: &str = "orbynodeHook";

// Implemented below: IntegrationManager (status/install/uninstall/rollback).

// ---------- Implementation ----------

/// Manages OrbyNode hook sections inside agent config files (JSON only for
/// M7; TOML targets get JSON-compatible handling when the manifest expands).
pub struct IntegrationManager<S: ConfigStore + 'static> {
    store: std::sync::Arc<S>,
}

impl<S: ConfigStore + 'static> IntegrationManager<S> {
    pub fn new(store: std::sync::Arc<S>) -> Self {
        IntegrationManager { store }
    }

    pub fn status(&self, target: Target) -> Result<IntegrationStatus, IntegrationError> {
        let path = target.settings_path();
        let Some(raw) = self
            .store
            .read(path)
            .map_err(|e| IntegrationError::Io(e.to_string()))?
        else {
            return Ok(IntegrationStatus {
                target,
                detected: false,
                installed: false,
            });
        };
        let installed = parse_json(&raw)
            .map(|v| v.get(HOOK_KEY).is_some())
            .unwrap_or(false);
        Ok(IntegrationStatus {
            target,
            detected: true,
            installed,
        })
    }

    /// Install the OrbyNode hook: backup, merge `orbynodeHook` into the
    /// existing JSON object (or create the file). Idempotent; unknown fields
    /// are preserved verbatim.
    pub fn install(&self, target: Target) -> Result<(), IntegrationError> {
        let path = target.settings_path();
        let existing = self
            .store
            .read(path)
            .map_err(|e| IntegrationError::Io(e.to_string()))?;
        let mut root = match existing {
            None => serde_json::Map::new(),
            Some(raw) => {
                // Backup before any modification of an existing file.
                self.store
                    .backup(path)
                    .map_err(|e| IntegrationError::Io(e.to_string()))?;
                match parse_json(&raw) {
                    Some(v) => v,
                    None => return Err(IntegrationError::UnsupportedConfig),
                }
            }
        };
        if root.get(HOOK_KEY).is_some() {
            return Ok(()); // already installed — idempotent no-op
        }
        root.insert(
            HOOK_KEY.to_owned(),
            serde_json::json!({
                "version": 1,
                "url": "http://127.0.0.1:7676",
            }),
        );
        let pretty = serde_json::to_string_pretty(&serde_json::Value::Object(root))
            .map_err(|e| IntegrationError::Io(e.to_string()))?;
        self.store
            .write(path, &pretty)
            .map_err(|e| IntegrationError::Io(e.to_string()))
    }

    /// Remove only the OrbyNode hook section, preserving everything else.
    pub fn uninstall(&self, target: Target) -> Result<(), IntegrationError> {
        let path = target.settings_path();
        let Some(raw) = self
            .store
            .read(path)
            .map_err(|e| IntegrationError::Io(e.to_string()))?
        else {
            return Ok(());
        };
        let Some(mut obj) = parse_json(&raw) else {
            return Err(IntegrationError::UnsupportedConfig);
        };
        obj.remove(HOOK_KEY);
        let pretty = serde_json::to_string_pretty(&serde_json::Value::Object(obj))
            .map_err(|e| IntegrationError::Io(e.to_string()))?;
        self.store
            .write(path, &pretty)
            .map_err(|e| IntegrationError::Io(e.to_string()))
    }

    /// Restore the most recent pre-install backup.
    pub fn rollback(&self, target: Target) -> Result<(), IntegrationError> {
        let path = target.settings_path();
        let backups = self
            .store
            .list_backups(path)
            .map_err(|e| IntegrationError::Io(e.to_string()))?;
        let Some(latest) = backups.last() else {
            return Ok(()); // nothing to roll back
        };
        self.store
            .restore(latest)
            .map_err(|e| IntegrationError::Io(e.to_string()))
    }
}

fn parse_json(raw: &str) -> Option<serde_json::Map<String, serde_json::Value>> {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()?
        .as_object()
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    struct MemStore {
        files: std::sync::Mutex<BTreeMap<String, String>>,
        next_id: std::sync::atomic::AtomicU64,
    }

    impl MemStore {
        fn new() -> Self {
            MemStore {
                files: std::sync::Mutex::new(BTreeMap::new()),
                next_id: std::sync::atomic::AtomicU64::new(1),
            }
        }
    }

    impl ConfigStore for MemStore {
        fn read(&self, rel: &str) -> std::io::Result<Option<String>> {
            Ok(self.files.lock().unwrap().get(rel).cloned())
        }

        fn write(&self, rel: &str, contents: &str) -> std::io::Result<()> {
            self.files
                .lock()
                .unwrap()
                .insert(rel.to_owned(), contents.to_owned());
            Ok(())
        }

        fn backup(&self, rel: &str) -> std::io::Result<String> {
            let id = self
                .next_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let backup_rel = format!("{rel}.orbynode-bak-{id}");
            let files = self.files.lock().unwrap();
            let content = files.get(rel).cloned().unwrap_or_default();
            drop(files);
            self.files
                .lock()
                .unwrap()
                .insert(backup_rel.clone(), content);
            Ok(backup_rel)
        }

        fn list_backups(&self, rel: &str) -> std::io::Result<Vec<String>> {
            let files = self.files.lock().unwrap();
            Ok(files
                .keys()
                .filter(|k| k.starts_with(&format!("{rel}.orbynode-bak-")))
                .cloned()
                .collect())
        }

        fn restore(&self, backup: &str) -> std::io::Result<()> {
            let idx = backup.rfind(".orbynode-bak-").unwrap();
            let original = backup[..idx].to_owned();
            let content = self.files.lock().unwrap().get(backup).cloned().unwrap();
            self.files.lock().unwrap().insert(original, content);
            Ok(())
        }
    }

    fn manager() -> (IntegrationManager<MemStore>, std::sync::Arc<MemStore>) {
        let store = std::sync::Arc::new(MemStore::new());
        (IntegrationManager::new(clone_store(&store)), store)
    }

    fn clone_store(s: &std::sync::Arc<MemStore>) -> std::sync::Arc<MemStore> {
        s.clone()
    }

    #[test]
    fn status_reports_detected_and_installed() {
        let (mgr, store) = manager();
        let path = Target::Claude.settings_path();
        // nothing on disk
        assert_eq!(
            mgr.status(Target::Claude).unwrap(),
            IntegrationStatus {
                target: Target::Claude,
                detected: false,
                installed: false
            }
        );
        // user config exists, no hook
        store
            .files
            .lock()
            .unwrap()
            .insert(path.to_owned(), "{\"model\":\"opus\"}".into());
        assert_eq!(
            mgr.status(Target::Claude).unwrap(),
            IntegrationStatus {
                target: Target::Claude,
                detected: true,
                installed: false
            }
        );
        // hook installed
        store.files.lock().unwrap().insert(
            path.to_owned(),
            format!(r#"{{"model":"opus","{HOOK_KEY}":{{"url":"http://127.0.0.1:7676"}}}}"#),
        );
        assert_eq!(
            mgr.status(Target::Claude).unwrap(),
            IntegrationStatus {
                target: Target::Claude,
                detected: true,
                installed: true
            }
        );
    }

    #[test]
    fn install_is_idempotent_and_preserves_unknown_fields() {
        let (mgr, store) = manager();
        let path = Target::Claude.settings_path();
        store.files.lock().unwrap().insert(
            path.to_owned(),
            r#"{"model":"opus","custom":{"nested":1}}"#.into(),
        );

        mgr.install(Target::Claude).unwrap();
        let after1 = store.read(path).unwrap().unwrap();
        mgr.install(Target::Claude).unwrap();
        let after2 = store.read(path).unwrap().unwrap();
        assert_eq!(after1, after2, "second install is a no-op");

        let v: serde_json::Value = serde_json::from_str(&after1).unwrap();
        assert_eq!(v["model"], "opus", "user fields preserved");
        assert_eq!(v["custom"]["nested"], 1, "nested unknown fields preserved");
        assert!(v.get(HOOK_KEY).is_some(), "hook written");
    }

    #[test]
    fn install_creates_backup_and_rollback_restores() {
        let (mgr, store) = manager();
        let path = Target::Claude.settings_path();
        store
            .files
            .lock()
            .unwrap()
            .insert(path.to_owned(), r#"{"model":"opus"}"#.into());

        mgr.install(Target::Claude).unwrap();
        assert!(
            !store.list_backups(path).unwrap().is_empty(),
            "backup taken"
        );

        mgr.rollback(Target::Claude).unwrap();
        let restored = store.read(path).unwrap().unwrap();
        assert_eq!(restored, r#"{"model":"opus"}"#, "original restored");
        let v: serde_json::Value = serde_json::from_str(&restored).unwrap();
        assert!(v.get(HOOK_KEY).is_none());
    }

    #[test]
    fn uninstall_removes_only_orbynode_section() {
        let (mgr, store) = manager();
        let path = Target::Gemini.settings_path();
        store
            .files
            .lock()
            .unwrap()
            .insert(path.to_owned(), r#"{"theme":"dark"}"#.into());
        mgr.install(Target::Gemini).unwrap();
        mgr.uninstall(Target::Gemini).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&store.read(path).unwrap().unwrap()).unwrap();
        assert_eq!(v["theme"], "dark");
        assert!(v.get(HOOK_KEY).is_none(), "hook removed");
    }

    #[test]
    fn install_creates_config_when_missing() {
        let (mgr, store) = manager();
        mgr.install(Target::Hermes).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&store.read(Target::Hermes.settings_path()).unwrap().unwrap())
                .unwrap();
        assert!(v.get(HOOK_KEY).is_some());
    }
}
