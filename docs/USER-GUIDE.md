# LOOM User Guide

> **Linked Orchestration Of Multirepos** — manage git worktrees across repositories.

## Quick Reference

| Command | Description | Key Flags |
|---------|-------------|-----------|
| `loom init` | First-run setup — creates `~/.config/loom/config.toml` | — |
| `loom new [name]` | Create a workspace with correlated worktrees | `--base`, `--repos`, `--preset` |
| `loom add <repo>` | Add a repo to an existing workspace | `--workspace` |
| `loom remove <repo>` | Remove a repo from the current workspace | `--force` |
| `loom list` / `loom ls` | List all workspaces | — |
| `loom status [name]` | Show repo status in a workspace | `--fetch` |
| `loom save` | Push workspace branches (+ sync manifest) | `--force` |
| `loom open <name>` | Restore a workspace from sync manifest | — |
| `loom down [name]` | Tear down a workspace (remove worktrees) | `--force` |
| `loom exec <cmd...>` | Run a command across all workspace repos | — |
| `loom shell [name]` | Open a terminal in the workspace directory | — |
| `loom refresh [name]` | Regenerate agent files from current config | `--preset` |
| `loom tui` | Open the interactive TUI | — |
| `loom completions <shell>` | Generate shell completions | — |

---

## Overview

LOOM creates **workspaces** — directories where multiple git repositories are checked out as worktrees on a common branch. This lets you work on cross-repo features without touching your main clones. When you're done, tear the workspace down. Your original repos are untouched.

### Core Concepts

- **Workspace** — a directory containing linked git worktrees for one or more repositories, plus a manifest (`.loom.json`) tracking its state.
- **Worktree** — a git worktree created from an existing repository clone. LOOM uses `git worktree add` under the hood, so each worktree shares history with the original repo.
- **Registry** — the set of repositories LOOM knows about, discovered by scanning directories at a specific depth (see [Scan Root Convention](#scan-root-convention) below).

---

## Getting Started

### Prerequisites

**Required:**

| Tool | Minimum Version | Check |
|------|----------------|-------|
| git | 2.22+ (worktree improvements) | `git --version` |
| Rust toolchain | stable | `rustc --version` |

**Recommended:**

| Tool | Purpose | Check |
|------|---------|-------|
| GitHub CLI (`gh`) | Used by `loom save` workflow instructions | `gh --version` |

**Optional:**

| Tool | Purpose |
|------|---------|
| Claude Code | AI agent integration (CLAUDE.md + settings.json generation) |

### Installation

```sh
cargo install --git https://github.com/subotic/loom.git
```

### Scan Root Convention

LOOM discovers repositories at exactly **two levels** below each scan root: `{scan_root}/{org}/{repo}`. Repos at other depths are invisible.

```
~/_github.com/                  ← scan root
├── dasch-swiss/                ← org level
│   ├── dsp-api/       ✓       ← discovered (2 levels deep)
│   ├── dsp-das/       ✓       ← discovered
│   └── ops-deploy/    ✓       ← discovered
├── subotic/
│   └── loom/          ✓       ← discovered
└── README.md                   ← ignored (not a directory)

~/code/
├── myproject/         ✗       ← NOT found (only 1 level deep)
└── org/
    └── repo/
        └── subdir/    ✗       ← NOT found (3 levels deep)
```

This convention matches the layout used by [ghq](https://github.com/x-motemen/ghq) and Go's `GOPATH`. If your repos aren't discovered, check the depth first.

### Setting Up: `loom init`

Run `loom init` to create your configuration. The wizard walks you through each setting interactively:

```
$ loom init

? Select scan roots: (Use arrows, space to toggle)
  ❯ ✓ /Users/you/_github.com
    ○ /Users/you/code

? Workspace root: ~/workspaces

? Terminal command: ghostty

? Branch prefix: loom

? Security flavor:
  ❯ Sandbox (recommended) — OS-level isolation with auto-allow
    Permissions — Explicit tool allowlists for fine-grained control
    Both — Sandbox for Bash + permissions for non-Bash tools
    Skip — Don't configure now (can be added later in config.toml)

✓ Config written to /Users/you/.config/loom/config.toml
```

**Notes:**
- LOOM auto-detects candidate scan roots from common locations (`~/_github.com`, `~/src`, `~/code`, `~/repos`, `~/Projects`, `~/dev`).
- Terminal detection reads the `TERM_PROGRAM` environment variable and maps it: `ghostty` → `ghostty`, `WezTerm` → `wezterm`, `iTerm.app` → `open -a iTerm`, `Apple_Terminal` → `open -a Terminal`, `vscode` → `code`.
- **Re-running `loom init`**: if a config already exists, LOOM asks before overwriting. Agent settings (`[agents]` section) are preserved during re-init — only `[registry]`, `[workspace]`, `[terminal]`, and `[defaults]` are updated.

### First Workspace: 5-Step Walkthrough

#### Step 1: Create a workspace

```
$ loom new
✓ Created workspace: amber-swift-fox
  Branch: loom/gentle-river-stone
  Path: /Users/you/workspaces/amber-swift-fox

  Repos:
    dsp-api  → /Users/you/workspaces/amber-swift-fox/dsp-api
    dsp-das  → /Users/you/workspaces/amber-swift-fox/dsp-das
```

Omitting the name generates a random `adjective-modifier-noun` name. The branch name is independently generated (also random) and prefixed with your `branch_prefix` (default: `loom/`).

#### Step 2: Explore what was created

```
$ loom status amber-swift-fox
Workspace: amber-swift-fox
Path: /Users/you/workspaces/amber-swift-fox
Repos: 2

REPO       BRANCH                    STATUS  AHEAD  BEHIND
dsp-api    loom/gentle-river-stone   clean   0      0
dsp-das    loom/gentle-river-stone   clean   0      0
```

#### Step 3: Run a command across repos

```
$ cd ~/workspaces/amber-swift-fox
$ loom exec git log --oneline -1
── dsp-api ──
a1b2c3d Initial commit

── dsp-das ──
d4e5f6a Initial commit
```

#### Step 4: Push your work

```
$ loom save
Pushed: dsp-api, dsp-das
Sync: updated amber-swift-fox manifest
```

`loom save` pushes the workspace branch for each repo to its remote. If `[sync]` is configured, it also writes a sync manifest for cross-machine restore.

#### Step 5: Clean up

```
$ cd ~
$ loom down amber-swift-fox
✓ Removed worktree: dsp-api
✓ Removed worktree: dsp-das
✓ Deleted branch: loom/gentle-river-stone (dsp-api)
✓ Deleted branch: loom/gentle-river-stone (dsp-das)
✓ Workspace amber-swift-fox torn down
```

> **Tip:** Don't run `loom down` from inside the workspace directory — your current directory becomes invalid after teardown.

### Minimal Configuration

The smallest working `config.toml`:

```toml
[registry]
scan_roots = ["~/_github.com"]

[workspace]
root = "~/workspaces"
```

Everything else is optional. See [Configuration Reference](#configuration-reference) for all options.
