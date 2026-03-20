use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

/// Open the workspace in the configured editor.
///
/// Handles the `open -a` macOS pattern (same as `shell.rs`).
/// All other commands are treated as single-binary invocations with the
/// workspace path as the only argument.
pub fn open_editor(editor_command: &str, ws_path: &Path) -> Result<()> {
    tracing::debug!(editor = %editor_command, path = %ws_path.display(), "launching editor");

    let result = if let Some(app) = editor_command.strip_prefix("open -a ") {
        // macOS "open -a AppName" pattern (e.g., "open -a Cursor")
        Command::new("open").args(["-a", app]).arg(ws_path).spawn()
    } else {
        Command::new(editor_command).arg(ws_path).spawn()
    };

    result.with_context(|| {
        format!(
            "Failed to launch editor '{}'. Check your config editor.command.",
            editor_command
        )
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_editor_nonexistent_command() {
        let dir = tempfile::tempdir().unwrap();
        let result = open_editor("nonexistent-editor-xyz-12345", dir.path());
        assert!(result.is_err());
    }
}
