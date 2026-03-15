use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use loom_core::config::{AgentsConfig, UpdateConfig, Config, DefaultsConfig, RegistryConfig, WorkspaceConfig};

/// Create a git repo at the given path with an initial commit.
pub fn create_test_repo(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();

    std::process::Command::new("git")
        .args(["init", "-b", "main", &dir.to_string_lossy()])
        .env("LC_ALL", "C")
        .output()
        .expect("git init failed");

    std::process::Command::new("git")
        .args([
            "-C",
            &dir.to_string_lossy(),
            "commit",
            "--allow-empty",
            "-m",
            "initial commit",
        ])
        .env("LC_ALL", "C")
        .output()
        .expect("git commit failed");

    dir.to_path_buf()
}

/// Create a valid Config pointing at temp directories.
pub fn create_test_config(scan_root: &Path, workspace_root: &Path) -> Config {
    Config {
        registry: RegistryConfig {
            scan_roots: vec![scan_root.to_path_buf()],
        },
        workspace: WorkspaceConfig {
            root: workspace_root.to_path_buf(),
        },
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

/// A temporary workspace environment that cleans up on drop.
pub struct TempWorkspace {
    #[allow(dead_code)] // Held for Drop to keep temp directory alive
    pub dir: tempfile::TempDir,
    pub scan_root: PathBuf,
    pub workspace_root: PathBuf,
    pub config: Config,
}

impl TempWorkspace {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let scan_root = dir.path().join("code");
        let workspace_root = dir.path().join("loom");
        std::fs::create_dir_all(&scan_root).unwrap();
        std::fs::create_dir_all(&workspace_root).unwrap();

        let config = create_test_config(&scan_root, &workspace_root);

        Self {
            dir,
            scan_root,
            workspace_root,
            config,
        }
    }

    /// Add a test git repo at scan_root/{org}/{name}
    pub fn add_repo(&self, org: &str, name: &str) -> PathBuf {
        let repo_path = self.scan_root.join(org).join(name);
        create_test_repo(&repo_path)
    }
}
