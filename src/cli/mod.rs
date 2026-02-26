use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "loom")]
#[command(about = "Linked Orchestration Of Multirepos — manage git worktrees across repositories")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// First-run setup — creates ~/.config/loom/config.toml
    Init,
    /// Create a new workspace with correlated worktrees
    New {
        /// Workspace name (lowercase alphanumeric + hyphens)
        name: String,
        /// Base branch for worktrees (default: repo default branch)
        #[arg(long)]
        base: Option<String>,
    },
    /// Add a repo to an existing workspace
    Add {
        /// Repo name to add
        repo: String,
    },
    /// Remove a repo from the current workspace
    Remove {
        /// Repo name to remove
        repo: String,
        /// Force removal even with uncommitted changes
        #[arg(long)]
        force: bool,
    },
    /// List workspaces and their repos
    #[command(alias = "ls")]
    List,
    /// Show status of all repos in a workspace
    Status,
    /// Save workspace state and push branches
    Save,
    /// Open the interactive TUI
    Tui,
    /// Tear down a workspace (remove worktrees)
    Down {
        /// Workspace name to tear down
        name: String,
        /// Force removal even with uncommitted changes
        #[arg(long)]
        force: bool,
    },
    /// Run a command across all repos in a workspace
    Exec {
        /// Command to run
        #[arg(trailing_var_arg = true)]
        cmd: Vec<String>,
    },
    /// Open a shell in the workspace directory
    Shell,
}

impl Cli {
    pub fn run(self) -> anyhow::Result<()> {
        match self.command {
            Command::Init => {
                println!("loom init — not yet implemented");
            }
            Command::New { name, base } => {
                println!("loom new {name} — not yet implemented");
                if let Some(b) = base {
                    println!("  --base {b}");
                }
            }
            Command::Add { repo } => {
                println!("loom add {repo} — not yet implemented");
            }
            Command::Remove { repo, force } => {
                println!("loom remove {repo} (force={force}) — not yet implemented");
            }
            Command::List => {
                println!("loom list — not yet implemented");
            }
            Command::Status => {
                println!("loom status — not yet implemented");
            }
            Command::Save => {
                println!("loom save — not yet implemented");
            }
            Command::Tui => {
                println!("loom tui — not yet implemented");
            }
            Command::Down { name, force } => {
                println!("loom down {name} (force={force}) — not yet implemented");
            }
            Command::Exec { cmd } => {
                println!("loom exec {} — not yet implemented", cmd.join(" "));
            }
            Command::Shell => {
                println!("loom shell — not yet implemented");
            }
        }
        Ok(())
    }
}
