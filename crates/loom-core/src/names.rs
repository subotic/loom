use crate::manifest;

/// Adjectives for random name generation (56 words).
/// All lowercase ASCII, no overlap with MODIFIERS or NOUNS.
pub const ADJECTIVES: &[&str] = &[
    "bold", "brave", "bright", "calm", "clear", "cool", "crisp", "dusk", "deep", "fair", "fast",
    "fine", "firm", "free", "glad", "gold", "good", "gray", "keen", "kind", "late", "lean",
    "live", "long", "lost", "loud", "mild", "neat", "next", "open", "pale", "past", "pure",
    "quick", "quiet", "rare", "real", "rich", "safe", "sane", "slim", "slow", "soft", "sole",
    "sure", "tall", "tame", "thin", "true", "vast", "warm", "wide", "wild", "wise", "young",
    "zero",
];

/// Color/material modifiers for random name generation (30 words).
/// All lowercase ASCII, no overlap with ADJECTIVES or NOUNS.
pub const MODIFIERS: &[&str] = &[
    "amber", "azure", "coral", "cream", "frost", "ivory", "jade", "lemon", "lilac", "maple",
    "misty", "ocean", "olive", "pearl", "plum", "raven", "royal", "ruby", "rusty", "sandy",
    "silk", "solar", "stone", "storm", "tidal", "timber", "velvet", "violet", "winter", "cedar",
];

/// Nature-themed nouns for random name generation (54 words).
/// All lowercase ASCII, no overlap with ADJECTIVES or MODIFIERS.
pub const NOUNS: &[&str] = &[
    "bear", "brook", "cliff", "crane", "creek", "dawn", "deer", "dove", "dune", "eagle", "elm",
    "fern", "finch", "flame", "fox", "grove", "hawk", "heron", "hill", "lake", "lark", "leaf",
    "lynx", "marsh", "mesa", "moth", "oak", "otter", "owl", "peak", "pine", "pond", "rain",
    "reef", "ridge", "river", "robin", "sage", "shore", "sky", "sparrow", "spruce", "star",
    "crest", "stream", "swift", "thrush", "tide", "trail", "vale", "vine", "wave", "wren", "wolf",
];

/// Generate a random name using a specific RNG instance (for deterministic testing).
pub fn generate_with_rng(rng: &mut fastrand::Rng) -> String {
    let adj = ADJECTIVES[rng.usize(..ADJECTIVES.len())];
    let modifier = MODIFIERS[rng.usize(..MODIFIERS.len())];
    let noun = NOUNS[rng.usize(..NOUNS.len())];
    format!("{adj}-{modifier}-{noun}")
}

/// Generate a random name in the `adjective-modifier-noun` pattern.
///
/// Example: `quiet-silver-fox`
pub fn generate() -> String {
    generate_with_rng(&mut fastrand::Rng::new())
}

/// Generate a workspace name that doesn't collide with existing directories.
///
/// Returns the generated name, or an error after max retries.
pub fn generate_unique_workspace_name(
    workspace_root: &std::path::Path,
    max_retries: usize,
) -> anyhow::Result<String> {
    for _ in 0..max_retries {
        let name = generate();
        if manifest::validate_name(&name).is_ok() && !workspace_root.join(&name).exists() {
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
pub fn generate_unique_branch_name(
    branch_prefix: &str,
    repo_paths: &[std::path::PathBuf],
    max_retries: usize,
) -> anyhow::Result<String> {
    for _ in 0..max_retries {
        let slug = generate();
        let branch_name = format!("{branch_prefix}/{slug}");

        let collision = repo_paths.iter().any(|path| {
            let git = crate::git::GitRepo::new(path);
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
