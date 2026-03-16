# Code Review Guidelines

See [CONVENTIONS.md](CONVENTIONS.md) for the full architectural conventions (module structure, config patterns, output conventions, error handling, naming). This checklist highlights what reviewers should actively verify on every PR.

## Always check

### Documentation completeness
- New config fields have: reference table entry in USER-GUIDE.md, config-to-settings mapping row, `loom init` hint in all flavors, full annotated config example entry
- New CLI subcommands/flags have `--help` text and USER-GUIDE.md entry
- Generated file changes (CLAUDE.md, settings.json, .mcp.json) reflected in "Agent Integration" section counts and subsection lists
- If a feature is only discoverable by reading source, it's not done

### CLI ergonomics
- Frequently-used flags have short forms (e.g., `-p` for `--preset`)
- Suggest short-form flags for new options that will see regular use

### Serde correctness
- New struct fields have `#[serde(default)]` and `skip_serializing_if` attributes
- `is_empty()` methods updated when struct fields are added to any config struct
- Config-to-JSON field name mapping follows camelCase convention (e.g., `allow_local_binding` -> `allowLocalBinding`)
- Existing struct literals in tests updated with `..Default::default()` when fields are added

### Validation coverage
- New string fields validated for empty/whitespace entries
- New path-like fields have traversal guards (`validate_no_path_traversal`)
- New collections validated for duplicates where applicable
- Validation logic not duplicated — extract helpers (e.g., `validate_mcp_server`)

### Test coverage
- Snapshot tests (`insta`) for new JSON output paths
- TOML round-trip tests for new serde fields
- Validation tests for both accept and reject cases
- `is_empty()` tests confirming new fields make config non-empty

### Output correctness
- User-facing output uses `println!` (stdout) or `eprintln!` (stderr), never `tracing::info!`
- Diagnostic/debug output uses `tracing::debug!` / `tracing::warn!`, never `println!`
- New modules added to the module table in CONVENTIONS.md

### Commit & PR hygiene
- Commits follow [docs/commit-conventions.md](docs/commit-conventions.md) — **release-please parses these to generate changelogs and version bumps; malformed commits break the release pipeline**
- One topic per commit (rebase-merge = commits land as-is on `main`)
- Correct type prefix: `fix:` → patch bump, `feat:` → minor, `!` → major. Wrong type = wrong version.
- First line is the changelog entry — write it for humans, release-please uses it verbatim
- Clippy clean, fmt clean, all tests pass

### Security
- No `.unwrap()` on user-supplied input — use `anyhow::bail!` or `?`
- Path traversal guards on any user-supplied path that reaches the filesystem
- Sandbox config changes reviewed for escape vectors

## Style

- Prefer `BTreeMap` over `HashMap` for deterministic serialization output
- Use `..Default::default()` in test struct literals for forward compatibility
- Merge arrays with `merge_sorted` pattern (union, sort, dedup); booleans are global-only
- Conditional JSON emission: omit keys when value is `None`, empty vec, or empty map
- Follow existing patterns (e.g., `merge_sorted` vs `merge_sandbox_paths`, conditional JSON emission)
- New fields mirror structure of similar existing fields

## Skip

- Snapshot `.snap` file contents — verify they were accepted, don't review the JSON formatting
- [docs/commit-conventions.md](docs/commit-conventions.md) — stable reference doc
- Formatting-only changes (`cargo fmt` diffs)
