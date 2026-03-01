use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::{
    AgentsConfig, ClaudeCodeConfig, Config, DefaultsConfig, RegistryConfig, SandboxConfig,
    SandboxFilesystemConfig, SandboxNetworkConfig, TerminalConfig, WorkspaceConfig,
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

/// Security flavor for Claude Code permissions during `loom init`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityFlavor {
    /// OS-level sandbox isolation with auto-allow (recommended).
    Sandbox,
    /// Explicit tool allowlists for fine-grained control.
    Permissions,
    /// Sandbox for Bash commands + permissions for non-Bash tools.
    Both,
    /// Don't configure now — can be added later in config.toml.
    Skip,
}

/// Build a `ClaudeCodeConfig` from the selected security flavor.
pub fn build_claude_code_config(flavor: SecurityFlavor) -> ClaudeCodeConfig {
    match flavor {
        SecurityFlavor::Sandbox => ClaudeCodeConfig {
            sandbox: SandboxConfig {
                enabled: Some(true),
                auto_allow: Some(true),
                excluded_commands: vec!["docker".to_string()],
                filesystem: SandboxFilesystemConfig::default(),
                network: SandboxNetworkConfig {
                    allowed_domains: vec!["github.com".to_string()],
                },
                ..Default::default()
            },
            ..Default::default()
        },
        SecurityFlavor::Permissions => ClaudeCodeConfig {
            allowed_tools: vec!["Bash(gh issue *)".to_string(), "Bash(gh run *)".to_string()],
            ..Default::default()
        },
        SecurityFlavor::Both => ClaudeCodeConfig {
            allowed_tools: vec!["WebFetch(domain:docs.rs)".to_string()],
            sandbox: SandboxConfig {
                enabled: Some(true),
                auto_allow: Some(true),
                excluded_commands: vec!["docker".to_string()],
                filesystem: SandboxFilesystemConfig::default(),
                network: SandboxNetworkConfig {
                    allowed_domains: vec!["github.com".to_string()],
                },
                ..Default::default()
            },
            ..Default::default()
        },
        SecurityFlavor::Skip => ClaudeCodeConfig::default(),
    }
}

/// Return commented-out preset examples for the given flavor.
pub fn preset_comment_block(flavor: SecurityFlavor) -> &'static str {
    match flavor {
        SecurityFlavor::Sandbox => {
            "\
\n# Named presets — select per workspace with: loom new my-ws --preset rust
# [agents.claude-code.presets.rust.sandbox.filesystem]
# allow_write = [\"~/.cargo\"]
#
# [agents.claude-code.presets.rust.sandbox.network]
# allowed_domains = [\"docs.rs\", \"crates.io\"]
"
        }
        SecurityFlavor::Permissions => {
            "\
\n# Named presets — select per workspace with: loom new my-ws --preset rust
# [agents.claude-code.presets.rust]
# allowed_tools = [
#     \"Bash(cargo test *)\",
#     \"Bash(cargo fmt *)\",
#     \"Bash(cargo clippy *)\",
# ]
"
        }
        SecurityFlavor::Both => {
            "\
\n# Named presets — select per workspace with: loom new my-ws --preset rust
# [agents.claude-code.presets.rust]
# allowed_tools = []
#
# [agents.claude-code.presets.rust.sandbox.filesystem]
# allow_write = [\"~/.cargo\"]
#
# [agents.claude-code.presets.rust.sandbox.network]
# allowed_domains = [\"docs.rs\", \"crates.io\"]
"
        }
        SecurityFlavor::Skip => {
            "\
\n# --- Claude Code agent settings ---
# Choose a security flavor and uncomment the relevant sections.
#
# Sandbox — OS-level isolation:
# [agents.claude-code.sandbox]
# enabled = true
# auto_allow = true
# excluded_commands = [\"docker\"]
#
# [agents.claude-code.sandbox.filesystem]
# allow_write = []
#
# [agents.claude-code.sandbox.network]
# allowed_domains = [\"github.com\"]
#
# Permissions — explicit tool allowlists:
# [agents.claude-code]
# allowed_tools = [
#     \"Bash(gh issue *)\",
#     \"Bash(gh run *)\",
# ]
#
# Named presets — select per workspace with: loom new my-ws --preset rust
# [agents.claude-code.presets.rust]
# allowed_tools = [
#     \"Bash(cargo test *)\",
#     \"Bash(cargo fmt *)\",
#     \"Bash(cargo clippy *)\",
# ]
#
# [agents.claude-code.presets.rust.sandbox.filesystem]
# allow_write = [\"~/.cargo\"]
#
# [agents.claude-code.presets.rust.sandbox.network]
# allowed_domains = [\"docs.rs\", \"crates.io\"]
"
        }
    }
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
    claude_code: ClaudeCodeConfig,
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
            claude_code,
        },
    };

    Ok(config)
}

