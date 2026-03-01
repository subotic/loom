use std::path::Path;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::git::GitRepo;
use crate::manifest::{self, RepoManifestEntry, WorkspaceIndex, WorkspaceManifest};
use crate::registry::RepoEntry;
use crate::workspace::MANIFEST_FILENAME;

/// Add a repo to an existing workspace.
pub fn add_repo(
    config: &Config,
    ws_path: &Path,
    manifest: &mut WorkspaceManifest,
    repo: &RepoEntry,
) -> Result<()> {
    // Check if repo already in workspace
    if manifest.repos.iter().any(|r| r.name == repo.name) {
        anyhow::bail!(
            "Repository '{}' is already in workspace '{}'.",
            repo.name,
            manifest.name
        );
    }

    let branch_prefix = &config.defaults.branch_prefix;
    let branch_name = format!("{}/{}", branch_prefix, manifest.name);

    let git_repo = GitRepo::new(&repo.path);

    // Fetch latest state from origin (non-fatal)
    if let Err(e) = git_repo.fetch() {
        eprintln!(
            "  Warning: could not fetch '{}': {}. Using local state.",
            repo.name, e
        );
    }

    // Determine base branch
    let base = match &manifest.base_branch {
        Some(b) => b.clone(),
        None => {
            let branch = git_repo
                .default_branch()
                .unwrap_or_else(|_| "main".to_string());
            git_repo.resolve_start_point(&branch)
        }
    };

    // Create worktree
    let worktree_path = ws_path.join(&repo.name);

    match git_repo.worktree_add(&worktree_path, &branch_name, &base) {
        Ok(()) => {}
        Err(crate::git::GitError::BranchConflict { .. }) => {
            git_repo.worktree_remove(&worktree_path, true).ok();
            std::process::Command::new("git")
                .arg("-C")
                .arg(git_repo.path())
                .args([
                    "worktree",
                    "add",
                    &worktree_path.to_string_lossy(),
                    &branch_name,
                ])
                .env("LC_ALL", "C")
                .output()
                .context("Failed to add worktree with existing branch")?;
        }
        Err(e) => return Err(e.into()),
    }

    // Lock worktree
    let lock_reason = format!("loom:{}", manifest.name);
    git_repo.worktree_lock(&worktree_path, &lock_reason)?;

    let remote_url = git_repo.remote_url()?.unwrap_or_default();

    // Update manifest
    manifest.repos.push(RepoManifestEntry {
        name: repo.name.clone(),
        original_path: repo.path.clone(),
        worktree_path,
        branch: branch_name,
        remote_url,
    });

    manifest::write_manifest(&ws_path.join(MANIFEST_FILENAME), manifest)?;

    // Update state.json
    let state_path = config.workspace.root.join(".loom").join("state.json");
    let mut state = manifest::read_global_state(&state_path);
    state.upsert(WorkspaceIndex {
        name: manifest.name.clone(),
        path: ws_path.to_path_buf(),
        created: manifest.created,
        repo_count: manifest.repos.len(),
    });
    manifest::write_global_state(&state_path, &state)?;

    // Regenerate agent files (CLAUDE.md, .claude/settings.local.json)
    crate::agent::generate_agent_files(config, ws_path, manifest)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentsConfig, DefaultsConfig, RegistryConfig, WorkspaceConfig};
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
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        }
    }

    fn create_repo(dir: &Path, name: &str) -> RepoEntry {
        let path = dir.join("repos").join(name);
        std::fs::create_dir_all(&path).unwrap();
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
        RepoEntry {
            name: name.to_string(),
            org: "org".to_string(),
            path,
            remote_url: None,
        }
    }

    #[test]
    fn test_add_repo_basic() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let repo = create_repo(dir.path(), "new-repo");

        let ws_path = config.workspace.root.join("test-ws");
        std::fs::create_dir_all(&ws_path).unwrap();

        let mut manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![],
        };
        manifest::write_manifest(&ws_path.join(MANIFEST_FILENAME), &manifest).unwrap();

        // Register in state
        let state_path = config.workspace.root.join(".loom").join("state.json");
        let mut state = manifest::read_global_state(&state_path);
        state.upsert(WorkspaceIndex {
            name: "test-ws".to_string(),
            path: ws_path.clone(),
            created: manifest.created,
            repo_count: 0,
        });
        manifest::write_global_state(&state_path, &state).unwrap();

        add_repo(&config, &ws_path, &mut manifest, &repo).unwrap();

        assert_eq!(manifest.repos.len(), 1);
        assert_eq!(manifest.repos[0].name, "new-repo");
        assert!(ws_path.join("new-repo").exists());
    }

    #[test]
    fn test_add_duplicate_repo() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());

        let ws_path = config.workspace.root.join("test-ws");
        std::fs::create_dir_all(&ws_path).unwrap();

        let mut manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![RepoManifestEntry {
                name: "existing-repo".to_string(),
                original_path: dir.path().join("repos").join("existing-repo"),
                worktree_path: ws_path.join("existing-repo"),
                branch: "loom/test-ws".to_string(),
                remote_url: String::new(),
            }],
        };

        let repo = RepoEntry {
            name: "existing-repo".to_string(),
            org: "org".to_string(),
            path: dir.path().join("repos").join("existing-repo"),
            remote_url: None,
        };

        let result = add_repo(&config, &ws_path, &mut manifest, &repo);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("already in workspace")
        );
    }
}
