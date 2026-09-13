//! OrbyNode files — project-root-enforced file access (Plan §23).
//!
//! Security invariants (§23, §16): every path is canonicalized and must stay
//! inside the authorized project root; symlinks that escape the root are
//! rejected; no `..` traversal.

pub mod git;

// ---------- Public API ----------

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug)]
pub enum FilesError {
    /// Path escapes the project root (traversal or symlink).
    OutsideRoot,
    NotFound,
    Io(std::io::Error),
}

impl std::fmt::Display for FilesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilesError::OutsideRoot => write!(f, "path outside project root"),
            FilesError::NotFound => write!(f, "not found"),
            FilesError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for FilesError {}

// Implemented below: Workspace (root-enforced list/read/write/delete).

// ---------- Implementation ----------

/// Root-enforced file access over one authorized directory (Plan §23).
#[derive(Clone)]
pub struct Workspace {
    root: std::path::PathBuf,
}

impl Workspace {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Workspace { root: root.into() }
    }

    /// Canonicalize `rel` under the root, rejecting traversal and symlink
    /// escapes. Fail-closed: any canonicalization error is OutsideRoot/NotFound.
    fn resolve(&self, rel: &str) -> Result<std::path::PathBuf, FilesError> {
        // Reads/deletes: reject dot-dot outright — containment of a
        // non-existent traversal target is unknowable, so fail closed.
        if rel.split('/').any(|seg| seg == "..") {
            return Err(FilesError::OutsideRoot);
        }
        let candidate = self.root.join(rel);
        let canonical = match candidate.canonicalize() {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // For writes: resolve the deepest existing ancestor.
                return Err(FilesError::NotFound);
            }
            Err(e) => return Err(FilesError::Io(e)),
        };
        let root_canonical = self.root.canonicalize().map_err(FilesError::Io)?;
        if !canonical.starts_with(&root_canonical) {
            return Err(FilesError::OutsideRoot);
        }
        Ok(canonical)
    }

    /// Resolve for creation paths (file may not exist yet): canonicalize the
    /// nearest existing ancestor, then append the remaining segments.
    fn resolve_create(&self, rel: &str) -> Result<std::path::PathBuf, FilesError> {
        if rel.split('/').any(|seg| seg == "..") {
            return Err(FilesError::OutsideRoot);
        }
        let rel_path = std::path::Path::new(rel);
        let mut ancestor = rel_path.to_path_buf();
        let mut suffix: Vec<std::ffi::OsString> = Vec::new();
        loop {
            match self.root.join(&ancestor).canonicalize() {
                Ok(existing) => {
                    let root_canonical = self.root.canonicalize().map_err(FilesError::Io)?;
                    if !existing.starts_with(&root_canonical) {
                        return Err(FilesError::OutsideRoot);
                    }
                    let mut resolved = existing;
                    for seg in suffix.iter().rev() {
                        resolved.push(seg);
                    }
                    if !resolved.starts_with(&root_canonical) {
                        return Err(FilesError::OutsideRoot);
                    }
                    return Ok(resolved);
                }
                Err(_) => match ancestor.file_name().map(|n| n.to_os_string()) {
                    Some(name) => {
                        suffix.push(name);
                        if !ancestor.pop() {
                            return Err(FilesError::OutsideRoot);
                        }
                    }
                    None => return Err(FilesError::OutsideRoot),
                },
            }
        }
    }

    pub fn list(&self, rel: &str) -> Result<Vec<FileEntry>, FilesError> {
        let dir = self.resolve(rel)?;
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&dir).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FilesError::NotFound
            } else {
                FilesError::Io(e)
            }
        })? {
            let entry = entry.map_err(FilesError::Io)?;
            let meta = entry.metadata().map_err(FilesError::Io)?;
            out.push(FileEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                is_dir: meta.is_dir(),
                size: meta.len(),
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    pub fn read(&self, rel: &str) -> Result<Vec<u8>, FilesError> {
        let path = self.resolve(rel)?;
        std::fs::read(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FilesError::NotFound
            } else {
                FilesError::Io(e)
            }
        })
    }

    pub fn read_binary(&self, rel: &str) -> Result<Vec<u8>, FilesError> {
        self.read(rel)
    }

    pub fn write(&self, rel: &str, contents: &str) -> Result<(), FilesError> {
        self.write_binary(rel, contents.as_bytes())
    }

    pub fn write_binary(&self, rel: &str, contents: &[u8]) -> Result<(), FilesError> {
        let path = self.resolve_create(rel)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(FilesError::Io)?;
        }
        std::fs::write(&path, contents).map_err(FilesError::Io)
    }

    pub fn delete(&self, rel: &str) -> Result<(), FilesError> {
        let path = self.resolve(rel)?;
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(FilesError::NotFound),
            Err(e) => {
                // Directory?
                std::fs::remove_dir(&path).map_err(|_| FilesError::Io(e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("orbynode-files-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn ws(tag: &str) -> (Workspace, PathBuf) {
        let root = tmp_root(tag);
        (Workspace::new(root.clone()), root)
    }

    #[test]
    fn lists_directory_entries() {
        let (w, root) = ws("list");
        std::fs::write(root.join("a.txt"), "hello").unwrap();
        std::fs::create_dir_all(root.join("sub")).unwrap();
        let entries = w.list(".").unwrap();
        let names: Vec<String> = entries.into_iter().map(|e| e.name).collect();
        assert!(names.contains(&"a.txt".to_owned()));
        assert!(names.contains(&"sub".to_owned()));
    }

    #[test]
    fn traversal_is_rejected() {
        let (w, root) = ws("traverse");
        std::fs::write(root.join("safe.txt"), "x").unwrap();
        assert!(matches!(
            w.read("../secret.txt"),
            Err(FilesError::OutsideRoot)
        ));
        assert!(matches!(
            w.read("a/../../secret"),
            Err(FilesError::OutsideRoot)
        ));
        w.write("ok.txt", "fine").unwrap();
        assert!(matches!(
            w.write("../evil.txt", "x"),
            Err(FilesError::OutsideRoot)
        ));
    }

    #[test]
    fn symlink_escape_is_rejected() {
        let (w, root) = ws("symlink");
        let outside = tmp_root("symlink-outside");
        std::fs::write(outside.join("secret.txt"), "top secret").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, root.join("linked")).unwrap();
        match w.read("linked/secret.txt") {
            Err(FilesError::OutsideRoot) => {}
            other => panic!("expected OutsideRoot, got {other:?}"),
        }
    }

    #[test]
    fn read_write_delete_roundtrip() {
        let (w, root) = ws("rw");
        w.write("docs/readme.md", "# Hi").unwrap();
        assert_eq!(w.read("docs/readme.md").unwrap(), b"# Hi");
        assert!(root.join("docs/readme.md").exists());
        w.delete("docs/readme.md").unwrap();
        assert!(!root.join("docs/readme.md").exists());
        assert!(matches!(
            w.read("docs/readme.md"),
            Err(FilesError::NotFound)
        ));
    }

    #[test]
    fn binary_files_roundtrip() {
        let (w, _) = ws("bin");
        let data: Vec<u8> = (0..=255u8).collect();
        w.write_binary("blob.bin", &data).unwrap();
        assert_eq!(w.read_binary("blob.bin").unwrap(), data);
    }
}
