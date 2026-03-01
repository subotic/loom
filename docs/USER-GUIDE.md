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

---

## Security Flavors

When you run `loom init`, you choose a **security flavor** that determines how AI agents are sandboxed. Each flavor solves a different problem.

### Comparison

| Aspect | Sandbox | Permissions | Both | Skip |
|--------|---------|-------------|------|------|
| **Problem it solves** | Prevent file/network access outside allowed paths | Control which tools an agent can use | Maximum isolation | No restrictions |
| **Bash isolation** | Yes — OS-level sandbox | No | Yes | No |
| **Tool allowlists** | No | Yes — explicit `permissions.allow` list | Yes | No |
| **Auto-allow Bash** | Yes (when sandboxed) | No | Yes (sandboxed commands only) | No |
| **Config complexity** | Low | Medium | High | None |
| **Recommended for** | Most users | Fine-grained control | Production/sensitive repos | Quick experiments |

### Decision Flow

1. **Do you want OS-level sandboxing?**
   - Yes → Sandbox or Both
   - No → Permissions or Skip
2. **Do you want explicit tool allowlists?**
   - Yes → Permissions or Both
   - No → Sandbox or Skip
3. **Want both?** → Both. **Neither?** → Skip.

### Persona Recommendations

- **"I just want it to work safely"** → **Sandbox** (recommended default)
- **"I need fine-grained control over which tools can run"** → **Permissions**
- **"Maximum security for sensitive repos"** → **Both**
- **"I'll configure security later"** → **Skip**

### What Gets Generated

Each flavor generates a different `.claude/settings.json`. Here's what each produces:

**Sandbox:**

```json
{
  "additionalDirectories": ["..."],
  "sandbox": {
    "enabled": true,
    "autoAllowBashIfSandboxed": true,
    "excludedCommands": ["docker"],
    "filesystem": {
      "allowWrite": ["~/.config/loom"]
    },
    "network": {
      "allowedDomains": ["github.com"]
    }
  }
}
```

**Permissions:**

```json
{
  "additionalDirectories": ["..."],
  "permissions": {
    "allow": [
      "Bash(cargo test *)",
      "Bash(gh issue *)",
      "WebFetch(domain:docs.rs)"
    ]
  }
}
```

**Both:**

```json
{
  "additionalDirectories": ["..."],
  "permissions": {
    "allow": ["Bash(cargo test *)", "Bash(gh issue *)"]
  },
  "sandbox": {
    "enabled": true,
    "autoAllowBashIfSandboxed": true,
    "excludedCommands": ["docker"],
    "filesystem": {
      "allowWrite": ["~/.config/loom"]
    }
  }
}
```

**Skip:**

```json
{
  "additionalDirectories": ["..."]
}
```

### Changing Flavors After Init

Edit `config.toml` to add or modify the `[agents.claude-code]` section, then regenerate:

```sh
loom refresh
```

This regenerates `.claude/settings.json` with the new settings.

### Permission Pattern Syntax

Tool allowlist entries use the format `ToolName(specifier)`:

| Tool | Pattern | Matches |
|------|---------|---------|
| Bash | `Bash(cargo test *)` | `cargo test`, `cargo test --release`, `cargo test my_test` |
| Bash | `Bash(gh issue *)` | `gh issue list`, `gh issue create`, etc. |
| WebFetch | `WebFetch(domain:docs.rs)` | Fetch from docs.rs |

**Matching rules:**
- Specifiers use **word-boundary** matching.
- `Bash(cargo test *)` matches `cargo test --release` but **not** `cargo testing`.
- The wildcard `*` matches any remaining arguments.

**Common mistakes:**
- Missing parentheses: `Bash cargo test` → must be `Bash(cargo test *)`
- Wrong capitalization: `bash(cargo test *)` → must be `Bash(...)` (capital B)
- Missing wildcard: `Bash(cargo test)` → matches only exact `cargo test` with no arguments

---

