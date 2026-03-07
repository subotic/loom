# Reviewer Guidelines

Checklist for human and AI reviewers. Not every item applies to every PR -- use judgment.

## Documentation & Discoverability

- [ ] New config fields: reference table in [USER-GUIDE.md](USER-GUIDE.md), config-to-settings mapping table, `loom init` hints (all flavors), full config example
- [ ] New CLI subcommands/flags: `--help` text and USER-GUIDE.md entry
- [ ] If a feature is only discoverable by reading source, it's not done

## CLI Ergonomics

- [ ] Frequently-used flags have short forms (e.g., `-p` for `--preset`)
- [ ] Suggest short-form flags for new options that will see regular use

## Commit & PR Hygiene

- [ ] Commits follow [docs/commit-conventions.md](commit-conventions.md)
- [ ] One topic per commit (rebase-merge = commits land as-is on `main`)
- [ ] PR description follows the template

## Rust Quality

- [ ] Clippy clean, fmt clean, all tests pass
- [ ] Serde attributes correct (`default`, `skip_serializing_if`, `rename`)
- [ ] `is_empty()` methods updated when struct fields change
- [ ] Validation coverage: empty entries, duplicates, path shape where applicable
- [ ] Snapshot tests for JSON output, round-trip tests for TOML serde

## Consistency

- [ ] Follow existing patterns (e.g., `merge_sorted` vs `merge_sandbox_paths`, conditional JSON emission)
- [ ] New fields mirror structure of similar existing fields
- [ ] Init template hints present in all relevant flavors

## Security

- [ ] Path traversal guards on user-supplied paths
- [ ] No `unsafe`, no `.unwrap()` on user input
- [ ] Sandbox config changes reviewed for escape vectors
