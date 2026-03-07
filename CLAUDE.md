# LOOM

Rust CLI/TUI for multi-repo workspace orchestration with git worktrees.

See [docs/USER-GUIDE.md](docs/USER-GUIDE.md) for the full user guide.

## Commit Conventions

This repo uses **rebase merges only** — every commit lands on `main` as-is.

- **One topic per commit.** Each commit is a single, self-contained logical change.
- **Conventional Commits** with scopes (e.g., `feat(config):`, `fix(git):`). Scopes match crate modules.
- **First line** is the changelog entry — write it for humans.
- **Breaking changes:** add `!` after the type and include a `BREAKING CHANGE:` footer.

Full details: [docs/commit-conventions.md](docs/commit-conventions.md)

## Review Context

See [docs/reviewer-guidelines.md](docs/reviewer-guidelines.md) for the review checklist (docs, CLI ergonomics, Rust quality, security).
