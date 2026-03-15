use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use self_update::cargo_crate_version;

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
    record_check()?;

    let status = self_update::backends::github::Update::configure()
        .repo_owner("subotic")
        .repo_name("loom")
        .bin_name("loom")
        .show_download_progress(show_progress)
        .no_confirm(true)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;

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
        .repo_owner("subotic")
        .repo_name("loom")
        .bin_name("loom")
        .current_version(cargo_crate_version!())
        .build()?
        .get_latest_release()?;

    Ok((current, latest.version))
}

/// Whether updates are disabled via env var or config.
pub fn is_disabled() -> bool {
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
    fn test_is_disabled_default() {
        // When LOOM_DISABLE_UPDATE is not set, should not be disabled
        // (can't reliably test env var state in parallel tests,
        //  so just verify the function doesn't panic)
        let _ = is_disabled();
    }

    #[test]
    fn test_timestamp_path() {
        let path = timestamp_path().unwrap();
        assert!(path.ends_with(".config/loom/last_update_check"));
    }
}
