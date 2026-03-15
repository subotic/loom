# Code Review Guidelines

## Always check

### Documentation completeness
- New config fields have: reference table entry in USER-GUIDE.md, config-to-settings mapping row, `loom init` hint in all flavors, full annotated config example entry
- New CLI subcommands/flags have `--help` text and USER-GUIDE.md entry
- Generated file changes (CLAUDE.md, settings.json, .mcp.json) reflected in "Agent Integration" section counts and subsection lists

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

### Security
- No `.unwrap()` on user-supplied input — use `anyhow::bail!` or `?`
- Path traversal guards on any user-supplied path that reaches the filesystem
- Sandbox config changes reviewed for escape vectors

## Style

- Prefer `BTreeMap` over `HashMap` for deterministic serialization output
- Use `..Default::default()` in test struct literals for forward compatibility
- Merge arrays with `merge_sorted` pattern (union, sort, dedup); booleans are global-only
- Conditional JSON emission: omit keys when value is `None`, empty vec, or empty map

## Skip

- Snapshot `.snap` file contents — verify they were accepted, don't review the JSON formatting
- `docs/commit-conventions.md` and `docs/reviewer-guidelines.md` — stable reference docs
- Formatting-only changes (`cargo fmt` diffs)
