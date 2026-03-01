use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::Config;
use crate::git::repo::{GitRepo, clone_repo};
use crate::manifest::sync::{SyncManifest, SyncRepoEntry};
use crate::manifest::{self, RepoManifestEntry, WorkspaceIndex, WorkspaceManifest};
use crate::registry;
use crate::registry::url::normalize_url;
use crate::workspace::MANIFEST_FILENAME;

/// Result of an open operation.
#[derive(Debug)]
pub struct OpenResult {
    pub path: PathBuf,
    pub name: String,
    pub repos_restored: usize,
    pub repos_cloned: Vec<String>,
    pub repos_failed: Vec<(String, String)>,
    pub warnings: Vec<String>,
}

/// Open (reconstruct) a workspace from a sync manifest.
///
/// Steps:
/// 1. Pull sync repo
/// 2. Read sync manifest
/// 3. Match remote URLs to local repos
/// 4. Clone missing repos
/// 5. Create worktrees
/// 6. Generate agent files
/// 7. Update state
pub fn open_workspace(config: &Config, name: &str) -> Result<OpenResult> {
    let sync_config = config
        .sync
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!(
            "Sync is not configured. `loom open` requires a sync repo to reconstruct workspaces. \
             Set `[sync]` in `~/.config/loom/config.toml` or use `loom new` to create a local workspace."
        ))?;

    // Pull sync repo (best effort — may fail if no remote configured)
    let sync_git = GitRepo::new(&sync_config.repo);
    if let Err(e) = sync_git.pull_rebase() {
        eprintln!(
            "Warning: Could not pull sync repo ({}). Using local copy.",
            e
        );
    }

    // Read sync manifest
    let manifest_path = sync_config
        .repo
        .join(&sync_config.path)
        .join(format!("{name}.json"));
    if !manifest_path.exists() {
        anyhow::bail!(
            "Workspace '{}' not found in sync repo at {}",
            name,
            manifest_path.display()
        );
    }

    let content = std::fs::read_to_string(&manifest_path).with_context(|| {
        format!(
            "Failed to read sync manifest at {}",
            manifest_path.display()
        )
    })?;
    let sync_manifest: SyncManifest = serde_json::from_str(&content).with_context(|| {
        format!(
            "Failed to parse sync manifest at {}",
            manifest_path.display()
        )
    })?;

    // Check if workspace already exists locally
    let ws_path = config.workspace.root.join(name);
    let existing_manifest = if ws_path.join(MANIFEST_FILENAME).exists() {
        Some(manifest::read_manifest(&ws_path.join(MANIFEST_FILENAME))?)
    } else {
        None
    };

    // Discover local repos for URL matching
    let local_repos =
        registry::discover_repos(&config.registry.scan_roots, Some(&config.workspace.root));

    let mut repos_restored = 0;
    let mut repos_cloned = Vec::new();
    let mut repos_failed = Vec::new();
    let mut warnings = Vec::new();
    let mut ws_repos = Vec::new();

    let branch_prefix = &config.defaults.branch_prefix;

    for sync_repo in &sync_manifest.repos {
        match restore_repo(
            config,
            &ws_path,
            sync_repo,
            &local_repos,
            existing_manifest.as_ref(),
            branch_prefix,
            name,
        ) {
            Ok(RestoreResult::Restored(entry)) => {
                ws_repos.push(entry);
                repos_restored += 1;
            }
            Ok(RestoreResult::Cloned(entry)) => {
                repos_cloned.push(sync_repo.name.clone());
                ws_repos.push(entry);
                repos_restored += 1;
            }
            Ok(RestoreResult::Skipped(warning)) => {
                warnings.push(warning);
            }
            Err(e) => {
                repos_failed.push((sync_repo.name.clone(), e.to_string()));
            }
        }
    }

    // Check for repos that exist locally but not in sync manifest
    if let Some(ref existing) = existing_manifest {
        for local_repo in &existing.repos {
            let in_sync = sync_manifest
                .repos
                .iter()
                .any(|r| r.name == local_repo.name);
            if !in_sync {
                warnings.push(format!(
                    "Repo '{}' exists locally but not in sync manifest (keeping it).",
                    local_repo.name
                ));
                ws_repos.push(local_repo.clone());
            }
        }
    }

    // Create workspace directory
    std::fs::create_dir_all(&ws_path)
        .with_context(|| format!("Failed to create workspace directory {}", ws_path.display()))?;

    // Write workspace manifest, preserving preset from existing local manifest
    let existing_preset = existing_manifest.as_ref().and_then(|m| m.preset.clone());
    let ws_manifest = WorkspaceManifest {
        name: name.to_string(),
        branch: sync_manifest.repos.first().map(|r| r.branch.clone()),
        created: sync_manifest.created,
        base_branch: None,
        preset: existing_preset,
        repos: ws_repos,
    };
    manifest::write_manifest(&ws_path.join(MANIFEST_FILENAME), &ws_manifest)?;

    // Generate agent files
    crate::agent::generate_agent_files(config, &ws_path, &ws_manifest)?;

    // Update state.json
    let state_path = config.workspace.root.join(".loom").join("state.json");
    std::fs::create_dir_all(state_path.parent().unwrap()).ok();
    let mut state = manifest::read_global_state(&state_path);
    state.upsert(WorkspaceIndex {
        name: name.to_string(),
        path: ws_path.clone(),
        created: ws_manifest.created,
        repo_count: ws_manifest.repos.len(),
    });
    manifest::write_global_state(&state_path, &state)?;

    Ok(OpenResult {
        path: ws_path,
        name: name.to_string(),
        repos_restored,
        repos_cloned,
        repos_failed,
        warnings,
    })
}

