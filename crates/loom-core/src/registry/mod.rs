pub mod url;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub use url::{CanonicalUrl, normalize_url};

/// A discovered git repository in the registry.
#[derive(Debug, Clone)]
pub struct RepoEntry {
    /// Short display name (basename, or `org/repo` if ambiguous)
    pub name: String,
    /// Organization/owner directory name
    pub org: String,
    /// Absolute path to the repository root
    pub path: PathBuf,
    /// Remote URL for origin (if available)
    pub remote_url: Option<String>,
}

/// Discover git repositories under the given scan roots.
///
/// Scan depth: `{scan_root}/{org}/{repo}` (exactly 2 levels).
/// Convention: repos live at `~/code/{org}/{repo}`.
///
/// Deduplicates across overlapping scan_roots using canonical paths.
/// Excludes directories under `workspace_root` (avoids scanning worktrees).
pub fn discover_repos(scan_roots: &[PathBuf], workspace_root: Option<&Path>) -> Vec<RepoEntry> {
    let mut seen_paths: HashSet<PathBuf> = HashSet::new();
    let mut entries = Vec::new();

    // Canonicalize workspace root for exclusion comparison
    let ws_canonical = workspace_root.and_then(|p| std::fs::canonicalize(p).ok());

    for scan_root in scan_roots {
        let root = match std::fs::canonicalize(scan_root) {
            Ok(p) => p,
            Err(_) => continue, // Skip non-existent roots
        };

        // Read org-level directories (depth 1)
        let org_dirs = match std::fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for org_entry in org_dirs.flatten() {
            let org_path = org_entry.path();
            if !org_path.is_dir() {
                continue;
            }

            // Skip workspace root to avoid scanning worktrees as repos
            if let Some(ref ws) = ws_canonical
                && (org_path.starts_with(ws) || ws.starts_with(&org_path))
            {
                continue;
            }

            let org_name = org_entry.file_name().to_string_lossy().to_string();

            // Skip hidden directories
            if org_name.starts_with('.') {
                continue;
            }

            // Read repo-level directories (depth 2)
            let repo_dirs = match std::fs::read_dir(&org_path) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            for repo_entry in repo_dirs.flatten() {
                let repo_path = repo_entry.path();
                if !repo_path.is_dir() {
                    continue;
                }

                let repo_name = repo_entry.file_name().to_string_lossy().to_string();

                // Skip hidden directories
                if repo_name.starts_with('.') {
                    continue;
                }

                // Check if it's a git repo
                if !repo_path.join(".git").exists() {
                    continue;
                }

                // Deduplicate by canonical path
                let canonical = match std::fs::canonicalize(&repo_path) {
                    Ok(p) => p,
                    Err(_) => continue,
                };

                // Skip if under workspace root
                if let Some(ref ws) = ws_canonical
                    && canonical.starts_with(ws)
                {
                    continue;
                }

                if !seen_paths.insert(canonical) {
                    continue; // Already discovered
                }

                // Get remote URL (best effort)
                let remote_url = crate::git::GitRepo::new(&repo_path)
                    .remote_url()
                    .ok()
                    .flatten();

                entries.push(RepoEntry {
                    name: repo_name,
                    org: org_name.clone(),
                    path: repo_path,
                    remote_url,
                });
            }
        }
    }

    // Handle name collisions: disambiguate repos with the same basename
    disambiguate_names(&mut entries);

    // Sort by (org, name) for consistent ordering
    entries.sort_by(|a, b| (&a.org, &a.name).cmp(&(&b.org, &b.name)));

    entries
}

/// Find a local repo matching a remote URL via canonical URL comparison.
pub fn match_by_url<'a>(repos: &'a [RepoEntry], url: &str) -> Option<&'a RepoEntry> {
    let target = normalize_url(url)?;
    repos
        .iter()
        .find(|r| r.remote_url.as_deref().and_then(normalize_url).as_ref() == Some(&target))
}

