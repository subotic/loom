use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Global state index, stored at `~/loom/.loom/state.json`.
///
/// Tracks all known workspaces for quick listing without scanning the filesystem.
/// Written atomically with `.bak` backup for corruption recovery.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GlobalState {
    #[serde(default)]
    pub workspaces: Vec<WorkspaceIndex>,
}

/// Summary entry for a workspace in the global state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceIndex {
    pub name: String,
    pub path: PathBuf,
    pub created: DateTime<Utc>,
    #[serde(default)]
    pub repo_count: usize,
}

impl GlobalState {
    /// Add or update a workspace entry.
    pub fn upsert(&mut self, entry: WorkspaceIndex) {
        if let Some(existing) = self.workspaces.iter_mut().find(|w| w.name == entry.name) {
            *existing = entry;
        } else {
            self.workspaces.push(entry);
        }
    }

    /// Remove a workspace by name. Returns true if found and removed.
    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.workspaces.len();
        self.workspaces.retain(|w| w.name != name);
        self.workspaces.len() < before
    }

    /// Find a workspace by name.
    pub fn find(&self, name: &str) -> Option<&WorkspaceIndex> {
        self.workspaces.iter().find(|w| w.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_state_round_trip() {
        let state = GlobalState {
            workspaces: vec![WorkspaceIndex {
                name: "feature-x".to_string(),
                path: PathBuf::from("/loom/feature-x"),
                created: Utc::now(),
                repo_count: 3,
            }],
        };

        let json = serde_json::to_string_pretty(&state).unwrap();
        let parsed: GlobalState = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.workspaces.len(), 1);
        assert_eq!(parsed.workspaces[0].name, "feature-x");
        assert_eq!(parsed.workspaces[0].repo_count, 3);
    }

    #[test]
    fn test_global_state_camel_case() {
        let state = GlobalState {
            workspaces: vec![WorkspaceIndex {
                name: "test".to_string(),
                path: PathBuf::from("/loom/test"),
                created: Utc::now(),
                repo_count: 2,
            }],
        };

        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("repoCount"));
        assert!(!json.contains("repo_count"));
    }

    #[test]
    fn test_global_state_default_empty() {
        let json = "{}";
        let state: GlobalState = serde_json::from_str(json).unwrap();
        assert!(state.workspaces.is_empty());
    }

    #[test]
    fn test_upsert_new() {
        let mut state = GlobalState::default();
        state.upsert(WorkspaceIndex {
            name: "ws1".to_string(),
            path: PathBuf::from("/loom/ws1"),
            created: Utc::now(),
            repo_count: 2,
        });
        assert_eq!(state.workspaces.len(), 1);
    }

    #[test]
    fn test_upsert_update() {
        let mut state = GlobalState::default();
        state.upsert(WorkspaceIndex {
            name: "ws1".to_string(),
            path: PathBuf::from("/loom/ws1"),
            created: Utc::now(),
            repo_count: 2,
        });
        state.upsert(WorkspaceIndex {
            name: "ws1".to_string(),
            path: PathBuf::from("/loom/ws1"),
            created: Utc::now(),
            repo_count: 5,
        });
        assert_eq!(state.workspaces.len(), 1);
        assert_eq!(state.workspaces[0].repo_count, 5);
    }

    #[test]
    fn test_remove() {
        let mut state = GlobalState::default();
        state.upsert(WorkspaceIndex {
            name: "ws1".to_string(),
            path: PathBuf::from("/loom/ws1"),
            created: Utc::now(),
            repo_count: 1,
        });
        assert!(state.remove("ws1"));
        assert!(state.workspaces.is_empty());
        assert!(!state.remove("ws1")); // Already removed
    }

    #[test]
    fn test_find() {
        let mut state = GlobalState::default();
        state.upsert(WorkspaceIndex {
            name: "ws1".to_string(),
            path: PathBuf::from("/loom/ws1"),
            created: Utc::now(),
            repo_count: 1,
        });
        assert!(state.find("ws1").is_some());
        assert!(state.find("ws2").is_none());
    }
}
