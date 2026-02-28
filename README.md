# LOOM

**Linked Orchestration Of Multirepos** — a Rust CLI/TUI that makes a distributed monorepo feel like a monorepo for the duration of a workstream.

## The Problem

Developers working with tightly-coupled repositories face a growing friction problem. When a change in one repo requires corresponding changes in others, main checkouts become contested. Branches conflict, uncommitted changes block new work, and switching between parallel tasks means manual cleanup.

Git worktrees solve this for single repositories. But **no tool coordinates worktrees across multiple repositories** with lifecycle management, agent-ready environments, and cross-machine portability.

AI coding agents amplify the problem — they need clean, isolated, pre-configured multi-repo environments on demand.

## How It Works

LOOM creates a centralized workspace directory containing correlated git worktrees — one per repo, all on a namespaced branch (`loom/{workspace-name}`). The workspace is self-contained, isolated from your main checkouts, and disposable when done.

```
~/workspaces/sipi-xyz/
├── .loom.json          # workspace manifest
├── CLAUDE.md           # generated — agent context
├── .claude/
│   └── settings.local.json
├── sipi/               # ← git worktree
├── dsp-api/            # ← git worktree
└── dsp-das/            # ← git worktree
```

## Quick Start

```sh
# Install
cargo install --git https://github.com/subotic/loom.git

# First-time setup
loom init

# Create a workspace
loom new my-feature

# Check status
loom status

# Run a command across all repos
loom exec cargo check

# Push branches and sync state
loom save

# Tear down when done
loom down
```

## Commands

| Command | Description |
|---------|-------------|
| `loom init` | First-run setup — creates config with scan roots, workspace root, sync repo |
| `loom new <name>` | Create a workspace with correlated worktrees |
| `loom add <repo>` | Add a repo to the current workspace |
| `loom remove <repo>` | Remove a repo (refuses if dirty) |
| `loom list` | List all workspaces |
| `loom status` | Per-repo branch, dirty state, ahead/behind |
| `loom exec <cmd>` | Run a command across all repos in the workspace |
| `loom shell` | Open a terminal in the workspace |
| `loom save` | Push branches and sync workspace manifest |
| `loom open <name>` | Reconstruct a workspace from sync manifest (cross-machine) |
| `loom down` | Tear down a workspace (safe — warns on dirty repos) |

## Configuration

LOOM stores its config at `~/.config/loom/config.toml`:

```toml
[registry]
scan_roots = ["~/_github.com"]    # where to discover repos

[workspace]
root = "~/workspaces"              # where workspaces live

[sync]
repo = "~/_github.com/user/pkm"   # git repo for cross-machine sync
path = "loom/"                     # directory within sync repo

[terminal]
command = "ghostty"                # terminal for `loom shell`

[agents]
enabled = ["claude-code"]          # agent integrations to generate
```

## Agent Integration

Workspaces are agent-ready out of the box. LOOM generates:

- **`CLAUDE.md`** — workspace context with repo table, branches, and sources
- **`.claude/settings.local.json`** — `additionalDirectories` so Claude Code can access all repos

Drop into any workspace and start an AI coding session immediately.

## Cross-Machine Sync

`loom save` pushes workspace manifests to a sync repo (e.g., your PKM). On another machine, `loom open my-feature` reconstructs the workspace — cloning missing repos, creating worktrees, and generating agent files.

## License

MIT
