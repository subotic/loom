# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/subotic/loom/releases/tag/v0.1.0) - 2026-03-16

### Added

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
- *(cli)* add loom refresh command
- *(cli)* add --preset flag to loom new and loom refresh
- *(config)* add interactive security flavor prompt to loom init
- *(cli)* make workspace name optional in loom new
- *(cli)* show branch in workspace list
- *(cli)* change org selection from MultiSelect to Select
- *(cli)* add --groups flag to loom new
- *(cli)* add PRESET column to loom list output
- *(cli)* wire --verbose and --quiet flags to tracing subscriber
- *(agent)* replace repo config warnings with structured confirmations
- *(update)* add self-update mechanism with loom update command

### Fixed

- *(config)* address PR review findings
- *(names)* address review findings for random naming PR
