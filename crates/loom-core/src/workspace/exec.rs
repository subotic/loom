use std::process::Command;

use anyhow::Result;

use crate::manifest::WorkspaceManifest;

/// Result of running a command across repos.
#[derive(Debug)]
pub struct ExecResult {
    pub results: Vec<RepoExecResult>,
}

/// Result of running a command in a single repo.
#[derive(Debug)]
pub struct RepoExecResult {
    pub repo_name: String,
    pub exit_code: i32,
    pub success: bool,
}

impl ExecResult {
    /// Whether all repos succeeded.
    pub fn all_success(&self) -> bool {
        self.results.iter().all(|r| r.success)
    }
}

/// Run a command in each repo's worktree sequentially.
///
/// Stdout/stderr are inherited (streamed to the terminal).
pub fn exec_in_workspace(manifest: &WorkspaceManifest, cmd: &[String]) -> Result<ExecResult> {
    if cmd.is_empty() {
        anyhow::bail!("No command provided.");
    }

    let mut results = Vec::new();

    for repo in &manifest.repos {
        if !repo.worktree_path.exists() {
            eprintln!("=== {} === (missing, skipped)", repo.name);
            results.push(RepoExecResult {
                repo_name: repo.name.clone(),
                exit_code: -1,
                success: false,
            });
            continue;
        }

        eprintln!("=== {} ===", repo.name);

        let status = Command::new(&cmd[0])
            .args(&cmd[1..])
            .current_dir(&repo.worktree_path)
            .env("LC_ALL", "C")
            .status();

        match status {
            Ok(s) => {
                let code = s.code().unwrap_or(-1);
                results.push(RepoExecResult {
                    repo_name: repo.name.clone(),
                    exit_code: code,
                    success: s.success(),
                });
            }
            Err(e) => {
                eprintln!("  Error: {}", e);
                results.push(RepoExecResult {
                    repo_name: repo.name.clone(),
                    exit_code: -1,
                    success: false,
                });
            }
        }
    }

    Ok(ExecResult { results })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RepoManifestEntry;

    #[test]
    fn test_exec_empty_command() {
        let manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![],
        };

        let result = exec_in_workspace(&manifest, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_exec_in_real_repo() {
        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path().join("my-repo");
        std::fs::create_dir_all(&repo_path).unwrap();
        std::process::Command::new("git")
            .args(["init", "-b", "main", &repo_path.to_string_lossy()])
            .env("LC_ALL", "C")
            .output()
            .unwrap();

        let manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![RepoManifestEntry {
                name: "my-repo".to_string(),
                original_path: repo_path.clone(),
                worktree_path: repo_path,
                branch: "main".to_string(),
                remote_url: String::new(),
            }],
        };

        let result =
            exec_in_workspace(&manifest, &["echo".to_string(), "hello".to_string()]).unwrap();
        assert!(result.all_success());
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].exit_code, 0);
    }

    #[test]
    fn test_exec_missing_repo() {
        let manifest = WorkspaceManifest {
            name: "test-ws".to_string(),
            created: chrono::Utc::now(),
            base_branch: None,
            preset: None,
            repos: vec![RepoManifestEntry {
                name: "missing-repo".to_string(),
                original_path: std::path::PathBuf::from("/nonexistent"),
                worktree_path: std::path::PathBuf::from("/nonexistent"),
                branch: "main".to_string(),
                remote_url: String::new(),
            }],
        };

        let result =
            exec_in_workspace(&manifest, &["echo".to_string(), "hello".to_string()]).unwrap();
        assert!(!result.all_success());
        assert_eq!(result.results[0].exit_code, -1);
    }
}
