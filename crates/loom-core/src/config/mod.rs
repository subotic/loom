pub mod init;

use std::collections::BTreeMap;
use std::io::Write;
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
    /// Named repo groups for quick workspace creation.
    /// Each group maps a name to a list of repo names (bare or org/name).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub groups: BTreeMap<String, Vec<String>>,
    /// Per-repo settings (e.g., workflow). Keyed by repo name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub repos: BTreeMap<String, RepoConfig>,
    /// Specs conventions for generated CLAUDE.md.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specs: Option<SpecsConfig>,
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
    /// Root directory for all workspaces (default: ~/workspaces)
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

/// A marketplace source for Claude Code plugins.
/// MVP supports GitHub sources only; other source types can be added later.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketplaceEntry {
    /// Marketplace name (used as key in generated JSON)
    pub name: String,
    /// GitHub repo in "owner/repo" format
    pub repo: String,
}

/// Sandbox filesystem isolation settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SandboxFilesystemConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow_write: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deny_write: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deny_read: Vec<String>,
}

impl SandboxFilesystemConfig {
    pub(crate) fn is_empty(&self) -> bool {
        self.allow_write.is_empty() && self.deny_write.is_empty() && self.deny_read.is_empty()
    }
}

/// Sandbox network isolation settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SandboxNetworkConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_domains: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow_unix_sockets: Vec<String>,
}

impl SandboxNetworkConfig {
    pub(crate) fn is_empty(&self) -> bool {
        self.allowed_domains.is_empty() && self.allow_unix_sockets.is_empty()
    }
}

/// OS-level sandbox configuration for Claude Code.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SandboxConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_allow: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded_commands: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_unsandboxed_commands: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_weaker_network_isolation: Option<bool>,
    #[serde(default, skip_serializing_if = "SandboxFilesystemConfig::is_empty")]
    pub filesystem: SandboxFilesystemConfig,
    #[serde(default, skip_serializing_if = "SandboxNetworkConfig::is_empty")]
    pub network: SandboxNetworkConfig,
}

impl SandboxConfig {
    pub(crate) fn is_empty(&self) -> bool {
        self.enabled.is_none()
            && self.auto_allow.is_none()
            && self.excluded_commands.is_empty()
            && self.allow_unsandboxed_commands.is_none()
            && self.enable_weaker_network_isolation.is_none()
            && self.filesystem.is_empty()
            && self.network.is_empty()
    }
}

/// Sandbox settings within a preset (arrays only, no booleans).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PresetSandboxConfig {
    #[serde(default, skip_serializing_if = "SandboxFilesystemConfig::is_empty")]
    pub filesystem: SandboxFilesystemConfig,
    #[serde(default, skip_serializing_if = "SandboxNetworkConfig::is_empty")]
    pub network: SandboxNetworkConfig,
}

impl PresetSandboxConfig {
    pub(crate) fn is_empty(&self) -> bool {
        self.filesystem.is_empty() && self.network.is_empty()
    }
}

/// A named permission preset (e.g., "rust", "node").
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PermissionPreset {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "PresetSandboxConfig::is_empty")]
    pub sandbox: PresetSandboxConfig,
}

/// Effort level for adaptive reasoning (Opus 4.6 / Sonnet 4.6 only).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EffortLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClaudeCodeConfig {
    /// Claude model alias or full model ID (e.g., "opus", "claude-opus-4-6")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Effort level for adaptive reasoning (e.g., "low", "medium", "high")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort_level: Option<EffortLevel>,

    /// Extra marketplace repos
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_known_marketplaces: Vec<MarketplaceEntry>,

    /// Enabled plugins (e.g., ["pluginName@marketplaceName"])
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enabled_plugins: Vec<String>,

    /// MCP JSON servers to enable (e.g., ["linear", "notion"])
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enabled_mcp_servers: Vec<String>,

    /// Global permission allowlist entries (e.g., ["Bash(cargo test *)"])
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,

    /// Global sandbox configuration
    #[serde(default, skip_serializing_if = "SandboxConfig::is_empty")]
    pub sandbox: SandboxConfig,

    /// Named permission presets (selected per workspace via --preset)
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub presets: BTreeMap<String, PermissionPreset>,
}

impl ClaudeCodeConfig {
    /// Returns true when all fields are empty (used by serde skip_serializing_if and init re-check).
    pub fn is_empty(&self) -> bool {
        self.model.is_none()
            && self.effort_level.is_none()
            && self.extra_known_marketplaces.is_empty()
            && self.enabled_plugins.is_empty()
            && self.enabled_mcp_servers.is_empty()
            && self.allowed_tools.is_empty()
            && self.sandbox.is_empty()
            && self.presets.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsConfig {
    /// Which agents to configure (e.g., ["claude-code"])
    #[serde(default)]
    pub enabled: Vec<String>,

    /// Claude Code-specific settings
    #[serde(
        default,
        rename = "claude-code",
        skip_serializing_if = "ClaudeCodeConfig::is_empty"
    )]
    pub claude_code: ClaudeCodeConfig,
}