## Permission Presets

Presets let you define named bundles of permissions that can be applied per-workspace.

### Defining a Preset

```toml
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

### Selecting a Preset

```sh
loom new my-feature --preset rust     # At creation time
loom refresh --preset rust            # Change preset on existing workspace
loom refresh --preset ""              # Remove the current preset
```

### Merge Rules

When a preset is applied, its settings are **merged** with the global config:

| Field Type | Merge Behavior | Example Fields |
|------------|---------------|----------------|
| Arrays | Global **∪** Preset (union, sorted, deduplicated) | `allowed_tools`, `filesystem.allow_write`, `network.allowed_domains` |
| Booleans | Global only (presets cannot override) | `sandbox.enabled`, `sandbox.auto_allow`, `sandbox.excluded_commands` |

**Why:** Presets can only **add** permissions, never remove global restrictions. Boolean flags (enabled, auto_allow) are enforced by the global config only — the preset schema excludes them.

### Worked Example

**Global config:**

```toml
[agents.claude-code]
allowed_tools = ["Bash(gh issue *)", "Bash(gh run *)"]

[agents.claude-code.sandbox]
enabled = true
auto_allow = true

[agents.claude-code.sandbox.filesystem]
allow_write = ["~/.config/loom"]

[agents.claude-code.sandbox.network]
allowed_domains = ["github.com"]
```

**Rust preset:**

```toml
[agents.claude-code.presets.rust]
allowed_tools = ["Bash(cargo test *)", "Bash(cargo clippy *)"]

[agents.claude-code.presets.rust.sandbox.filesystem]
allow_write = ["~/.cargo"]

[agents.claude-code.presets.rust.sandbox.network]
allowed_domains = ["docs.rs", "crates.io"]
```

**Resulting `settings.json` with `--preset rust`:**

```json
{
  "additionalDirectories": ["..."],
  "permissions": {
    "allow": [
      "Bash(cargo clippy *)",
      "Bash(cargo test *)",
      "Bash(gh issue *)",
      "Bash(gh run *)"
    ]
  },
  "sandbox": {
    "enabled": true,
    "autoAllowBashIfSandboxed": true,
    "filesystem": {
      "allowWrite": ["~/.cargo", "~/.config/loom"]
    },
    "network": {
      "allowedDomains": ["crates.io", "docs.rs", "github.com"]
    }
  }
}
```

All arrays are merged, sorted, and deduplicated. Booleans come from global config only.

---

## Agent Integration

When `[agents]` is configured, LOOM generates two files per workspace:

### Generated `CLAUDE.md`

The workspace root gets a `CLAUDE.md` containing:

- **Workspace name** and link to LOOM
- **Repositories table** — directory, branch, source (+ workflow column if `[repos]` configured)
- **Working instructions** — how to use `loom exec`, `loom save`, `loom status`
- **Workflows section** — PR vs push instructions per repo (only if `[repos]` entries exist)
- **Specs section** — PRD/plan path conventions (only if `[specs]` configured)

### Generated `.claude/settings.json`

| config.toml Field | settings.json Field |
|---|---|
| (always) | `additionalDirectories` (paths to repo worktrees) |
| `model` | `model` |
| `allowed_tools` + preset | `permissions.allow` |
| `sandbox.*` | `sandbox.*` (camelCase mapped) |
| `enabled_plugins` | `enabledPlugins` (map of name → `true`) |
| `extra_known_marketplaces` | `extraKnownMarketplaces` |

### Plugin Configuration

Plugins are specified by name and marketplace:

```toml
enabled_plugins = ["my-plugin@my-marketplace"]
extra_known_marketplaces = [
    { name = "my-marketplace", repo = "owner/plugins-repo" },
]
```

> **Warning:** If any `enabled_plugins` key is wrong or the marketplace isn't in `extra_known_marketplaces` (or globally registered), the plugin **silently won't load**. Always verify plugin activation after config changes.

### Regeneration Triggers

Agent files are regenerated automatically by: `loom new`, `loom add`, `loom remove`, `loom open`, `loom refresh`.

> **Note:** After changing workspace composition (add/remove repos), restart Claude Code to pick up the updated `additionalDirectories`.

---

## Cross-Machine Sync

LOOM supports restoring workspaces across machines using a **save/open** model (inspired by [chezmoi](https://www.chezmoi.io/)). Save publishes workspace state; open recreates it elsewhere.

### Prerequisites

- `[sync]` must be configured in `config.toml`
- The sync repo must be a git repository with a remote

### Saving (Machine A)

`loom save` does two things:

1. **Pushes workspace branches** to each repo's remote (`git push -u origin <branch>`)
2. **Writes a sync manifest** (JSON) to the sync repo, commits, and pushes it

```
Machine A                     Git Remotes              Sync Repo
─────────                     ───────────              ─────────
loom save ──push branches──→  repo origins
          ──write manifest──→                          {sync.repo}/{sync.path}/{name}.json
          ──commit + push──→                           sync repo remote