/// Save config to the default path with commented preset examples appended.
///
/// Used for fresh `loom init` where no config exists yet.
pub fn save_init_config(config: &Config, flavor: SecurityFlavor) -> Result<()> {
    let path = Config::path()?;
    save_init_config_to(config, flavor, &path)
}

/// Save config to a specific path with commented preset examples appended.
pub fn save_init_config_to(config: &Config, flavor: SecurityFlavor, path: &Path) -> Result<()> {
    let mut content =
        toml::to_string_pretty(config).context("Failed to serialize config to TOML")?;

    content.push_str(preset_comment_block(flavor));

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory {}", parent.display()))?;
    }

    // Atomic write
    let parent = path.parent().unwrap_or(Path::new("."));
    let tmp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("Failed to create temp file in {}", parent.display()))?;
    std::fs::write(tmp.path(), content.as_bytes())
        .with_context(|| "Failed to write config to temp file".to_string())?;
    tmp.persist(path)
        .with_context(|| format!("Failed to persist config to {}", path.display()))?;

    Ok(())
}

/// Update an existing config file preserving the `[agents.*]` sections and comments.
///
/// Uses `toml_edit` to parse the existing file, update only the non-agent sections
/// (registry, workspace, terminal, defaults), and write back. The `[agents.claude-code]`
/// section (including commented-out preset examples) is preserved byte-for-byte.
pub fn update_non_agent_config(config: &Config) -> Result<()> {
    let path = Config::path()?;
    update_non_agent_config_at(config, &path)
}

