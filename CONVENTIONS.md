# LOOM Architectural Conventions

Project-specific architectural context for the loom repository.

## Stack

- **Language:** Rust (stable)
- **Crates:** `loom-cli` (binary), `loom-core` (library)
- **Key deps:** `serde`/`toml`/`serde_json` (config), `anyhow` (errors), `clap` (CLI), `ratatui` (TUI), `insta` (snapshot tests), `tempfile` (test fixtures), `chrono` (timestamps)
- **Test framework:** `cargo test` + `insta` snapshots (`cargo insta accept` for new/changed)

## Module Structure

Scopes match crate modules and map to conventional commit scopes:

| Module | Scope | Responsibility |
|--------|-------|---------------|
| `config/` | `config` | TOML parsing, validation, `Config` struct tree |
| `config/init.rs` | `config` | `loom init` wizard logic, security flavors |
| `agent/` | `agent` | Agent file generation dispatch, `GeneratedFile` trait |
| `agent/claude_code.rs` | `agent` | CLAUDE.md, settings.json, .mcp.json generation |
| `workspace/` | `workspace` | Workspace lifecycle (new, add, remove, down, status) |
| `git/` | `git` | Git command wrapper, worktree operations |
| `manifest/` | `manifest` | `.loom.json` read/write, name validation |
| `registry/` | `registry` | Repo discovery from scan roots |
| `sync/` | `sync` | Cross-machine save/open via sync repo |
| `tui/` | `tui` | Ratatui terminal UI |

## Config Pattern

All config flows through `Config` -> `ClaudeCodeConfig` -> generated files.

### Adding a new config field

1. **Struct field** in `config/mod.rs` with `#[serde(default, skip_serializing_if = "...")]`
2. **Update `is_empty()`** on the parent struct
3. **Validation** in `validate_agent_config()` — use existing helpers (`validate_no_empty_entries`, `validate_no_duplicates`, `validate_mcp_server`)
4. **Emit** in the appropriate generator function in `agent/claude_code.rs`
5. **Tests:** TOML round-trip, validation accept/reject, snapshot for generated output, `is_empty()` assertion
6. **Docs:** USER-GUIDE.md reference table, annotated example, section overview table

### Serde conventions

- `Option<T>` fields: `skip_serializing_if = "Option::is_none"`
- `Vec<T>` fields: `skip_serializing_if = "Vec::is_empty"`
- `BTreeMap<K,V>` fields: `skip_serializing_if = "BTreeMap::is_empty"`
- Custom structs: `skip_serializing_if = "StructName::is_empty"`
- Always use `BTreeMap` over `HashMap` for deterministic output
- TOML uses `snake_case`; JSON output uses `camelCase` (manual mapping in generators)

## Settings Generation

`generate_settings()` builds a `serde_json::Map` manually (not via derive) to control key naming and conditional emission:

```rust
if !value.is_empty() {
    obj["camelCaseKey"] = serde_json::json!(value);
}
```

### Merge semantics

- **Arrays** (allowed_tools, filesystem paths, domains): global union preset, sorted, deduped via `merge_sorted()`
- **Booleans** (sandbox.enabled, auto_allow): global only — presets cannot override
- **MCP servers**: preset overrides global by name (last-wins in BTreeMap insert)
- **Sandbox paths**: absolute paths converted to `//` prefix via `to_sandbox_path()`; unix sockets are NOT converted

## Error Handling

- Use `anyhow::Result` and `anyhow::bail!` for all user-facing errors
- Validation errors include context path (e.g., `"agents.claude-code.presets.{name}.mcp_servers.{srv}"`)
- No `panic!` or `.unwrap()` on user input — `expect()` only for infallible operations (e.g., serde on known-good types)

## Testing Patterns

- **Snapshot tests** (`insta::assert_snapshot!`): for all generated JSON/Markdown output. Accept with `cargo insta accept`.
- **Round-trip tests**: serialize to TOML, deserialize back, assert equality
- **Validation tests**: both positive (valid input passes) and negative (invalid input returns descriptive error)
- **Test helpers**: `test_manifest()`, `test_config()`, `test_config_full()` in `claude_code.rs` tests
- **Struct literals**: use `..Default::default()` for forward compatibility when new fields are added

## Naming Conventions

| Entity | Convention | Example |
|--------|-----------|---------|
| Config struct | PascalCase + `Config` suffix | `SandboxNetworkConfig` |
| Config field | snake_case | `allow_local_binding` |
| JSON output key | camelCase | `allowLocalBinding` |
| Validation fn | `validate_` prefix | `validate_mcp_server()` |
| Generator fn | `generate_` prefix | `generate_mcp_json()` |
| Builder fn | `build_` prefix | `build_sandbox_json()` |
| Test fn | `test_` prefix + descriptive | `test_settings_with_allow_local_binding_snapshot` |
