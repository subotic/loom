use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

/// Open a terminal window at the given workspace path.
///
/// The terminal command is read from config (e.g., "ghostty", "wezterm").
/// The process is spawned and detached (returns immediately).
pub fn open_terminal(terminal_command: &str, ws_path: &Path) -> Result<()> {
    let ws_str = ws_path.to_string_lossy();
    tracing::debug!(terminal = %terminal_command, path = %ws_str, "launching terminal");

    let result = match terminal_command {
        "ghostty" => {
            if cfg!(target_os = "macos")
                && std::path::Path::new("/Applications/Ghostty.app").exists()
            {
                Command::new("open")
                    .args(["-a", "Ghostty"])
                    .arg(ws_path)
                    .spawn()
            } else {
                Command::new("ghostty")
                    .arg(format!("--working-directory={ws_str}"))
                    .spawn()
            }
        }
        "wezterm" => Command::new("wezterm")
            .args(["start", "--cwd"])
            .arg(ws_path)
            .spawn(),
        cmd if cmd.starts_with("open -a") => {
            // e.g. "open -a iTerm" or "open -a Terminal"
            let app = cmd.strip_prefix("open -a ").unwrap_or("Terminal");
            Command::new("open").args(["-a", app]).arg(ws_path).spawn()
        }
        "code" => Command::new("code").arg(ws_path).spawn(),
        other => {
            // Generic fallback: try running it directly with the path
            Command::new(other).arg(ws_path).spawn()
        }
    };

    match result {
        Ok(_child) => Ok(()),
        Err(e) => Err(e).with_context(|| {
            format!(
                "Failed to launch terminal '{}'. Check your config terminal.command.",
                terminal_command
            )
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_terminal_nonexistent_command() {
        let dir = tempfile::TempDir::new_in(std::env::current_dir().unwrap()).unwrap();
        let result = open_terminal("nonexistent-terminal-xyz-12345", dir.path());
        assert!(result.is_err());
    }

    /// On macOS with Ghostty.app installed, `open_terminal("ghostty", ...)` should
    /// not fail with "No such file or directory" — it should use `open -a Ghostty`.
    #[test]
    #[cfg(target_os = "macos")]
    fn test_ghostty_macos_uses_open_when_app_exists() {
        let dir = tempfile::TempDir::new_in(std::env::current_dir().unwrap()).unwrap();
        let has_app = std::path::Path::new("/Applications/Ghostty.app").exists();
        let result = open_terminal("ghostty", dir.path());
        if has_app {
            // Should succeed (spawns `open -a Ghostty` which returns immediately)
            assert!(
                result.is_ok(),
                "ghostty should use 'open -a Ghostty' on macOS"
            );
        } else {
            // No Ghostty.app → falls back to bare `ghostty` binary → likely fails
            assert!(result.is_err());
        }
    }
}
