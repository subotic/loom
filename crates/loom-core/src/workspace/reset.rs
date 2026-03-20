use anyhow::Result;

use crate::git::GitRepo;
use crate::manifest::{RepoManifestEntry, WorkspaceManifest};

/// Result of resetting a workspace.
pub struct ResetResult {
    pub repos_reset: Vec<String>,
    pub repos_failed: Vec<(String, String)>,
}

/// Reset all repos in a workspace to the workspace's base branch (or the repo's default branch).
///
/// For each repo: discard all changes, fetch origin, and rebase onto the
/// base branch. If rebase fails, falls back to hard reset.
pub fn reset_workspace(
    manifest: &WorkspaceManifest,
    on_progress: impl Fn(super::ProgressEvent),
) -> Result<ResetResult> {
    let mut repos_reset = Vec::new();
    let mut repos_failed = Vec::new();
    let total = manifest.repos.len();

    for (i, repo) in manifest.repos.iter().enumerate() {
        on_progress(super::ProgressEvent::RepoStarted {
            name: repo.name.clone(),
            index: i,
            total,
        });

        match reset_repo(repo, manifest.base_branch.as_deref()) {
            Ok(()) => {
                repos_reset.push(repo.name.clone());
                on_progress(super::ProgressEvent::RepoComplete {
                    name: repo.name.clone(),
                });
            }
            Err(e) => {
                let msg = e.to_string();
                repos_failed.push((repo.name.clone(), msg.clone()));
                on_progress(super::ProgressEvent::RepoFailed {
                    name: repo.name.clone(),
                    error: msg,
                });
            }
        }
    }

    Ok(ResetResult {
        repos_reset,
        repos_failed,
    })
}

fn reset_repo(repo: &RepoManifestEntry, base_branch: Option<&str>) -> Result<()> {
    let git = GitRepo::new(&repo.worktree_path);

    // 1. Discard all changes (staged, unstaged, untracked)
    git.reset_hard()?;
    git.clean_untracked()?;

    // 2. Fetch origin (non-fatal — stale refs just mean rebase uses older state)
    if let Err(e) = git.fetch() {
        tracing::warn!(repo = %repo.name, error = %e, "fetch failed, using local state");
    }

    // 3. Determine target branch: workspace base_branch > repo default > "main"
    let branch = match base_branch {
        Some(b) => b.to_string(),
        None => git.default_branch().unwrap_or_else(|_| "main".to_string()),
    };
    let target = format!("origin/{branch}");

    if git.ref_exists(&target).unwrap_or(false) {
        // 4. Rebase onto origin/main; fall back to hard reset on conflict
        if let Err(_e) = git.rebase(&target) {
            git.rebase_abort().ok();
            git.reset_hard_to(&target)?;
        }
    } else {
        tracing::debug!(
            repo = %repo.name,
            target = %target,
            "remote ref not found, skipping rebase"
        );
    }

    Ok(())
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
    fn test_reset_clean_repo() {
        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path().join("my-repo");
        create_git_repo(&repo_path);

        let manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
            branch: None,
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![RepoManifestEntry {
                name: "my-repo".to_string(),
                original_path: repo_path.clone(),
                worktree_path: repo_path,
                branch: "main".to_string(),
                remote_url: String::new(),
            }],
        };

        let result = reset_workspace(&manifest, |_| {}).unwrap();
        assert_eq!(result.repos_reset.len(), 1);
        assert!(result.repos_failed.is_empty());
    }

    #[test]
    fn test_reset_dirty_repo() {
        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path().join("my-repo");
        create_git_repo(&repo_path);

        // Make it dirty
        std::fs::write(repo_path.join("dirty.txt"), "content").unwrap();

        let git = GitRepo::new(&repo_path);
        assert!(git.is_dirty().unwrap());

        let manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
            branch: None,
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![RepoManifestEntry {
                name: "my-repo".to_string(),
                original_path: repo_path.clone(),
                worktree_path: repo_path.clone(),
                branch: "main".to_string(),
                remote_url: String::new(),
            }],
        };

        let result = reset_workspace(&manifest, |_| {}).unwrap();
        assert_eq!(result.repos_reset.len(), 1);

        // Verify it's clean now
        assert!(!git.is_dirty().unwrap());
        assert!(!repo_path.join("dirty.txt").exists());
    }
}
