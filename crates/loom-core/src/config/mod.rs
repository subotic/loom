pub mod init;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Top-level LOOM configuration, stored at ~/.config/loom/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Directories to scan recursively for git repos
    pub scan_roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Root directory for all workspaces (default: ~/loom)
    pub root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Path to sync repo (e.g., PKM repo)
    pub repo: PathBuf,
    /// Subdirectory within sync repo for workspace manifests
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    /// Terminal command to open (e.g., "ghostty", "wezterm")
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    /// Branch prefix for worktrees (default: "loom")
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            branch_prefix: default_branch_prefix(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsConfig {
    /// Which agents to configure (e.g., ["claude-code"])
    #[serde(default)]
    pub enabled: Vec<String>,
}

impl Default for AgentsConfig {
    fn default() -> Self {
        Self {
            enabled: vec!["claude-code".to_string()],
        }
    }
}

fn default_branch_prefix() -> String {
    "loom".to_string()
}

/// Expand ~ and environment variables in a path using shellexpand
fn expand_path(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    let expanded = shellexpand::tilde(&s);
    PathBuf::from(expanded.as_ref())
}

impl Config {
    /// Load config from ~/.config/loom/config.toml
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        Self::load_from(&path)
    }

    /// Load config from a specific path (useful for testing)
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            anyhow::bail!(
                "Configuration not found at {}. Run `loom init` to create a config file.",
                path.display()
            );
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;
        let mut config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config at {}", path.display()))?;
        config.expand_paths();
        Ok(config)
    }

    /// Save config to ~/.config/loom/config.toml atomically
    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        self.save_to(&path)
    }

    /// Save config to a specific path atomically (useful for testing)
    pub fn save_to(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize config to TOML")?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory {}", parent.display())
            })?;
        }

        // Atomic write: create temp file in same dir, write, then persist (rename)
        let parent = path.parent().unwrap_or(Path::new("."));
        let tmp = tempfile::NamedTempFile::new_in(parent)
            .with_context(|| format!("Failed to create temp file in {}", parent.display()))?;
        std::fs::write(tmp.path(), content.as_bytes())
            .with_context(|| "Failed to write config to temp file".to_string())?;
        tmp.persist(path)
            .with_context(|| format!("Failed to persist config to {}", path.display()))?;

        Ok(())
    }

    /// Sensible defaults for `loom init`
    pub fn default_config() -> Self {
        Self {
            registry: RegistryConfig {
                scan_roots: Vec::new(),
            },
            workspace: WorkspaceConfig {
                root: PathBuf::from("~/loom"),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            agents: AgentsConfig::default(),
        }
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

    /// Expand ~ in all PathBuf fields (post-deserialization step)
    fn expand_paths(&mut self) {
        for root in &mut self.registry.scan_roots {
            *root = expand_path(root);
        }
        self.workspace.root = expand_path(&self.workspace.root);
        if let Some(sync) = &mut self.sync {
            sync.repo = expand_path(&sync.repo);
        }
    }

    /// Validate the loaded config
    pub fn validate(&self) -> Result<()> {
        // Validate scan_roots paths exist
        for root in &self.registry.scan_roots {
            if !root.exists() {
                anyhow::bail!(
                    "scan_roots path `{}` does not exist. Create it or update config.",
                    root.display()
                );
            }
            if !root.is_dir() {
                anyhow::bail!("scan_roots path `{}` is not a directory.", root.display());
            }
        }

        // Validate workspace root parent exists (we can create the root itself)
        let ws_root = &self.workspace.root;
        if let Some(parent) = ws_root.parent()
            && !parent.as_os_str().is_empty()
            && !parent.exists()
        {
            anyhow::bail!(
                "workspace.root parent `{}` does not exist. Create it first.",
                parent.display()
            );
        }

        // Validate branch_prefix is a valid git ref component
        let prefix = &self.defaults.branch_prefix;
        if prefix.is_empty() {
            anyhow::bail!("defaults.branch_prefix cannot be empty.");
        }
        if prefix.contains(' ') || prefix.contains("..") || prefix.starts_with('.') {
            anyhow::bail!(
                "defaults.branch_prefix `{}` is not a valid git ref component.",
                prefix
            );
        }

        Ok(())
    }
}

/// Load config from standard path, returning actionable error if missing.
/// Use this in every command that needs config (all except `init`).
pub fn ensure_config_loaded() -> Result<Config> {
    let config = Config::load()?;
    config.validate()?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toml_round_trip() {
        let config = Config {
            registry: RegistryConfig {
                scan_roots: vec![PathBuf::from("/home/user/code")],
            },
            workspace: WorkspaceConfig {
                root: PathBuf::from("/home/user/loom"),
            },
            sync: Some(SyncConfig {
                repo: PathBuf::from("/home/user/pkm"),
                path: "loom".to_string(),
            }),
            terminal: Some(TerminalConfig {
                command: "ghostty".to_string(),
            }),
            defaults: DefaultsConfig {
                branch_prefix: "loom".to_string(),
            },
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
            },
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.registry.scan_roots, config.registry.scan_roots);
        assert_eq!(parsed.workspace.root, config.workspace.root);
        assert_eq!(parsed.defaults.branch_prefix, "loom");
        assert!(parsed.sync.is_some());
        assert!(parsed.terminal.is_some());
    }

    #[test]
    fn test_missing_optional_fields() {
        let toml_str = r#"
[registry]
scan_roots = ["/code"]

[workspace]
root = "/loom"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.sync.is_none());
        assert!(config.terminal.is_none());
        assert_eq!(config.defaults.branch_prefix, "loom");
        // Default agents config includes claude-code
        assert_eq!(config.agents.enabled, vec!["claude-code"]);
    }

    #[test]
    fn test_tilde_expansion() {
        let path = PathBuf::from("~/code");
        let expanded = expand_path(&path);
        assert!(!expanded.to_string_lossy().contains('~'));
        assert!(expanded.to_string_lossy().len() > 6); // longer than ~/code
    }

    #[test]
    fn test_config_path() {
        let path = Config::path().unwrap();
        let path_str = path.to_string_lossy();
        assert!(path_str.contains(".config/loom/config.toml"));
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        let config = Config {
            registry: RegistryConfig {
                scan_roots: vec![dir.path().to_path_buf()],
            },
            workspace: WorkspaceConfig {
                root: dir.path().join("workspaces"),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            agents: AgentsConfig::default(),
        };

        config.save_to(&config_path).unwrap();
        let loaded = Config::load_from(&config_path).unwrap();

        assert_eq!(loaded.registry.scan_roots, config.registry.scan_roots);
        assert_eq!(loaded.workspace.root, config.workspace.root);
    }

    #[test]
    fn test_validate_invalid_branch_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            registry: RegistryConfig {
                scan_roots: vec![dir.path().to_path_buf()],
            },
            workspace: WorkspaceConfig {
                root: dir.path().to_path_buf(),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig {
                branch_prefix: "..invalid".to_string(),
            },
            agents: AgentsConfig::default(),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_missing_scan_root() {
        let config = Config {
            registry: RegistryConfig {
                scan_roots: vec![PathBuf::from("/nonexistent/path/abc123")],
            },
            workspace: WorkspaceConfig {
                root: PathBuf::from("/tmp"),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            agents: AgentsConfig::default(),
        };

        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("does not exist"));
    }

    #[test]
    fn test_load_nonexistent() {
        let result = Config::load_from(Path::new("/nonexistent/config.toml"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("loom init"));
    }

    #[test]
    fn test_default_config() {
        let config = Config::default_config();
        assert!(config.registry.scan_roots.is_empty());
        assert_eq!(config.workspace.root, PathBuf::from("~/loom"));
        assert_eq!(config.defaults.branch_prefix, "loom");
        assert_eq!(config.agents.enabled, vec!["claude-code"]);
    }
}
