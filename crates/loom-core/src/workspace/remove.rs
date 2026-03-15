use std::path::Path;

use anyhow::Result;

use crate::config::Config;
use crate::git::GitRepo;
use crate::manifest::{self, WorkspaceIndex, WorkspaceManifest};
use crate::workspace::MANIFEST_FILENAME;

/// Remove a repo from a workspace.
///
/// Returns an error if the repo has uncommitted changes (unless `force` is true)
/// or if it's the last repo in the workspace.
pub fn remove_repo(
    config: &Config,
    ws_path: &Path,
    manifest: &mut WorkspaceManifest,
    repo_name: &str,
    force: bool,
) -> Result<Vec<crate::agent::MatchedRepoConfig>> {
    // Find the repo
    let repo_idx = manifest
        .repos
        .iter()
        .position(|r| r.name == repo_name)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Repository '{}' not found in workspace '{}'.",
                repo_name,
                manifest.name
            )
        })?;

    // Refuse if last repo
    if manifest.repos.len() == 1 {
        anyhow::bail!(
            "This is the last repo in workspace '{}'. Use `loom down {}` to tear down the workspace.",
            manifest.name,
            manifest.name
        );
    }

    let repo_entry = &manifest.repos[repo_idx];

    // Dirty check
    if repo_entry.worktree_path.exists() {
        let git = GitRepo::new(&repo_entry.worktree_path);
        if !force && git.is_dirty().unwrap_or(false) {
            anyhow::bail!(
                "Repository '{}' has uncommitted changes. Use --force to remove anyway.",
                repo_name
            );
        }

        // Unlock and remove worktree
        let original_git = GitRepo::new(&repo_entry.original_path);
        original_git.worktree_unlock(&repo_entry.worktree_path).ok();
        original_git
            .worktree_remove(&repo_entry.worktree_path, force)
            .ok();

        // Delete the branch (safe delete only, unless force)
        original_git.branch_delete(&repo_entry.branch, force).ok();
    }

    // Remove from manifest
    manifest.repos.remove(repo_idx);
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

    // Regenerate agent files (CLAUDE.md, .claude/settings.json)
    let applied = crate::agent::generate_agent_files(config, ws_path, manifest)?;

    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        AgentsConfig, DefaultsConfig, RegistryConfig, UpdateConfig, WorkspaceConfig,
    };
    use crate::manifest::RepoManifestEntry;
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
            update: UpdateConfig::default(),
        }
    }

    #[test]
    fn test_remove_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let ws_path = config.workspace.root.join("test-ws");
        std::fs::create_dir_all(&ws_path).unwrap();

        let mut manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
            branch: None,
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![RepoManifestEntry {
                name: "repo-a".to_string(),
                original_path: dir.path().join("repo-a"),
                worktree_path: ws_path.join("repo-a"),
                branch: "loom/test-ws".to_string(),
                remote_url: String::new(),
            }],
        };

        let result = remove_repo(&config, &ws_path, &mut manifest, "nonexistent", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_remove_last_repo() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let ws_path = config.workspace.root.join("test-ws");
        std::fs::create_dir_all(&ws_path).unwrap();

        let mut manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
            branch: None,
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![RepoManifestEntry {
                name: "repo-a".to_string(),
                original_path: dir.path().join("repo-a"),
                worktree_path: ws_path.join("repo-a"),
                branch: "loom/test-ws".to_string(),
                remote_url: String::new(),
            }],
        };

        let result = remove_repo(&config, &ws_path, &mut manifest, "repo-a", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("last repo"));
    }
}
