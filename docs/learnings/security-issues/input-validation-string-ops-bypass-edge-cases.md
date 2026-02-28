---
title: "String-Based Validators Miss Edge Cases in Security-Critical Code"
date: 2026-02-28
category: security-issues
component: rust_crate
module: loom-core/config-validation, loom-core/agent-file-generation
problem_type: validation-gap
severity: high
symptoms:
  - "contains('@') accepted '@', '@marketplace', 'plugin@' as valid plugin identifiers"
  - "contains('..') missed absolute path traversal via Path::join('/etc/passwd')"
  - "contains('..') false-positived on filenames like 'v2..3/file'"
  - "unwrap_or_else silently produced empty settings files on serialization failure"
  - "Renamed generated file left stale legacy copy causing potential conflicts"
root_cause: "Validators used simple string operations (contains, boolean OR) instead of structured parsing — string presence checks cannot express structural constraints like 'non-empty part before AND after delimiter'"
tags:
  - input-validation
  - path-traversal
  - rust
  - string-operations
  - semantic-validation
  - security-guard
  - code-review
  - error-handling
  - serde
related: []
issue: "https://github.com/subotic/loom/pull/19"
---

# String-Based Validators Miss Edge Cases in Security-Critical Code

## Problem

PR #19 added Claude Code marketplace/plugin configuration to loom. Four rounds of automated code review caught a recurring pattern: validators and security guards that appeared correct at first glance but had subtle gaps because they used simple string operations instead of structured parsing.

**Observed symptoms across 4 review rounds:**

1. Plugin format validator (`contains('@')`) accepted `"@"`, `"@marketplace"`, `"plugin@"` — all structurally invalid
2. Path traversal guard (`contains("..")`) missed absolute paths (`"/etc/passwd"` bypasses `Path::join` base entirely on Unix)
3. Path traversal guard had false positives on legitimate filenames (`"v2..3/file"` contains `..` substring but has no `ParentDir` component)
4. `unwrap_or_else(|_| "{}")` silently wrote empty settings files, masking any serialization failure
5. Renaming `settings.local.json` to `settings.json` without deleting the old file left stale artifacts

## Investigation

### Review 1 — Found 2 bugs
- Plugin validation: `plugin.is_empty() || !plugin.contains('@')` only checks that `@` exists somewhere
- Path traversal: `file.relative_path.contains("..")` checks for the substring but not the path semantics

### Review 2 — Refinements
- `Path::components()` is more precise than `contains("..")` — avoids false positives
- `expect()` over `unwrap_or_else` for infallible operations surfaces bugs instead of hiding them
- `pub(crate)` for serde-internal helpers minimizes public API

### Review 3 — Migration gap
- Old `settings.local.json` never cleaned up — users get both files after upgrade

### Review 4 — False positive
- Flagged `enabledPlugins` JSON format as wrong — verified it was actually correct

Three of four reviews required code changes. The fourth was a false positive.

## Root Cause

The underlying pattern across all issues: **string operations don't capture the semantics of structured data**. `contains('@')` checks for character presence, not "non-empty name before @ and non-empty marketplace after @." `contains("..")` checks for a substring, not "this path has a ParentDir component." `unwrap_or_else` with a default masks the distinction between "this can fail and here's the fallback" and "this cannot fail."

## Solution

### 1. Plugin validation: `split_once` with destructuring guards

```rust
// BEFORE — accepted "@", "@marketplace", "plugin@"
if plugin.is_empty() || !plugin.contains('@') {
    anyhow::bail!("...");
}

// AFTER — rejects all invalid structures
match plugin.split_once('@') {
    Some((name, marketplace)) if !name.is_empty() && !marketplace.is_empty() => {}
    _ => {
        anyhow::bail!(
            "agents.claude-code.enabled_plugins: '{}' must be in \
             'pluginName@marketplaceName' format.",
            plugin
        );
    }
}
```

### 2. Path traversal: `Path::components()` + `is_absolute()`

```rust
// BEFORE — missed absolute paths, false-positived on "v2..3/file"
if file.relative_path.contains("..") {
    anyhow::bail!("...");
}

// AFTER — semantically correct
let rel = std::path::Path::new(&file.relative_path);
if rel.is_absolute()
    || rel.components().any(|c| c == std::path::Component::ParentDir)
{
    anyhow::bail!(
        "Agent '{}' produced an invalid relative path: {}",
        agent_name, file.relative_path
    );
}
```

### 3. Silent fallback replaced with `expect()`

```rust
// BEFORE — silently writes empty settings on failure
serde_json::to_string_pretty(&obj).unwrap_or_else(|_| "{}".to_string())

// AFTER — crashes immediately if the "impossible" happens
serde_json::to_string_pretty(&obj).expect("serde_json::Value is always serializable")
```

### 4. Legacy file cleanup

```rust
// Added before generating new files
if agent_name == "claude-code" {
    let legacy = ws_path.join(".claude/settings.local.json");
    if legacy.exists() {
        std::fs::remove_file(&legacy)?;
    }
}
```

## Prevention

### Patterns to adopt

1. **Parse, don't validate.** When a string has internal structure (`name@marketplace`, `owner/repo`, paths), parse it into typed parts immediately. `split_once` with guards is the minimum; a newtype struct like `PluginId { name, marketplace }` is ideal.

2. **Use `std::path::Path` for all path reasoning.** Never use string operations (`contains`, `starts_with`) to make security decisions about paths. `Path::components()` and `is_absolute()` handle platform-specific semantics that strings cannot.

3. **`expect()` for infallible operations, `?` for fallible ones.** Reserve `unwrap_or_else` for cases where the fallback is genuinely correct behavior. For operations that cannot fail by construction, `expect("reason")` documents the invariant and crashes loudly if the assumption breaks.

4. **Every file rename must include cleanup.** When a PR changes the name or location of a generated file, it must delete the old file (with an `if exists` guard for idempotency).

### Anti-patterns to avoid

- `contains('@')` / `contains("..")` / `starts_with("/")` as validators for structured strings
- `unwrap_or_else(|_| default)` on infallible operations — dead code that masks bugs
- Renaming generated artifacts without deleting the old version

### Test cases to add

- **Degenerate inputs for format validators**: `"@"`, `"@marketplace"`, `"plugin@"`, `""`, `"no-delimiter"`
- **Path escape vectors**: `"../etc/passwd"`, `"/etc/passwd"`, `"foo/../../bar"` — AND legitimate paths containing dots: `"v2..3/file"`, `".hidden/config"`
- **Migration tests**: create legacy file, run generation, assert legacy is gone and new file exists
- **Round-trip serialization**: verify `serde_json::Value` variants all serialize successfully (documents the infallibility assumption)

### CI checks

```yaml
# Flag string-based path security checks
- name: Deny string path checks
  run: |
    if grep -rn '\.contains("\.\.")' crates/; then
      echo "::error::Use Path::components() instead of string contains(..)"
      exit 1
    fi

# Flag silent serialization fallbacks
- name: Flag silent fallbacks
  run: |
    if grep -rn 'to_string_pretty.*unwrap_or_else' crates/; then
      echo "::warning::Verify this fallback is intentional"
    fi
```

## References

- [PR #19](https://github.com/subotic/loom/pull/19) — feature PR with 4 review rounds
- `crates/loom-core/src/config/mod.rs` — plugin/marketplace validation
- `crates/loom-core/src/agent/mod.rs` — path traversal guard, legacy file cleanup
- `crates/loom-core/src/agent/claude_code.rs` — settings generation
- `crates/loom-core/src/registry/url.rs` — existing example of thorough input validation with `split_once` pattern
