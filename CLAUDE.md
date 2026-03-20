# LOOM

Rust CLI/TUI for multi-repo workspace orchestration with git worktrees.

See [docs/USER-GUIDE.md](docs/USER-GUIDE.md) for the full user guide.

## Commit Conventions

**This is critical.** release-plz parses commit messages on `main` to generate changelogs and determine version bumps. Malformed commits produce broken changelogs or missed releases.

This repo uses **rebase merges only** — every commit lands on `main` as-is.

- **One topic per commit.** Each commit is a single, self-contained logical change.
- **Conventional Commits** with scopes (e.g., `feat(config):`, `fix(git):`). Scopes match crate modules.
- **First line** is the changelog entry — write it for humans. release-plz uses it verbatim.
- **Breaking changes:** add `!` after the type and include a `BREAKING CHANGE:` footer.
- **Type determines version bump:** `fix:` → patch, `feat:` → minor, `!` / `BREAKING CHANGE:` → major.
- **Non-release types** (`docs:`, `test:`, `ci:`, `chore:`, `refactor:`, `style:`, `build:`) do not trigger a release on their own.

Full details: [docs/commit-conventions.md](docs/commit-conventions.md)

## Before Pushing

Run these checks before every push. Do not push if any fail.

```
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

## Review Context

See [REVIEW.md](REVIEW.md) for the review checklist and [CONVENTIONS.md](CONVENTIONS.md) for architectural conventions.