/// Update config at a specific path, preserving agents section and comments.
pub fn update_non_agent_config_at(config: &Config, path: &Path) -> Result<()> {
    let existing = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read existing config at {}", path.display()))?;

    let mut doc: toml_edit::DocumentMut = existing
        .parse()
        .with_context(|| "Failed to parse existing config with toml_edit")?;

    // Update registry.scan_roots
    let mut scan_roots_array = toml_edit::Array::new();
    for root in &config.registry.scan_roots {
        scan_roots_array.push(root.display().to_string());
    }
    doc["registry"]["scan_roots"] = toml_edit::value(scan_roots_array);

    // Update workspace.root
    doc["workspace"]["root"] = toml_edit::value(config.workspace.root.display().to_string());

    // Update terminal
    if let Some(ref terminal) = config.terminal {
        // Ensure [terminal] table exists
        if !doc.contains_key("terminal") {
            doc["terminal"] = toml_edit::Item::Table(toml_edit::Table::new());
        }
        doc["terminal"]["command"] = toml_edit::value(&terminal.command);
    } else if doc.contains_key("terminal") {
        doc.remove("terminal");
    }

    // Update defaults.branch_prefix
    if !doc.contains_key("defaults") {
        doc["defaults"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    doc["defaults"]["branch_prefix"] = toml_edit::value(&config.defaults.branch_prefix);

    // Write back atomically
    let content = doc.to_string();
    let parent = path.parent().unwrap_or(Path::new("."));
    let tmp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("Failed to create temp file in {}", parent.display()))?;
    std::fs::write(tmp.path(), content.as_bytes())
        .with_context(|| "Failed to write config to temp file".to_string())?;
    tmp.persist(path)
        .with_context(|| format!("Failed to persist config to {}", path.display()))?;

    Ok(())
}

/// Create required directories after init.
///
/// Separate from config saving so the CLI can choose the appropriate save strategy
/// (fresh init vs re-init with preservation).
pub fn finalize_init(config: &Config) -> Result<()> {
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
            ClaudeCodeConfig::default(),
        )
        .unwrap();

        assert_eq!(config.registry.scan_roots, vec![scan_root]);
        assert_eq!(config.defaults.branch_prefix, "loom");
        assert!(config.terminal.is_some());
    }

    #[test]
    fn test_create_config_with_sandbox_flavor() {
        let dir = tempfile::tempdir().unwrap();
        let scan_root = dir.path().join("code");
        std::fs::create_dir_all(&scan_root).unwrap();

        let cc = build_claude_code_config(SecurityFlavor::Sandbox);
        let config = create_config(
            vec![scan_root],
            dir.path().join("loom"),
            Some("ghostty".to_string()),
            "loom".to_string(),
            vec!["claude-code".to_string()],
            cc,
        )
        .unwrap();

        assert_eq!(config.agents.claude_code.sandbox.enabled, Some(true));
        assert_eq!(config.agents.claude_code.sandbox.auto_allow, Some(true));
        assert!(config.agents.claude_code.allowed_tools.is_empty());
    }

    #[test]
    fn test_create_config_with_permissions_flavor() {
        let dir = tempfile::tempdir().unwrap();
        let scan_root = dir.path().join("code");
        std::fs::create_dir_all(&scan_root).unwrap();

        let cc = build_claude_code_config(SecurityFlavor::Permissions);
        let config = create_config(
            vec![scan_root],
            dir.path().join("loom"),
            None,
            "loom".to_string(),
            vec!["claude-code".to_string()],
            cc,
        )
        .unwrap();

        assert!(config.agents.claude_code.sandbox.is_empty());
        assert_eq!(config.agents.claude_code.allowed_tools.len(), 2);
    }

    #[test]
    fn test_create_config_with_both_flavor() {
        let dir = tempfile::tempdir().unwrap();
        let scan_root = dir.path().join("code");
        std::fs::create_dir_all(&scan_root).unwrap();

        let cc = build_claude_code_config(SecurityFlavor::Both);
        let config = create_config(
            vec![scan_root],
            dir.path().join("loom"),
            None,
            "loom".to_string(),
            vec!["claude-code".to_string()],
            cc,
        )
        .unwrap();

        assert_eq!(config.agents.claude_code.sandbox.enabled, Some(true));
        assert_eq!(config.agents.claude_code.allowed_tools.len(), 1);
    }

    #[test]
    fn test_create_config_with_skip_flavor() {
        let dir = tempfile::tempdir().unwrap();
        let scan_root = dir.path().join("code");
        std::fs::create_dir_all(&scan_root).unwrap();

        let cc = build_claude_code_config(SecurityFlavor::Skip);
        let config = create_config(
            vec![scan_root],
            dir.path().join("loom"),
            None,
            "loom".to_string(),
            vec!["claude-code".to_string()],
            cc,
        )
        .unwrap();

        assert!(config.agents.claude_code.is_empty());
    }

    #[test]
    fn test_save_init_config_includes_preset_comments() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        let config = Config {
            registry: RegistryConfig {
                scan_roots: vec![PathBuf::from("/code")],
            },
            workspace: WorkspaceConfig {
                root: PathBuf::from("/workspaces"),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: build_claude_code_config(SecurityFlavor::Sandbox),
            },
        };

        save_init_config_to(&config, SecurityFlavor::Sandbox, &config_path).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("# Named presets"));
        assert!(content.contains("# allow_write"));
        assert!(content.contains("[agents.claude-code.sandbox]"));
        assert!(content.contains("enabled = true"));
    }

    #[test]
    fn test_save_init_config_skip_has_all_flavors() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        let config = Config {
            registry: RegistryConfig {
                scan_roots: vec![PathBuf::from("/code")],
            },
            workspace: WorkspaceConfig {
                root: PathBuf::from("/workspaces"),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            agents: AgentsConfig::default(),
        };

        save_init_config_to(&config, SecurityFlavor::Skip, &config_path).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("# Sandbox"));
        assert!(content.contains("# Permissions"));
        assert!(content.contains("# Named presets"));
    }

    #[test]
    fn test_update_non_agent_config_preserves_agents() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        // Write an initial config with agent settings + comments
        let initial = r#"[registry]
