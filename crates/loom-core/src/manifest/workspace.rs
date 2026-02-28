use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Per-workspace manifest, stored as `.loom.json` in the workspace directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceManifest {
    pub name: String,
    pub created: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(default)]
    pub repos: Vec<RepoManifestEntry>,
}

/// Entry for a single repo within a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoManifestEntry {
    pub name: String,
    pub original_path: PathBuf,
    pub worktree_path: PathBuf,
    pub branch: String,
    pub remote_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_manifest_round_trip() {
        let manifest = WorkspaceManifest {
            name: "my-feature".to_string(),
            created: Utc::now(),
            base_branch: Some("main".to_string()),
            preset: None,
            repos: vec![RepoManifestEntry {
                name: "dsp-api".to_string(),
                original_path: PathBuf::from("/code/dasch-swiss/dsp-api"),
                worktree_path: PathBuf::from("/loom/my-feature/dsp-api"),
                branch: "loom/my-feature".to_string(),
                remote_url: "git@github.com:dasch-swiss/dsp-api.git".to_string(),
            }],
        };

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let parsed: WorkspaceManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "my-feature");
        assert_eq!(parsed.base_branch, Some("main".to_string()));
        assert_eq!(parsed.repos.len(), 1);
        assert_eq!(parsed.repos[0].name, "dsp-api");
    }

    #[test]
    fn test_workspace_manifest_camel_case() {
        let manifest = WorkspaceManifest {
            name: "test".to_string(),
            created: Utc::now(),
            base_branch: Some("develop".to_string()),
            preset: None,
            repos: vec![],
        };

        let json = serde_json::to_string(&manifest).unwrap();
        assert!(json.contains("baseBranch"));
        assert!(!json.contains("base_branch"));
    }

    #[test]
    fn test_workspace_manifest_optional_fields() {
        // Deserialize without optional fields
        let json = r#"{
            "name": "minimal",
            "created": "2026-01-15T10:00:00Z"
        }"#;

        let manifest: WorkspaceManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.name, "minimal");
        assert!(manifest.base_branch.is_none());
        assert!(manifest.repos.is_empty());
    }
}
