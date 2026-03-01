/// Maximum number of attempts when generating a unique name.
pub const MAX_NAME_RETRIES: usize = 10;

/// Adjectives for random name generation (56 words).
/// All lowercase ASCII, no overlap with MODIFIERS or NOUNS.
pub const ADJECTIVES: &[&str] = &[
    "bold", "brave", "bright", "calm", "clear", "cool", "crisp", "dusk", "deep", "fair", "fast",
    "fine", "firm", "free", "glad", "gold", "good", "gray", "keen", "kind", "late", "lean", "live",
    "long", "lost", "loud", "mild", "neat", "next", "open", "pale", "past", "pure", "quick",
    "quiet", "rare", "real", "rich", "safe", "sane", "slim", "slow", "soft", "sole", "sure",
    "tall", "tame", "thin", "true", "vast", "warm", "wide", "wild", "wise", "young", "zero",
];

/// Color/material modifiers for random name generation (30 words).
/// All lowercase ASCII, no overlap with ADJECTIVES or NOUNS.
pub const MODIFIERS: &[&str] = &[
    "amber", "azure", "coral", "cream", "frost", "ivory", "jade", "lemon", "lilac", "maple",
    "misty", "ocean", "olive", "pearl", "plum", "raven", "royal", "ruby", "rusty", "sandy", "silk",
    "solar", "stone", "storm", "tidal", "timber", "velvet", "violet", "winter", "cedar",
];

/// Nature-themed nouns for random name generation (54 words).
/// All lowercase ASCII, no overlap with ADJECTIVES or MODIFIERS.
pub const NOUNS: &[&str] = &[
    "bear", "brook", "cliff", "crane", "creek", "dawn", "deer", "dove", "dune", "eagle", "elm",
    "fern", "finch", "flame", "fox", "grove", "hawk", "heron", "hill", "lake", "lark", "leaf",
    "lynx", "marsh", "mesa", "moth", "oak", "otter", "owl", "peak", "pine", "pond", "rain", "reef",
    "ridge", "river", "robin", "sage", "shore", "sky", "sparrow", "spruce", "star", "crest",
    "stream", "swift", "thrush", "tide", "trail", "vale", "vine", "wave", "wren", "wolf",
];

/// Generate a random name using a specific RNG instance (for deterministic testing).
#[must_use]
pub fn generate_with_rng(rng: &mut fastrand::Rng) -> String {
    let adj = ADJECTIVES[rng.usize(..ADJECTIVES.len())];
    let modifier = MODIFIERS[rng.usize(..MODIFIERS.len())];
    let noun = NOUNS[rng.usize(..NOUNS.len())];
    format!("{adj}-{modifier}-{noun}")
}

/// Generate a random name in the `adjective-modifier-noun` pattern.
///
/// Example: `quiet-silver-fox`
#[must_use]
pub fn generate() -> String {
    generate_with_rng(&mut fastrand::Rng::new())
}

/// Generate a workspace name that doesn't collide with existing directories.
///
/// Returns the generated name, or an error after max retries.
#[must_use = "returns the generated name or an error"]
pub fn generate_unique_workspace_name(
    workspace_root: &std::path::Path,
    max_retries: usize,
) -> anyhow::Result<String> {
    let mut rng = fastrand::Rng::new();
    for _ in 0..max_retries {
        let name = generate_with_rng(&mut rng);
        // Generated names are always valid (lowercase ASCII + hyphens)
        if !workspace_root.join(&name).exists() {
            return Ok(name);
        }
    }
    anyhow::bail!(
        "Could not generate a unique workspace name after {max_retries} attempts. \
         This is extremely unlikely — please provide a name explicitly."
    )
}

