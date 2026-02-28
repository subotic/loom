use anyhow::Result;

use crate::agent::{AgentGenerator, GeneratedFile};
use crate::config::Config;
use crate::manifest::WorkspaceManifest;

/// Generates CLAUDE.md and .claude/settings.json for Claude Code.
pub struct ClaudeCodeGenerator;

impl AgentGenerator for ClaudeCodeGenerator {
    fn name(&self) -> &str {
        "claude-code"
    }

    fn generate(
        &self,
        manifest: &WorkspaceManifest,
        config: &Config,
    ) -> Result<Vec<GeneratedFile>> {
        let claude_md = generate_claude_md(manifest);
        let settings = generate_settings(manifest, &config.agents.claude_code);

        Ok(vec![
            GeneratedFile {
                relative_path: "CLAUDE.md".to_string(),
                content: claude_md,
            },
            GeneratedFile {
                relative_path: ".claude/settings.json".to_string(),
                content: settings,
            },
        ])
    }
}

/// Generate workspace CLAUDE.md content.
fn generate_claude_md(manifest: &WorkspaceManifest) -> String {
    let mut md = String::new();

    md.push_str(&format!("# LOOM Workspace: {}\n\n", manifest.name));
    md.push_str(
        "This workspace was created by [LOOM](https://github.com/subotic/loom) \
         and contains linked worktrees for multi-repo development.\n\n",
    );

    if !manifest.repos.is_empty() {
        md.push_str("## Repositories\n\n");
        md.push_str("| Directory | Branch | Source |\n");
        md.push_str("|-----------|--------|--------|\n");

        for repo in &manifest.repos {
            let dir = repo.name.as_str();
            let branch = repo.branch.as_str();
            let source = if repo.remote_url.is_empty() {
                repo.original_path.display().to_string()
            } else {
                repo.remote_url.clone()
            };
            md.push_str(&format!("| `{dir}` | `{branch}` | {source} |\n"));
        }

        md.push('\n');
    }

    md.push_str("## Working in this workspace\n\n");
    md.push_str("- Each subdirectory is a git worktree checked out on the workspace branch.\n");
    md.push_str("- Changes in one repo's worktree do not affect the original repo until pushed.\n");
    md.push_str("- Each repo may have its own `CLAUDE.md` with repo-specific context.\n");
    md.push_str("- Use `loom exec <cmd>` to run a command across all repos.\n");
    md.push_str("- Use `loom save` to push all branches.\n");
    md.push_str("- Use `loom status` to see per-repo branch and dirty state.\n");

    md
}

