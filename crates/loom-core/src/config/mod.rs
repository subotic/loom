use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Top-level LOOM configuration, stored at ~/.config/loom/config.toml
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub registry: RegistryConfig,
    pub workspace: WorkspaceConfig,
    #[serde(default)]
    pub sync: Option<SyncConfig>,
    #[serde(default)]
    pub terminal: Option<TerminalConfig>,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub agents: AgentsConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Directories to scan recursively for git repos
    pub scan_roots: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Root directory for all workspaces (default: ~/loom)
    pub root: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Path to sync repo (e.g., PKM repo)
    pub repo: PathBuf,
    /// Subdirectory within sync repo for workspace manifests
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerminalConfig {
    /// Terminal command to open (e.g., "ghostty", "wezterm")
    pub command: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DefaultsConfig {
    /// Branch prefix for worktrees (default: "loom")
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AgentsConfig {
    /// Which agents to configure (e.g., ["claude-code"])
    #[serde(default)]
    pub enabled: Vec<String>,
}

fn default_branch_prefix() -> String {
    "loom".to_string()
}

impl Config {
    /// Load config from ~/.config/loom/config.toml
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            anyhow::bail!(
                "Configuration not found at {}. Run `loom init` to create a config file.",
                path.display()
            );
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config at {}", path.display()))?;
        Ok(config)
    }

    /// Path to the config file: ~/.config/loom/config.toml
    ///
    /// Hardcoded to ~/.config/loom/ for cross-platform consistency.
    /// This matches developer tool conventions (ripgrep, bat, starship)
    /// and avoids the `directories` crate's macOS path (~Library/Application Support/).
    pub fn path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        Ok(home.join(".config").join("loom").join("config.toml"))
    }
}