impl Default for AgentsConfig {
    fn default() -> Self {
        Self {
            enabled: vec!["claude-code".to_string()],
            claude_code: ClaudeCodeConfig::default(),
        }
    }
}

/// Push workflow for a repository.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Workflow {
    /// Create a branch off origin/main, commit, push, open a PR.
    #[default]
    Pr,
    /// Commit on the workspace branch, push directly to main.
    Push,
}

impl std::fmt::Display for Workflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Workflow::Pr => write!(f, "pr"),
            Workflow::Push => write!(f, "push"),
        }
    }
}

impl Workflow {
    /// Human-readable label for the repos table (e.g., "PR to `main`").
    pub fn label(self, default_branch: &str) -> String {
        match self {
            Workflow::Pr => format!("PR to `{default_branch}`"),
            Workflow::Push => format!("Push to `{default_branch}`"),
        }
    }
}

/// Per-repo configuration, keyed by repo name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoConfig {
    /// Push workflow: Pr (default) or Push.
    #[serde(default)]
    pub workflow: Workflow,
}

/// Specs conventions for generated CLAUDE.md.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecsConfig {
    /// Path to specs directory, relative to workspace root.
    pub path: String,
}

fn default_branch_prefix() -> String {
    "loom".to_string()
}

/// Validate that a preset name exists in the config's preset map.
///
/// Returns `Ok(())` if the preset exists, or a descriptive error listing available presets.
pub fn validate_preset_exists(
    presets: &BTreeMap<String, PermissionPreset>,
    preset_name: &str,
) -> Result<()> {
    if presets.contains_key(preset_name) {
        return Ok(());
    }
    let available: Vec<&str> = presets.keys().map(|s| s.as_str()).collect();
    if available.is_empty() {
        anyhow::bail!(
            "Preset '{}' not found. No presets defined in config.toml.",
            preset_name
        );
    } else {
        anyhow::bail!(
            "Preset '{}' not found. Available presets: {}",
            preset_name,
            available.join(", ")
        );
    }
}

/// Validate a Claude Code permission entry has the form `ToolName(specifier)`.
fn validate_permission_entry(entry: &str, context: &str) -> Result<()> {
    let trimmed = entry.trim();
    if trimmed.is_empty() {
        anyhow::bail!("{context}: permission entry cannot be empty");
    }
    if !trimmed.ends_with(')') || !trimmed.contains('(') {
        anyhow::bail!("{context}: invalid format '{trimmed}' — expected ToolName(specifier)");
    }
    let paren_idx = trimmed.find('(').expect("already checked for '('");
    let specifier = &trimmed[paren_idx + 1..trimmed.len() - 1];
    if specifier.trim().is_empty() {
        anyhow::bail!("{context}: specifier in '{trimmed}' cannot be empty");
    }
    let tool_name = &trimmed[..paren_idx];
    if !tool_name.starts_with("mcp__")
        && !tool_name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase())
    {
        anyhow::bail!(
            "{context}: tool name must start with uppercase letter or 'mcp__', got '{tool_name}'"
        );
    }
    Ok(())
}

/// Validate that no entry in a string list is empty or whitespace-only.
fn validate_no_empty_entries(entries: &[String], context: &str) -> Result<()> {
    for entry in entries {
        if entry.trim().is_empty() {
            anyhow::bail!("{context}: entries cannot be empty or whitespace-only");
        }
    }
    Ok(())
}

/// Validate that a string list has no duplicates.
fn validate_no_duplicates(entries: &[String], context: &str) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for entry in entries {
        if !seen.insert(entry) {
            anyhow::bail!("{context}: duplicate entry '{entry}'");
        }
    }
    Ok(())
}

/// Validate that a path has no parent-directory (`..`) components and is not absolute.
pub(crate) fn validate_no_path_traversal(path: &str, context: &str) -> Result<()> {
    use std::path::{Component, Path as StdPath};
    let p = StdPath::new(path);
    if p.is_absolute() {
        anyhow::bail!("{context}: path must not be absolute: '{path}'");
    }
    if p.components().any(|c| c == Component::ParentDir) {
        anyhow::bail!("{context}: path must not contain '..' components: '{path}'");
    }
    Ok(())
}

