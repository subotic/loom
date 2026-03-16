use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use self_update::backends::github::UpdateBuilder;
use self_update::cargo_crate_version;

const REPO_OWNER: &str = "subotic";
const REPO_NAME: &str = "loom";
const BIN_NAME: &str = "loom";

/// How often to check for updates (in seconds).
const CHECK_INTERVAL_SECS: u64 = 3600; // 1 hour

/// Return a pre-configured update builder for GitHub Releases.
///
/// Callers can add options (e.g., `.show_download_progress()`) before `.build()`.
fn updater() -> UpdateBuilder {
    let mut builder = self_update::backends::github::Update::configure();
    builder
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .current_version(cargo_crate_version!());
    builder
}

/// Check for updates and apply immediately if found.
///
/// Used by `loom update` (explicit). Not used on startup — startup uses
/// `check_version_throttled()` which only checks without downloading.
///
/// - `show_progress`: show download progress bar
///
/// Returns the new version string if updated, `None` if already up-to-date.
pub fn check_and_update(show_progress: bool) -> Result<Option<String>> {
    let status = updater()
        .show_download_progress(show_progress)
        .no_confirm(true)
        .build()?
        .update()?;

    if status.updated() {
        Ok(Some(status.version().to_string()))
    } else {
        Ok(None)
    }
}

/// Lightweight version check with hourly rate limiting.
///
/// Used on startup to notify the user of available updates without
/// downloading or replacing the binary.
///
/// - `force`: bypass the hourly rate limit
///
/// Returns `Ok(Some(latest_version))` if a newer version is available,
/// `Ok(None)` if up-to-date or throttled.
///
/// Records the check timestamp on both success and failure to prevent
/// retry storms when the network is unreachable.
pub fn check_version_throttled(force: bool) -> Result<Option<String>> {
    if !force && !should_check()? {
        return Ok(None);
    }

    let result = check_version();

    // Record timestamp regardless of outcome — prevents retry storms
    // when the GitHub API is unreachable (e.g., corporate firewalls).
    let _ = record_check();

    let (current, latest) = result?;
    if latest != current {
        Ok(Some(latest))
    } else {
        Ok(None)
    }
}

/// Fetch the latest release version from GitHub without downloading or applying.
///
/// Returns `(current_version, latest_version)`.
pub fn check_version() -> Result<(String, String)> {
    let current = cargo_crate_version!().to_string();
    let latest = updater().build()?.get_latest_release()?;
    Ok((current, latest.version))
}

/// Whether updates are disabled via `LOOM_DISABLE_UPDATE` env var.
///
/// Accepts truthy values: `"1"`, `"true"`, `"yes"` (case-insensitive).
///
/// Note: config-based opt-out (`update.enabled = false`) is checked separately
/// in `main.rs` after config is loaded.
pub fn is_disabled_by_env() -> bool {
    std::env::var("LOOM_DISABLE_UPDATE")
        .is_ok_and(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"))
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
    fn test_is_disabled_by_env_truthy_values() {
        for (value, expected) in [
            ("1", true),
            ("true", true),
            ("TRUE", true),
            ("yes", true),
            ("Yes", true),
            ("0", false),
            ("false", false),
            ("no", false),
            ("", false),
        ] {
            // SAFETY: test runs single-threaded; no concurrent env var access.
            unsafe {
                std::env::set_var("LOOM_DISABLE_UPDATE", value);
            }
            assert_eq!(
                is_disabled_by_env(),
                expected,
                "LOOM_DISABLE_UPDATE={value:?} should be {expected}"
            );
        }

        // SAFETY: test runs single-threaded; no concurrent env var access.
        unsafe {
            std::env::remove_var("LOOM_DISABLE_UPDATE");
        }
        assert!(!is_disabled_by_env(), "unset should be false");
    }

    #[test]
    fn test_timestamp_path() {
        let path = timestamp_path().unwrap();
        assert!(path.ends_with(".config/loom/last_update_check"));
    }
}
