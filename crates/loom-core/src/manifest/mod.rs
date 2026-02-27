pub mod state;
pub mod sync;
pub mod workspace;

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Serialize, de::DeserializeOwned};

pub use state::{GlobalState, WorkspaceIndex};
pub use sync::{SyncManifest, SyncRepoEntry, SyncStatus};
pub use workspace::{RepoManifestEntry, WorkspaceManifest};

/// Read a JSON manifest from disk.
///
/// For `GlobalState`, falls back to `.bak` on parse failure, then returns default.
pub fn read_manifest<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read manifest at {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse manifest at {}", path.display()))
}

/// Read GlobalState with backup recovery.
///
/// On read/parse failure, tries `.bak` file. If both fail, returns empty state.
pub fn read_global_state(path: &Path) -> GlobalState {
    // Try primary file
    if let Ok(state) = read_manifest::<GlobalState>(path) {
        return state;
    }

    // Try backup
    let bak = path.with_extension("json.bak");
    if let Ok(state) = read_manifest::<GlobalState>(&bak) {
        return state;
    }

    // Return empty state
    GlobalState::default()
}

/// Write a JSON manifest atomically using tempfile + rename.
///
/// For `GlobalState`, creates a `.bak` backup before writing.
pub fn write_manifest<T: Serialize>(path: &Path, data: &T) -> Result<()> {
    let content =
        serde_json::to_string_pretty(data).context("Failed to serialize manifest to JSON")?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    // Atomic write: temp file in same dir, then persist (rename)
    let parent = path.parent().unwrap_or(Path::new("."));
    let tmp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("Failed to create temp file in {}", parent.display()))?;
    std::fs::write(tmp.path(), content.as_bytes())
        .with_context(|| "Failed to write manifest to temp file".to_string())?;
    tmp.persist(path)
        .with_context(|| format!("Failed to persist manifest to {}", path.display()))?;

    Ok(())
}

/// Write GlobalState with `.bak` backup.
pub fn write_global_state(path: &Path, state: &GlobalState) -> Result<()> {
    // Create backup of existing file
    if path.exists() {
        let bak = path.with_extension("json.bak");
        let _ = std::fs::copy(path, &bak); // Best effort backup
    }

    write_manifest(path, state)
}

/// Validate a workspace name.
///
/// Rules:
/// - Lowercase alphanumeric + hyphens only
/// - Max 63 chars (git branch component limit)
/// - Must not start or end with hyphen
/// - Must not be empty
pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Workspace name cannot be empty.");
    }

    if name.len() > 63 {
        anyhow::bail!(
            "Workspace name '{}' exceeds 63 character limit ({} chars).",
            name,
            name.len()
        );
    }

    if name.starts_with('-') || name.ends_with('-') {
        anyhow::bail!(
            "Workspace name '{}' must not start or end with a hyphen.",
            name
        );
    }

    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        anyhow::bail!(
            "Workspace name '{}' contains invalid characters. Use lowercase alphanumeric and hyphens only.",
            name
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_valid() {
        assert!(validate_name("my-feature").is_ok());
        assert!(validate_name("fix-123").is_ok());
        assert!(validate_name("a").is_ok());
        assert!(validate_name("abc123").is_ok());
    }

    #[test]
    fn test_validate_name_empty() {
        assert!(validate_name("").is_err());
    }

    #[test]
    fn test_validate_name_too_long() {
        let long = "a".repeat(64);
        assert!(validate_name(&long).is_err());

        let ok = "a".repeat(63);
        assert!(validate_name(&ok).is_ok());
    }

    #[test]
    fn test_validate_name_hyphen_edges() {
        assert!(validate_name("-start").is_err());
        assert!(validate_name("end-").is_err());
        assert!(validate_name("-both-").is_err());
    }

    #[test]
    fn test_validate_name_invalid_chars() {
        assert!(validate_name("has spaces").is_err());
        assert!(validate_name("UPPERCASE").is_err());
        assert!(validate_name("under_score").is_err());
        assert!(validate_name("dot.name").is_err());
        assert!(validate_name("slash/name").is_err());
    }

    #[test]
    fn test_read_write_manifest_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.json");

        let manifest = WorkspaceManifest {
            name: "test".to_string(),
            created: chrono::Utc::now(),
            base_branch: Some("main".to_string()),
            repos: vec![],
        };

        write_manifest(&path, &manifest).unwrap();
        let loaded: WorkspaceManifest = read_manifest(&path).unwrap();

        assert_eq!(loaded.name, "test");
        assert_eq!(loaded.base_branch, Some("main".to_string()));
    }

    #[test]
    fn test_global_state_backup_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");

        // Write initial state (no .bak yet since file doesn't exist)
        let mut state = GlobalState::default();
        state.upsert(WorkspaceIndex {
            name: "ws1".to_string(),
            path: std::path::PathBuf::from("/loom/ws1"),
            created: chrono::Utc::now(),
            repo_count: 2,
        });
        write_global_state(&path, &state).unwrap();

        // Write again — this time the first write becomes the .bak
        write_global_state(&path, &state).unwrap();

        // Corrupt the primary file
        std::fs::write(&path, "not json").unwrap();

        // Should fall back to .bak
        let recovered = read_global_state(&path);
        assert_eq!(recovered.workspaces.len(), 1);
        assert_eq!(recovered.workspaces[0].name, "ws1");
    }

    #[test]
    fn test_global_state_both_corrupt() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");

        // Write corrupt files
        std::fs::write(&path, "bad").unwrap();
        std::fs::write(path.with_extension("json.bak"), "also bad").unwrap();

        // Should return empty state
        let state = read_global_state(&path);
        assert!(state.workspaces.is_empty());
    }

    #[test]
    fn test_global_state_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");

        let state = read_global_state(&path);
        assert!(state.workspaces.is_empty());
    }
}
