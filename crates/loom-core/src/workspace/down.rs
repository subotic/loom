use std::path::Path;

use anyhow::Result;

use crate::config::Config;
use crate::git::GitRepo;
use crate::manifest::{self, WorkspaceManifest};
use crate::workspace::MANIFEST_FILENAME;

/// Pre-flight check result for tearing down a workspace.
#[derive(Debug)]
pub struct DownCheck {
    pub clean_repos: Vec<String>,
    pub dirty_repos: Vec<(String, usize)>, // (name, change_count)
    pub missing_repos: Vec<String>,
}

/// Check the state of all repos before tearing down.
pub fn check_workspace(manifest: &WorkspaceManifest) -> DownCheck {
    let mut clean = Vec::new();
    let mut dirty = Vec::new();
    let mut missing = Vec::new();

    for repo in &manifest.repos {
        if !repo.worktree_path.exists() {
            missing.push(repo.name.clone());
            continue;
        }

        let git = GitRepo::new(&repo.worktree_path);
        match git.change_count() {
            Ok(0) => clean.push(repo.name.clone()),
            Ok(n) => dirty.push((repo.name.clone(), n)),
            Err(_) => clean.push(repo.name.clone()), // Can't check, treat as clean
        }
    }

    DownCheck {
        clean_repos: clean,
        dirty_repos: dirty,
        missing_repos: missing,
    }
}

/// Tear down a workspace: remove worktrees, delete branches, clean up state.
///
/// `repos_to_remove` specifies which repos to actually remove (allows partial teardown).
/// If all repos are removed, the workspace directory and state entry are cleaned up.
pub fn teardown_workspace(
    config: &Config,
    ws_path: &Path,
    manifest: &mut WorkspaceManifest,
    repos_to_remove: &[String],
    force: bool,
) -> Result<TeardownResult> {
    let mut removed = Vec::new();
    let mut failed = Vec::new();

    for repo_name in repos_to_remove {
        let repo_idx = match manifest.repos.iter().position(|r| r.name == *repo_name) {
            Some(idx) => idx,
            None => continue,
        };

        let repo_entry = &manifest.repos[repo_idx];

        if repo_entry.worktree_path.exists() {
            let original_git = GitRepo::new(&repo_entry.original_path);

            // Unlock
            original_git.worktree_unlock(&repo_entry.worktree_path).ok();

            // Remove worktree
            match original_git.worktree_remove(&repo_entry.worktree_path, force) {
                Ok(()) => {}
                Err(e) => {
                    failed.push((repo_name.clone(), e.to_string()));
                    continue;
                }
            }

            // Delete branch
            original_git.branch_delete(&repo_entry.branch, force).ok();
        }

        removed.push(repo_name.clone());
    }

    // Remove successfully removed repos from manifest
    manifest.repos.retain(|r| !removed.contains(&r.name));

    let matched_configs = if manifest.repos.is_empty() {
        // Full teardown: remove workspace directory and state entry
        std::fs::remove_dir_all(ws_path).ok();

        let state_path = config.workspace.root.join(".loom").join("state.json");
        let mut state = manifest::read_global_state(&state_path);
        state.remove(&manifest.name);
        manifest::write_global_state(&state_path, &state)?;

        Vec::new()
    } else {
        // Partial teardown: update manifest with remaining repos
        manifest::write_manifest(&ws_path.join(MANIFEST_FILENAME), manifest)?;

        let state_path = config.workspace.root.join(".loom").join("state.json");
        let mut state = manifest::read_global_state(&state_path);
        state.upsert(crate::manifest::WorkspaceIndex {
            name: manifest.name.clone(),
            path: ws_path.to_path_buf(),
            created: manifest.created,
            repo_count: manifest.repos.len(),
        });
        manifest::write_global_state(&state_path, &state)?;

        // Regenerate agent files for remaining repos
        crate::agent::generate_agent_files(config, ws_path, manifest)?
    };

    Ok(TeardownResult {
        removed,
        failed,
        remaining: manifest.repos.iter().map(|r| r.name.clone()).collect(),
        matched_configs,
    })
}