```

`loom save` works **without** `[sync]` configured — it just pushes branches and skips the manifest.

### Opening (Machine B)

`loom open <name>` reconstructs a workspace from the sync manifest:

1. **Pulls the sync repo** to get the latest manifests
2. **Reads the manifest** for the named workspace
3. **Clones missing repos** to `{first_scan_root}/{org}/{repo}`
4. **Creates worktrees** and generates agent files

```
Sync Repo Remote              Machine B
────────────────              ─────────
          ←──git pull───────  loom open my-feature
          ──read manifest──→  clone missing repos
                              create worktrees
                              generate agent files
```

`loom open` **requires** `[sync]` configured — it errors clearly if absent.

### What Happens When...

| Scenario | What Happens | Recovery |
|----------|-------------|----------|
| Save with no changes | Pushes sync manifest only | — |
| Open with workspace already existing | Reconciles — adds missing repos | — |
| Open with repo already cloned locally | Reuses existing clone | — |
| Open with repo not cloned | Clones from `remoteUrl` in manifest | — |
| Save with dirty repos | Skips dirty repos (unless `--force`) | Commit or stash, then re-save |
| Two machines save concurrently | Last write wins (sync manifest overwritten) | Re-save from source machine |
| Open after workspace was torn down on source | Still works — manifest persists in sync repo | — |

### Conflict Handling

Sync manifests use **last-write-wins** — there's no automatic merge. This is by design (simplicity over cleverness).

- **Worst case:** a stale manifest causes `loom open` to create a workspace from old state
- **Recovery:** `loom down` the stale workspace, then `loom save` from the source machine
- **Prevention:** avoid saving the same workspace from multiple machines simultaneously

### Asymmetry as a Feature

`save` works without `[sync]` — it always pushes branches safely. `open` requires `[sync]` because it needs the manifest to know what to reconstruct. This asymmetry means `save` is always safe to run.

---

## TUI Guide

Launch the interactive TUI with:

```sh
loom tui
```

### Screens and Keybindings

#### WorkspaceList (main screen)

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Enter` | Open workspace detail |
| `n` | Start new workspace wizard |
| `r` | Refresh list |
| `q` / `Esc` | Quit |

**Columns:** `NAME`, `REPOS`, `STATUS`, `CREATED`

> **Note:** The TUI list shows fewer columns than `loom list` (no BRANCH or PRESET columns).

#### WorkspaceDetail

| Key | Action |
|-----|--------|
| `Esc` | Back to list |
| `d` | Tear down workspace (shows confirmation) |
| `q` | Quit |

Shows: workspace path, repo count, and per-repo table (REPO, BRANCH, STATUS, AHEAD/BEHIND).

#### NewWizard (4 steps)

