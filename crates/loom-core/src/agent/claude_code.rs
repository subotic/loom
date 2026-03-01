use anyhow::Result;

use crate::agent::{AgentGenerator, GeneratedFile};
use crate::config::{Config, Workflow};
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
        let claude_md = generate_claude_md(manifest, config);
        let settings = generate_settings(
            manifest,
            &config.agents.claude_code,
            manifest.preset.as_deref(),
        );

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
fn generate_claude_md(manifest: &WorkspaceManifest, config: &Config) -> String {
    let mut md = String::new();

    let has_workflow_config = !config.repos.is_empty();
    let default_branch = manifest.base_branch.as_deref().unwrap_or("main");

    md.push_str(&format!("# LOOM Workspace: {}\n\n", manifest.name));
    md.push_str(
        "This workspace was created by [LOOM](https://github.com/subotic/loom) \
         and contains linked worktrees for multi-repo development.\n\n",
    );

    if !manifest.repos.is_empty() {
        md.push_str("## Repositories\n\n");

        if has_workflow_config {
            md.push_str("| Directory | Branch | Source | Workflow |\n");
            md.push_str("|-----------|--------|--------|----------|\n");
        } else {
            md.push_str("| Directory | Branch | Source |\n");
            md.push_str("|-----------|--------|--------|\n");
        }

        for repo in &manifest.repos {
            let dir = repo.name.as_str();
            let branch = repo.branch.as_str();
            let source = if repo.remote_url.is_empty() {
                repo.original_path.display().to_string()
            } else {
                repo.remote_url.clone()
            };

            if has_workflow_config {
                let workflow = config
                    .repos
                    .get(dir)
                    .map(|r| r.workflow)
                    .unwrap_or_default();
                let workflow_label = workflow.label(default_branch);
                md.push_str(&format!(
                    "| `{dir}` | `{branch}` | {source} | {workflow_label} |\n"
                ));
            } else {
                md.push_str(&format!("| `{dir}` | `{branch}` | {source} |\n"));
            }
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

    // Workflows subsection (only when config has repo entries AND manifest has repos)
    if has_workflow_config && !manifest.repos.is_empty() {
        let has_pr = config.repos.values().any(|r| r.workflow == Workflow::Pr);
        let has_push = config.repos.values().any(|r| r.workflow == Workflow::Push);
        let ws_branch = manifest.branch_name(&config.defaults.branch_prefix);

        md.push_str("\n### Workflows\n\n");
        if has_pr {
            md.push_str(&format!(
                "- **PR to `{default_branch}`**: Create a branch off `origin/{default_branch}`, \
                 commit there, push, and open a PR. Do not push the workspace branch.\n"
            ));
        }
        if has_push {
            md.push_str(&format!(
                "- **Push to `{default_branch}`**: Commit on the workspace branch, then push \
                 directly to `{default_branch}` (`git push origin HEAD:{default_branch}`).\n"
            ));
        }
        if has_pr {
            md.push_str(&format!(
                "\nAfter creating a PR, continue working on the workspace branch `{ws_branch}`.\n"
            ));
        }
    }

    // Specs section
    if let Some(specs) = &config.specs {
        md.push_str("\n## Specs (PRDs and Implementation Plans)\n\n");
        md.push_str("Specs for this workspace are stored in:\n\n```\n");
        md.push_str(&format!("{}/\n", specs.path));
        md.push_str("└── YYYY-MM-DD-{title-slug}/\n");
        md.push_str("    ├── {nn}-{topic}-PRD.md\n");
        md.push_str("    └── {nn}-{type}-{topic}-plan.md\n");
        md.push_str("```\n\n");
        md.push_str(
            "- **Folder naming:** `YYYY-MM-DD-{title-slug}/` (creation date of first artifact)\n",
        );
        md.push_str(
            "- **File naming:** `{nn}` is per-folder sequential numbering \
             (zero-padded, starting at `01`)\n",
        );
        md.push_str("  - PRDs: `{nn}-{topic}-PRD.md`\n");
        md.push_str(
            "  - Plans: `{nn}-{type}-{topic}-plan.md` where `{type}` is \
             `feat`, `fix`, or `refactor`\n",
        );
        md.push_str("- PRDs and plans share the same numbering sequence within a folder\n");
    }

    md
}

/// Merge two string slices into a sorted, deduplicated Vec.
fn merge_sorted(global: &[String], preset: &[String]) -> Vec<String> {
    let mut merged = global.to_vec();
    merged.extend(preset.iter().cloned());
    merged.sort();
    merged.dedup();
    merged
}

/// Build the sandbox JSON object from global config and optional preset.
fn build_sandbox_json(
    sandbox: &crate::config::SandboxConfig,
    preset: Option<&crate::config::PermissionPreset>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut obj = serde_json::Map::new();

    if let Some(enabled) = sandbox.enabled {
        obj.insert("enabled".to_string(), serde_json::json!(enabled));
    }
    if let Some(auto_allow) = sandbox.auto_allow {
        obj.insert(
            "autoAllowBashIfSandboxed".to_string(),
            serde_json::json!(auto_allow),
        );
    }
    if !sandbox.excluded_commands.is_empty() {
        obj.insert(
            "excludedCommands".to_string(),
            serde_json::json!(sandbox.excluded_commands),
        );
    }
    if let Some(allow_unsandboxed) = sandbox.allow_unsandboxed_commands {
        obj.insert(
            "allowUnsandboxedCommands".to_string(),
            serde_json::json!(allow_unsandboxed),
        );
    }

    // Merge filesystem arrays: global ∪ preset
    let preset_fs = preset.map(|p| &p.sandbox.filesystem);
    let allow_write = merge_sorted(
        &sandbox.filesystem.allow_write,
        preset_fs.map_or(&[], |fs| &fs.allow_write),
    );
    let deny_write = merge_sorted(
        &sandbox.filesystem.deny_write,
        preset_fs.map_or(&[], |fs| &fs.deny_write),
    );
    let deny_read = merge_sorted(
        &sandbox.filesystem.deny_read,
        preset_fs.map_or(&[], |fs| &fs.deny_read),
    );

    if !allow_write.is_empty() || !deny_write.is_empty() || !deny_read.is_empty() {
        let mut fs_obj = serde_json::Map::new();
        if !allow_write.is_empty() {
            fs_obj.insert("allowWrite".to_string(), serde_json::json!(allow_write));
        }
        if !deny_write.is_empty() {
            fs_obj.insert("denyWrite".to_string(), serde_json::json!(deny_write));
        }
        if !deny_read.is_empty() {
            fs_obj.insert("denyRead".to_string(), serde_json::json!(deny_read));
        }
        obj.insert("filesystem".to_string(), serde_json::Value::Object(fs_obj));
    }

    // Merge network arrays: global ∪ preset
    let allowed_domains = merge_sorted(
        &sandbox.network.allowed_domains,
        preset.map_or(&[], |p| &p.sandbox.network.allowed_domains),
    );
    if !allowed_domains.is_empty() {
        obj.insert(
            "network".to_string(),
            serde_json::json!({ "allowedDomains": allowed_domains }),
        );
    }

    obj
}

/// Generate .claude/settings.json content.
///
/// If `preset_name` is provided, merges the named preset's settings with global config.
fn generate_settings(
    manifest: &WorkspaceManifest,
    cc_config: &crate::config::ClaudeCodeConfig,
    preset_name: Option<&str>,
) -> String {
    let paths: Vec<String> = manifest
        .repos
        .iter()
        .map(|r| r.worktree_path.display().to_string())
        .collect();

    let mut obj = serde_json::json!({
        "additionalDirectories": paths
    });

    if let Some(ref model) = cc_config.model {
        obj["model"] = serde_json::json!(model);
    }

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

    // Resolve the preset (if any). Upstream callers (generate_agent_files,
    // create_workspace, run_refresh) validate the preset exists via
    // validate_preset_exists() before reaching here. A miss falls back to
    // global-only settings, which is safe but should not happen in practice.
    let preset = preset_name.and_then(|name| cc_config.presets.get(name));

    // Build permissions.allow from global + preset allowed_tools
    let allow = merge_sorted(
        &cc_config.allowed_tools,
        preset.map_or(&[], |p| &p.allowed_tools),
    );
    if !allow.is_empty() {
        obj["permissions"] = serde_json::json!({ "allow": allow });
    }

    // Build sandbox from global config + preset arrays
    if !cc_config.sandbox.is_empty() || preset.is_some_and(|p| !p.sandbox.is_empty()) {
        let sandbox_obj = build_sandbox_json(&cc_config.sandbox, preset);
        if !sandbox_obj.is_empty() {
            obj["sandbox"] = serde_json::Value::Object(sandbox_obj);
        }
    }

    serde_json::to_string_pretty(&obj).expect("serde_json::Value is always serializable")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        AgentsConfig, ClaudeCodeConfig, DefaultsConfig, MarketplaceEntry, PermissionPreset,
        PresetSandboxConfig, RegistryConfig, RepoConfig, SandboxConfig, SandboxFilesystemConfig,
        SandboxNetworkConfig, SpecsConfig, Workflow, WorkspaceConfig,
    };
    use crate::manifest::RepoManifestEntry;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn test_manifest() -> WorkspaceManifest {
        WorkspaceManifest {
            name: "my-feature".to_string(),
            branch: None,
            created: chrono::DateTime::parse_from_rfc3339("2026-02-27T10:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            base_branch: Some("main".to_string()),
            preset: None,
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
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        }
    }

    #[test]
    fn test_claude_md_snapshot() {
        let manifest = test_manifest();
        let config = test_config();
        let content = generate_claude_md(&manifest, &config);
        insta::assert_snapshot!(content);
    }

    #[test]
    fn test_settings_snapshot() {
        let manifest = test_manifest();
        let cc_config = ClaudeCodeConfig::default();
        let content = generate_settings(&manifest, &cc_config, None);
        insta::assert_snapshot!(content);
    }

    #[test]
    fn test_settings_with_model_snapshot() {
        let manifest = test_manifest();
        let cc_config = ClaudeCodeConfig {
            model: Some("opus".to_string()),
            ..Default::default()
        };
        let content = generate_settings(&manifest, &cc_config, None);
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
            ..Default::default()
        };
        let content = generate_settings(&manifest, &cc_config, None);
        insta::assert_snapshot!(content);
    }

    #[test]
    fn test_claude_md_empty_repos() {
        let manifest = WorkspaceManifest {
            name: "empty-ws".to_string(),
            branch: None,
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![],
        };
        let config = test_config();

        let content = generate_claude_md(&manifest, &config);
        assert!(content.contains("# LOOM Workspace: empty-ws"));
        assert!(!content.contains("## Repositories"));
    }

    #[test]
    fn test_settings_empty_repos() {
        let manifest = WorkspaceManifest {
            name: "empty-ws".to_string(),
            branch: None,
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![],
        };

        let cc_config = ClaudeCodeConfig::default();
        let content = generate_settings(&manifest, &cc_config, None);
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
            branch: None,
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![RepoManifestEntry {
                name: "my-repo".to_string(),
                original_path: PathBuf::from("/code/my-repo"),
                worktree_path: PathBuf::from("/loom/local-ws/my-repo"),
                branch: "loom/local-ws".to_string(),
                remote_url: String::new(),
            }],
        };

        let config = test_config();
        let content = generate_claude_md(&manifest, &config);
        // Should fall back to original_path when remote_url is empty
        assert!(content.contains("/code/my-repo"));
    }

    fn test_config_with_workflows() -> Config {
        let mut config = test_config();
        config.repos.insert(
            "dsp-api".to_string(),
            RepoConfig {
                workflow: Workflow::Pr,
            },
        );
        config.repos.insert(
            "dsp-das".to_string(),
            RepoConfig {
                workflow: Workflow::Push,
            },
        );
        config
    }

    fn test_config_with_specs() -> Config {
        let mut config = test_config();
        config.specs = Some(SpecsConfig {
            path: "pkm/01 - PROJECTS/Personal/LOOM/specs".to_string(),
        });
        config
    }

    fn test_config_full() -> Config {
        let mut config = test_config_with_workflows();
        config.specs = Some(SpecsConfig {
            path: "pkm/01 - PROJECTS/Personal/LOOM/specs".to_string(),
        });
        config
    }

    #[test]
    fn test_claude_md_with_workflows_snapshot() {
        let manifest = test_manifest();
        let config = test_config_with_workflows();
        let content = generate_claude_md(&manifest, &config);
        insta::assert_snapshot!(content);
    }

    #[test]
    fn test_claude_md_with_specs_snapshot() {
        let manifest = test_manifest();
        let config = test_config_with_specs();
        let content = generate_claude_md(&manifest, &config);
        insta::assert_snapshot!(content);
    }

    #[test]
    fn test_claude_md_full_snapshot() {
        let manifest = test_manifest();
        let config = test_config_full();
        let content = generate_claude_md(&manifest, &config);
        insta::assert_snapshot!(content);
    }

    #[test]
    fn test_claude_md_custom_default_branch() {
        let mut manifest = test_manifest();
        manifest.base_branch = Some("develop".to_string());
        let config = test_config_with_workflows();
        let content = generate_claude_md(&manifest, &config);
        assert!(content.contains("PR to `develop`"));
        assert!(content.contains("Push to `develop`"));
        assert!(!content.contains("PR to `main`"));
    }

    #[test]
    fn test_claude_md_empty_repos_with_workflow_config() {
        let manifest = WorkspaceManifest {
            name: "empty-ws".to_string(),
            branch: None,
            created: chrono::DateTime::parse_from_rfc3339("2026-02-27T10:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            base_branch: None,
            preset: None,
            repos: vec![],
        };
        let config = test_config_with_workflows();
        let content = generate_claude_md(&manifest, &config);
        // Empty manifest repos should suppress the table entirely,
        // even when config.repos has entries
        assert!(!content.contains("## Repositories"));
        assert!(!content.contains("Workflow"));
    }

    #[test]
    fn test_settings_marketplaces_only() {
        let manifest = test_manifest();
        let cc_config = ClaudeCodeConfig {
            extra_known_marketplaces: vec![MarketplaceEntry {
                name: "my-plugins".to_string(),
                repo: "owner/my-plugins".to_string(),
            }],
            ..Default::default()
        };
        let content = generate_settings(&manifest, &cc_config, None);
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.get("extraKnownMarketplaces").is_some());
        assert!(parsed.get("enabledPlugins").is_none());
    }

    #[test]
    fn test_settings_plugins_only() {
        let manifest = test_manifest();
        let cc_config = ClaudeCodeConfig {
            enabled_plugins: vec!["pkm@global-marketplace".to_string()],
            ..Default::default()
        };
        let content = generate_settings(&manifest, &cc_config, None);
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.get("extraKnownMarketplaces").is_none());
        assert!(parsed.get("enabledPlugins").is_some());
    }

    #[test]
    fn test_settings_with_permissions_snapshot() {
        let manifest = test_manifest();
        let cc_config = ClaudeCodeConfig {
            allowed_tools: vec![
                "Bash(gh issue *)".to_string(),
                "Bash(gh run *)".to_string(),
                "WebFetch(domain:docs.rs)".to_string(),
            ],
            ..Default::default()
        };
        let content = generate_settings(&manifest, &cc_config, None);
        insta::assert_snapshot!(content);
    }

    #[test]
    fn test_settings_with_sandbox_snapshot() {
        let manifest = test_manifest();
        let cc_config = ClaudeCodeConfig {
            sandbox: SandboxConfig {
                enabled: Some(true),
                auto_allow: Some(true),
                excluded_commands: vec!["docker".to_string()],
                allow_unsandboxed_commands: Some(false),
                filesystem: SandboxFilesystemConfig {
                    allow_write: vec!["~/.config/loom".to_string()],
                    ..Default::default()
                },
                network: SandboxNetworkConfig {
                    allowed_domains: vec!["github.com".to_string()],
                },
            },
            ..Default::default()
        };
        let content = generate_settings(&manifest, &cc_config, None);
        insta::assert_snapshot!(content);
    }

    #[test]
    fn test_settings_with_preset_snapshot() {
        let manifest = test_manifest();

        let mut presets = BTreeMap::new();
        presets.insert(
            "rust".to_string(),
            PermissionPreset {
                allowed_tools: vec![
                    "Bash(cargo clippy *)".to_string(),
                    "Bash(cargo fmt *)".to_string(),
                    "Bash(cargo test *)".to_string(),
                ],
                sandbox: PresetSandboxConfig {
                    filesystem: SandboxFilesystemConfig {
                        allow_write: vec!["~/.cargo".to_string()],
                        ..Default::default()
                    },
                    network: SandboxNetworkConfig {
                        allowed_domains: vec!["crates.io".to_string(), "docs.rs".to_string()],
                    },
                },
            },
        );

        let cc_config = ClaudeCodeConfig {
            allowed_tools: vec![
                "Bash(gh issue *)".to_string(),
                "Bash(gh run *)".to_string(),
                "WebFetch(domain:docs.rs)".to_string(),
            ],
            sandbox: SandboxConfig {
                enabled: Some(true),
                auto_allow: Some(true),
                excluded_commands: vec!["docker".to_string()],
                allow_unsandboxed_commands: Some(false),
                filesystem: SandboxFilesystemConfig {
                    allow_write: vec!["~/.config/loom".to_string()],
                    ..Default::default()
                },
                network: SandboxNetworkConfig {
                    allowed_domains: vec!["github.com".to_string()],
                },
            },
            presets,
            ..Default::default()
        };

        let content = generate_settings(&manifest, &cc_config, Some("rust"));
        insta::assert_snapshot!(content);
    }

    #[test]
    fn test_settings_with_all_features_snapshot() {
        let manifest = test_manifest();

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
                    },
                },
            },
        );

        let cc_config = ClaudeCodeConfig {
            extra_known_marketplaces: vec![MarketplaceEntry {
                name: "test-marketplace".to_string(),
                repo: "org/test-plugins".to_string(),
            }],
            enabled_plugins: vec!["pkm@test-marketplace".to_string()],
            allowed_tools: vec!["Bash(gh issue *)".to_string()],
            sandbox: SandboxConfig {
                enabled: Some(true),
                auto_allow: Some(true),
                excluded_commands: vec!["docker".to_string()],
                // Intentionally None — verifies that the key is omitted from JSON when unset,
                // complementing the preset snapshot test which sets it to Some(false).
                allow_unsandboxed_commands: None,
                filesystem: SandboxFilesystemConfig {
                    allow_write: vec!["~/.config/loom".to_string()],
                    ..Default::default()
                },
                network: SandboxNetworkConfig {
                    allowed_domains: vec!["github.com".to_string()],
                },
            },
            presets,
            ..Default::default()
        };

        let content = generate_settings(&manifest, &cc_config, Some("rust"));
        insta::assert_snapshot!(content);
    }

    #[test]
    fn test_settings_no_permissions_when_empty() {
        let manifest = test_manifest();
        let cc_config = ClaudeCodeConfig::default();
        let content = generate_settings(&manifest, &cc_config, None);
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.get("permissions").is_none());
        assert!(parsed.get("sandbox").is_none());
    }

    #[test]
    fn test_settings_preset_not_found_uses_global_only() {
        let manifest = test_manifest();
        let cc_config = ClaudeCodeConfig {
            allowed_tools: vec!["Bash(gh issue *)".to_string()],
            ..Default::default()
        };
        // Non-existent preset — only global tools should appear
        let content = generate_settings(&manifest, &cc_config, Some("nonexistent"));
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        let allow = parsed["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), 1);
        assert_eq!(allow[0], "Bash(gh issue *)");
    }
}