scan_roots = ["/old/code"]

[workspace]
root = "/old/workspaces"

[defaults]
branch_prefix = "loom"

[agents]
enabled = ["claude-code"]

[agents.claude-code.sandbox]
enabled = true
auto_allow = true

# Named presets — select per workspace with: loom new my-ws --preset rust
# [agents.claude-code.presets.rust.sandbox.filesystem]
# allow_write = ["~/.cargo"]
"#;
        std::fs::write(&config_path, initial).unwrap();

        // Build a new config with updated non-agent values
        let config = Config {
            registry: RegistryConfig {
                scan_roots: vec![PathBuf::from("/new/code")],
            },
            workspace: WorkspaceConfig {
                root: PathBuf::from("/new/workspaces"),
            },
            sync: None,
            terminal: Some(TerminalConfig {
                command: "wezterm".to_string(),
            }),
            defaults: DefaultsConfig {
                branch_prefix: "dev".to_string(),
            },
            agents: AgentsConfig::default(), // Doesn't matter — agents section is preserved
        };

        update_non_agent_config_at(&config, &config_path).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();

        // Non-agent sections are updated
        assert!(content.contains("/new/code"));
        assert!(content.contains("/new/workspaces"));
        assert!(content.contains("wezterm"));
        assert!(content.contains("\"dev\""));

        // Old values are gone
        assert!(!content.contains("/old/code"));
        assert!(!content.contains("/old/workspaces"));

        // Agent section is preserved (including comments)
        assert!(content.contains("[agents.claude-code.sandbox]"));
        assert!(content.contains("enabled = true"));
        assert!(content.contains("# Named presets"));
        assert!(content.contains("# allow_write"));
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

        // Test directory creation
        std::fs::create_dir_all(&ws_root).unwrap();
        let state_dir = ws_root.join(".loom");
        std::fs::create_dir_all(&state_dir).unwrap();

        assert!(ws_root.exists());
        assert!(state_dir.exists());
    }

    #[test]
    fn test_build_claude_code_config_sandbox() {
        let cc = build_claude_code_config(SecurityFlavor::Sandbox);
        assert_eq!(cc.sandbox.enabled, Some(true));
        assert_eq!(cc.sandbox.auto_allow, Some(true));
        assert_eq!(cc.sandbox.excluded_commands, vec!["docker"]);
        assert_eq!(cc.sandbox.network.allowed_domains, vec!["github.com"]);
        assert!(cc.allowed_tools.is_empty());
        assert!(cc.presets.is_empty());
    }

    #[test]
    fn test_build_claude_code_config_permissions() {
        let cc = build_claude_code_config(SecurityFlavor::Permissions);
        assert!(cc.sandbox.is_empty());
        assert_eq!(cc.allowed_tools.len(), 2);
        assert!(cc.allowed_tools.contains(&"Bash(gh issue *)".to_string()));
    }

    #[test]
    fn test_build_claude_code_config_both() {
        let cc = build_claude_code_config(SecurityFlavor::Both);
        assert_eq!(cc.sandbox.enabled, Some(true));
        assert!(!cc.allowed_tools.is_empty());
    }

    #[test]
    fn test_build_claude_code_config_skip() {
        let cc = build_claude_code_config(SecurityFlavor::Skip);
        assert!(cc.is_empty());
    }

    #[test]
    fn test_preset_comment_block_not_empty() {
        // All flavors produce non-empty comment blocks
        for flavor in [
            SecurityFlavor::Sandbox,
            SecurityFlavor::Permissions,
            SecurityFlavor::Both,
            SecurityFlavor::Skip,
        ] {
            let block = preset_comment_block(flavor);
            assert!(!block.is_empty(), "{flavor:?} should produce comments");
            assert!(
                block.contains('#'),
                "{flavor:?} should contain comment markers"
            );
        }
    }
}