| Step | Keys |
|------|------|
| **1. Enter name** | Type name, `Enter` to confirm (empty = random name), `Esc` to cancel |
| **2. Select org groups** | `Space` to toggle, `Enter` to confirm, `Esc` to go back |
| **3. Select repos** | `Space` to toggle, `Enter` to confirm, `Esc` to go back |
| **4. Confirm** | `Enter` to create, `Esc` to go back |

- If only one org exists, step 2 is skipped (auto-selected).
- Name accepts lowercase alphanumeric + hyphens only.

#### ConfirmDialog (teardown)

| Key | Action |
|-----|--------|
| `y` | Confirm teardown |
| `n` / `Esc` | Cancel |

### TUI vs CLI Comparison

| Feature | TUI | CLI |
|---------|-----|-----|
| Create workspace | Yes (wizard) | Yes (`loom new`) |
| Choose name | Yes | Yes |
| Random name | Yes (leave empty) | Yes (omit `[NAME]`) |
| Select repos | Interactive toggle | `--repos` flag or interactive |
| `--base` flag | No | Yes |
| `--preset` flag | No | Yes |
| `--repos` (non-interactive) | No | Yes |
| View workspaces | Yes | `loom list` |
| View workspace detail | Yes | `loom status` |
| Tear down workspace | Yes | `loom down` |
| `--force` teardown | No (uses safe delete) | Yes |
| Dirty repo teardown | Fails (no interactive prompt) | Prompts interactively |
| List columns | NAME, REPOS, STATUS, CREATED | NAME, REPOS, STATUS, BRANCH, PRESET, CREATED |

**When to use the TUI:** Quick workspace creation and overview. **When to use the CLI:** Advanced flags (`--base`, `--preset`), scripting, force operations.

---

## Internal Files

LOOM uses several JSON files to track state. Most are not meant for manual editing.

### `.loom.json` — Workspace Manifest

**Location:** `{workspace_root}/.loom.json` (per workspace)
**Created by:** `loom new`
**Updated by:** `loom add`, `loom remove`
**Safe to edit manually:** Yes, carefully (e.g., change `preset`)