/// Result of a workspace teardown.
#[derive(Debug)]
pub struct TeardownResult {
    pub removed: Vec<String>,
    pub failed: Vec<(String, String)>,
    pub remaining: Vec<String>,
    pub matched_configs: Vec<crate::agent::MatchedRepoConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentsConfig, DefaultsConfig, RegistryConfig, WorkspaceConfig};
    use crate::manifest::{RepoManifestEntry, WorkspaceIndex};
    use std::collections::BTreeMap;

    fn test_config(dir: &Path) -> Config {
        let ws_root = dir.join("loom");
        std::fs::create_dir_all(ws_root.join(".loom")).unwrap();
        Config {
            registry: RegistryConfig { scan_roots: vec![] },
            workspace: WorkspaceConfig { root: ws_root },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        }
    }

    fn create_repo_with_worktree(
        dir: &Path,
        name: &str,
    ) -> (RepoManifestEntry, std::path::PathBuf) {
        let repo_path = dir.join("repos").join(name);
        std::fs::create_dir_all(&repo_path).unwrap();
        std::process::Command::new("git")
            .args(["init", "-b", "main", &repo_path.to_string_lossy()])
            .env("LC_ALL", "C")
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo_path.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .env("LC_ALL", "C")
            .output()
            .unwrap();

        // Add worktree
        let ws_path = dir.join("loom").join("test-ws");
        let wt_path = ws_path.join(name);
        std::process::Command::new("git")
            .args([
                "-C",
                &repo_path.to_string_lossy(),
                "worktree",
                "add",
                "-b",
                "loom/test-ws",
                &wt_path.to_string_lossy(),
            ])
            .env("LC_ALL", "C")
            .output()
            .unwrap();

        let entry = RepoManifestEntry {
            name: name.to_string(),
            original_path: repo_path,
            worktree_path: wt_path,
            branch: "loom/test-ws".to_string(),
            remote_url: String::new(),
        };

        (entry, ws_path)
    }

    #[test]
    fn test_check_workspace_clean() {
        let dir = tempfile::tempdir().unwrap();
        let (entry, _) = create_repo_with_worktree(dir.path(), "repo-a");

        let manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
            branch: None,
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![entry],
        };

        let check = check_workspace(&manifest);
        assert_eq!(check.clean_repos.len(), 1);
        assert!(check.dirty_repos.is_empty());
        assert!(check.missing_repos.is_empty());
    }

    #[test]
    fn test_check_workspace_dirty() {
        let dir = tempfile::tempdir().unwrap();
        let (entry, _) = create_repo_with_worktree(dir.path(), "repo-a");

        // Make dirty
        std::fs::write(entry.worktree_path.join("dirty.txt"), "content").unwrap();

        let manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
            branch: None,
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![entry],
        };

        let check = check_workspace(&manifest);
        assert!(check.clean_repos.is_empty());
        assert_eq!(check.dirty_repos.len(), 1);
    }

    #[test]
    fn test_teardown_full() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let (entry, ws_path) = create_repo_with_worktree(dir.path(), "repo-a");
        std::fs::create_dir_all(&ws_path).unwrap();

        let mut manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
            branch: None,
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![entry],
        };
        manifest::write_manifest(&ws_path.join(MANIFEST_FILENAME), &manifest).unwrap();

        // Register in state
        let state_path = config.workspace.root.join(".loom").join("state.json");
        let mut state = manifest::read_global_state(&state_path);
        state.upsert(WorkspaceIndex {
            name: "test-ws".to_string(),
            path: ws_path.clone(),
            created: manifest.created,
            repo_count: 1,
        });
        manifest::write_global_state(&state_path, &state).unwrap();

        let result = teardown_workspace(
            &config,
            &ws_path,
            &mut manifest,
            &["repo-a".to_string()],
            true,
        )
        .unwrap();

        assert_eq!(result.removed.len(), 1);
        assert!(result.failed.is_empty());
        assert!(result.remaining.is_empty());

        // State should be cleaned up
        let state = manifest::read_global_state(&state_path);
        assert!(state.find("test-ws").is_none());
    }
}