/// Validate sandbox filesystem and network entries for a given context prefix.
fn validate_sandbox_entries(
    fs: &SandboxFilesystemConfig,
    net: &SandboxNetworkConfig,
    context: &str,
) -> Result<()> {
    validate_no_empty_entries(
        &fs.allow_write,
        &format!("{context}.sandbox.filesystem.allow_write"),
    )?;
    validate_no_empty_entries(
        &fs.deny_write,
        &format!("{context}.sandbox.filesystem.deny_write"),
    )?;
    validate_no_empty_entries(
        &fs.deny_read,
        &format!("{context}.sandbox.filesystem.deny_read"),
    )?;
    validate_no_empty_entries(
        &net.allowed_domains,
        &format!("{context}.sandbox.network.allowed_domains"),
    )?;
    validate_no_empty_entries(
        &net.allow_unix_sockets,
        &format!("{context}.sandbox.network.allow_unix_sockets"),
    )?;
    validate_no_duplicates(
        &net.allow_unix_sockets,
        &format!("{context}.sandbox.network.allow_unix_sockets"),
    )?;
    for entry in &net.allow_unix_sockets {
        if !entry.starts_with('/') {
            anyhow::bail!(
                "{context}.sandbox.network.allow_unix_sockets: '{}' must be an absolute path",
                entry
            );
        }
        let p = Path::new(entry);
        if p.components().any(|c| c == std::path::Component::ParentDir) {
            anyhow::bail!(
                "{context}.sandbox.network.allow_unix_sockets: '{}' must not contain '..' components",
                entry
            );
        }
    }
    Ok(())
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
        let mut tmp = tempfile::NamedTempFile::new_in(parent)
            .with_context(|| format!("Failed to create temp file in {}", parent.display()))?;
        tmp.write_all(content.as_bytes())
            .with_context(|| "Failed to write config to temp file")?;
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
                root: PathBuf::from("~/workspaces"),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
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

    /// Validate only the agent config section (no path-existence checks).
    ///
    /// Use this during `loom init` where paths may not exist yet.
    /// Validate agent permission and sandbox syntax only.
    ///
    /// This checks allowed_tools format, sandbox paths, and preset entries without
    /// requiring that scan_roots or workspace.root exist on disk. Safe to call at
    /// init time before directories are created. Does **not** check marketplace/plugin
    /// entries — use [`validate()`](Self::validate) for the full post-load check.
    pub fn validate_agent_config(&self) -> Result<()> {
        let cc = &self.agents.claude_code;
        if let Some(ref model) = cc.model
            && model.trim().is_empty()
        {
            anyhow::bail!("agents.claude-code.model cannot be empty or whitespace-only");
        }
        for entry in &cc.allowed_tools {
            validate_permission_entry(entry, "agents.claude-code.allowed_tools")?;
        }
        validate_no_duplicates(&cc.allowed_tools, "agents.claude-code.allowed_tools")?;
        validate_sandbox_entries(
            &cc.sandbox.filesystem,
            &cc.sandbox.network,
            "agents.claude-code",
        )?;
        validate_no_empty_entries(
            &cc.sandbox.excluded_commands,
            "agents.claude-code.sandbox.excluded_commands",
        )?;
        validate_no_empty_entries(
            &cc.enabled_mcp_servers,
            "agents.claude-code.enabled_mcp_servers",
        )?;
        validate_no_duplicates(
            &cc.enabled_mcp_servers,
            "agents.claude-code.enabled_mcp_servers",
        )?;
        for (name, preset) in &cc.presets {
            let ctx = format!("agents.claude-code.presets.{name}");
            for entry in &preset.allowed_tools {
                validate_permission_entry(entry, &format!("{ctx}.allowed_tools"))?;
            }
            validate_no_duplicates(&preset.allowed_tools, &format!("{ctx}.allowed_tools"))?;
            validate_sandbox_entries(&preset.sandbox.filesystem, &preset.sandbox.network, &ctx)?;
        }
        Ok(())
    }

    /// Full post-load validation: path existence, branch prefix, marketplace/plugin
    /// entries, and all agent config (delegates to [`validate_agent_config()`](Self::validate_agent_config)).
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

        // Validate marketplace entries
        let mut seen_names = std::collections::HashSet::new();
        for entry in &self.agents.claude_code.extra_known_marketplaces {
            if entry.name.is_empty() {
                anyhow::bail!(
                    "agents.claude-code.extra_known_marketplaces: marketplace name cannot be empty."
                );
            }
            match entry.repo.split_once('/') {
                Some((owner, repo))
                    if !owner.is_empty() && !repo.is_empty() && !repo.contains('/') => {}
                _ => {
                    anyhow::bail!(
                        "agents.claude-code.extra_known_marketplaces: repo '{}' must be in 'owner/repo' format.",
                        entry.repo
                    );
                }
            }
            if !seen_names.insert(&entry.name) {
                anyhow::bail!(
                    "agents.claude-code.extra_known_marketplaces: duplicate marketplace name '{}'.",
                    entry.name
                );
            }
        }

        // Validate enabled_plugins format
        for plugin in &self.agents.claude_code.enabled_plugins {
            match plugin.split_once('@') {
                Some((name, marketplace)) if !name.is_empty() && !marketplace.is_empty() => {}
                _ => {
                    anyhow::bail!(
                        "agents.claude-code.enabled_plugins: '{}' must be in 'pluginName@marketplaceName' format.",
                        plugin
                    );
                }
            }
        }

        // Validate repos config keys
        for key in self.repos.keys() {
            if key.trim().is_empty() {
                anyhow::bail!("repos: key must not be empty or whitespace-only");
            }
        }

        // Validate groups.
        // Note: group repo names are NOT cross-validated against the registry here
        // because the registry is discovered at runtime from scan_roots (not available
        // at config load time). Validation of repo names happens at workspace creation.
        for (name, repos) in &self.groups {
            crate::manifest::validate_name(name)
                .with_context(|| format!("groups: invalid group name '{name}'"))?;
            if repos.is_empty() {
                anyhow::bail!("groups.{name}: group must contain at least one repo");
            }
            validate_no_empty_entries(repos, &format!("groups.{name}"))?;
            validate_no_duplicates(repos, &format!("groups.{name}"))?;
        }

        // Validate specs config
        if let Some(specs) = &self.specs {
            if specs.path.trim().is_empty() {
                anyhow::bail!("specs.path must not be empty");
            }
            validate_no_path_traversal(&specs.path, "specs.path")?;
        }

        // Validate agent config (permissions, sandbox, presets)
        self.validate_agent_config()?;

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
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                ..Default::default()
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
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
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
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
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
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
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
        assert_eq!(config.workspace.root, PathBuf::from("~/workspaces"));
        assert_eq!(config.defaults.branch_prefix, "loom");
        assert_eq!(config.agents.enabled, vec!["claude-code"]);
    }

    #[test]
    fn test_claude_code_config_round_trip() {
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
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    extra_known_marketplaces: vec![
                        MarketplaceEntry {
                            name: "my-plugins".to_string(),
                            repo: "owner/my-plugins".to_string(),
                        },
                        MarketplaceEntry {
                            name: "team-plugins".to_string(),
                            repo: "org/team-plugins".to_string(),
                        },
                    ],
                    enabled_plugins: vec![
                        "pkm@my-plugins".to_string(),
                        "eng@team-plugins".to_string(),
                    ],
                    enabled_mcp_servers: vec!["linear".to_string(), "notion".to_string()],
                    ..Default::default()
                },
            },
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(
            parsed.agents.claude_code.extra_known_marketplaces,
            config.agents.claude_code.extra_known_marketplaces
        );
        assert_eq!(
            parsed.agents.claude_code.enabled_plugins,
            config.agents.claude_code.enabled_plugins
        );
        assert_eq!(
            parsed.agents.claude_code.enabled_mcp_servers,
            config.agents.claude_code.enabled_mcp_servers
        );
    }

    #[test]
    fn test_claude_code_config_empty_suppressed_in_toml() {
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
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        // Empty ClaudeCodeConfig should not produce [agents.claude-code] section header
        assert!(
            !toml_str.contains("[agents.claude-code]"),
            "Empty claude-code config section should be suppressed in TOML:\n{toml_str}"
        );
    }

    #[test]
    fn test_missing_claude_code_section_deserializes() {
        let toml_str = r#"
[registry]
scan_roots = ["/code"]

[workspace]
root = "/loom"

[agents]
enabled = ["claude-code"]
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(
            config
                .agents
                .claude_code
                .extra_known_marketplaces
                .is_empty()
        );
        assert!(config.agents.claude_code.enabled_plugins.is_empty());
    }

    #[test]
    fn test_validate_duplicate_marketplace_name() {
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
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    extra_known_marketplaces: vec![
                        MarketplaceEntry {
                            name: "dupe".to_string(),
                            repo: "owner/repo1".to_string(),
                        },
                        MarketplaceEntry {
                            name: "dupe".to_string(),
                            repo: "owner/repo2".to_string(),
                        },
                    ],
                    ..Default::default()
                },
            },
        };

        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("duplicate marketplace name"));
    }

    #[test]
    fn test_validate_empty_marketplace_name() {
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
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    extra_known_marketplaces: vec![MarketplaceEntry {
                        name: String::new(),
                        repo: "owner/repo".to_string(),
                    }],
                    ..Default::default()
                },
            },
        };

        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("name cannot be empty"));
    }

    #[test]
    fn test_validate_invalid_marketplace_repo() {
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
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    extra_known_marketplaces: vec![MarketplaceEntry {
                        name: "test".to_string(),
                        repo: "no-slash".to_string(),
                    }],
                    ..Default::default()
                },
            },
        };

        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("owner/repo"));
    }

    #[test]
    fn test_validate_repo_with_multiple_slashes() {
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
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    extra_known_marketplaces: vec![MarketplaceEntry {
                        name: "test".to_string(),
                        repo: "a/b/c".to_string(),
                    }],
                    ..Default::default()
                },
            },
        };

        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("owner/repo"));
    }

    #[test]
    fn test_validate_plugin_empty_parts() {
        let dir = tempfile::tempdir().unwrap();
        // "@marketplace" — empty plugin name
        let config = Config {
            registry: RegistryConfig {
                scan_roots: vec![dir.path().to_path_buf()],
            },
            workspace: WorkspaceConfig {
                root: dir.path().to_path_buf(),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    enabled_plugins: vec!["@marketplace".to_string()],
                    ..Default::default()
                },
            },
        };
        assert!(config.validate().is_err());

        // "plugin@" — empty marketplace name
        let config2 = Config {
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    enabled_plugins: vec!["plugin@".to_string()],
                    ..Default::default()
                },
            },
            ..config.clone()
        };
        assert!(config2.validate().is_err());

        // "@" — both parts empty
        let config3 = Config {
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    enabled_plugins: vec!["@".to_string()],
                    ..Default::default()
                },
            },
            ..config
        };
        assert!(config3.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_plugin_format() {
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
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    enabled_plugins: vec!["no-at-sign".to_string()],
                    ..Default::default()
                },
            },
        };

        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("pluginName@marketplaceName"));
    }

    #[test]
    fn test_allowed_tools_round_trip() {
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
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    allowed_tools: vec![
                        "Bash(cargo test *)".to_string(),
                        "WebFetch(domain:docs.rs)".to_string(),
                    ],
                    ..Default::default()
                },
            },
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(
            parsed.agents.claude_code.allowed_tools,
            config.agents.claude_code.allowed_tools
        );
    }

    #[test]
    fn test_sandbox_config_round_trip() {
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
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    sandbox: SandboxConfig {
                        enabled: Some(true),
                        auto_allow: Some(true),
                        excluded_commands: vec!["docker".to_string()],
                        allow_unsandboxed_commands: Some(false),
                        enable_weaker_network_isolation: None,
                        filesystem: SandboxFilesystemConfig {
                            allow_write: vec!["~/.cargo".to_string()],
                            deny_write: vec![],
                            deny_read: vec![],
                        },
                        network: SandboxNetworkConfig {
                            allowed_domains: vec!["github.com".to_string()],
                            allow_unix_sockets: vec!["/tmp/ssh-agent.sock".to_string()],
                        },
                    },
                    ..Default::default()
                },
            },
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(
            parsed.agents.claude_code.sandbox,
            config.agents.claude_code.sandbox
        );
    }

    #[test]
    fn test_presets_round_trip() {
        let mut presets = BTreeMap::new();
        presets.insert(
            "rust".to_string(),
            PermissionPreset {
                allowed_tools: vec![
                    "Bash(cargo test *)".to_string(),
                    "Bash(cargo clippy *)".to_string(),
                ],
                sandbox: PresetSandboxConfig {
                    filesystem: SandboxFilesystemConfig {
                        allow_write: vec!["~/.cargo".to_string()],
                        ..Default::default()
                    },
                    network: SandboxNetworkConfig {
                        allowed_domains: vec!["docs.rs".to_string(), "crates.io".to_string()],
                        allow_unix_sockets: vec![],
                    },
                },
            },
        );

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
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    presets,
                    ..Default::default()
                },
            },
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(
            parsed.agents.claude_code.presets,
            config.agents.claude_code.presets
        );
    }

    #[test]
    fn test_sandbox_empty_suppressed() {
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
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(
            !toml_str.contains("sandbox"),
            "Empty sandbox should be suppressed:\n{toml_str}"
        );
        assert!(
            !toml_str.contains("allowed_tools"),
            "Empty allowed_tools should be suppressed:\n{toml_str}"
        );
        assert!(
            !toml_str.contains("presets"),
            "Empty presets should be suppressed:\n{toml_str}"
        );
    }

    #[test]
    fn test_sandbox_enabled_only_is_not_empty() {
        let sandbox = SandboxConfig {
            enabled: Some(true),
            ..Default::default()
        };
        assert!(
            !sandbox.is_empty(),
            "sandbox with enabled=true should not be empty"
        );
    }

    #[test]
    fn test_validate_permission_entries() {
        assert!(validate_permission_entry("Bash(cargo test *)", "test").is_ok());
        assert!(validate_permission_entry("mcp__slack__send(channel *)", "test").is_ok());
        assert!(validate_permission_entry("WebFetch(domain:docs.rs)", "test").is_ok());
        assert!(validate_permission_entry("Skill(eng:workflows:plan)", "test").is_ok());

        // Invalid cases
        assert!(validate_permission_entry("", "test").is_err());
        assert!(validate_permission_entry("   ", "test").is_err());
        assert!(validate_permission_entry("Bash", "test").is_err());
        assert!(validate_permission_entry("bash(cargo test *)", "test").is_err());
        // Empty specifier
        assert!(validate_permission_entry("Bash()", "test").is_err());
        assert!(validate_permission_entry("Bash(  )", "test").is_err());
    }

    #[test]
    fn test_validate_allowed_tools_duplicates() {
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
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    allowed_tools: vec![
                        "Bash(cargo test *)".to_string(),
                        "Bash(cargo test *)".to_string(),
                    ],
                    ..Default::default()
                },
            },
        };

        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("duplicate"));
    }

    #[test]
    fn test_validate_empty_model_rejected() {
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
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    model: Some("  ".to_string()),
                    ..Default::default()
                },
            },
        };

        let err = config.validate_agent_config().unwrap_err();
        assert!(
            err.to_string()
                .contains("cannot be empty or whitespace-only")
        );
    }

    #[test]
    fn test_validate_sandbox_empty_path() {
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
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    sandbox: SandboxConfig {
                        filesystem: SandboxFilesystemConfig {
                            allow_write: vec!["  ".to_string()],
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            },
        };

        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("empty or whitespace"));
    }

    #[test]
    fn test_validate_allow_unix_sockets_empty_entry() {
        let config = Config {
            registry: RegistryConfig { scan_roots: vec![] },
            workspace: WorkspaceConfig {
                root: PathBuf::from("/loom"),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    sandbox: SandboxConfig {
                        network: SandboxNetworkConfig {
                            allow_unix_sockets: vec!["  ".to_string()],
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            },
        };

        let err = config.validate_agent_config().unwrap_err();
        assert!(err.to_string().contains("empty or whitespace"));
        assert!(err.to_string().contains("allow_unix_sockets"));
    }

    #[test]
    fn test_validate_allow_unix_sockets_duplicate() {
        let config = Config {
            registry: RegistryConfig { scan_roots: vec![] },
            workspace: WorkspaceConfig {
                root: PathBuf::from("/loom"),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    sandbox: SandboxConfig {
                        network: SandboxNetworkConfig {
                            allow_unix_sockets: vec![
                                "/tmp/sock".to_string(),
                                "/tmp/sock".to_string(),
                            ],
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            },
        };

        let err = config.validate_agent_config().unwrap_err();
        assert!(err.to_string().contains("duplicate"));
        assert!(err.to_string().contains("allow_unix_sockets"));
    }

    #[test]
    fn test_validate_allow_unix_sockets_relative_path() {
        let config = Config {
            registry: RegistryConfig { scan_roots: vec![] },
            workspace: WorkspaceConfig {
                root: PathBuf::from("/loom"),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    sandbox: SandboxConfig {
                        network: SandboxNetworkConfig {
                            allow_unix_sockets: vec!["relative/path.sock".to_string()],
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            },
        };

        let err = config.validate_agent_config().unwrap_err();
        assert!(err.to_string().contains("must be an absolute path"));
    }

    #[test]
    fn test_validate_allow_unix_sockets_parent_dir() {
        let config = Config {
            registry: RegistryConfig { scan_roots: vec![] },
            workspace: WorkspaceConfig {
                root: PathBuf::from("/loom"),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    sandbox: SandboxConfig {
                        network: SandboxNetworkConfig {
                            allow_unix_sockets: vec!["/tmp/../etc/sock".to_string()],
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            },
        };

        let err = config.validate_agent_config().unwrap_err();
        assert!(err.to_string().contains("must not contain '..'"));
    }

    #[test]
    fn test_validate_enabled_mcp_servers_empty_entry() {
        let config = Config {
            registry: RegistryConfig { scan_roots: vec![] },
            workspace: WorkspaceConfig {
                root: PathBuf::from("/loom"),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    enabled_mcp_servers: vec!["".to_string()],
                    ..Default::default()
                },
            },
        };

        let err = config.validate_agent_config().unwrap_err();
        assert!(err.to_string().contains("empty or whitespace"));
        assert!(err.to_string().contains("enabled_mcp_servers"));
    }

    #[test]
    fn test_validate_enabled_mcp_servers_duplicates() {
        let config = Config {
            registry: RegistryConfig { scan_roots: vec![] },
            workspace: WorkspaceConfig {
                root: PathBuf::from("/loom"),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    enabled_mcp_servers: vec!["linear".to_string(), "linear".to_string()],
                    ..Default::default()
                },
            },
        };

        let err = config.validate_agent_config().unwrap_err();
        assert!(err.to_string().contains("duplicate"));
        assert!(err.to_string().contains("enabled_mcp_servers"));
    }

    #[test]
    fn test_validate_preset_invalid_permission() {
        let dir = tempfile::tempdir().unwrap();
        let mut presets = BTreeMap::new();
        presets.insert(
            "bad".to_string(),
            PermissionPreset {
                allowed_tools: vec!["bash(lowercase)".to_string()],
                ..Default::default()
            },
        );

        let config = Config {
            registry: RegistryConfig {
                scan_roots: vec![dir.path().to_path_buf()],
            },
            workspace: WorkspaceConfig {
                root: dir.path().to_path_buf(),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    presets,
                    ..Default::default()
                },
            },
        };

        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("presets.bad"));
    }

    #[test]
    fn test_full_config_round_trip() {
        let mut presets = BTreeMap::new();
        presets.insert(
            "rust".to_string(),
            PermissionPreset {
                allowed_tools: vec!["Bash(cargo test *)".to_string()],
                sandbox: PresetSandboxConfig {
                    filesystem: SandboxFilesystemConfig {
                        allow_write: vec!["~/.cargo".to_string()],
                        ..Default::default()
                    },
                    network: SandboxNetworkConfig {
                        allowed_domains: vec!["docs.rs".to_string()],
                        allow_unix_sockets: vec!["/tmp/preset.sock".to_string()],
                    },
                },
            },
        );

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
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
                claude_code: ClaudeCodeConfig {
                    extra_known_marketplaces: vec![MarketplaceEntry {
                        name: "test".to_string(),
                        repo: "org/test".to_string(),
                    }],
                    enabled_plugins: vec!["eng@test".to_string()],
                    enabled_mcp_servers: vec!["linear".to_string()],
                    allowed_tools: vec!["Bash(gh issue *)".to_string()],
                    sandbox: SandboxConfig {
                        enabled: Some(true),
                        auto_allow: Some(true),
                        excluded_commands: vec!["docker".to_string()],
                        allow_unsandboxed_commands: None,
                        enable_weaker_network_isolation: None,
                        filesystem: SandboxFilesystemConfig {
                            allow_write: vec!["~/.config/loom".to_string()],
                            deny_write: vec![],
                            deny_read: vec![],
                        },
                        network: SandboxNetworkConfig {
                            allowed_domains: vec!["github.com".to_string()],
                            allow_unix_sockets: vec!["/tmp/global.sock".to_string()],
                        },
                    },
                    presets,
                    ..Default::default()
                },
            },
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(
            parsed.agents.claude_code.allowed_tools,
            config.agents.claude_code.allowed_tools
        );
        assert_eq!(
            parsed.agents.claude_code.sandbox,
            config.agents.claude_code.sandbox
        );
        assert_eq!(
            parsed.agents.claude_code.presets,
            config.agents.claude_code.presets
        );
    }

    #[test]
    fn test_workflow_serde_valid_values() {
        let toml_pr = r#"workflow = "pr""#;
        let parsed: RepoConfig = toml::from_str(toml_pr).unwrap();
        assert_eq!(parsed.workflow, Workflow::Pr);

        let toml_push = r#"workflow = "push""#;
        let parsed: RepoConfig = toml::from_str(toml_push).unwrap();
        assert_eq!(parsed.workflow, Workflow::Push);
    }

    #[test]
    fn test_workflow_default_is_pr() {
        // Missing workflow field should default to Pr
        let toml_str = "";
        let parsed: RepoConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(parsed.workflow, Workflow::Pr);
    }

    #[test]
    fn test_workflow_invalid_value_rejected() {
        let toml_str = r#"workflow = "merge""#;
        let result: Result<RepoConfig, _> = toml::from_str(toml_str);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("merge") || err.contains("unknown variant"),
            "Error should mention the invalid value: {err}"
        );
    }

    #[test]
    fn test_workflow_uppercase_rejected() {
        let toml_str = r#"workflow = "PR""#;
        let result: Result<RepoConfig, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_workflow_label() {
        assert_eq!(Workflow::Pr.label("main"), "PR to `main`");
        assert_eq!(Workflow::Push.label("main"), "Push to `main`");
        assert_eq!(Workflow::Pr.label("develop"), "PR to `develop`");
    }

    #[test]
    fn test_validate_specs_empty_path() {
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
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: Some(SpecsConfig {
                path: "  ".to_string(),
            }),
            agents: AgentsConfig::default(),
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("must not be empty"));
    }

    #[test]
    fn test_validate_specs_path_traversal() {
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
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: Some(SpecsConfig {
                path: "../etc/passwd".to_string(),
            }),
            agents: AgentsConfig::default(),
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains(".."));
    }

    #[test]
    fn test_validate_specs_absolute_path() {
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
            defaults: DefaultsConfig::default(),
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: Some(SpecsConfig {
                path: "/etc/passwd".to_string(),
            }),
            agents: AgentsConfig::default(),
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("absolute"));
    }

    #[test]
    fn test_validate_no_path_traversal_accepts_legitimate_paths() {
        // Dots in filenames are fine
        assert!(validate_no_path_traversal("v2..3/file", "test").is_ok());
        assert!(validate_no_path_traversal(".hidden/config", "test").is_ok());
        assert!(validate_no_path_traversal("pkm/01 - PROJECTS/specs", "test").is_ok());
    }

    #[test]
    fn test_validate_no_path_traversal_rejects_bad_paths() {
        assert!(validate_no_path_traversal("../etc/passwd", "test").is_err());
        assert!(validate_no_path_traversal("foo/../../bar", "test").is_err());
        assert!(validate_no_path_traversal("/etc/passwd", "test").is_err());
    }

    #[test]
    fn test_toml_round_trip_with_repos_and_specs() {
        let mut repos = BTreeMap::new();
        repos.insert(
            "loom".to_string(),
            RepoConfig {
                workflow: Workflow::Pr,
            },
        );
        repos.insert(
            "pkm".to_string(),
            RepoConfig {
                workflow: Workflow::Push,
            },
        );

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
            groups: BTreeMap::new(),
            repos,
            specs: Some(SpecsConfig {
                path: "pkm/01 - PROJECTS/Personal/LOOM/specs".to_string(),
            }),
            agents: AgentsConfig::default(),
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.repos.len(), 2);
        assert_eq!(parsed.repos["loom"].workflow, Workflow::Pr);
        assert_eq!(parsed.repos["pkm"].workflow, Workflow::Push);
        assert_eq!(
            parsed.specs.as_ref().unwrap().path,
            "pkm/01 - PROJECTS/Personal/LOOM/specs"
        );
    }

    #[test]
    fn test_toml_round_trip_hyphenated_repo_names() {
        let mut repos = BTreeMap::new();
        repos.insert(
            "dsp-api".to_string(),
            RepoConfig {
                workflow: Workflow::Push,
            },
        );

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
            groups: BTreeMap::new(),
            repos,
            specs: None,
            agents: AgentsConfig::default(),
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[repos.dsp-api]"));

        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.repos["dsp-api"].workflow, Workflow::Push);
    }

    #[test]
    fn test_repos_and_specs_suppressed_when_empty() {
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
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(
            !toml_str.contains("[repos"),
            "Empty repos should be suppressed:\n{toml_str}"
        );
        assert!(
            !toml_str.contains("[specs"),
            "None specs should be suppressed:\n{toml_str}"
        );
    }

    #[test]
    fn test_groups_toml_round_trip() {
        let mut groups = BTreeMap::new();
        groups.insert(
            "dsp-stack".to_string(),
            vec![
                "dsp-api".to_string(),
                "dsp-das".to_string(),
                "sipi".to_string(),
            ],
        );
        groups.insert(
            "infra".to_string(),
            vec!["dsp-api".to_string(), "dsp-tools".to_string()],
        );

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
            groups: groups.clone(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[groups]"));
        assert!(toml_str.contains("dsp-stack"));
        assert!(toml_str.contains("infra"));

        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.groups, groups);
    }

    #[test]
    fn test_groups_empty_suppressed_in_toml() {
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
            groups: BTreeMap::new(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(
            !toml_str.contains("[groups"),
            "Empty groups should be suppressed:\n{toml_str}"
        );
    }

    #[test]
    fn test_groups_missing_from_toml_defaults_to_empty() {
        let toml_str = r#"
[registry]
scan_roots = ["/code"]

[workspace]
root = "/loom"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.groups.is_empty());
    }

    #[test]
    fn test_validate_groups_empty_group() {
        let dir = tempfile::tempdir().unwrap();
        let mut groups = BTreeMap::new();
        groups.insert("empty-group".to_string(), vec![]);

        let config = Config {
            registry: RegistryConfig {
                scan_roots: vec![dir.path().to_path_buf()],
            },
            workspace: WorkspaceConfig {
                root: dir.path().to_path_buf(),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            groups,
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        };

        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("must contain at least one repo"));
    }

    #[test]
    fn test_validate_groups_invalid_name() {
        let dir = tempfile::tempdir().unwrap();
        let mut groups = BTreeMap::new();
        groups.insert("UPPERCASE".to_string(), vec!["some-repo".to_string()]);

        let config = Config {
            registry: RegistryConfig {
                scan_roots: vec![dir.path().to_path_buf()],
            },
            workspace: WorkspaceConfig {
                root: dir.path().to_path_buf(),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            groups,
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        };

        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("invalid group name"));
    }

    #[test]
    fn test_validate_groups_duplicate_entries() {
        let dir = tempfile::tempdir().unwrap();
        let mut groups = BTreeMap::new();
        groups.insert(
            "dupes".to_string(),
            vec!["repo-a".to_string(), "repo-a".to_string()],
        );

        let config = Config {
            registry: RegistryConfig {
                scan_roots: vec![dir.path().to_path_buf()],
            },
            workspace: WorkspaceConfig {
                root: dir.path().to_path_buf(),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            groups,
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        };

        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("duplicate entry"));
    }

    #[test]
    fn test_validate_groups_empty_entry() {
        let dir = tempfile::tempdir().unwrap();
        let mut groups = BTreeMap::new();
        groups.insert(
            "has-empty".to_string(),
            vec!["repo-a".to_string(), "  ".to_string()],
        );

        let config = Config {
            registry: RegistryConfig {
                scan_roots: vec![dir.path().to_path_buf()],
            },
            workspace: WorkspaceConfig {
                root: dir.path().to_path_buf(),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            groups,
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        };

        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("empty or whitespace-only"));
    }

    #[test]
    fn test_validate_groups_valid() {
        let dir = tempfile::tempdir().unwrap();
        let mut groups = BTreeMap::new();
        groups.insert(
            "dsp-stack".to_string(),
            vec!["dsp-api".to_string(), "sipi".to_string()],
        );

        let config = Config {
            registry: RegistryConfig {
                scan_roots: vec![dir.path().to_path_buf()],
            },
            workspace: WorkspaceConfig {
                root: dir.path().to_path_buf(),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            groups,
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_effort_level_round_trip() {
        let toml_str = r#"
[registry]
scan_roots = ["/code"]

[workspace]
root = "/loom"

[agents.claude-code]
effort_level = "high"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.agents.claude_code.effort_level,
            Some(EffortLevel::High)
        );

        let serialized = toml::to_string_pretty(&config).unwrap();
        let reparsed: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(
            reparsed.agents.claude_code.effort_level,
            Some(EffortLevel::High)
        );
    }

    #[test]
    fn test_effort_level_invalid_value_rejected() {
        let toml_str = r#"
[registry]
scan_roots = ["/code"]

[workspace]
root = "/loom"

[agents.claude-code]
effort_level = "max"
"#;
        let result = toml::from_str::<Config>(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_effort_level_none_keeps_config_empty() {
        let config = ClaudeCodeConfig::default();
        assert!(config.is_empty());
        assert!(config.effort_level.is_none());
    }

    #[test]
    fn test_effort_level_some_makes_config_non_empty() {
        let config = ClaudeCodeConfig {
            effort_level: Some(EffortLevel::Medium),
            ..Default::default()
        };
        assert!(!config.is_empty());
    }
}
