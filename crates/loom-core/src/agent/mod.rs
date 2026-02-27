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
    fn generate(&self, manifest: &WorkspaceManifest) -> Result<Vec<GeneratedFile>>;
}

/// Run all enabled agent generators and write their files into the workspace.
///
/// Called after workspace composition changes (new, add, remove, partial down).
pub fn generate_agent_files(
    config: &Config,
    ws_path: &Path,
    manifest: &WorkspaceManifest,
) -> Result<()> {
    for agent_name in &config.agents.enabled {
        let generator: Box<dyn AgentGenerator> = match agent_name.as_str() {
            "claude-code" => Box::new(claude_code::ClaudeCodeGenerator),
            other => {
                eprintln!("Warning: unknown agent '{}', skipping.", other);
                continue;
            }
        };

        let files = generator.generate(manifest)?;
        for file in &files {
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
            agents: AgentsConfig {
                enabled: vec!["claude-code".to_string()],
            },
        };

        let manifest = WorkspaceManifest {
            name: "my-ws".to_string(),
            created: chrono::Utc::now(),
            base_branch: None,
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
        assert!(ws_path.join(".claude/settings.local.json").exists());
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
            agents: AgentsConfig {
                enabled: vec!["unknown-agent".to_string()],
            },
        };

        let manifest = WorkspaceManifest {
            name: "my-ws".to_string(),
            created: chrono::Utc::now(),
            base_branch: None,
            repos: vec![],
        };

        // Should not error, just skip
        generate_agent_files(&config, &ws_path, &manifest).unwrap();

        // No files written
        assert!(!ws_path.join("CLAUDE.md").exists());
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
            agents: AgentsConfig { enabled: vec![] },
        };

        let manifest = WorkspaceManifest {
            name: "my-ws".to_string(),
            created: chrono::Utc::now(),
            base_branch: None,
            repos: vec![],
        };

        generate_agent_files(&config, &ws_path, &manifest).unwrap();
    }
}