enum RestoreResult {
    Restored(RepoManifestEntry),
    Cloned(RepoManifestEntry),
    Skipped(String),
}

/// Restore a single repo: find locally or clone, then create worktree.
fn restore_repo(
    config: &Config,
    ws_path: &Path,
    sync_repo: &SyncRepoEntry,
    local_repos: &[registry::RepoEntry],
    existing_manifest: Option<&WorkspaceManifest>,
    branch_prefix: &str,
    ws_name: &str,
) -> Result<RestoreResult> {
    // Check if already exists in current workspace manifest
    if let Some(existing) = existing_manifest
        && let Some(entry) = existing.repos.iter().find(|r| r.name == sync_repo.name)
        && entry.worktree_path.exists()
    {
        // Already present — check for branch divergence
        let git = GitRepo::new(&entry.worktree_path);
        let current_branch = git.current_branch().unwrap_or_default();
        if current_branch != sync_repo.branch {
            return Ok(RestoreResult::Skipped(format!(
                "Repo '{}' already exists on branch '{}' (sync expects '{}').",
                sync_repo.name, current_branch, sync_repo.branch
            )));
        }
        return Ok(RestoreResult::Restored(entry.clone()));
    }

    // Find repo locally by URL matching
    let sync_canonical = normalize_url(&sync_repo.remote_url);
    let local_match = local_repos.iter().find(|r| {
        r.remote_url
            .as_deref()
            .is_some_and(|url| normalize_url(url) == sync_canonical)
    });

    let repo_path = match local_match {
        Some(local) => local.path.clone(),
        None => {
            // Clone the repo
            let clone_target = derive_clone_path(config, &sync_repo.remote_url)?;

            // Check if target exists but has different remote
            if clone_target.exists() {
                let existing_git = GitRepo::new(&clone_target);
                if let Ok(Some(url)) = existing_git.remote_url()
                    && normalize_url(&url) != sync_canonical
                {
                    anyhow::bail!(
                        "Directory {} exists but remote URL differs (expected {}, found {}). \
                         Clone manually or update config.",
                        clone_target.display(),
                        sync_repo.remote_url,
                        url
                    );
                }
                clone_target
            } else {
                clone_repo(&sync_repo.remote_url, &clone_target).with_context(|| {
                    format!(
                        "Failed to clone {} to {}",
                        sync_repo.remote_url,
                        clone_target.display()
                    )
                })?;
                clone_target
            }
        }
    };

    // Fetch to ensure remote refs are up-to-date
    let git = GitRepo::new(&repo_path);
    git.fetch().ok(); // Best effort

    // Create worktree
    let worktree_path = ws_path.join(&sync_repo.name);
    let branch_name = format!("{branch_prefix}/{ws_name}");

    if !worktree_path.exists() {
        // Try creating worktree with new branch
        match git.worktree_add(&worktree_path, &branch_name, &sync_repo.branch) {
            Ok(()) => {}
            Err(crate::git::GitError::BranchConflict { .. }) => {
                // Branch exists — try using it
                git.worktree_remove(&worktree_path, true).ok();
                std::process::Command::new("git")
                    .arg("-C")
                    .arg(git.path())
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
        let lock_reason = format!("loom:{ws_name}");
        git.worktree_lock(&worktree_path, &lock_reason).ok();
    }

    let was_cloned = local_match.is_none();
    let entry = RepoManifestEntry {
        name: sync_repo.name.clone(),
        original_path: repo_path,
        worktree_path,
        branch: branch_name,
        remote_url: sync_repo.remote_url.clone(),
    };

    if was_cloned {
        Ok(RestoreResult::Cloned(entry))
    } else {
        Ok(RestoreResult::Restored(entry))
    }
}

/// Derive a local path for cloning from a remote URL.
///
/// Uses the first scan_root and the canonical URL structure:
/// `github.com/org/repo` → `{first_scan_root}/org/repo`
fn derive_clone_path(config: &Config, remote_url: &str) -> Result<PathBuf> {
    let scan_root =
        config.registry.scan_roots.first().ok_or_else(|| {
            anyhow::anyhow!("No scan roots configured. Cannot derive clone path.")
        })?;

    let canonical = normalize_url(remote_url)
        .ok_or_else(|| anyhow::anyhow!("Cannot normalize URL '{}'", remote_url))?;
    let canonical_str = canonical.as_str();
    // canonical is like "github.com/org/repo"
    // We want "{scan_root}/org/repo"
    let parts: Vec<&str> = canonical_str.splitn(2, '/').collect();
    if parts.len() < 2 {
        anyhow::bail!(
            "Cannot derive clone path from URL '{}' (canonical: '{}')",
            remote_url,
            canonical_str
        );
    }

    // Skip the host, use org/repo
    Ok(scan_root.join(parts[1]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        AgentsConfig, DefaultsConfig, RegistryConfig, SyncConfig, WorkspaceConfig,
    };
    use std::collections::BTreeMap;

    #[test]
    fn test_derive_clone_path() {
        let config = Config {
            registry: RegistryConfig {
                scan_roots: vec![PathBuf::from("/code")],
            },
            workspace: WorkspaceConfig {
                root: PathBuf::from("/loom"),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        };

        let path = derive_clone_path(&config, "git@github.com:dasch-swiss/dsp-api.git").unwrap();
        assert_eq!(path, PathBuf::from("/code/dasch-swiss/dsp-api"));
    }

    #[test]
    fn test_derive_clone_path_https() {
        let config = Config {
            registry: RegistryConfig {
                scan_roots: vec![PathBuf::from("/home/user/code")],
            },
            workspace: WorkspaceConfig {
                root: PathBuf::from("/loom"),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        };

        let path = derive_clone_path(&config, "https://github.com/org/repo.git").unwrap();
        assert_eq!(path, PathBuf::from("/home/user/code/org/repo"));
    }

    #[test]
    fn test_derive_clone_path_no_scan_roots() {
        let config = Config {
            registry: RegistryConfig { scan_roots: vec![] },
            workspace: WorkspaceConfig {
                root: PathBuf::from("/loom"),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        };

        let result = derive_clone_path(&config, "git@github.com:org/repo.git");
        assert!(result.is_err());
    }

    #[test]
    fn test_open_no_sync_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            registry: RegistryConfig { scan_roots: vec![] },
            workspace: WorkspaceConfig {
                root: dir.path().to_path_buf(),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        };

        let result = open_workspace(&config, "test-ws");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Sync is not configured")
        );
    }

    #[test]
    fn test_open_missing_manifest() {
        let dir = tempfile::tempdir().unwrap();

        // Create a sync repo
        let sync_repo_path = dir.path().join("sync-repo");
        std::fs::create_dir_all(&sync_repo_path).unwrap();
        std::process::Command::new("git")
            .args(["init", "-b", "main", &sync_repo_path.to_string_lossy()])
            .env("LC_ALL", "C")
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &sync_repo_path.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .env("LC_ALL", "C")
            .output()
            .unwrap();

        let ws_root = dir.path().join("loom");
        std::fs::create_dir_all(ws_root.join(".loom")).unwrap();

        let config = Config {
            registry: RegistryConfig { scan_roots: vec![] },
            workspace: WorkspaceConfig { root: ws_root },
            sync: Some(SyncConfig {
                repo: sync_repo_path,
                path: "loom".to_string(),
            }),
            terminal: None,
            defaults: DefaultsConfig::default(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        };

        let result = open_workspace(&config, "nonexistent-ws");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
