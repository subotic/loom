use std::path::PathBuf;

/// Typed errors for git operations, enabling testable error matching.
/// Keep `anyhow` in the CLI layer; use `GitError` in loom-core for structured handling.
#[derive(thiserror::Error, Debug)]
pub enum GitError {
    #[error("git is not installed or not in PATH")]
    NotInstalled,

    #[error("git version {found} is below minimum required {required}")]
    VersionTooOld { found: String, required: String },

    #[error("not a git repository: {}", path.display())]
    NotARepo { path: PathBuf },

    #[error("branch '{branch}' is already checked out at another worktree")]
    BranchConflict { branch: String },

    #[error("repository has uncommitted changes")]
    DirtyWorktree,

    #[error("git command failed: {command}\n{stderr}")]
    CommandFailed { command: String, stderr: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