/// Generate .claude/settings.json content.
fn generate_settings(
    manifest: &WorkspaceManifest,
    cc_config: &crate::config::ClaudeCodeConfig,
) -> String {
    let paths: Vec<String> = manifest
        .repos
        .iter()
        .map(|r| r.worktree_path.display().to_string())
        .collect();

    let mut obj = serde_json::json!({
        "additionalDirectories": paths
    });

    if !cc_config.extra_known_marketplaces.is_empty() {
        let mut marketplaces = serde_json::Map::new();
        for entry in &cc_config.extra_known_marketplaces {
            marketplaces.insert(
                entry.name.clone(),
                serde_json::json!({
                    "source": {
                        "source": "github",
                        "repo": entry.repo
                    }
                }),
            );
        }
        obj["extraKnownMarketplaces"] = serde_json::Value::Object(marketplaces);
    }

    if !cc_config.enabled_plugins.is_empty() {
        let plugins: serde_json::Map<String, serde_json::Value> = cc_config
            .enabled_plugins
            .iter()
            .map(|p| (p.clone(), serde_json::Value::Bool(true)))
            .collect();
        obj["enabledPlugins"] = serde_json::Value::Object(plugins);
    }

    serde_json::to_string_pretty(&obj).expect("serde_json::Value is always serializable")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        AgentsConfig, ClaudeCodeConfig, DefaultsConfig, MarketplaceEntry, RegistryConfig,
        WorkspaceConfig,
    };
    use crate::manifest::RepoManifestEntry;
    use std::path::PathBuf;

    fn test_manifest() -> WorkspaceManifest {
        WorkspaceManifest {
            name: "my-feature".to_string(),
            created: chrono::DateTime::parse_from_rfc3339("2026-02-27T10:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            base_branch: Some("main".to_string()),
            repos: vec![
                RepoManifestEntry {
                    name: "dsp-api".to_string(),
                    original_path: PathBuf::from("/code/dasch-swiss/dsp-api"),
                    worktree_path: PathBuf::from("/loom/my-feature/dsp-api"),
                    branch: "loom/my-feature".to_string(),
                    remote_url: "git@github.com:dasch-swiss/dsp-api.git".to_string(),
                },
                RepoManifestEntry {
                    name: "dsp-das".to_string(),
                    original_path: PathBuf::from("/code/dasch-swiss/dsp-das"),
                    worktree_path: PathBuf::from("/loom/my-feature/dsp-das"),
                    branch: "loom/my-feature".to_string(),
                    remote_url: "git@github.com:dasch-swiss/dsp-das.git".to_string(),
                },
            ],
        }
    }

    fn test_config() -> Config {
        Config {
            registry: RegistryConfig { scan_roots: vec![] },
            workspace: WorkspaceConfig {
                root: PathBuf::from("/loom"),
            },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            agents: AgentsConfig::default(),
        }
    }

    #[test]
    fn test_claude_md_snapshot() {
        let manifest = test_manifest();
        let content = generate_claude_md(&manifest);
        insta::assert_snapshot!(content);
    }

    #[test]
    fn test_settings_snapshot() {
        let manifest = test_manifest();
        let cc_config = ClaudeCodeConfig::default();
        let content = generate_settings(&manifest, &cc_config);
        insta::assert_snapshot!(content);
    }

    #[test]
    fn test_settings_with_plugins_snapshot() {
        let manifest = test_manifest();
        let cc_config = ClaudeCodeConfig {
            extra_known_marketplaces: vec![MarketplaceEntry {
                name: "test-marketplace".to_string(),
                repo: "org/test-plugins".to_string(),
            }],
            enabled_plugins: vec!["pkm@test-marketplace".to_string()],
        };
        let content = generate_settings(&manifest, &cc_config);
        insta::assert_snapshot!(content);
    }

    #[test]
    fn test_claude_md_empty_repos() {
        let manifest = WorkspaceManifest {
            name: "empty-ws".to_string(),
            created: chrono::Utc::now(),
            base_branch: None,
            repos: vec![],
        };

        let content = generate_claude_md(&manifest);
        assert!(content.contains("# LOOM Workspace: empty-ws"));
        assert!(!content.contains("## Repositories"));
    }

    #[test]
    fn test_settings_empty_repos() {
        let manifest = WorkspaceManifest {
            name: "empty-ws".to_string(),
            created: chrono::Utc::now(),
            base_branch: None,
            repos: vec![],
        };

        let cc_config = ClaudeCodeConfig::default();
        let content = generate_settings(&manifest, &cc_config);
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        let dirs = parsed["additionalDirectories"].as_array().unwrap();
        assert!(dirs.is_empty());
        // No extra keys when config is empty
        assert!(parsed.get("extraKnownMarketplaces").is_none());
        assert!(parsed.get("enabledPlugins").is_none());
    }

    #[test]
    fn test_generator_trait() {
        let generator = ClaudeCodeGenerator;
        assert_eq!(generator.name(), "claude-code");

        let manifest = test_manifest();
        let config = test_config();
        let files = generator.generate(&manifest, &config).unwrap();
        assert_eq!(files.len(), 2);

        let claude_md = files.iter().find(|f| f.relative_path == "CLAUDE.md");
        assert!(claude_md.is_some());

        let settings = files
            .iter()
            .find(|f| f.relative_path == ".claude/settings.json");
        assert!(settings.is_some());
    }

    #[test]
    fn test_claude_md_no_remote_url() {
        let manifest = WorkspaceManifest {
            name: "local-ws".to_string(),
            created: chrono::Utc::now(),
            base_branch: None,
            repos: vec![RepoManifestEntry {
                name: "my-repo".to_string(),
                original_path: PathBuf::from("/code/my-repo"),
                worktree_path: PathBuf::from("/loom/local-ws/my-repo"),
                branch: "loom/local-ws".to_string(),
                remote_url: String::new(),
            }],
        };

        let content = generate_claude_md(&manifest);
        // Should fall back to original_path when remote_url is empty
        assert!(content.contains("/code/my-repo"));
    }

    #[test]
    fn test_settings_marketplaces_only() {
        let manifest = test_manifest();
        let cc_config = ClaudeCodeConfig {
            extra_known_marketplaces: vec![MarketplaceEntry {
                name: "my-plugins".to_string(),
                repo: "owner/my-plugins".to_string(),
            }],
            enabled_plugins: vec![],
        };
        let content = generate_settings(&manifest, &cc_config);
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.get("extraKnownMarketplaces").is_some());
        assert!(parsed.get("enabledPlugins").is_none());
    }

    #[test]
    fn test_settings_plugins_only() {
        let manifest = test_manifest();
        let cc_config = ClaudeCodeConfig {
            extra_known_marketplaces: vec![],
            enabled_plugins: vec!["pkm@global-marketplace".to_string()],
        };
        let content = generate_settings(&manifest, &cc_config);
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.get("extraKnownMarketplaces").is_none());
        assert!(parsed.get("enabledPlugins").is_some());
    }
}
