use std::path::Path;
use std::process::Command;

use super::error::GitError;

/// Output from a git command execution
#[derive(Debug)]
#[allow(dead_code)] // stderr used by callers inspecting error details
pub struct GitOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Builder for git commands with uniform `LC_ALL=C` and error handling.
///
/// Usage:
/// ```ignore
/// GitCommand::new(repo_path)
///     .args(&["worktree", "add", path, "-b", branch, base])
///     .run()?
/// ```
pub struct GitCommand<'a> {
    repo_path: &'a Path,
    args: Vec<String>,
}

impl<'a> GitCommand<'a> {
    pub fn new(repo_path: &'a Path) -> Self {
        Self {
            repo_path,
            args: Vec::new(),
        }
    }

    pub fn args(mut self, args: &[&str]) -> Self {
        self.args.extend(args.iter().map(|s| s.to_string()));
        self
    }

    pub fn arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }

    /// Execute the git command and return structured output.
    /// Fails with `GitError::CommandFailed` if exit code is non-zero.
    pub fn run(self) -> Result<GitOutput, GitError> {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.repo_path)
            .args(&self.args)
            .env("LC_ALL", "C")
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    GitError::NotInstalled
                } else {
                    GitError::Io(e)
                }
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        if exit_code != 0 {
            // Detect specific error patterns
            if stderr.contains("is already checked out at") {
                let branch = self
                    .args
                    .iter()
                    .skip_while(|a| *a != "-b")
                    .nth(1)
                    .cloned()
                    .unwrap_or_default();
                return Err(GitError::BranchConflict { branch });
            }

            return Err(GitError::CommandFailed {
                command: format!(
                    "git -C {} {}",
                    self.repo_path.display(),
                    self.args.join(" ")
                ),
                stderr,
            });
        }

        Ok(GitOutput {
            stdout,
            stderr,
            exit_code,
        })
    }

    /// Execute allowing non-zero exit codes (returns output regardless).
    /// Use for commands where non-zero is informational, not an error.
    pub fn run_unchecked(self) -> Result<GitOutput, GitError> {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.repo_path)
            .args(&self.args)
            .env("LC_ALL", "C")
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    GitError::NotInstalled
                } else {
                    GitError::Io(e)
                }
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        Ok(GitOutput {
            stdout,
            stderr,
            exit_code,
        })
    }
}

/// Run a git command without a repo context (e.g., `git --version`, `git clone`)
pub fn git_global(args: &[&str]) -> Result<GitOutput, GitError> {
    let output = Command::new("git")
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                GitError::NotInstalled
            } else {
                GitError::Io(e)
            }
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    if exit_code != 0 {
        return Err(GitError::CommandFailed {
            command: format!("git {}", args.join(" ")),
            stderr,
        });
    }

    Ok(GitOutput {
        stdout,
        stderr,
        exit_code,
    })
}
