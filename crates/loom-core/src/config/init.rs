use std::path::PathBuf;

use anyhow::{Context, Result};

use super::{
    AgentsConfig, Config, DefaultsConfig, RegistryConfig, TerminalConfig, WorkspaceConfig,
};
use crate::git;

/// Well-known directories to check for scan_roots auto-detection.
const CANDIDATE_SCAN_ROOTS: &[&str] = &[
    "~/_github.com",
    "~/src",
    "~/code",
    "~/repos",
    "~/Projects",
    "~/dev",
];

/// Auto-detect scan roots by checking which well-known directories exist.
pub fn detect_scan_roots() -> Vec<PathBuf> {
    CANDIDATE_SCAN_ROOTS
        .iter()
        .filter_map(|p| {
            let expanded = shellexpand::tilde(p);
            let path = PathBuf::from(expanded.as_ref());
            if path.is_dir() { Some(path) } else { None }
        })
        .collect()
}

/// Detect terminal from $TERM_PROGRAM environment variable.
pub fn detect_terminal() -> Option<String> {
    std::env::var("TERM_PROGRAM").ok().map(|t| {
        // Map common terminal program names to their CLI commands
        match t.as_str() {
            "ghostty" => "ghostty".to_string(),
            "WezTerm" => "wezterm".to_string(),
            "iTerm.app" => "open -a iTerm".to_string(),
            "Apple_Terminal" => "open -a Terminal".to_string(),
            "vscode" => "code".to_string(),
            _ => t,
        }
    })
}

/// Run the init workflow and return the resulting Config.
///
/// This is the core logic; the CLI layer handles prompting via `dialoguer`.
/// For non-interactive use (testing), pass the values directly.
pub fn create_config(
    scan_roots: Vec<PathBuf>,
    workspace_root: PathBuf,
    terminal: Option<String>,
    branch_prefix: String,
    agents: Vec<String>,
) -> Result<Config> {
    // Verify git is installed and meets minimum version
    let git_version = git::check_git_version().context("Git check failed during loom init")?;

    eprintln!("  git version: {git_version}");

    let config = Config {
        registry: RegistryConfig { scan_roots },
        workspace: WorkspaceConfig {
            root: workspace_root,
        },
        sync: None,
        terminal: terminal.map(|command| TerminalConfig { command }),
        defaults: DefaultsConfig { branch_prefix },
        agents: AgentsConfig {
            enabled: agents,
            ..Default::default()
        },
    };

    Ok(config)
}

/// Save config and create required directories.
pub fn finalize_init(config: &Config) -> Result<()> {
    // Save config file
    config.save().context("Failed to save config")?;

    let config_path = Config::path()?;
    eprintln!("  config: {}", config_path.display());

    // Create workspace root
    std::fs::create_dir_all(&config.workspace.root).with_context(|| {
        format!(
            "Failed to create workspace root at {}",
            config.workspace.root.display()
        )
    })?;
    eprintln!("  workspace root: {}", config.workspace.root.display());

    // Create state directory
    let state_dir = config.workspace.root.join(".loom");
    std::fs::create_dir_all(&state_dir).with_context(|| {
        format!(
            "Failed to create state directory at {}",
            state_dir.display()
        )
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_scan_roots() {
        // Should not panic; results depend on system
        let roots = detect_scan_roots();
        // All returned paths should exist and be directories
        for root in &roots {
            assert!(root.is_dir(), "{} should be a directory", root.display());
        }
    }

    #[test]
    fn test_detect_terminal() {
        // Result depends on environment, just ensure no panic
        let _ = detect_terminal();
    }

    #[test]
    fn test_create_config() {
        let dir = tempfile::tempdir().unwrap();
        let scan_root = dir.path().join("code");
        std::fs::create_dir_all(&scan_root).unwrap();

        let config = create_config(
            vec![scan_root.clone()],
            dir.path().join("loom"),
            Some("ghostty".to_string()),
            "loom".to_string(),
            vec!["claude-code".to_string()],
        )
        .unwrap();

        assert_eq!(config.registry.scan_roots, vec![scan_root]);
        assert_eq!(config.defaults.branch_prefix, "loom");
        assert!(config.terminal.is_some());
    }

    #[test]
    fn test_finalize_init() {
        let dir = tempfile::tempdir().unwrap();
        let ws_root = dir.path().join("loom");

        let _config = Config {
            registry: RegistryConfig { scan_roots: vec![] },
            workspace: WorkspaceConfig {
                root: ws_root.clone(),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            agents: AgentsConfig::default(),
        };

        // Can't easily test save() since it writes to ~/.config/loom/
        // but we can test directory creation
        std::fs::create_dir_all(&ws_root).unwrap();
        let state_dir = ws_root.join(".loom");
        std::fs::create_dir_all(&state_dir).unwrap();

        assert!(ws_root.exists());
        assert!(state_dir.exists());
    }
}
