use std::path::{Path, PathBuf};

use super::command::GitCommand;
use super::error::GitError;

/// Parsed entry from `git worktree list --porcelain`
#[derive(Debug, Clone)]
pub struct WorktreeEntry {
    pub path: PathBuf,
    pub head: String,
    pub branch: Option<String>,
    pub is_bare: bool,
    pub is_locked: bool,
    pub lock_reason: Option<String>,
}

/// A git repository handle providing typed methods for git operations.
/// All operations shell out to `git -C {path}` with `LC_ALL=C`.
#[derive(Debug, Clone)]
pub struct GitRepo {
    path: PathBuf,
}

impl GitRepo {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn git(&self) -> GitCommand<'_> {
        GitCommand::new(&self.path)
    }

    // --- Repository checks ---

    /// Check if the path is a git repository (has .git directory or file)
    pub fn is_git_repo(&self) -> bool {
        self.path.join(".git").exists()
    }

    /// Get the default branch name (main or master)
    pub fn default_branch(&self) -> Result<String, GitError> {
        // Try refs/remotes/origin/HEAD first
        let output = self
            .git()
            .args(&["symbolic-ref", "refs/remotes/origin/HEAD", "--short"])
            .run_unchecked()?;

        if output.exit_code == 0 {
            let branch = output.stdout.trim().to_string();
            // Strip "origin/" prefix if present
            return Ok(branch
                .strip_prefix("origin/")
                .unwrap_or(&branch)
                .to_string());
        }

        // Fallback: check if main or master exists
        for name in &["main", "master"] {
            let output = self
                .git()
                .args(&["rev-parse", "--verify", name])
                .run_unchecked()?;
            if output.exit_code == 0 {
                return Ok(name.to_string());
            }
        }

        Err(GitError::CommandFailed {
            command: "detect default branch".to_string(),
            stderr: "Could not determine default branch. No origin/HEAD, main, or master found."
                .to_string(),
        })
    }

    // --- Status ---

    /// Check if the working tree has uncommitted changes
    pub fn is_dirty(&self) -> Result<bool, GitError> {
        let output = self.git().args(&["status", "--porcelain"]).run()?;
        Ok(!output.stdout.trim().is_empty())
    }

    /// Count the number of changed files (staged + unstaged + untracked).
    pub fn change_count(&self) -> Result<usize, GitError> {
        let output = self.git().args(&["status", "--porcelain"]).run()?;
        Ok(output.stdout.trim().lines().count())
    }

    /// Get the current branch name (empty string if detached HEAD)
    pub fn current_branch(&self) -> Result<String, GitError> {
        let output = self.git().args(&["branch", "--show-current"]).run()?;
        Ok(output.stdout.trim().to_string())
    }

    /// Get ahead/behind counts relative to a base branch
    pub fn ahead_behind(&self, base: &str) -> Result<(u32, u32), GitError> {
        let output = self
            .git()
            .args(&[
                "rev-list",
                "--left-right",
                "--count",
                &format!("{base}...HEAD"),
            ])
            .run_unchecked()?;

        if output.exit_code != 0 {
            return Ok((0, 0)); // Can't compare, return zero
        }

        let parts: Vec<&str> = output.stdout.trim().split('\t').collect();
        if parts.len() == 2 {
            let behind = parts[0].parse().unwrap_or(0);
            let ahead = parts[1].parse().unwrap_or(0);
            Ok((ahead, behind))
        } else {
            Ok((0, 0))
        }
    }

    // --- Worktree operations ---

    /// Add a new worktree
    pub fn worktree_add(&self, path: &Path, branch: &str, base: &str) -> Result<(), GitError> {
        self.git()
            .args(&[
                "worktree",
                "add",
                &path.to_string_lossy(),
                "-b",
                branch,
                base,
            ])
            .run()?;
        Ok(())
    }

    /// Remove a worktree
    pub fn worktree_remove(&self, path: &Path, force: bool) -> Result<(), GitError> {
        let mut cmd = self.git().args(&["worktree", "remove"]);
        if force {
            cmd = cmd.arg("--force");
        }
        cmd.arg(&path.to_string_lossy()).run()?;
        Ok(())
    }

    /// Lock a worktree with a reason
    pub fn worktree_lock(&self, path: &Path, reason: &str) -> Result<(), GitError> {
        self.git()
            .args(&[
                "worktree",
                "lock",
                &path.to_string_lossy(),
                "--reason",
                reason,
            ])
            .run()?;
        Ok(())
    }

    /// Unlock a worktree
    pub fn worktree_unlock(&self, path: &Path) -> Result<(), GitError> {
        self.git()
            .args(&["worktree", "unlock", &path.to_string_lossy()])
            .run()?;
        Ok(())
    }

    /// Prune stale worktree entries
    pub fn worktree_prune(&self) -> Result<(), GitError> {
        self.git().args(&["worktree", "prune"]).run()?;
        Ok(())
    }

    /// List worktrees using `--porcelain` format for reliable parsing
    pub fn worktree_list(&self) -> Result<Vec<WorktreeEntry>, GitError> {
        let output = self
            .git()
            .args(&["worktree", "list", "--porcelain"])
            .run()?;

        Ok(parse_worktree_porcelain(&output.stdout))
    }

    // --- Branch operations ---

    /// Delete a local branch
    pub fn branch_delete(&self, name: &str, force: bool) -> Result<(), GitError> {
        let flag = if force { "-D" } else { "-d" };
        self.git().args(&["branch", flag, name]).run()?;
        Ok(())
    }

    /// Check if a local branch exists
    pub fn branch_exists(&self, name: &str) -> Result<bool, GitError> {
        let output = self.git().args(&["branch", "--list", name]).run()?;
        Ok(!output.stdout.trim().is_empty())
    }

    // --- Remote operations ---

    /// Push a branch and set up tracking
    pub fn push_tracking(&self, branch: &str) -> Result<(), GitError> {
        self.git().args(&["push", "-u", "origin", branch]).run()?;
        Ok(())
    }

    /// Fetch from origin
    pub fn fetch(&self) -> Result<(), GitError> {
        self.git().args(&["fetch", "origin"]).run()?;
        Ok(())
    }

    /// Get the remote URL for origin
    pub fn remote_url(&self) -> Result<Option<String>, GitError> {
        let output = self
            .git()
            .args(&["remote", "get-url", "origin"])
            .run_unchecked()?;

        if output.exit_code == 0 {
            Ok(Some(output.stdout.trim().to_string()))
        } else {
            Ok(None)
        }
    }
}