```json
{
  "name": "my-feature",
  "branch": "loom/bold-cedar-hawk",
  "created": "2026-02-27T10:00:00Z",
  "baseBranch": "main",
  "preset": "rust",
  "repos": [
    {
      "name": "dsp-api",
      "originalPath": "/code/dasch-swiss/dsp-api",
      "worktreePath": "/workspaces/my-feature/dsp-api",
      "branch": "loom/bold-cedar-hawk",
      "remoteUrl": "git@github.com:dasch-swiss/dsp-api.git"
    }
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | Workspace name |
| `branch` | `string?` | Workspace branch name. Older manifests without this field fall back to `{prefix}/{name}`. |
| `created` | `datetime` | Creation timestamp (UTC) |
| `baseBranch` | `string?` | Base branch for worktrees (e.g., `"main"`) |
| `preset` | `string?` | Applied permission preset name |
| `repos[].name` | `string` | Repository name |
| `repos[].originalPath` | `string` | Path to the original repo clone |
| `repos[].worktreePath` | `string` | Path to the worktree in the workspace |
| `repos[].branch` | `string` | Branch name for this worktree |
| `repos[].remoteUrl` | `string` | Git remote URL |

### `state.json` — Global State

**Location:** `{workspace_root}/.loom/state.json` (singleton)
**Created by:** `loom new`
**Updated by:** all workspace lifecycle commands
**Safe to edit manually:** No — use `loom` commands instead

```json
{
  "workspaces": [
    {
      "name": "my-feature",
      "path": "/Users/you/workspaces/my-feature",
      "created": "2026-02-27T10:00:00Z",
      "repoCount": 2
    }
  ]
}
```

A `.bak` backup is written before each update. If the primary file is corrupt, LOOM tries the backup.

### Sync Manifest

**Location:** `{sync.repo}/{sync.path}/{workspace-name}.json`
**Created by:** `loom save`
**Read by:** `loom open`
**Safe to edit manually:** No

```json
{
  "name": "my-feature",
  "created": "2026-02-27T10:00:00Z",
  "status": "active",
  "branch": "loom/bold-cedar-hawk",
  "repos": [
    {
      "name": "dsp-api",
      "remoteUrl": "git@github.com:dasch-swiss/dsp-api.git",
      "branch": "loom/bold-cedar-hawk"
    }
  ]
}
```

The sync manifest is minimal — only what's needed to reconstruct the workspace on another machine (no local paths).

**Data flow:** `loom save` writes manifest → pushes to sync repo → `loom open` on another machine reads it → clones repos → creates worktrees.

### `config.toml`

**Location:** `~/.config/loom/config.toml`
**Created by:** `loom init`
**Safe to edit:** Yes — this is the primary user-edited file

See [Configuration Reference](#configuration-reference) for all options.

### Generated Agent Files

These are **not** internal state files — they are regenerated on every relevant operation:

- `CLAUDE.md` — workspace root
- `.claude/settings.json` — workspace root

See [Agent Integration](#agent-integration) for details.

---

## Troubleshooting

### Init Failures

**No git repos found in scan roots**
Check the [scan root convention](#scan-root-convention). Repos must be at exactly 2 levels: `{root}/{org}/{repo}`.

**Invalid TOML after manual config edit**
Validate syntax with a TOML checker. Common issues: missing quotes around strings with special characters, incorrect table nesting.

**Ctrl+C during init prompts**
Safe — no partial config is written. Re-run `loom init`.

**Workspace root directory doesn't exist**
`loom init` creates it automatically. If you later delete it, recreate it or update `config.toml`.

### Common Errors

**"Repo not found"**
The repo name doesn't match any repo in the registry. Check that it exists at the correct scan root depth (`{root}/{org}/{repo}`).

**"Workspace already exists"**
A workspace with that name is already registered. Use `loom open` to restore it, or `loom down` to tear it down first.

**Dirty repos blocking operations**
`loom save`, `loom remove`, and `loom down` refuse to act on repos with uncommitted changes. Options:
- Commit or stash your changes first
- Use `--force` to override

**"Sync repo push failed"**
The sync repo has a conflict or network issue. Resolve manually:
```sh
cd ~/path/to/sync-repo
git pull --rebase
git push
```

**Ctrl+C during workspace creation**
Safe — the workspace may be partially created. Use `loom down` to clean up.

**Running `loom down` from inside the workspace**
Your current directory becomes invalid after teardown. `cd` to a different directory first, then run `loom down`.

**Upgrading from older LOOM**
The old `.claude/settings.local.json` is automatically cleaned up when agent files are regenerated. Old manifests without a `branch` field are forward-compatible (fall back to `{prefix}/{name}`).

**Plugin not loading**
Check the `enabled_plugins` format: must be `"pluginName@marketplaceName"`. Verify the marketplace is registered in `extra_known_marketplaces` or globally in `~/.claude/plugins/known_marketplaces.json`. Plugins fail **silently** if the key is wrong.

### Config Validation Errors

**`"scan_roots path '...' does not exist"`**
The specified scan root directory doesn't exist on disk. Create it or remove it from `scan_roots`.

**`"invalid format '...' — expected ToolName(specifier)"`**
An `allowed_tools` entry doesn't match the expected pattern. Use `ToolName(specifier)` format, e.g., `Bash(cargo test *)`.

**`"Preset '...' not found. Available presets: ..."`**
The `--preset` argument doesn't match any preset defined in `[agents.claude-code.presets.*]`. Check config.toml for available preset names.

### Worktree Issues

**Worktree lock files**
Git worktrees use lock files to prevent concurrent operations. If a lock file persists after a crash, investigate what process holds it before deleting. Check with `git worktree list` in the original repo.

**Partial teardown recovery**
If `loom down` was interrupted, run it again — it's idempotent and handles partially torn-down workspaces.
