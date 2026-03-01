use std::path::PathBuf;

use anyhow::Result;

use crate::config::Config;
use crate::git::GitRepo;
use crate::manifest::{self, WorkspaceManifest};

/// Summary of a single workspace for listing.
#[derive(Debug)]
pub struct WorkspaceSummary {
    pub name: String,
    pub branch: String,
    pub path: PathBuf,
    pub repo_count: usize,
    pub status: WorkspaceHealth,
    pub created: chrono::DateTime<chrono::Utc>,
    pub preset: Option<String>,
}

/// Health status of a workspace.
#[derive(Debug, PartialEq, Eq)]
pub enum WorkspaceHealth {
    /// All repos clean
    Clean,
    /// Some repos have uncommitted changes
    Dirty(usize),
    /// Manifest is missing or corrupt
    Broken(String),
}

/// List all workspaces with their status.
pub fn list_workspaces(config: &Config) -> Result<Vec<WorkspaceSummary>> {
    let state_path = config.workspace.root.join(".loom").join("state.json");
    let state = manifest::read_global_state(&state_path);

    let mut summaries = Vec::new();

    for ws_index in &state.workspaces {
        let manifest_path = ws_index.path.join(crate::workspace::MANIFEST_FILENAME);

        let summary = match manifest::read_manifest::<WorkspaceManifest>(&manifest_path) {
            Ok(manifest) => {
                // Quick dirty check per repo
                let mut dirty_count = 0;
                for repo in &manifest.repos {
                    if repo.worktree_path.exists() {
                        let git = GitRepo::new(&repo.worktree_path);
                        if git.is_dirty().unwrap_or(false) {
                            dirty_count += 1;
                        }
                    }
                }

                let status = if dirty_count > 0 {
                    WorkspaceHealth::Dirty(dirty_count)
                } else {
                    WorkspaceHealth::Clean
                };

                let branch = manifest.branch_name(&config.defaults.branch_prefix);
                WorkspaceSummary {
                    name: manifest.name,
                    branch,
                    path: ws_index.path.clone(),
                    repo_count: manifest.repos.len(),
                    status,
                    created: ws_index.created,
                    preset: manifest.preset,
                }
            }
            Err(e) => {
                // Best-effort fallback; may be wrong for workspaces with random branch names
                let branch = format!("{}/{}", config.defaults.branch_prefix, ws_index.name);
                WorkspaceSummary {
                    name: ws_index.name.clone(),
                    branch,
                    path: ws_index.path.clone(),
                    repo_count: ws_index.repo_count,
                    status: WorkspaceHealth::Broken(e.to_string()),
                    created: ws_index.created,
                    preset: None,
                }
            }
        };

        summaries.push(summary);
    }

    Ok(summaries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentsConfig, DefaultsConfig, RegistryConfig, WorkspaceConfig};
    use crate::manifest::{RepoManifestEntry, WorkspaceIndex, WorkspaceManifest};
    use std::collections::BTreeMap;

    fn test_config(dir: &std::path::Path) -> Config {
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

    #[test]
    fn test_list_empty() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let summaries = list_workspaces(&config).unwrap();
        assert!(summaries.is_empty());
    }

    #[test]
    fn test_list_with_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());

        // Create workspace directory and manifest
        let ws_path = config.workspace.root.join("test-ws");
        std::fs::create_dir_all(&ws_path).unwrap();

        let manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
            branch: None,
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![],
        };
        manifest::write_manifest(
            &ws_path.join(crate::workspace::MANIFEST_FILENAME),
            &manifest,
        )
        .unwrap();

        // Register in state.json
        let state_path = config.workspace.root.join(".loom").join("state.json");
        let mut state = manifest::read_global_state(&state_path);
        state.upsert(WorkspaceIndex {
            name: "test-ws".to_string(),
            path: ws_path,
            created: chrono::Utc::now(),
            repo_count: 0,
        });
        manifest::write_global_state(&state_path, &state).unwrap();

        let summaries = list_workspaces(&config).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].name, "test-ws");
        assert_eq!(summaries[0].status, WorkspaceHealth::Clean);
    }

    #[test]
    fn test_list_broken_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());

        // Create workspace directory but no manifest
        let ws_path = config.workspace.root.join("broken-ws");
        std::fs::create_dir_all(&ws_path).unwrap();

        // Register in state.json
        let state_path = config.workspace.root.join(".loom").join("state.json");
        let mut state = manifest::read_global_state(&state_path);
        state.upsert(WorkspaceIndex {
            name: "broken-ws".to_string(),
            path: ws_path,
            created: chrono::Utc::now(),
            repo_count: 2,
        });
        manifest::write_global_state(&state_path, &state).unwrap();

        let summaries = list_workspaces(&config).unwrap();
        assert_eq!(summaries.len(), 1);
        assert!(matches!(summaries[0].status, WorkspaceHealth::Broken(_)));
    }

    #[test]
    fn test_list_dirty_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());

        // Create a real git repo for the worktree
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

        // Add an untracked file to make it dirty
        std::fs::write(repo_path.join("dirty.txt"), "dirty").unwrap();

        // Create workspace pointing to this repo
        let ws_path = config.workspace.root.join("dirty-ws");
        std::fs::create_dir_all(&ws_path).unwrap();

        let manifest = WorkspaceManifest {
            name: "dirty-ws".to_string(),
            branch: None,
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![RepoManifestEntry {
                name: "my-repo".to_string(),
                original_path: repo_path.clone(),
                worktree_path: repo_path,
                branch: "loom/dirty-ws".to_string(),
                remote_url: String::new(),
            }],
        };
        manifest::write_manifest(
            &ws_path.join(crate::workspace::MANIFEST_FILENAME),
            &manifest,
        )
        .unwrap();

        let state_path = config.workspace.root.join(".loom").join("state.json");
        let mut state = manifest::read_global_state(&state_path);
        state.upsert(WorkspaceIndex {
            name: "dirty-ws".to_string(),
            path: ws_path,
            created: chrono::Utc::now(),
            repo_count: 1,
        });
        manifest::write_global_state(&state_path, &state).unwrap();

        let summaries = list_workspaces(&config).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].status, WorkspaceHealth::Dirty(1));
    }
}
