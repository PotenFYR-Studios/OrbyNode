//! OrbyNode git — CLI-based Git integration (Plan §6: "CLI-based Git
//! integration initially"). One git invocation per operation; the project
//! watcher (M10+) fans status out (§83).

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct GitStatus {
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub changed: Vec<ChangedFile>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ChangedFile {
    pub path: String,
    /// XY porcelain status: M/A/D/R/U/?/…
    pub status: String,
    pub staged: bool,
}

#[derive(Debug)]
pub enum GitError {
    NotARepository,
    Git(String),
    Io(std::io::Error),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::NotARepository => write!(f, "not a git repository"),
            GitError::Git(e) => write!(f, "git error: {e}"),
            GitError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for GitError {}

/// Runs git against one repository working tree.
#[derive(Clone)]
pub struct GitRepo {
    root: PathBuf,
}

impl GitRepo {
    pub fn open(root: impl Into<PathBuf>) -> Self {
        GitRepo { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn run(&self, args: &[&str]) -> Result<String, GitError> {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .output()
            .map_err(GitError::Io)?;
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            if stderr.contains("not a git repository") {
                return Err(GitError::NotARepository);
            }
            return Err(GitError::Git(stderr.trim().to_owned()));
        }
        Ok(stdout)
    }

    pub fn status(&self) -> Result<GitStatus, GitError> {
        let branch = match self.run(&["rev-parse", "--abbrev-ref", "HEAD"]) {
            Ok(b) => b.trim().to_owned(),
            Err(GitError::Git(e)) if e.contains("ambiguous argument") => {
                // Unborn branch (no commits yet).
                self.run(&["symbolic-ref", "--short", "HEAD"])?
                    .trim()
                    .to_owned()
            }
            Err(e) => return Err(e),
        };
        let (ahead, behind) = self.ahead_behind()?;
        let porcelain = self.run(&["status", "--porcelain"])?;
        let changed = porcelain
            .lines()
            .filter(|l| l.len() >= 3)
            .map(|line| {
                let x = line.as_bytes()[0] as char;
                let y = line.as_bytes()[1] as char;
                ChangedFile {
                    path: line[3..].to_owned(),
                    status: if x == '?' {
                        "??".to_owned()
                    } else {
                        format!("{x}{y}")
                    },
                    staged: x != ' ' && x != '?',
                }
            })
            .collect();
        Ok(GitStatus {
            branch,
            ahead,
            behind,
            changed,
        })
    }

    fn ahead_behind(&self) -> Result<(u32, u32), GitError> {
        let upstream = self
            .run(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
            .ok();
        let Some(upstream) = upstream else {
            return Ok((0, 0));
        };
        let out = self.run(&[
            "rev-list",
            "--left-right",
            "--count",
            &format!("{upstream}...HEAD"),
        ])?;
        let mut parts = out.split_whitespace();
        let behind = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
        let ahead = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
        Ok((ahead, behind))
    }

    pub fn stage(&self, paths: &[&str]) -> Result<(), GitError> {
        let mut args = vec!["add", "--"];
        args.extend_from_slice(paths);
        self.run(&args).map(|_| ())
    }

    pub fn commit(&self, message: &str) -> Result<(), GitError> {
        self.run(&["commit", "-m", message]).map(|_| ())
    }

    /// Diff of the working tree (unstaged + staged), bounded by caller.
    pub fn diff(&self) -> Result<String, GitError> {
        self.run(&["diff", "HEAD"])
    }

    pub fn branches(&self) -> Result<Vec<String>, GitError> {
        Ok(self
            .run(&["branch", "--format=%(refname:short)"])?
            .lines()
            .map(str::to_owned)
            .collect())
    }

    pub fn create_branch(&self, name: &str) -> Result<(), GitError> {
        self.run(&["branch", name]).map(|_| ())
    }

    pub fn switch(&self, name: &str) -> Result<(), GitError> {
        self.run(&["switch", name]).map(|_| ())
    }

    pub fn log(&self, count: u32) -> Result<Vec<String>, GitError> {
        Ok(self
            .run(&["log", &format!("-{count}"), "--format=%h %an %s"])?
            .lines()
            .map(str::to_owned)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_repo(tag: &str) -> GitRepo {
        let dir = std::env::temp_dir().join(format!("orbynode-git-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let run = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(&dir)
                .output()
                .unwrap()
        };
        run(&["init", "-q", "-b", "main"]);
        run(&["config", "user.email", "test@orbynode.local"]);
        run(&["config", "user.name", "OrbyNode Test"]);
        GitRepo::open(dir)
    }

    #[test]
    fn status_reports_branch_and_changes() {
        let repo = fresh_repo("status");
        std::fs::write(repo.root().join("a.txt"), "hello").unwrap();
        repo.stage(&["a.txt"]).unwrap();
        repo.commit("init").unwrap();
        let st = repo.status().unwrap();
        assert_eq!(st.branch, "main");
        assert!(st.changed.is_empty(), "clean tree after commit");
        // untracked file appears
        std::fs::write(repo.root().join("b.txt"), "x").unwrap();
        let st = repo.status().unwrap();
        assert!(
            st.changed
                .iter()
                .any(|f| f.path == "b.txt" && f.status == "??")
        );
        // staged file shows staged=true
        repo.stage(&["b.txt"]).unwrap();
        let st = repo.status().unwrap();
        assert!(st.changed.iter().any(|f| f.path == "b.txt" && f.staged));
    }

    #[test]
    fn commit_clears_staged_changes() {
        let repo = fresh_repo("commit");
        std::fs::write(repo.root().join("f.txt"), "content").unwrap();
        repo.stage(&["f.txt"]).unwrap();
        repo.commit("add f.txt").unwrap();
        let st = repo.status().unwrap();
        assert!(st.changed.is_empty(), "clean after commit");
        let log = repo.log(5).unwrap();
        assert!(log.iter().any(|l| l.contains("add f.txt")));
    }

    #[test]
    fn branch_create_and_switch() {
        let repo = fresh_repo("branch");
        std::fs::write(repo.root().join("x"), "x").unwrap();
        repo.stage(&["x"]).unwrap();
        repo.commit("init").unwrap();
        repo.create_branch("feature/orby").unwrap();
        repo.switch("feature/orby").unwrap();
        assert_eq!(repo.status().unwrap().branch, "feature/orby");
        assert!(
            repo.branches()
                .unwrap()
                .contains(&"feature/orby".to_owned())
        );
    }

    #[test]
    fn diff_shows_working_changes() {
        let repo = fresh_repo("diff");
        std::fs::write(repo.root().join("d.txt"), "v1\n").unwrap();
        repo.stage(&["d.txt"]).unwrap();
        repo.commit("v1").unwrap();
        std::fs::write(repo.root().join("d.txt"), "v2\n").unwrap();
        let diff = repo.diff().unwrap();
        assert!(diff.contains("v2"), "diff contains new content");
    }

    #[test]
    fn non_repository_is_detected() {
        let dir = std::env::temp_dir().join(format!("orbynode-git-norepo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = GitRepo::open(&dir);
        assert!(matches!(repo.status(), Err(GitError::NotARepository)));
    }
}
