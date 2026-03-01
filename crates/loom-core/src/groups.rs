use std::collections::{BTreeMap, HashSet};

use anyhow::Result;

use crate::registry::RepoEntry;

/// A group entry in the selection UI: either a user-defined config group
/// or an auto-discovered org group.
#[derive(Debug, Clone)]
pub enum GroupEntry {
    ConfigGroup {
        name: String,
        repo_names: Vec<String>,
    },
    OrgGroup {
        name: String,
    },
}

/// Resolve named groups to a set of matching repos.
///
/// Returns `(matched_repos, warnings)` where warnings describe repo names
/// that could not be found in the registry. Errors if any group name is
/// unknown (not defined in config).
pub fn resolve_groups(
    group_names: &[String],
    groups: &BTreeMap<String, Vec<String>>,
    all_repos: &[RepoEntry],
) -> Result<(Vec<RepoEntry>, Vec<String>)> {
    // Deduplicate input group names (preserving first occurrence order)
    let mut seen = HashSet::new();
    let unique_names: Vec<&String> = group_names
        .iter()
        .filter(|n| seen.insert(n.as_str()))
        .collect();

    // Validate all group names exist
    for name in &unique_names {
        if !groups.contains_key(name.as_str()) {
            let available: Vec<&str> = groups.keys().map(|s| s.as_str()).collect();
            if available.is_empty() {
                anyhow::bail!(
                    "Group '{}' not found. No groups defined in config.toml.",
                    name
                );
            } else {
                anyhow::bail!(
                    "Group '{}' not found. Available groups: {}",
                    name,
                    available.join(", ")
                );
            }
        }
    }

    let mut matched = Vec::new();
    let mut warnings = Vec::new();
    let mut seen_paths: HashSet<std::path::PathBuf> = HashSet::new();

    for group_name in &unique_names {
        let repo_names = &groups[group_name.as_str()];
        for repo_name in repo_names {
            let found = all_repos
                .iter()
                .find(|r| r.name == *repo_name || format!("{}/{}", r.org, r.name) == *repo_name);
            match found {
                Some(r) => {
                    if seen_paths.insert(r.path.clone()) {
                        matched.push(r.clone());
                    }
                }
                None => {
                    warnings.push(format!(
                        "Group '{}': repo '{}' not found in registry",
                        group_name, repo_name
                    ));
                }
            }
        }
    }

    // Sort by (org, name) for deterministic ordering
    matched.sort_by(|a, b| (&a.org, &a.name).cmp(&(&b.org, &b.name)));

    Ok((matched, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_repo(name: &str, org: &str) -> RepoEntry {
        RepoEntry {
            name: name.to_string(),
            org: org.to_string(),
            path: PathBuf::from(format!("/code/{}/{}", org, name)),
            remote_url: None,
        }
    }

    fn make_groups(entries: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        entries
            .iter()
            .map(|(name, repos)| {
                (
                    name.to_string(),
                    repos.iter().map(|s| s.to_string()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn test_resolve_all_match() {
        let repos = vec![
            make_repo("dsp-api", "dasch-swiss"),
            make_repo("dsp-das", "dasch-swiss"),
            make_repo("sipi", "dasch-swiss"),
        ];
        let groups = make_groups(&[("dsp-stack", &["dsp-api", "dsp-das", "sipi"])]);

        let (matched, warnings) =
            resolve_groups(&["dsp-stack".to_string()], &groups, &repos).unwrap();

        assert_eq!(matched.len(), 3);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_resolve_partial_match() {
        let repos = vec![
            make_repo("dsp-api", "dasch-swiss"),
            make_repo("sipi", "dasch-swiss"),
        ];
        let groups = make_groups(&[("stack", &["dsp-api", "dsp-das", "sipi"])]);

        let (matched, warnings) = resolve_groups(&["stack".to_string()], &groups, &repos).unwrap();

        assert_eq!(matched.len(), 2);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("dsp-das"));
    }

    #[test]
    fn test_resolve_no_match() {
        let repos = vec![make_repo("unrelated", "org")];
        let groups = make_groups(&[("stack", &["dsp-api"])]);

        let (matched, warnings) = resolve_groups(&["stack".to_string()], &groups, &repos).unwrap();

        assert!(matched.is_empty());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn test_resolve_unknown_group() {
        let repos = vec![make_repo("dsp-api", "dasch-swiss")];
        let groups = make_groups(&[("dsp-stack", &["dsp-api"])]);

        let err = resolve_groups(&["nonexistent".to_string()], &groups, &repos).unwrap_err();

        assert!(err.to_string().contains("not found"));
        assert!(err.to_string().contains("dsp-stack"));
    }

    #[test]
    fn test_resolve_unknown_group_no_groups_defined() {
        let repos = vec![make_repo("dsp-api", "dasch-swiss")];
        let groups = BTreeMap::new();

        let err = resolve_groups(&["nonexistent".to_string()], &groups, &repos).unwrap_err();

        assert!(err.to_string().contains("No groups defined"));
    }

    #[test]
    fn test_resolve_overlapping_groups_deduplicates() {
        let repos = vec![
            make_repo("dsp-api", "dasch-swiss"),
            make_repo("dsp-das", "dasch-swiss"),
            make_repo("sipi", "dasch-swiss"),
        ];
        let groups = make_groups(&[
            ("stack", &["dsp-api", "dsp-das"]),
            ("full", &["dsp-api", "sipi"]),
        ]);

        let (matched, warnings) =
            resolve_groups(&["stack".to_string(), "full".to_string()], &groups, &repos).unwrap();

        // dsp-api appears in both groups but should be deduplicated
        assert_eq!(matched.len(), 3);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_resolve_duplicate_group_names_deduplicated() {
        let repos = vec![make_repo("dsp-api", "dasch-swiss")];
        let groups = make_groups(&[("stack", &["dsp-api"])]);

        let (matched, warnings) =
            resolve_groups(&["stack".to_string(), "stack".to_string()], &groups, &repos).unwrap();

        assert_eq!(matched.len(), 1);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_resolve_by_org_name() {
        // Use bare name "api" in the registry so the group entry "dasch-swiss/api"
        // must match via the format!("{}/{}", r.org, r.name) path, not r.name directly.
        let repos = vec![make_repo("api", "dasch-swiss")];
        let groups = make_groups(&[("stack", &["dasch-swiss/api"])]);

        let (matched, warnings) = resolve_groups(&["stack".to_string()], &groups, &repos).unwrap();

        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].name, "api");
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_resolve_sorted_by_org_name() {
        let repos = vec![
            make_repo("sipi", "dasch-swiss"),
            make_repo("tools", "acme"),
            make_repo("dsp-api", "dasch-swiss"),
        ];
        let groups = make_groups(&[("all", &["sipi", "tools", "dsp-api"])]);

        let (matched, _) = resolve_groups(&["all".to_string()], &groups, &repos).unwrap();

        assert_eq!(matched[0].org, "acme");
        assert_eq!(matched[1].name, "dsp-api");
        assert_eq!(matched[2].name, "sipi");
    }
}
