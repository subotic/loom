mod command;
pub mod error;
pub mod repo;

pub use error::GitError;
pub use repo::{GitRepo, WorktreeEntry, check_git_version, clone_repo};
