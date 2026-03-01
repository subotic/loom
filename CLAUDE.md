# LOOM

Rust CLI/TUI for multi-repo workspace orchestration with git worktrees.

See [docs/USER-GUIDE.md](docs/USER-GUIDE.md) for the full user guide.

## Commit Conventions

This repo uses **rebase merges only** — every commit lands on `main` as-is. Commits must tell a clear story because they:

1. Become the permanent project history
2. Feed release-please to generate changelogs and version bumps
3. Are read by future contributors (human and AI) to understand *why* changes were made

### Rules

- **One topic per commit.** A PR may address multiple concerns, but each commit must be a single, self-contained logical change.
- **Use [Conventional Commits](https://www.conventionalcommits.org/):**
  - `feat:` / `feat(scope):` — new functionality (minor version bump)
  - `fix:` / `fix(scope):` — bug fix (patch version bump)
  - `refactor:` — code restructuring, no behavior change
  - `test:` — adding or updating tests
  - `ci:` — CI/CD changes
  - `docs:` — documentation only
  - `build:` — build system, dependencies
  - `chore:` — maintenance tasks
- **Scopes** match crate modules: `workspace`, `git`, `config`, `manifest`, `sync`, `tui`, `agent`, `registry`, `cli`. Non-code scopes: `learnings`, `ci`.
- **First line** is the changelog entry — write it for humans. Explain *what* changed, not *how*.
- **Body** (optional) explains *why* — the motivation, trade-offs, or context that isn't obvious from the diff.
- **Breaking changes:** add `!` after the type (e.g., `feat(config)!:`) and include a `BREAKING CHANGE:` footer.

### Examples

```
feat(git): base worktree branches on remote default branch

Fetch from origin before creating worktree branches so workspaces
start with the latest upstream state instead of the local HEAD.

Closes #7
```

```
fix(workspace): handle missing .loom.json gracefully

Return a clear error instead of panicking when .loom.json is
not found in the workspace root.
```
