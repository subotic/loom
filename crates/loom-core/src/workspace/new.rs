use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;

use crate::config::Config;
use crate::git::GitRepo;
use crate::manifest::{self, RepoManifestEntry, WorkspaceIndex, WorkspaceManifest};
use crate::registry::RepoEntry;

/// Result of creating a workspace.
#[derive(Debug)]
pub struct NewWorkspaceResult {
    pub path: PathBuf,
    pub name: String,
    pub branch: String,
    pub repos_added: usize,
    pub repos_failed: Vec<(String, String)>, // (name, error message)
}

/// Options for creating a new workspace.
pub struct NewWorkspaceOpts {
    pub name: String,
    pub repos: Vec<RepoEntry>,
    pub base_branch: Option<String>,
    pub preset: Option<String>,
}

/// Create a new workspace with worktrees for the selected repos.
pub fn create_workspace(config: &Config, opts: NewWorkspaceOpts) -> Result<NewWorkspaceResult> {
    // Validate workspace name
    manifest::validate_name(&opts.name)?;

    // Check for name collision
    let ws_path = config.workspace.root.join(&opts.name);
    if ws_path.exists() {
        anyhow::bail!(
            "Workspace '{}' already exists at {}. Choose a different name or run `loom down {}` first.",
            opts.name,
            ws_path.display(),
            opts.name
        );
    }

    // Validate repos
    if opts.repos.is_empty() {
        anyhow::bail!("Select at least one repository. A workspace requires at least one repo.");
    }

    // Validate preset exists in config (if provided)
    if let Some(ref preset_name) = opts.preset {
        crate::config::validate_preset_exists(&config.agents.claude_code.presets, preset_name)?;
    }

    // If --base is set, fetch and validate all repos have the ref
    if let Some(ref base) = opts.base_branch {
        for repo in &opts.repos {
            let git_repo = GitRepo::new(&repo.path);
            // Fetch so remote refs are available for validation
            if let Err(e) = git_repo.fetch() {
                eprintln!(
                    "  Warning: could not fetch '{}': {}. Using local state.",
                    repo.name, e
                );
            }
            if !git_repo.ref_exists(base)? {
                let hint = if !base.contains('/') {
                    let remote_ref = format!("origin/{}", base);
                    if git_repo.ref_exists(&remote_ref).unwrap_or(false) {
                        format!(
                            "\nHint: 'origin/{}' exists — use `--base origin/{}` for remote branches.",
                            base, base
                        )
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                anyhow::bail!("Ref '{}' not found in {}.{}", base, repo.name, hint);
            }
        }
    }

    // Create workspace directory
    std::fs::create_dir_all(&ws_path).with_context(|| {
        format!(
            "Failed to create workspace directory at {}",
            ws_path.display()
        )
    })?;

    let branch_prefix = &config.defaults.branch_prefix;
    let now = Utc::now();

    // Generate a random branch name that doesn't collide with existing refs
    let repo_paths: Vec<PathBuf> = opts.repos.iter().map(|r| r.path.clone()).collect();
    let branch_name = crate::names::generate_unique_branch_name(branch_prefix, &repo_paths, 10)?;

    // Write state.json FIRST (with 0 repos) — Ctrl+C safety
    let state_path = config.workspace.root.join(".loom").join("state.json");
    let mut state = manifest::read_global_state(&state_path);
    state.upsert(WorkspaceIndex {
        name: opts.name.clone(),
        path: ws_path.clone(),
        created: now,
        repo_count: 0,
    });
    manifest::write_global_state(&state_path, &state)?;

    // Initialize workspace manifest
    let mut ws_manifest = WorkspaceManifest {
        name: opts.name.clone(),
        branch: Some(branch_name.clone()),
        created: now,
        base_branch: opts.base_branch.clone(),
        preset: opts.preset.clone(),
        repos: Vec::new(),
    };

    let mut repos_added = 0;
    let mut repos_failed = Vec::new();

    // Process each repo
    for repo in &opts.repos {
        let branch_name = branch_name.clone();

        match add_repo_to_workspace(
            &ws_path,
            repo,
            &branch_name,
            opts.base_branch.as_deref(),
            &opts.name,
        ) {
            Ok(entry) => {
                ws_manifest.repos.push(entry);
                repos_added += 1;

                // Write manifest after each successful repo (Ctrl+C safety)
                manifest::write_manifest(&ws_path.join(super::MANIFEST_FILENAME), &ws_manifest)?;

                // Update state.json repo count
                state.upsert(WorkspaceIndex {
                    name: opts.name.clone(),
                    path: ws_path.clone(),
                    created: now,
                    repo_count: repos_added,
                });
                manifest::write_global_state(&state_path, &state)?;
            }
            Err(e) => {
                repos_failed.push((repo.name.clone(), e.to_string()));
            }
        }
    }

    // Final manifest write
    manifest::write_manifest(&ws_path.join(super::MANIFEST_FILENAME), &ws_manifest)?;

    // Generate agent files (CLAUDE.md, .claude/settings.local.json)
    crate::agent::generate_agent_files(config, &ws_path, &ws_manifest)?;

    Ok(NewWorkspaceResult {
        path: ws_path,
        name: opts.name,
        branch: branch_name,
        repos_added,
        repos_failed,
    })
}

/// Add a single repo to a workspace: create worktree, lock it, update manifest.
fn add_repo_to_workspace(
    ws_path: &Path,
    repo: &RepoEntry,
    branch_name: &str,
    base_branch: Option<&str>,
    ws_name: &str,
) -> Result<RepoManifestEntry> {
    let git_repo = GitRepo::new(&repo.path);

    // Fetch latest state from origin (non-fatal)
    if let Err(e) = git_repo.fetch() {
        eprintln!(
            "  Warning: could not fetch '{}': {}. Using local state.",
            repo.name, e
        );
    }

    // Determine base branch
    let base = match base_branch {
        Some(b) => b.to_string(),
        None => {
            let branch = git_repo
                .default_branch()
                .unwrap_or_else(|_| "main".to_string());
            git_repo.resolve_start_point(&branch)
        }
    };

    // Targeted stale cleanup: remove only LOOM-owned stale worktrees
    cleanup_stale_loom_worktrees(&git_repo)?;

    // Create worktree directory
    let worktree_path = ws_path.join(&repo.name);

    // Add worktree
    match git_repo.worktree_add(&worktree_path, branch_name, &base) {
        Ok(()) => {}
        Err(crate::git::GitError::BranchConflict { .. }) => {
            // Branch already exists — reuse it (safest non-destructive default)
            // The worktree_add failed, so try with the existing branch
            // Remove the -b flag by using worktree add with just the path and branch
            git_repo.worktree_remove(&worktree_path, true).ok(); // Clean up partial add
            std::process::Command::new("git")
                .arg("-C")
                .arg(git_repo.path())
                .args([
                    "worktree",
                    "add",
                    &worktree_path.to_string_lossy(),
                    branch_name,
                ])
                .env("LC_ALL", "C")
                .output()
                .context("Failed to add worktree with existing branch")?;
        }
        Err(e) => return Err(e.into()),
    }

    // Lock worktree with loom identifier
    let lock_reason = format!("loom:{ws_name}");
    git_repo.worktree_lock(&worktree_path, &lock_reason)?;

    let remote_url = git_repo.remote_url()?.unwrap_or_default();

    Ok(RepoManifestEntry {
        name: repo.name.clone(),
        original_path: repo.path.clone(),
        worktree_path,
        branch: branch_name.to_string(),
        remote_url,
    })
}

/// Remove only LOOM-owned worktrees whose directory no longer exists.
/// Does NOT use `git worktree prune` (which is global and would remove non-LOOM worktrees).
fn cleanup_stale_loom_worktrees(git_repo: &GitRepo) -> Result<()> {
    let worktrees = git_repo.worktree_list()?;

    for wt in &worktrees {
        // Only touch LOOM-owned worktrees
        let is_loom_owned = wt
            .lock_reason
            .as_deref()
            .is_some_and(|r| r.starts_with("loom:"))
            || wt.branch.as_deref().is_some_and(|b| b.starts_with("loom/"));

        if !is_loom_owned {
            continue;
        }

        // Only remove if the directory no longer exists
        if !wt.path.exists() {
            // Unlock if locked
            if wt.is_locked {
                git_repo.worktree_unlock(&wt.path).ok();
            }
            git_repo.worktree_remove(&wt.path, true).ok();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentsConfig, DefaultsConfig, RegistryConfig, WorkspaceConfig};
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
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        }
    }

    fn create_repo(dir: &std::path::Path, org: &str, name: &str) -> RepoEntry {
        let path = dir.join(org).join(name);
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
            org: org.to_string(),
            path,
            remote_url: None,
        }
    }

    #[test]
    fn test_create_workspace_basic() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let repo = create_repo(dir.path(), "org", "my-repo");

        let result = create_workspace(
            &config,
            NewWorkspaceOpts {
                name: "test-ws".to_string(),
                repos: vec![repo],
                base_branch: None,
                preset: None,
            },
        )
        .unwrap();

        assert_eq!(result.name, "test-ws");
        assert_eq!(result.repos_added, 1);
        assert!(result.repos_failed.is_empty());
        assert!(result.path.join(super::super::MANIFEST_FILENAME).exists());
    }

    #[test]
    fn test_create_workspace_name_collision() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let repo = create_repo(dir.path(), "org", "my-repo");

        // Create the workspace directory to simulate collision
        std::fs::create_dir_all(config.workspace.root.join("existing")).unwrap();

        let result = create_workspace(
            &config,
            NewWorkspaceOpts {
                name: "existing".to_string(),
                repos: vec![repo],
                base_branch: None,
                preset: None,
            },
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_create_workspace_empty_repos() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());

        let result = create_workspace(
            &config,
            NewWorkspaceOpts {
                name: "empty".to_string(),
                repos: vec![],
                base_branch: None,
                preset: None,
            },
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least one"));
    }

    #[test]
    fn test_create_workspace_invalid_name() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());

        let result = create_workspace(
            &config,
            NewWorkspaceOpts {
                name: "INVALID NAME".to_string(),
                repos: vec![],
                base_branch: None,
                preset: None,
            },
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_create_workspace_state_written_first() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let repo = create_repo(dir.path(), "org", "my-repo");

        create_workspace(
            &config,
            NewWorkspaceOpts {
                name: "state-test".to_string(),
                repos: vec![repo],
                base_branch: None,
                preset: None,
            },
        )
        .unwrap();

        // Verify state.json exists and has the workspace
        let state_path = config.workspace.root.join(".loom").join("state.json");
        let state = manifest::read_global_state(&state_path);
        assert!(state.find("state-test").is_some());
        assert_eq!(state.find("state-test").unwrap().repo_count, 1);
    }
}
