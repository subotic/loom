pub mod claude_code;

use std::path::Path;

use anyhow::Result;

use crate::config::Config;
use crate::manifest::WorkspaceManifest;

/// A file to be written into the workspace by an agent generator.
#[derive(Debug)]
pub struct GeneratedFile {
    /// Path relative to the workspace root.
    pub relative_path: String,
    /// File content.
    pub content: String,
}

/// Trait for generating agent configuration files in a workspace.
pub trait AgentGenerator {
    /// Human-readable name for this generator (e.g., "claude-code").
    fn name(&self) -> &str;

    /// Generate files for the given workspace.
    fn generate(&self, manifest: &WorkspaceManifest, config: &Config)
    -> Result<Vec<GeneratedFile>>;
}

/// Run all enabled agent generators and write their files into the workspace.
///
/// Called after workspace composition changes (new, add, remove, partial down).
pub fn generate_agent_files(
    config: &Config,
    ws_path: &Path,
    manifest: &WorkspaceManifest,
) -> Result<()> {
    // Validate preset exists (stale preset check)
    if let Some(ref preset_name) = manifest.preset {
        crate::config::validate_preset_exists(&config.agents.claude_code.presets, preset_name)?;
    }

    // Warn about configured repo names that don't match any manifest entry
    for repo_key in config.repos.keys() {
        if !manifest.repos.iter().any(|r| r.name == *repo_key) {
            eprintln!(
                "Warning: [repos.{repo_key}] in config does not match any workspace repo, ignoring."
            );
        }
    }

    for agent_name in &config.agents.enabled {
        let generator: Box<dyn AgentGenerator> = match agent_name.as_str() {
            "claude-code" => Box::new(claude_code::ClaudeCodeGenerator),
            other => {
                eprintln!("Warning: unknown agent '{}', skipping.", other);
                continue;
            }
        };

        // Clean up legacy files from previous versions
        if agent_name == "claude-code" {
            let legacy = ws_path.join(".claude/settings.local.json");
            if legacy.exists() {
                std::fs::remove_file(&legacy)?;
            }
        }

        let files = generator.generate(manifest, config)?;
        for file in &files {
            // Guard against path traversal and absolute paths in relative_path
            crate::config::validate_no_path_traversal(
                &file.relative_path,
                &format!("agent '{agent_name}' relative path"),
            )?;
            let full_path = ws_path.join(&file.relative_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&full_path, &file.content)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentsConfig, DefaultsConfig, RegistryConfig, WorkspaceConfig};
    use crate::manifest::RepoManifestEntry;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn test_generate_agent_files_writes_files() {
        let dir = tempfile::tempdir().unwrap();
        let ws_path = dir.path().join("my-ws");
        std::fs::create_dir_all(&ws_path).unwrap();

        let config = Config {
            registry: RegistryConfig { scan_roots: vec![] },
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
                ..Default::default()
            },
        };

        let manifest = WorkspaceManifest {
            name: "my-ws".to_string(),
            branch: None,
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![RepoManifestEntry {
                name: "dsp-api".to_string(),
                original_path: PathBuf::from("/code/dasch-swiss/dsp-api"),
                worktree_path: ws_path.join("dsp-api"),
                branch: "loom/my-ws".to_string(),
                remote_url: "git@github.com:dasch-swiss/dsp-api.git".to_string(),
            }],
        };

        generate_agent_files(&config, &ws_path, &manifest).unwrap();

        assert!(ws_path.join("CLAUDE.md").exists());
        assert!(ws_path.join(".claude/settings.json").exists());
    }

    #[test]
    fn test_generate_agent_files_removes_legacy_settings() {
        let dir = tempfile::tempdir().unwrap();
        let ws_path = dir.path().join("my-ws");
        let legacy_path = ws_path.join(".claude/settings.local.json");
        std::fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
        std::fs::write(&legacy_path, "{}").unwrap();
        assert!(legacy_path.exists());

        let config = Config {
            registry: RegistryConfig { scan_roots: vec![] },
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
                ..Default::default()
            },
        };

        let manifest = WorkspaceManifest {
            name: "my-ws".to_string(),
            branch: None,
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![],
        };

        generate_agent_files(&config, &ws_path, &manifest).unwrap();

        // Legacy file should be removed
        assert!(!legacy_path.exists());
        // New file should exist
        assert!(ws_path.join(".claude/settings.json").exists());
    }

    #[test]
    fn test_generate_agent_files_unknown_agent_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let ws_path = dir.path().join("my-ws");
        std::fs::create_dir_all(&ws_path).unwrap();

        let config = Config {
            registry: RegistryConfig { scan_roots: vec![] },
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
                enabled: vec!["unknown-agent".to_string()],
                ..Default::default()
            },
        };

        let manifest = WorkspaceManifest {
            name: "my-ws".to_string(),
            branch: None,
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![],
        };

        // Should not error, just skip
        generate_agent_files(&config, &ws_path, &manifest).unwrap();

        // No files written
        assert!(!ws_path.join("CLAUDE.md").exists());
    }

    /// Helper: test the path traversal guard in isolation.
    fn is_safe_relative_path(path: &str) -> bool {
        use std::path::{Component, Path};
        let p = Path::new(path);
        !p.is_absolute() && !p.components().any(|c| c == Component::ParentDir)
    }

    #[test]
    fn test_path_traversal_guard() {
        // Safe paths
        assert!(is_safe_relative_path("CLAUDE.md"));
        assert!(is_safe_relative_path(".claude/settings.json"));

        // Path traversal with ..
        assert!(!is_safe_relative_path("../etc/passwd"));
        assert!(!is_safe_relative_path("foo/../../bar"));

        // Absolute paths
        assert!(!is_safe_relative_path("/etc/passwd"));
        assert!(!is_safe_relative_path("/tmp/evil"));
    }

    #[test]
    fn test_generate_agent_files_no_agents() {
        let dir = tempfile::tempdir().unwrap();
        let ws_path = dir.path().join("my-ws");
        std::fs::create_dir_all(&ws_path).unwrap();

        let config = Config {
            registry: RegistryConfig { scan_roots: vec![] },
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
                enabled: vec![],
                ..Default::default()
            },
        };

        let manifest = WorkspaceManifest {
            name: "my-ws".to_string(),
            branch: None,
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![],
        };

        generate_agent_files(&config, &ws_path, &manifest).unwrap();
    }
}