/// Generate a unique branch name that doesn't collide with existing refs in any repo.
///
/// Returns the full branch name including prefix (e.g., `loom/quiet-silver-fox`).
#[must_use = "returns the generated branch name or an error"]
pub fn generate_unique_branch_name(
    branch_prefix: &str,
    repo_paths: &[std::path::PathBuf],
    max_retries: usize,
) -> anyhow::Result<String> {
    let mut rng = fastrand::Rng::new();
    for _ in 0..max_retries {
        let slug = generate_with_rng(&mut rng);
        let branch_name = format!("{branch_prefix}/{slug}");

        let collision = repo_paths.iter().any(|path| {
            let git = crate::git::GitRepo::new(path);
            // Treat errors as "no collision" — worst case the branch name already
            // exists, which worktree_add handles via BranchConflict fallback.
            git.ref_exists(&branch_name).unwrap_or(false)
        });

        if !collision {
            return Ok(branch_name);
        }
    }
    anyhow::bail!(
        "Could not generate a unique branch name after {max_retries} attempts. \
         This is extremely unlikely — please file a bug report."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest;
    use std::collections::HashSet;

    #[test]
    fn test_generate_deterministic() {
        let mut rng1 = fastrand::Rng::with_seed(42);
        let name1 = generate_with_rng(&mut rng1);
        let mut rng2 = fastrand::Rng::with_seed(42);
        let name2 = generate_with_rng(&mut rng2);
        assert_eq!(name1, name2, "Same seed should produce same name");
    }

    #[test]
    fn test_generate_format() {
        for _ in 0..100 {
            let name = generate();
            let parts: Vec<&str> = name.split('-').collect();
            assert_eq!(parts.len(), 3, "Name should have exactly 3 parts: {name}");
            assert!(
                ADJECTIVES.contains(&parts[0]),
                "Bad adjective: {}",
                parts[0]
            );
            assert!(MODIFIERS.contains(&parts[1]), "Bad modifier: {}", parts[1]);
            assert!(NOUNS.contains(&parts[2]), "Bad noun: {}", parts[2]);
            assert!(
                manifest::validate_name(&name).is_ok(),
                "Generated name fails validation: {name}"
            );
        }
    }

    #[test]
    fn test_no_duplicate_words_across_lists() {
        let all_adj: HashSet<_> = ADJECTIVES.iter().collect();
        let all_mod: HashSet<_> = MODIFIERS.iter().collect();
        let all_noun: HashSet<_> = NOUNS.iter().collect();
        assert!(
            all_adj.is_disjoint(&all_mod),
            "Overlap between ADJECTIVES and MODIFIERS"
        );
        assert!(
            all_adj.is_disjoint(&all_noun),
            "Overlap between ADJECTIVES and NOUNS"
        );
        assert!(
            all_mod.is_disjoint(&all_noun),
            "Overlap between MODIFIERS and NOUNS"
        );
    }

    #[test]
    fn test_all_words_are_valid_name_components() {
        for word in ADJECTIVES
            .iter()
            .chain(MODIFIERS.iter())
            .chain(NOUNS.iter())
        {
            assert!(
                word.chars().all(|c| c.is_ascii_lowercase()),
                "Non-lowercase word: {word}"
            );
            assert!(!word.is_empty(), "Empty word in list");
        }
    }

    #[test]
    fn test_word_list_sizes() {
        assert!(ADJECTIVES.len() >= 50);
        assert!(MODIFIERS.len() >= 25);
        assert!(NOUNS.len() >= 50);
    }

    #[test]
    fn test_unique_workspace_name_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let result = generate_unique_workspace_name(dir.path(), 10);
        assert!(result.is_ok());
        let name = result.unwrap();
        assert_eq!(name.split('-').count(), 3);
    }

    #[test]
    fn test_unique_workspace_name_avoids_existing() {
        let dir = tempfile::tempdir().unwrap();
        // Pre-create some directories
        for _ in 0..5 {
            let name = generate();
            std::fs::create_dir(dir.path().join(&name)).unwrap();
        }
        // Should still succeed (90K namespace, 10 retries)
        let result = generate_unique_workspace_name(dir.path(), 10);
        let name = result.unwrap();
        assert!(
            !dir.path().join(&name).exists(),
            "Returned name collides with existing dir: {name}"
        );
    }

    #[test]
    fn test_unique_branch_name_no_repos() {
        // With no repos, no collision check needed
        let result = generate_unique_branch_name("loom", &[], 10);
        assert!(result.is_ok());
        let name = result.unwrap();
        assert!(name.starts_with("loom/"));
    }

    #[test]
    fn test_manifest_branch_name_fallback() {
        let manifest = crate::manifest::WorkspaceManifest {
            name: "my-feature".to_string(),
            branch: None,
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![],
        };
        assert_eq!(manifest.branch_name("loom"), "loom/my-feature");
    }

    #[test]
    fn test_manifest_branch_name_stored() {
        let manifest = crate::manifest::WorkspaceManifest {
            name: "my-feature".to_string(),
            branch: Some("loom/bold-cedar-hawk".to_string()),
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![],
        };
        assert_eq!(manifest.branch_name("loom"), "loom/bold-cedar-hawk");
    }
}
