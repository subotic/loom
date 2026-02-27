use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

/// Open a terminal window at the given workspace path.
///
/// The terminal command is read from config (e.g., "ghostty", "wezterm").
/// The process is spawned and detached (returns immediately).
pub fn open_terminal(terminal_command: &str, ws_path: &Path) -> Result<()> {
    let ws_str = ws_path.to_string_lossy();

    let result = match terminal_command {
        "ghostty" => Command::new("ghostty")
            .arg(format!("--working-directory={ws_str}"))
            .spawn(),
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
        let dir = tempfile::tempdir().unwrap();
        let result = open_terminal("nonexistent-terminal-xyz-12345", dir.path());
        assert!(result.is_err());
    }
}
