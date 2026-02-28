use std::path::Path;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::git::GitRepo;
use crate::manifest::WorkspaceManifest;
use crate::manifest::sync::{SyncManifest, SyncRepoEntry, SyncStatus};

/// Result of a save operation.
#[derive(Debug)]
pub struct SaveResult {
    pub pushed: Vec<String>,
    pub push_failed: Vec<(String, String)>,
    pub dirty_skipped: Vec<String>,
    pub sync_ok: bool,
    pub sync_error: Option<String>,
}

/// Push branches and optionally sync the workspace manifest.
///
/// For each repo:
/// - If clean (or force): push the branch
/// - If dirty and not force: skip with warning
///
/// If sync is configured, write the sync manifest to the sync repo.
pub fn save_workspace(
    config: &Config,
    _ws_path: &Path,
    manifest: &WorkspaceManifest,
    force: bool,
) -> Result<SaveResult> {
    let mut pushed = Vec::new();
    let mut push_failed = Vec::new();
    let mut dirty_skipped = Vec::new();

    // Push each repo's branch
    for repo in &manifest.repos {
        if !repo.worktree_path.exists() {
            push_failed.push((repo.name.clone(), "worktree missing".to_string()));
            continue;
        }

        let git = GitRepo::new(&repo.worktree_path);

        // Check dirty status
        let is_dirty = git.is_dirty().unwrap_or(false);
        if is_dirty && !force {
            dirty_skipped.push(repo.name.clone());
            continue;
        }

        // Push the branch
        match git.push_tracking(&repo.branch) {
            Ok(()) => pushed.push(repo.name.clone()),
            Err(e) => push_failed.push((repo.name.clone(), e.to_string())),
        }
    }

    // Sync manifest to sync repo (if configured)
    let (sync_ok, sync_error) = match &config.sync {
        Some(sync_config) => match write_sync_manifest(sync_config, manifest) {
            Ok(()) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        },
        None => (true, None), // No sync configured = success (no-op)
    };

    Ok(SaveResult {
        pushed,
        push_failed,
        dirty_skipped,
        sync_ok,
        sync_error,
    })
}

/// Write workspace manifest to sync repo, commit, and push.
fn write_sync_manifest(
    sync_config: &crate::config::SyncConfig,
    manifest: &WorkspaceManifest,
) -> Result<()> {
    let sync_git = GitRepo::new(&sync_config.repo);

    // Pull sync repo with rebase
    match sync_git.pull_rebase() {
        Ok(()) => {}
        Err(_) => {
            // Abort rebase if it failed
            sync_git.rebase_abort().ok();
            anyhow::bail!(
                "Sync repo has conflicts. Resolve manually in {} and re-run `loom save`.",
                sync_config.repo.display()
            );
        }
    }

    // Generate sync manifest
    let sync_manifest = SyncManifest {
        name: manifest.name.clone(),
        created: manifest.created,
        status: SyncStatus::Active,
        repos: manifest
            .repos
            .iter()
            .map(|r| SyncRepoEntry {
                name: r.name.clone(),
                remote_url: r.remote_url.clone(),
                branch: r.branch.clone(),
            })
            .collect(),
    };

    let json = serde_json::to_string_pretty(&sync_manifest)
        .context("Failed to serialize sync manifest")?;

    // Write to sync repo
    let sync_dir = sync_config.repo.join(&sync_config.path);
    std::fs::create_dir_all(&sync_dir)
        .with_context(|| format!("Failed to create sync directory {}", sync_dir.display()))?;

    let manifest_path = sync_dir.join(format!("{}.json", manifest.name));
    std::fs::write(&manifest_path, &json)
        .with_context(|| format!("Failed to write sync manifest {}", manifest_path.display()))?;

    // Git add, commit, push
    let relative_path = format!("{}/{}.json", sync_config.path, manifest.name);
    sync_git
        .add(&relative_path)
        .context("Failed to stage sync manifest")?;

    let commit_msg = format!("loom: update {}", manifest.name);
    // Commit may fail if nothing changed (same content) — that's OK
    sync_git.commit(&commit_msg).ok();

    sync_git
        .push()
        .context("Failed to push sync repo. Changes committed locally but not pushed.")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentsConfig, DefaultsConfig, RegistryConfig, WorkspaceConfig};
    use crate::manifest::RepoManifestEntry;
    use std::path::PathBuf;

    fn test_config(dir: &Path) -> Config {
        let ws_root = dir.join("loom");
        std::fs::create_dir_all(ws_root.join(".loom")).unwrap();
        Config {
            registry: RegistryConfig { scan_roots: vec![] },
            workspace: WorkspaceConfig { root: ws_root },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            agents: AgentsConfig::default(),
        }
    }

    #[test]
    fn test_save_missing_worktree() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let ws_path = config.workspace.root.join("test-ws");

        let manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![RepoManifestEntry {
                name: "missing-repo".to_string(),
                original_path: PathBuf::from("/nonexistent"),
                worktree_path: PathBuf::from("/nonexistent/wt"),
                branch: "loom/test-ws".to_string(),
                remote_url: String::new(),
            }],
        };

        let result = save_workspace(&config, &ws_path, &manifest, false).unwrap();
        assert!(result.pushed.is_empty());
        assert_eq!(result.push_failed.len(), 1);
        assert!(result.dirty_skipped.is_empty());
    }

    #[test]
    fn test_save_dirty_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let ws_path = config.workspace.root.join("test-ws");

        // Create a git repo with dirty state
        let repo_path = dir.path().join("repos").join("my-repo");
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
        // Make dirty
        std::fs::write(repo_path.join("dirty.txt"), "content").unwrap();

        let manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
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

        let result = save_workspace(&config, &ws_path, &manifest, false).unwrap();
        assert!(result.pushed.is_empty());
        assert!(result.push_failed.is_empty());
        assert_eq!(result.dirty_skipped.len(), 1);
        assert_eq!(result.dirty_skipped[0], "my-repo");
    }

    #[test]
    fn test_save_empty_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let ws_path = config.workspace.root.join("test-ws");

        let manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![],
        };

        let result = save_workspace(&config, &ws_path, &manifest, false).unwrap();
        assert!(result.pushed.is_empty());
        assert!(result.push_failed.is_empty());
        assert!(result.dirty_skipped.is_empty());
        assert!(result.sync_ok);
    }
}