/// Clone a repository (no GitRepo context needed)
pub fn clone_repo(url: &str, target: &Path) -> Result<(), GitError> {
    super::command::git_global(&["clone", url, &target.to_string_lossy()])?;
    Ok(())
}

/// Check git version and return it. Errors if < minimum.
pub fn check_git_version() -> Result<String, GitError> {
    let output = super::command::git_global(&["--version"])?;
    let version_str = output.stdout.trim();

    // Parse "git version 2.43.0" format
    let version = version_str
        .strip_prefix("git version ")
        .unwrap_or(version_str);

    let parts: Vec<u32> = version.split('.').filter_map(|p| p.parse().ok()).collect();

    let (major, minor) = match parts.as_slice() {
        [major, minor, ..] => (*major, *minor),
        [major] => (*major, 0),
        _ => {
            return Err(GitError::CommandFailed {
                command: "git --version".to_string(),
                stderr: format!("Could not parse git version: {version}"),
            });
        }
    };

    if major < 2 || (major == 2 && minor < 22) {
        return Err(GitError::VersionTooOld {
            found: version.to_string(),
            required: "2.22".to_string(),
        });
    }

    Ok(version.to_string())
}

/// Parse `git worktree list --porcelain` output into structured entries.
///
/// Porcelain format example:
/// ```text
/// worktree /path/to/main
/// HEAD abc1234
/// branch refs/heads/main
///
/// worktree /path/to/feature
/// HEAD def5678
/// branch refs/heads/feature
/// locked reason: loom:my-workspace
/// ```
fn parse_worktree_porcelain(output: &str) -> Vec<WorktreeEntry> {
    let mut entries = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_head = String::new();
    let mut current_branch: Option<String> = None;
    let mut is_bare = false;
    let mut is_locked = false;
    let mut lock_reason: Option<String> = None;

    for line in output.lines() {
        if line.is_empty() {
            // End of entry
            if let Some(path) = current_path.take() {
                entries.push(WorktreeEntry {
                    path,
                    head: std::mem::take(&mut current_head),
                    branch: current_branch.take(),
                    is_bare,
                    is_locked,
                    lock_reason: lock_reason.take(),
                });
            }
            is_bare = false;
            is_locked = false;
        } else if let Some(path) = line.strip_prefix("worktree ") {
            current_path = Some(PathBuf::from(path));
        } else if let Some(head) = line.strip_prefix("HEAD ") {
            current_head = head.to_string();
        } else if let Some(branch) = line.strip_prefix("branch ") {
            // Strip refs/heads/ prefix
            current_branch = Some(
                branch
                    .strip_prefix("refs/heads/")
                    .unwrap_or(branch)
                    .to_string(),
            );
        } else if line == "bare" {
            is_bare = true;
        } else if line == "locked" {
            is_locked = true;
        } else if let Some(reason) = line.strip_prefix("locked ") {
            is_locked = true;
            lock_reason = Some(reason.to_string());
        }
    }

    // Handle last entry (if output doesn't end with blank line)
    if let Some(path) = current_path.take() {
        entries.push(WorktreeEntry {
            path,
            head: current_head,
            branch: current_branch,
            is_bare,
            is_locked,
            lock_reason,
        });
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_worktree_porcelain_basic() {
        let output = "\
worktree /Users/dev/code/repo
HEAD abc123def456
branch refs/heads/main

worktree /Users/dev/loom/ws/repo
HEAD def789abc012
branch refs/heads/loom/my-feature

";
        let entries = parse_worktree_porcelain(output);
        assert_eq!(entries.len(), 2);

        assert_eq!(entries[0].path, PathBuf::from("/Users/dev/code/repo"));
        assert_eq!(entries[0].head, "abc123def456");
        assert_eq!(entries[0].branch.as_deref(), Some("main"));
        assert!(!entries[0].is_bare);
        assert!(!entries[0].is_locked);

        assert_eq!(entries[1].path, PathBuf::from("/Users/dev/loom/ws/repo"));
        assert_eq!(entries[1].branch.as_deref(), Some("loom/my-feature"));
    }

    #[test]
    fn test_parse_worktree_porcelain_locked() {
        let output = "\
worktree /Users/dev/loom/ws/repo
HEAD def789
branch refs/heads/loom/feature
locked loom:my-workspace

";
        let entries = parse_worktree_porcelain(output);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_locked);
        assert_eq!(entries[0].lock_reason.as_deref(), Some("loom:my-workspace"));
    }

    #[test]
    fn test_parse_worktree_porcelain_bare() {
        let output = "\
worktree /Users/dev/code/repo.git
HEAD abc123
bare

";
        let entries = parse_worktree_porcelain(output);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_bare);
        assert!(entries[0].branch.is_none());
    }

    #[test]
    fn test_parse_worktree_porcelain_no_trailing_newline() {
        let output = "\
worktree /path/to/repo
HEAD abc123
branch refs/heads/main";

        let entries = parse_worktree_porcelain(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].branch.as_deref(), Some("main"));
    }

    #[test]
    fn test_check_git_version() {
        // This test requires git to be installed
        let result = check_git_version();
        assert!(result.is_ok(), "git should be installed: {result:?}");
        let version = result.unwrap();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_git_repo_is_git_repo() {
        let dir = tempfile::tempdir().unwrap();

        // Not a git repo
        let repo = GitRepo::new(dir.path());
        assert!(!repo.is_git_repo());

        // Init it
        std::process::Command::new("git")
            .args(["init", &dir.path().to_string_lossy()])
            .env("LC_ALL", "C")
            .output()
            .unwrap();

        assert!(repo.is_git_repo());
    }

    #[test]
    fn test_git_repo_current_branch() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        // Init repo with initial commit
        std::process::Command::new("git")
            .args(["init", "-b", "main", &path.to_string_lossy()])
            .env("LC_ALL", "C")
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args([
                "-C",
                &path.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .env("LC_ALL", "C")
            .output()
            .unwrap();

        let repo = GitRepo::new(path);
        let branch = repo.current_branch().unwrap();
        assert_eq!(branch, "main");
    }

    #[test]
    fn test_git_repo_is_dirty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        std::process::Command::new("git")
            .args(["init", "-b", "main", &path.to_string_lossy()])
            .env("LC_ALL", "C")
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args([
                "-C",
                &path.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .env("LC_ALL", "C")
            .output()
            .unwrap();

        let repo = GitRepo::new(path);

        // Clean state
        assert!(!repo.is_dirty().unwrap());

        // Create untracked file
        std::fs::write(path.join("test.txt"), "hello").unwrap();
        assert!(repo.is_dirty().unwrap());
    }
}
