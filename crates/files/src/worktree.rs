//! Worktree-per-task isolation (Plan §25): create/list/remove git worktrees
//! under `~/.orbynode/worktrees/<project>/<task-slug>` with `agent/*` branches.

use std::path::{Path, PathBuf};

use super::git::{GitError, GitRepo};

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Worktree {
    pub path: String,
    pub branch: String,
}

/// Slugifies a task title into a filesystem-safe directory name.
pub fn slugify(title: &str) -> String {
    let slug: String = title
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let trimmed = slug.trim_matches('-').to_lowercase();
    let collapsed: Vec<&str> = trimmed.split('-').filter(|s| !s.is_empty()).collect();
    let out = collapsed.join("-");
    if out.is_empty() { "task".to_owned() } else { out.chars().take(48).collect() }
}

/// Manages worktrees for one project repository.
pub struct WorktreeManager {
    repo: GitRepo,
    /// Base dir, usually `~/.orbynode/worktrees/<project>`.
    base: PathBuf,
}

impl WorktreeManager {
    pub fn new(repo: GitRepo, base: impl Into<PathBuf>) -> Self {
        WorktreeManager { repo, base: base.into() }
    }

    /// Create (or reuse) the worktree + branch for a task.
    pub fn ensure(&self, task_title: &str) -> Result<Worktree, GitError> {
        let slug = slugify(task_title);
        let path = self.base.join(&slug);
        let branch = format!("agent/{slug}");
        std::fs::create_dir_all(&self.base).map_err(GitError::Io)?;
        if path.exists() {
            return Ok(Worktree { path: path.to_string_lossy().to_string(), branch });
        }
        // Worktree with a new branch off HEAD.
        let out = std::process::Command::new("git")
            .args(["worktree", "add", "-b", &branch])
            .arg(&path)
            .current_dir(self.repo.root())
            .output()
            .map_err(GitError::Io)?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            // Branch already exists: attach worktree without -b.
            if stderr.contains("already exists") {
                let out2 = std::process::Command::new("git")
                    .args(["worktree", "add"])
                    .arg(&path)
                    .arg(&branch)
                    .current_dir(self.repo.root())
                    .output()
                    .map_err(GitError::Io)?;
                if !out2.status.success() {
                    return Err(GitError::Git(String::from_utf8_lossy(&out2.stderr).trim().to_owned()));
                }
            } else {
                return Err(GitError::Git(stderr.trim().to_owned()));
            }
        }
        Ok(Worktree { path: path.to_string_lossy().to_string(), branch })
    }

    pub fn list(&self) -> Result<Vec<Worktree>, GitError> {
        let out = std::process::Command::new("git")
            .args(["worktree", "list", "--porcelain"])
            .current_dir(self.repo.root())
            .output()
            .map_err(GitError::Io)?;
        if !out.status.success() {
            return Err(GitError::Git(String::from_utf8_lossy(&out.stderr).trim().to_owned()));
        }
        let text = String::from_utf8_lossy(&out.stdout);
        let mut out_vec = Vec::new();
        let mut current_path = String::new();
        for line in text.lines() {
            if let Some(p) = line.strip_prefix("worktree ") {
                current_path = p.to_owned();
            } else if let Some(b) = line.strip_prefix("branch ") {
                let branch = b.trim_start_matches("refs/heads/").to_owned();
                if current_path.starts_with(self.base.to_string_lossy().as_ref()) {
                    out_vec.push(Worktree { path: current_path.clone(), branch });
                }
            }
        }
        Ok(out_vec)
    }

    /// Remove a worktree dir (keeps the branch). Refuses paths outside base.
    pub fn remove(&self, task_title: &str) -> Result<(), GitError> {
        let slug = slugify(task_title);
        let path = self.base.join(&slug);
        if !path.starts_with(&self.base) {
            return Err(GitError::Git("worktree path outside base".to_owned()));
        }
        let out = std::process::Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(&path)
            .current_dir(self.repo.root())
            .output()
            .map_err(GitError::Io)?;
        if !out.status.success() {
            return Err(GitError::Git(String::from_utf8_lossy(&out.stderr).trim().to_owned()));
        }
        Ok(())
    }
}

/// Convenience: base dir per Plan §25 example.
pub fn default_base(home: &Path, project_name: &str) -> PathBuf {
    home.join(".orbynode").join("worktrees").join(project_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_repo(tag: &str) -> (GitRepo, PathBuf) {
        let dir = std::env::temp_dir().join(format!("orbynode-wt-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let run = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .args(args)
                .current_dir(&dir)
                .output()
                .unwrap();
            assert!(out.status.success(), "git {args:?} failed: {}", String::from_utf8_lossy(&out.stderr));
        };
        run(&["init", "-q", "-b", "main"]);
        run(&["config", "user.email", "t@t"]);
        run(&["config", "user.name", "T"]);
        std::fs::write(dir.join("seed.txt"), "seed").unwrap();
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "seed"]);
        (GitRepo::open(dir.clone()), dir)
    }

    #[test]
    fn slugify_titles() {
        assert_eq!(slugify("Add OAuth login!"), "add-oauth-login");
        assert_eq!(slugify("  --weird__title--  "), "weird-title");
        assert_eq!(slugify("///"), "task");
    }

    #[test]
    fn ensure_creates_worktree_and_branch() {
        let (repo, _) = fresh_repo("create");
        let base = std::env::temp_dir().join(format!("orbynode-wt-base-{}", std::process::id()));
        let mgr = WorktreeManager::new(repo, &base);
        let wt = mgr.ensure("Add OAuth").unwrap();
        assert!(Path::new(&wt.path).exists());
        assert_eq!(wt.branch, "agent/add-oauth");
        assert!(wt.path.contains("add-oauth"));
        // Idempotent: ensure again returns the same worktree.
        let wt2 = mgr.ensure("Add OAuth").unwrap();
        assert_eq!(wt.path, wt2.path);
        let listed = mgr.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].branch, "agent/add-oauth");
    }

    #[test]
    fn remove_deletes_worktree_keeps_branch() {
        let (repo, _) = fresh_repo("remove");
        let base = std::env::temp_dir().join(format!("orbynode-wt-rm-{}", std::process::id()));
        let mgr = WorktreeManager::new(repo, &base);
        let wt = mgr.ensure("Fix bug").unwrap();
        assert!(Path::new(&wt.path).exists());
        mgr.remove("Fix bug").unwrap();
        assert!(!Path::new(&wt.path).exists());
        // Branch survives for the merge flow (§25 step 7).
        assert!(mgr.repo.branches().unwrap().iter().any(|b| b == "agent/fix-bug"));
    }

    #[test]
    fn two_agents_can_work_in_isolation() {
        // M9 acceptance: two agents on one repository without sharing a tree.
        let (repo, _) = fresh_repo("iso");
        let base = std::env::temp_dir().join(format!("orbynode-wt-iso-{}", std::process::id()));
        let mgr = WorktreeManager::new(repo, &base);
        let backend = mgr.ensure("backend task").unwrap();
        let frontend = mgr.ensure("frontend task").unwrap();
        assert_ne!(backend.path, frontend.path);
        assert_ne!(backend.branch, frontend.branch);
        std::fs::write(Path::new(&backend.path).join("backend.txt"), "b").unwrap();
        std::fs::write(Path::new(&frontend.path).join("frontend.txt"), "f").unwrap();
        assert!(Path::new(&backend.path).join("backend.txt").exists());
        assert!(!Path::new(&frontend.path).join("backend.txt").exists());
    }
}
