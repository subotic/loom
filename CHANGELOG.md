# Changelog
All notable changes to this project will be documented in this file.

## [0.2.0](https://github.com/subotic/loom/compare/v0.1.0...v0.2.0) - 2026-03-20

### Added

- *(workspace)* add loom editor and loom reset commands
- *(registry)* configurable scan_depth for flexible directory layouts
- *(cli)* wire new commands, add flags, branch naming, and progress

### Fixed

- *(exec)* show command name, directory, and PATH hints on failure
- *(shell)* detect Ghostty.app on macOS and use `open -a` at runtime

## [0.1.0] - 2026-03-17

### Added

- scaffold Rust CLI with clap subcommands and module structure
- *(config)* harden config module with tilde expansion, atomic writes, validation
- add git abstraction layer with typed errors and worktree support
- add repo registry with discovery and URL normalization
- add manifest types with atomic I/O and backup recovery
- add workspace detection, CI workflow, and test infrastructure
- implement loom init with interactive prompts
- implement loom new with workspace creation and worktree management
- implement loom list and loom status commands
- implement loom add, remove, and down commands
- implement loom exec and loom shell commands
- implement agent integration (CLAUDE.md and settings.local.json generation)
- implement loom save and loom open (cross-machine sync)
- implement TUI with ratatui (workspace list, detail, new wizard)
- add two-step org/repo selection in `loom new` and TUI wizard
- *(config)* change default workspace folder to ~/workspaces
- *(git)* base worktree branches on remote default branch
- *(config)* add ClaudeCodeConfig for marketplace/plugin settings
- *(cli)* add loom refresh command
- *(config)* add permission presets, sandbox, and allowed_tools to ClaudeCodeConfig
- *(manifest)* add optional preset field to WorkspaceManifest
- *(agent)* extend generate_settings for permissions, sandbox, and presets
- *(cli)* add --preset flag to loom new and loom refresh
- *(config)* add interactive security flavor prompt to loom init
- *(config)* add Workflow, RepoConfig, SpecsConfig with validation
- *(agent)* generate workflow and specs sections in CLAUDE.md
- *(config)* append repos/specs examples to generated config.toml
- *(config)* support model field in agents.claude-code config
- *(names)* add random name generator with word lists
- *(manifest)* add branch field to WorkspaceManifest
- *(workspace)* use random branch names for new workspaces
- *(cli)* make workspace name optional in loom new
- *(tui)* support random workspace names in wizard
- *(cli)* show branch in workspace list
- *(tui)* change org selection from multi-select to single-select
- *(cli)* change org selection from MultiSelect to Select
- *(config)* add groups field for named repository collections
- *(groups)* add group resolution helper
- *(cli)* add --groups flag to loom new
- *(tui)* show config groups alongside org groups in wizard
- *(cli)* add PRESET column to loom list output
- *(config)* add enableWeakerNetworkIsolation to sandbox config
- *(config)* add effortLevel support to ClaudeCodeConfig
- *(config)* add enabled_mcp_servers setting
- *(config)* add allow_unix_sockets setting
- *(cli)* wire --verbose and --quiet flags to tracing subscriber
- *(agent)* replace repo config warnings with structured confirmations
- *(config)* add allow_local_binding, env, MCP servers, and bare tool names
- *(update)* add self-update mechanism with loom update command

### Fixed

- *(workspace)* validate --base after fetching from origin
- *(workspace)* add hint for remote-only branches in --base error
- *(ci)* grant write permission and add review context to Claude Code workflow
- *(ci)* enable sticky comment so Claude review posts to PR
- *(ci)* switch Claude review to direct prompt for reliable PR comments
- *(config)* address PR review findings
- *(names)* address review findings for random naming PR
- *(tui)* guard against empty groups and avoid cloning in Cancel handlers
- *(tui)* add defensive guards from PR review
- *(docs)* update stale settings.local.json references to settings.json
- *(agent)* convert sandbox paths to // prefix and auto-inject .git dirs
