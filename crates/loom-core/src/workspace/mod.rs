pub mod add;
pub mod down;
pub mod exec;
pub mod list;
pub mod new;
pub mod remove;
pub mod shell;
pub mod status;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::Config;
use crate::manifest::WorkspaceManifest;

/// The manifest filename placed at the workspace root.
pub const MANIFEST_FILENAME: &str = ".loom.json";

/// Detect if `cwd` (or any ancestor) is inside a loom workspace.
///
/// Walks up from `cwd` looking for `.loom.json`. Returns the workspace
/// root directory and the loaded manifest if found.
pub fn detect_workspace(cwd: &Path) -> Result<Option<(PathBuf, WorkspaceManifest)>> {
    let mut current = cwd.to_path_buf();
    loop {
        let manifest_path = current.join(MANIFEST_FILENAME);
        if manifest_path.exists() {
            let manifest = crate::manifest::read_manifest(&manifest_path).with_context(|| {
                format!(
                    "Failed to read workspace manifest at {}",
                    manifest_path.display()
                )
            })?;
            return Ok(Some((current, manifest)));
        }

        if !current.pop() {
            break;
        }
    }
    Ok(None)
}

/// Resolve a workspace by explicit name or by detecting from cwd.
///
/// - If `name` is Some, looks up `config.workspace.root/{name}/.loom.json`
/// - If `name` is None, detects from `cwd`
pub fn resolve_workspace(
    name: Option<&str>,
    cwd: &Path,
    config: &Config,
) -> Result<(PathBuf, WorkspaceManifest)> {
    match name {
        Some(name) => {
            let ws_path = config.workspace.root.join(name);
            let manifest_path = ws_path.join(MANIFEST_FILENAME);
            if !manifest_path.exists() {
                anyhow::bail!("Workspace '{}' not found at {}", name, ws_path.display());
            }
            let manifest = crate::manifest::read_manifest(&manifest_path)?;
            Ok((ws_path, manifest))
        }
        None => detect_workspace(cwd)?.ok_or_else(|| {
            anyhow::anyhow!("Not inside a loom workspace. Specify a workspace name or cd into one.")
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{WorkspaceManifest, write_manifest};
    use std::collections::BTreeMap;

    fn create_test_workspace(root: &Path, name: &str) -> PathBuf {
        let ws_path = root.join(name);
        std::fs::create_dir_all(&ws_path).unwrap();

        let manifest = WorkspaceManifest {
            name: name.to_string(),
            branch: None,
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![],
        };

        write_manifest(&ws_path.join(MANIFEST_FILENAME), &manifest).unwrap();
        ws_path
    }

    #[test]
    fn test_detect_workspace_at_root() {
        let dir = tempfile::tempdir().unwrap();
        let ws_path = create_test_workspace(dir.path(), "test-ws");

        let result = detect_workspace(&ws_path).unwrap();
        assert!(result.is_some());
        let (path, manifest) = result.unwrap();
        assert_eq!(path, ws_path);
        assert_eq!(manifest.name, "test-ws");
    }

    #[test]
    fn test_detect_workspace_from_subdirectory() {
        let dir = tempfile::tempdir().unwrap();
        let ws_path = create_test_workspace(dir.path(), "test-ws");

        // Create a subdirectory
        let sub = ws_path.join("some").join("nested").join("dir");
        std::fs::create_dir_all(&sub).unwrap();

        let result = detect_workspace(&sub).unwrap();
        assert!(result.is_some());
        let (path, _) = result.unwrap();
        assert_eq!(path, ws_path);
    }

    #[test]
    fn test_detect_workspace_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let result = detect_workspace(dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_workspace_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let ws_root = dir.path().join("workspaces");
        create_test_workspace(&ws_root, "my-feature");

        let config = Config {
            registry: crate::config::RegistryConfig { scan_roots: vec![] },
            workspace: crate::config::WorkspaceConfig { root: ws_root },
            sync: None,
            terminal: None,
            defaults: crate::config::DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: crate::config::AgentsConfig::default(),
        };

        let (path, manifest) = resolve_workspace(Some("my-feature"), dir.path(), &config).unwrap();
        assert!(path.ends_with("my-feature"));
        assert_eq!(manifest.name, "my-feature");
    }

    #[test]
    fn test_resolve_workspace_by_name_not_found() {
        let dir = tempfile::tempdir().unwrap();

        let config = Config {
            registry: crate::config::RegistryConfig { scan_roots: vec![] },
            workspace: crate::config::WorkspaceConfig {
                root: dir.path().to_path_buf(),
            },
            sync: None,
            terminal: None,
            defaults: crate::config::DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: crate::config::AgentsConfig::default(),
        };

        let result = resolve_workspace(Some("nonexistent"), dir.path(), &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_resolve_workspace_from_cwd() {
        let dir = tempfile::tempdir().unwrap();
        let ws_path = create_test_workspace(dir.path(), "detected-ws");

        let config = Config {
            registry: crate::config::RegistryConfig { scan_roots: vec![] },
            workspace: crate::config::WorkspaceConfig {
                root: dir.path().to_path_buf(),
            },
            sync: None,
            terminal: None,
            defaults: crate::config::DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: crate::config::AgentsConfig::default(),
        };

        let (_, manifest) = resolve_workspace(None, &ws_path, &config).unwrap();
        assert_eq!(manifest.name, "detected-ws");
    }
}
