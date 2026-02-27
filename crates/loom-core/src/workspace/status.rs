use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::git::GitRepo;
use crate::manifest::WorkspaceManifest;

/// Detailed status for a single repo in a workspace.
#[derive(Debug)]
pub struct RepoStatus {
    pub name: String,
    pub worktree_path: PathBuf,
    pub branch: String,
    pub is_dirty: bool,
    pub change_count: usize,
    pub ahead: u32,
    pub behind: u32,
    pub exists: bool,
}

/// Status of an entire workspace.
#[derive(Debug)]
pub struct WorkspaceStatus {
    pub name: String,
    pub path: PathBuf,
    pub base_branch: Option<String>,
    pub repos: Vec<RepoStatus>,
}

/// Get detailed status for a workspace.
///
/// If `fetch` is true, runs `git fetch origin` per repo before computing ahead/behind.
pub fn workspace_status(
    manifest: &WorkspaceManifest,
    ws_path: &Path,
    fetch: bool,
) -> Result<WorkspaceStatus> {
    let mut repos = Vec::new();

    for repo_entry in &manifest.repos {
        let status = if repo_entry.worktree_path.exists() {
            let git = GitRepo::new(&repo_entry.worktree_path);

            // Optional fetch
            if fetch {
                git.fetch().ok(); // Best-effort, don't fail the whole status
            }

            let is_dirty = git.is_dirty().unwrap_or(false);
            let change_count = git.change_count().unwrap_or(0);
            let branch = git
                .current_branch()
                .unwrap_or_else(|_| repo_entry.branch.clone());

            // Compute ahead/behind against the base branch
            let base = manifest.base_branch.as_deref().unwrap_or("main");
            let (ahead, behind) = git.ahead_behind(base).unwrap_or((0, 0));

            RepoStatus {
                name: repo_entry.name.clone(),
                worktree_path: repo_entry.worktree_path.clone(),
                branch,
                is_dirty,
                change_count,
                ahead,
                behind,
                exists: true,
            }
        } else {
            // Worktree path doesn't exist — orphaned entry
            RepoStatus {
                name: repo_entry.name.clone(),
                worktree_path: repo_entry.worktree_path.clone(),
                branch: repo_entry.branch.clone(),
                is_dirty: false,
                change_count: 0,
                ahead: 0,
                behind: 0,
                exists: false,
            }
        };

        repos.push(status);
    }

    Ok(WorkspaceStatus {
        name: manifest.name.clone(),
        path: ws_path.to_path_buf(),
        base_branch: manifest.base_branch.clone(),
        repos,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RepoManifestEntry;

    fn create_git_repo(path: &std::path::Path) {
        std::fs::create_dir_all(path).unwrap();
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
    }

    #[test]
    fn test_status_clean_repo() {
        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path().join("my-repo");
        create_git_repo(&repo_path);

        let manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
            created: chrono::Utc::now(),
            base_branch: None,
            repos: vec![RepoManifestEntry {
                name: "my-repo".to_string(),
                original_path: repo_path.clone(),
                worktree_path: repo_path,
                branch: "loom/test-ws".to_string(),
                remote_url: String::new(),
            }],
        };

        let ws_path = dir.path().join("ws");
        let status = workspace_status(&manifest, &ws_path, false).unwrap();
        assert_eq!(status.repos.len(), 1);
        assert!(status.repos[0].exists);
        assert!(!status.repos[0].is_dirty);
        assert_eq!(status.repos[0].change_count, 0);
    }

    #[test]
    fn test_status_dirty_repo() {
        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path().join("my-repo");
        create_git_repo(&repo_path);

        // Make it dirty
        std::fs::write(repo_path.join("dirty.txt"), "content").unwrap();

        let manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
            created: chrono::Utc::now(),
            base_branch: None,
            repos: vec![RepoManifestEntry {
                name: "my-repo".to_string(),
                original_path: repo_path.clone(),
                worktree_path: repo_path,
                branch: "loom/test-ws".to_string(),
                remote_url: String::new(),
            }],
        };

        let ws_path = dir.path().join("ws");
        let status = workspace_status(&manifest, &ws_path, false).unwrap();
        assert!(status.repos[0].is_dirty);
        assert!(status.repos[0].change_count > 0);
    }

    #[test]
    fn test_status_missing_worktree() {
        let dir = tempfile::tempdir().unwrap();

        let manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
            created: chrono::Utc::now(),
            base_branch: None,
            repos: vec![RepoManifestEntry {
                name: "my-repo".to_string(),
                original_path: dir.path().join("nonexistent"),
                worktree_path: dir.path().join("nonexistent"),
                branch: "loom/test-ws".to_string(),
                remote_url: String::new(),
            }],
        };

        let ws_path = dir.path().join("ws");
        let status = workspace_status(&manifest, &ws_path, false).unwrap();
        assert!(!status.repos[0].exists);
    }
}