/// Disambiguate repos with the same basename by prefixing with `org/`.
fn disambiguate_names(entries: &mut [RepoEntry]) {
    // Count occurrences of each name
    let mut name_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for entry in entries.iter() {
        *name_counts.entry(entry.name.clone()).or_insert(0) += 1;
    }

    // Disambiguate duplicates
    for entry in entries.iter_mut() {
        if name_counts.get(&entry.name).copied().unwrap_or(0) > 1 {
            entry.name = format!("{}/{}", entry.org, entry.name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_repos_basic() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create org/repo structure with git init
        let repo_path = root.join("myorg").join("myrepo");
        std::fs::create_dir_all(&repo_path).unwrap();
        std::process::Command::new("git")
            .args(["init", &repo_path.to_string_lossy()])
            .env("LC_ALL", "C")
            .output()
            .unwrap();

        let entries = discover_repos(&[root.to_path_buf()], None);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "myrepo");
        assert_eq!(entries[0].org, "myorg");
    }

    #[test]
    fn test_discover_repos_skips_non_git() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create org/repo WITHOUT git init
        let repo_path = root.join("myorg").join("not-a-repo");
        std::fs::create_dir_all(&repo_path).unwrap();

        let entries = discover_repos(&[root.to_path_buf()], None);
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_discover_repos_skips_hidden() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Hidden org dir
        let hidden = root.join(".hidden").join("repo");
        std::fs::create_dir_all(&hidden).unwrap();
        std::process::Command::new("git")
            .args(["init", &hidden.to_string_lossy()])
            .env("LC_ALL", "C")
            .output()
            .unwrap();

        let entries = discover_repos(&[root.to_path_buf()], None);
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_discover_repos_deduplicates() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create org/repo
        let repo_path = root.join("myorg").join("myrepo");
        std::fs::create_dir_all(&repo_path).unwrap();
        std::process::Command::new("git")
            .args(["init", &repo_path.to_string_lossy()])
            .env("LC_ALL", "C")
            .output()
            .unwrap();

        // Scan the same root twice — should still find only 1
        let entries = discover_repos(&[root.to_path_buf(), root.to_path_buf()], None);
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_discover_repos_excludes_workspace_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create org/repo that is inside workspace root
        let ws_root = root.join("loom-workspaces");
        let repo_path = ws_root.join("myorg").join("myrepo");
        std::fs::create_dir_all(&repo_path).unwrap();
        std::process::Command::new("git")
            .args(["init", &repo_path.to_string_lossy()])
            .env("LC_ALL", "C")
            .output()
            .unwrap();

        // Should exclude repos under workspace root
        let entries = discover_repos(&[root.to_path_buf()], Some(&ws_root));
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_disambiguate_names() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Two repos with same basename under different orgs
        for org in &["org-a", "org-b"] {
            let repo_path = root.join(org).join("shared-name");
            std::fs::create_dir_all(&repo_path).unwrap();
            std::process::Command::new("git")
                .args(["init", &repo_path.to_string_lossy()])
                .env("LC_ALL", "C")
                .output()
                .unwrap();
        }

        let entries = discover_repos(&[root.to_path_buf()], None);
        assert_eq!(entries.len(), 2);

        // Both should be disambiguated with org prefix
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"org-a/shared-name"));
        assert!(names.contains(&"org-b/shared-name"));
    }

    #[test]
    fn test_match_by_url_ssh_to_https() {
        let entries = vec![RepoEntry {
            name: "repo".to_string(),
            org: "org".to_string(),
            path: PathBuf::from("/code/org/repo"),
            remote_url: Some("git@github.com:org/repo.git".to_string()),
        }];

        let found = match_by_url(&entries, "https://github.com/org/repo");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "repo");
    }

    #[test]
    fn test_match_by_url_no_match() {
        let entries = vec![RepoEntry {
            name: "repo".to_string(),
            org: "org".to_string(),
            path: PathBuf::from("/code/org/repo"),
            remote_url: Some("git@github.com:org/repo.git".to_string()),
        }];

        let found = match_by_url(&entries, "https://github.com/other-org/other-repo");
        assert!(found.is_none());
    }
}
