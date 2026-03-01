use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Sync status for a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    Active,
    Partial,
    Closed,
}

/// Per-workspace sync manifest, stored in the sync repo at `loom/{name}.json`.
///
/// Contains enough information to reconstruct the workspace on another machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncManifest {
    pub name: String,
    pub created: DateTime<Utc>,
    #[serde(default = "default_sync_status")]
    pub status: SyncStatus,
    /// The workspace branch name (e.g., `loom/bold-cedar-hawk`).
    /// Added after random branch naming was introduced; older manifests
    /// won't have this field, so it defaults to `None` for backward compatibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default)]
    pub repos: Vec<SyncRepoEntry>,
}

/// Minimal repo info needed to reconstruct a worktree on another machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRepoEntry {
    pub name: String,
    pub remote_url: String,
    pub branch: String,
}

fn default_sync_status() -> SyncStatus {
    SyncStatus::Active
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_manifest_round_trip() {
        let manifest = SyncManifest {
            name: "my-feature".to_string(),
            created: Utc::now(),
            status: SyncStatus::Active,
            branch: Some("loom/bold-cedar-hawk".to_string()),
            repos: vec![SyncRepoEntry {
                name: "dsp-api".to_string(),
                remote_url: "git@github.com:dasch-swiss/dsp-api.git".to_string(),
                branch: "loom/my-feature".to_string(),
            }],
        };

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let parsed: SyncManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "my-feature");
        assert_eq!(parsed.status, SyncStatus::Active);
        assert_eq!(parsed.branch, Some("loom/bold-cedar-hawk".to_string()));
        assert_eq!(parsed.repos.len(), 1);
    }

    #[test]
    fn test_sync_status_serialization() {
        assert_eq!(
            serde_json::to_string(&SyncStatus::Active).unwrap(),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&SyncStatus::Partial).unwrap(),
            "\"partial\""
        );
        assert_eq!(
            serde_json::to_string(&SyncStatus::Closed).unwrap(),
            "\"closed\""
        );
    }

    #[test]
    fn test_sync_manifest_defaults() {
        let json = r#"{
            "name": "minimal",
            "created": "2026-01-15T10:00:00Z"
        }"#;

        let manifest: SyncManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.status, SyncStatus::Active);
        assert!(manifest.branch.is_none());
        assert!(manifest.repos.is_empty());
    }
}
