# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/subotic/loom/releases/tag/v0.1.0) - 2026-03-16

### Added

- *(update)* add self-update mechanism with loom update command
- *(agent)* replace repo config warnings with structured confirmations
- *(cli)* wire --verbose and --quiet flags to tracing subscriber
- *(cli)* add PRESET column to loom list output
- *(cli)* add --groups flag to loom new
- *(cli)* change org selection from MultiSelect to Select
- *(cli)* show branch in workspace list
- *(cli)* make workspace name optional in loom new
- *(config)* add interactive security flavor prompt to loom init
- *(cli)* add --preset flag to loom new and loom refresh
- *(cli)* add loom refresh command
- *(config)* change default workspace folder to ~/workspaces
- add two-step org/repo selection in `loom new` and TUI wizard
- implement TUI with ratatui (workspace list, detail, new wizard)
- implement loom save and loom open (cross-machine sync)
- implement agent integration (CLAUDE.md and settings.local.json generation)
- implement loom exec and loom shell commands
- implement loom add, remove, and down commands
- implement loom list and loom status commands
- implement loom new with workspace creation and worktree management
- implement loom init with interactive prompts

### Fixed

- *(names)* address review findings for random naming PR
- *(config)* address PR review findings

### Other

- *(release)* fix release-please for Cargo workspace
- apply rustfmt formatting
- *(config)* improve docs, invariants, and preset UX
- address review findings from permission presets
- fix clippy warnings and apply cargo fmt
- Phase 0 scaffold alignment
- restructure into workspace with lib + bin crates
