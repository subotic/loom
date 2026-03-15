use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use self_update::cargo_crate_version;

const REPO_OWNER: &str = "subotic";
const REPO_NAME: &str = "loom";
const BIN_NAME: &str = "loom";

/// How often to check for updates (in seconds).
const CHECK_INTERVAL_SECS: u64 = 3600; // 1 hour

/// Check for updates and apply immediately if found.
///
/// - `force`: bypass the hourly rate limit
/// - `show_progress`: show download progress bar (true for `loom update`, false for auto-update)
///
/// Returns the new version string if updated, `None` if already up-to-date or throttled.
pub fn check_and_update(force: bool, show_progress: bool) -> Result<Option<String>> {
    if !force && !should_check()? {
        return Ok(None);
    }

    let status = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .show_download_progress(show_progress)
        .no_confirm(true)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;

    // Record timestamp after successful check (not before — a failed check
    // should not consume the hourly window).
    record_check()?;

    if status.updated() {
        Ok(Some(status.version().to_string()))
    } else {
        Ok(None)
    }
}

/// Fetch the latest release version from GitHub without downloading or applying.
///
/// Returns `(current_version, latest_version)`.
pub fn check_version() -> Result<(String, String)> {
    let current = cargo_crate_version!().to_string();

    let latest = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .current_version(cargo_crate_version!())
        .build()?
        .get_latest_release()?;

    Ok((current, latest.version))
}

/// Whether updates are disabled via `LOOM_DISABLE_UPDATE=1` env var.
///
/// Note: config-based opt-out (`update.enabled = false`) is checked separately
/// in `main.rs` after config is loaded.
pub fn is_disabled_by_env() -> bool {
    std::env::var("LOOM_DISABLE_UPDATE").is_ok_and(|v| v == "1")
}

// --- Rate limiting ---

fn timestamp_path() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    Ok(home.join(".config").join("loom").join("last_update_check"))
}

fn should_check() -> Result<bool> {
    let path = timestamp_path()?;
    if !path.exists() {
        return Ok(true);
    }

    let content = fs::read_to_string(&path)?;
    let last_check: u64 = content.trim().parse().unwrap_or(0);
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    Ok(now.saturating_sub(last_check) >= CHECK_INTERVAL_SECS)
}

fn record_check() -> Result<()> {
    let path = timestamp_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    fs::write(&path, now.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_disabled_by_env_default() {
        let _ = is_disabled_by_env();
    }

    #[test]
    fn test_timestamp_path() {
        let path = timestamp_path().unwrap();
        assert!(path.ends_with(".config/loom/last_update_check"));
    }
}
