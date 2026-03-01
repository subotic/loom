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

---

## Command Reference

> **Workspace resolution:** Most commands auto-detect the current workspace by walking up from your working directory looking for `.loom.json`. Commands with a `[NAME]` argument let you override this.

> **Global flags:** `--no-color`, `--verbose` (`-v`), `--quiet` (`-q`), and `--json` are defined but **not yet functional** (planned). The `NO_COLOR` environment variable is also recognized but not yet wired up.

### `loom init`

First-run setup — creates `~/.config/loom/config.toml`.

```
loom init
```

No arguments, no flags — fully interactive. See [Setting Up: loom init](#setting-up-loom-init) for the walkthrough.

**Behavior:**
- Auto-detects candidate scan roots from common locations.
- Auto-detects terminal from `TERM_PROGRAM` environment variable.
- If config already exists: asks before overwriting. Preserves `[agents]` section during re-init.

---

### `loom new`

Create a new workspace with correlated worktrees.

```
loom new [NAME] [--base BRANCH] [--repos REPO,...] [--preset NAME]
```

| Flag | Description |
|------|-------------|
| `[NAME]` | Workspace name. Optional — a random `adjective-modifier-noun` name is generated if omitted. |
| `--base BRANCH` | Base branch for worktrees (default: each repo's default branch). Fetches and validates the ref exists. |
| `--repos REPO,...` | Comma-separated repo names. Skips interactive repo selection. |
| `--preset NAME` | Apply a named permission preset from config.toml. |

**Examples:**

```
$ loom new my-feature --repos dsp-api,dsp-das
✓ Created workspace: my-feature
  Branch: loom/bold-cedar-hawk
  Path: /Users/you/workspaces/my-feature

  Repos:
    dsp-api  → /Users/you/workspaces/my-feature/dsp-api
    dsp-das  → /Users/you/workspaces/my-feature/dsp-das
```

```
$ loom new --preset rust
✓ Created workspace: amber-swift-fox
  Branch: loom/gentle-river-stone
  ...
```

**Notes:**
- **Workspace name** and **branch name** are independently generated. The manifest stores the branch name; older manifests without a `branch` field fall back to `{prefix}/{workspace-name}`.
- **Name validation:** lowercase alphanumeric + hyphens, max 63 characters, no leading or trailing hyphens.
- **Interactive mode** (no `--repos`): presents org group multi-select → repo multi-select. If only one org exists, the org selection is skipped.
- **Partial failure:** if a worktree fails for one repo, others continue. Errors are reported at the end.

---

### `loom add`

Add a repo to an existing workspace.

```
loom add <REPO> [--workspace NAME]
```

| Flag | Description |
|------|-------------|
| `<REPO>` | Required. Must match a registered repo name. |
| `--workspace NAME` | Target workspace. Defaults to the workspace detected from cwd. |

---

### `loom remove`

Remove a repo from the current workspace.

```
loom remove <REPO> [--force]
```

| Flag | Description |
|------|-------------|
| `<REPO>` | Required. The repo name to remove. |
| `--force` | Force removal even with uncommitted changes. |

**Notes:**
- Refuses if the repo has uncommitted changes (use `--force` to override).
- Refuses if it's the last repo — use `loom down` to tear down the entire workspace.
- Must be inside the workspace (no `--workspace` flag).

---

### `loom list` (alias: `ls`)

List all workspaces.

```
loom list
loom ls
```

**Columns:** `NAME`, `REPOS`, `STATUS`, `BRANCH`, `PRESET`, `CREATED`

**Example output:**

```
$ loom list
NAME                 REPOS  STATUS       BRANCH                         PRESET       CREATED
my-feature           3      clean        loom/bold-cedar-hawk            rust         2026-02-28
another-ws           2      1 dirty      loom/gentle-river-stone         -            2026-02-27
```

**Status values:** `clean`, `N dirty` (N repos with uncommitted changes), `broken: <msg>` (manifest missing or corrupt).

---

### `loom status`

Show status of all repos in a workspace.

```
loom status [NAME] [--fetch]
```

| Flag | Description |
|------|-------------|
| `[NAME]` | Workspace name. Optional — detects from cwd. |
| `--fetch` | Run `git fetch` in each repo before showing status. |

**Example output:**

```
$ loom status
Workspace: my-feature
Path: /Users/you/workspaces/my-feature
Repos: 3

REPO       BRANCH                    STATUS  AHEAD  BEHIND
dsp-api    loom/bold-cedar-hawk      clean   0      0
dsp-das    loom/bold-cedar-hawk      clean   2      0
ops-deploy loom/bold-cedar-hawk      dirty   1      3
```

**Notes:**
- Inside a workspace → shows detailed per-repo view.
- Outside a workspace with no `[NAME]` → falls back to `loom list`.

---

### `loom save`

Push workspace branches to their remotes.

```
loom save [--force]
```

| Flag | Description |
|------|-------------|
| `--force` | Push committed work even for repos with uncommitted changes. |

**Example output:**

```
$ loom save
Pushed: dsp-api, dsp-das
Skipped (dirty): ops-deploy (use --force to push anyway)
Sync: updated my-feature manifest
```

**Important:**
- `save` always pushes the **workspace branch** (via `git push -u origin <branch>`). It does **not** push to main.
- `--force` pushes committed work in dirty repos. It does **not** auto-commit.
- The `workflow` field in `[repos.<name>]` config (`"pr"` vs `"push"`) only affects the generated CLAUDE.md instructions for AI agents — it does not change `save` behavior.
- If `[sync]` is configured: writes a sync manifest JSON to the sync repo, commits, and pushes.
- Must be inside a workspace (no `[name]` argument).

---

### `loom open`

Restore a workspace from a sync manifest (cross-machine restore).

```
loom open <NAME>
```

| Flag | Description |
|------|-------------|
| `<NAME>` | Required. Workspace name from the sync manifest. |

**Requires** `[sync]` configured in config.toml — errors clearly if absent.

**Behavior:**
- Pulls the sync repo, reads the manifest for the given workspace.
- Clones missing repos to `{first_scan_root}/{org}/{repo}`.
- If a workspace already exists locally, reconciles repos (adds missing ones).
- Creates worktrees, generates agent files.

See [Cross-Machine Sync](#cross-machine-sync) for the full workflow.

---

### `loom down`

Tear down a workspace — remove worktrees and clean up.

```
loom down [NAME] [--force]
```

| Flag | Description |
|------|-------------|
| `[NAME]` | Workspace name. Optional — detects from cwd. |
| `--force` | Skip interactive prompts, force-delete branches with `-D`. |

**Example output:**

```
$ loom down my-feature
? Workspace has dirty repos. Remove them too? (y/n) y
✓ Removed worktree: dsp-api
✓ Removed worktree: dsp-das
✓ Deleted branch: loom/bold-cedar-hawk (dsp-api)
✓ Deleted branch: loom/bold-cedar-hawk (dsp-das)
✓ Workspace my-feature torn down
```

**Notes:**
- Without `--force`: prompts interactively if any repos have uncommitted changes.
- Branch deletion uses `git branch -d` (safe delete). With `--force`, uses `git branch -D`.
- **Warning:** if run from inside the workspace, your cwd becomes invalid after teardown. `cd` out first.

---

### `loom exec`

Run a command across all repos in the current workspace.

```
loom exec <CMD>...
```

| Flag | Description |
|------|-------------|
| `<CMD>...` | The command and its arguments to run in each repo. |

**Example:**

```
$ loom exec git log --oneline -1
── dsp-api ──
a1b2c3d Add metadata validation

── dsp-das ──
d4e5f6a Update component tests
```

**Notes:**
- Runs sequentially in each repo worktree.
- Exit code is non-zero if any repo command fails.
- Must be inside a workspace (no `[name]` argument).

---

### `loom shell`

Open a terminal in the workspace directory.

```
loom shell [NAME]
```

| Flag | Description |
|------|-------------|
| `[NAME]` | Workspace name. Optional — detects from cwd. |

Uses the `[terminal]` config command, falling back to `TERM_PROGRAM` env var detection.

---

### `loom refresh`

Regenerate agent files (CLAUDE.md and `.claude/settings.json`) from current config.

```
loom refresh [NAME] [--preset NAME]
```

| Flag | Description |
|------|-------------|
| `[NAME]` | Workspace name. Optional — detects from cwd. |
| `--preset NAME` | Set the permission preset. Pass `--preset ""` to remove the current preset. |

Useful after editing `config.toml` to regenerate agent files with updated settings.

---

### `loom tui`

Open the interactive TUI (terminal user interface).

```
loom tui
```

No arguments, no flags. See [TUI Guide](#tui-guide) for keybindings and screens.

---

### `loom completions`

Generate shell completions.

```
loom completions <SHELL>
```

| Flag | Description |
|------|-------------|
| `<SHELL>` | Required. One of: `bash`, `zsh`, `fish`, `elvish`, `powershell`. |

**Example:**

```sh
# Add to your ~/.zshrc:
eval "$(loom completions zsh)"
```

---

## Configuration Reference

Configuration lives at `~/.config/loom/config.toml`. Created by `loom init`, edited by hand.

### Section Overview

| Section | Required | Default When Absent |
|---------|----------|-------------------|
| `[registry]` | Yes | — |
| `[workspace]` | Yes | — |
| `[sync]` | No | Sync disabled (`loom open` unavailable) |
| `[terminal]` | No | Detected from `TERM_PROGRAM` env var |
| `[defaults]` | No | `branch_prefix = "loom"` |
| `[repos.<name>]` | No | `workflow = "pr"` for all repos |
| `[specs]` | No | No specs section in generated CLAUDE.md |
| `[agents]` | No | No agent files generated |
| `[agents.claude-code]` | No | Minimal settings.json (directories only) |
| `[agents.claude-code.sandbox]` | No | No sandbox isolation |
| `[agents.claude-code.presets.<name>]` | No | No presets available |

### Minimal Configuration

```toml
[registry]
scan_roots = ["~/_github.com"]

[workspace]
root = "~/workspaces"
```

### Full Annotated Example

```toml
# ── Required ──────────────────────────────────────────────

[registry]
scan_roots = ["~/_github.com", "~/code"]  # Dirs scanned for repos (2-level depth)

[workspace]
root = "~/workspaces"                      # Root directory for all workspaces

# ── Optional ──────────────────────────────────────────────

[sync]
repo = "~/path/to/sync-repo"              # Git repo for cross-machine sync manifests
path = "loom/"                             # Subdirectory within sync repo

[terminal]
command = "ghostty"                        # Terminal for `loom shell`

[defaults]
branch_prefix = "loom"                     # Prefix for worktree branches (loom/<random-name>)

# Per-repo workflow overrides
[repos.my-library]
workflow = "push"                          # "pr" (default) or "push" — affects CLAUDE.md only

[specs]
path = "specs/"                            # Relative path for specs section in generated CLAUDE.md

# ── Agent Integration ─────────────────────────────────────

[agents]
enabled = ["claude-code"]                  # Which agent integrations to generate

[agents.claude-code]
model = "opus"                             # Pin Claude model (optional)
allowed_tools = [                          # Global tool allowlist
    "Bash(gh issue *)",
    "Bash(gh run *)",
    "WebFetch(domain:docs.rs)",
]
enabled_plugins = ["my-plugin@my-marketplace"]
extra_known_marketplaces = [
    { name = "my-marketplace", repo = "owner/plugins-repo" },
]

# OS-level sandbox isolation
[agents.claude-code.sandbox]
enabled = true
auto_allow = true                          # Auto-allow Bash if sandboxed
excluded_commands = ["docker"]             # Commands that bypass sandbox
allow_unsandboxed_commands = false         # Block unsandboxed commands entirely

[agents.claude-code.sandbox.filesystem]
allow_write = ["~/.cargo", "~/.config/loom"]
deny_write = []
deny_read = []

[agents.claude-code.sandbox.network]
allowed_domains = ["github.com", "docs.rs", "crates.io"]

# Named presets — selected per workspace with --preset
[agents.claude-code.presets.rust]
allowed_tools = [
    "Bash(cargo test *)",
    "Bash(cargo fmt *)",
    "Bash(cargo clippy *)",
]

[agents.claude-code.presets.rust.sandbox.filesystem]
allow_write = ["~/.cargo"]

[agents.claude-code.presets.rust.sandbox.network]
allowed_domains = ["docs.rs", "crates.io"]
```

### Per-Section Reference

#### `[registry]` (Required)

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `scan_roots` | `string[]` | — | Directories to scan for git repos. Tilde-expanded. Repos must be at exactly 2 levels deep: `{root}/{org}/{repo}`. |

#### `[workspace]` (Required)

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `root` | `string` | — | Root directory where workspaces are created. Tilde-expanded. A `.loom/` subdirectory stores global state. |

#### `[sync]` (Optional)

Omit this entire section to disable cross-machine sync. `loom save` still pushes branches without it.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `repo` | `string` | — | Path to a local git repo used for storing sync manifests. Tilde-expanded. |
| `path` | `string` | — | Subdirectory within the sync repo for manifest files. |

#### `[terminal]` (Optional)

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `command` | `string` | Auto-detected | Terminal command for `loom shell`. Falls back to `TERM_PROGRAM` env var. |

#### `[defaults]` (Optional)

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `branch_prefix` | `string` | `"loom"` | Prefix for worktree branch names. Branches are created as `{prefix}/{random-name}`. |

#### `[repos.<name>]` (Optional, per-repo)

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `workflow` | `string` | `"pr"` | `"pr"` or `"push"`. Controls the workflow instructions in the generated CLAUDE.md. `"pr"`: create branch, open PR. `"push"`: push directly to main. Does **not** affect `loom save` behavior. |

#### `[specs]` (Optional)

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `path` | `string` | — | Relative path within the workspace for PRD/plan specs. Added to the generated CLAUDE.md. |

#### `[agents]` (Optional)

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | `string[]` | `[]` | Agent integrations to generate. Currently only `"claude-code"` is supported. |

#### `[agents.claude-code]` (Optional)

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `model` | `string` | — | Pin a Claude model (e.g., `"opus"`, `"sonnet"`). |
| `allowed_tools` | `string[]` | `[]` | Global tool allowlist. Format: `ToolName(specifier)`. See [Permission Pattern Syntax](#permission-pattern-syntax). |
| `enabled_plugins` | `string[]` | `[]` | Plugins to enable. Format: `"pluginName@marketplaceName"`. |
| `extra_known_marketplaces` | `table[]` | `[]` | Additional plugin marketplace sources. Each entry: `{ name = "...", repo = "owner/repo" }`. |

> **Warning:** If any `enabled_plugins` key is wrong or the marketplace isn't registered, the plugin silently won't load. Verify plugin activation after config changes.

#### `[agents.claude-code.sandbox]` (Optional)

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | `bool` | `false` | Enable OS-level sandbox isolation. |
| `auto_allow` | `bool` | `false` | Auto-allow Bash commands when sandboxed. Maps to `autoAllowBashIfSandboxed` in settings.json. |
| `excluded_commands` | `string[]` | `[]` | Commands that bypass the sandbox. |
| `allow_unsandboxed_commands` | `bool` | — | Whether to allow unsandboxed commands at all. |

#### `[agents.claude-code.sandbox.filesystem]` (Optional)

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `allow_write` | `string[]` | `[]` | Paths the sandbox allows writing to. |
| `deny_write` | `string[]` | `[]` | Paths explicitly denied for writing. |
| `deny_read` | `string[]` | `[]` | Paths explicitly denied for reading. |

#### `[agents.claude-code.sandbox.network]` (Optional)

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `allowed_domains` | `string[]` | `[]` | Network domains the sandbox allows access to. |

#### `[agents.claude-code.presets.<name>]` (Optional)

Named permission presets. See [Permission Presets](#permission-presets) for details.

| Option | Type | Description |
|--------|------|-------------|
| `allowed_tools` | `string[]` | Additional tool allowlist entries (merged with global). |
| `sandbox.filesystem.allow_write` | `string[]` | Additional write-allowed paths (merged with global). |
| `sandbox.filesystem.deny_write` | `string[]` | Additional write-denied paths (merged with global). |
| `sandbox.filesystem.deny_read` | `string[]` | Additional read-denied paths (merged with global). |
| `sandbox.network.allowed_domains` | `string[]` | Additional allowed domains (merged with global). |

### Example Configurations

#### Solo Developer (minimal, no sync, no agents)

```toml
[registry]
scan_roots = ["~/code"]

[workspace]
root = "~/workspaces"
```

#### Multi-Machine Sync

```toml
[registry]
scan_roots = ["~/_github.com"]

[workspace]
root = "~/workspaces"

[sync]
repo = "~/dotfiles"
path = "loom/"
```

#### AI Agent Setup (sandbox + presets)

```toml
[registry]
scan_roots = ["~/_github.com"]

[workspace]
root = "~/workspaces"

[agents]
enabled = ["claude-code"]

[agents.claude-code]
model = "opus"
allowed_tools = [
    "Bash(gh issue *)",
    "Bash(gh run *)",
]

[agents.claude-code.sandbox]
enabled = true
auto_allow = true
excluded_commands = ["docker"]

[agents.claude-code.sandbox.filesystem]
allow_write = ["~/.config/loom"]

[agents.claude-code.sandbox.network]
allowed_domains = ["github.com", "api.github.com"]

[agents.claude-code.presets.rust]
allowed_tools = [
    "Bash(cargo test *)",
    "Bash(cargo clippy *)",
    "Bash(cargo fmt *)",
]

[agents.claude-code.presets.rust.sandbox.filesystem]
allow_write = ["~/.cargo"]

[agents.claude-code.presets.rust.sandbox.network]
allowed_domains = ["docs.rs", "crates.io"]
```

#### Mixed Workflow (PR + push repos)

```toml
[registry]
scan_roots = ["~/_github.com"]

[workspace]
root = "~/workspaces"

[repos.dsp-api]
workflow = "pr"

[repos.pkm]
workflow = "push"

[specs]
path = "pkm/specs"

[agents]
enabled = ["claude-code"]
```
