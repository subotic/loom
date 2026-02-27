use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "loom")]
#[command(about = "Linked Orchestration Of Multirepos — manage git worktrees across repositories")]
#[command(version)]
#[command(propagate_version = true)]
#[command(subcommand_required = true)]
pub struct Cli {
    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Output as JSON (machine-readable)
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// First-run setup — creates ~/.config/loom/config.toml
    Init,

    /// Create a new workspace with correlated worktrees
    New {
        /// Workspace name (lowercase alphanumeric + hyphens)
        name: String,
        /// Base branch for worktrees (default: repo default branch)
        #[arg(long)]
        base: Option<String>,
        /// Repos to include (comma-separated, non-interactive mode)
        #[arg(long, value_delimiter = ',')]
        repos: Option<Vec<String>>,
    },

    /// Add a repo to an existing workspace
    Add {
        /// Repo name to add
        repo: String,
        /// Workspace name (if not inside a workspace directory)
        #[arg(long)]
        workspace: Option<String>,
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
    Status {
        /// Workspace name (optional — detects from cwd if inside a workspace)
        name: Option<String>,
        /// Fetch from remotes before showing status
        #[arg(long)]
        fetch: bool,
    },

    /// Save workspace state and push branches
    Save {
        /// Push committed work even for repos with uncommitted changes
        #[arg(long)]
        force: bool,
    },

    /// Restore a workspace from sync manifest
    Open {
        /// Workspace name to open
        name: String,
    },

    /// Open the interactive TUI
    Tui,

    /// Tear down a workspace (remove worktrees)
    Down {
        /// Workspace name (optional — detects from cwd if inside a workspace)
        name: Option<String>,
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
    Shell {
        /// Workspace name (optional — detects from cwd if inside a workspace)
        name: Option<String>,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

impl Cli {
    /// Resolve effective verbosity: --quiet wins over --verbose
    #[allow(dead_code)] // Used in Phase 2 when commands check verbosity
    pub fn verbosity(&self) -> Verbosity {
        if self.quiet {
            Verbosity::Quiet
        } else {
            match self.verbose {
                0 => Verbosity::Normal,
                1 => Verbosity::Verbose,
                _ => Verbosity::Trace,
            }
        }
    }

    /// Whether colored output should be used.
    /// Disabled by --no-color flag, NO_COLOR env var, --json flag, or non-TTY stdout.
    #[allow(dead_code)] // Used in Phase 2 when commands emit colored output
    pub fn use_color(&self) -> bool {
        !self.no_color && std::env::var_os("NO_COLOR").is_none() && !self.json
    }

    fn run_init(&self) -> anyhow::Result<()> {
        use dialoguer::{Confirm, Input, MultiSelect};
        use loom_core::config::init;

        // Check if config already exists
        let config_path = loom_core::config::Config::path()?;
        if config_path.exists() {
            let update = Confirm::new()
                .with_prompt(format!(
                    "Config already exists at {}. Update it?",
                    config_path.display()
                ))
                .default(false)
                .interact()?;
            if !update {
                println!("Keeping existing config.");
                return Ok(());
            }
        }

        println!("Setting up loom...\n");

        // Auto-detect scan roots
        let detected = init::detect_scan_roots();
        let scan_roots: Vec<std::path::PathBuf> = if detected.is_empty() {
            let input: String = Input::new()
                .with_prompt(
                    "No standard code directories found. Enter scan root paths (comma-separated)",
                )
                .interact_text()?;
            input
                .split(',')
                .map(|s| std::path::PathBuf::from(s.trim()))
                .collect()
        } else {
            let labels: Vec<String> = detected.iter().map(|p| p.display().to_string()).collect();
            let defaults: Vec<bool> = vec![true; labels.len()];
            let selections = MultiSelect::new()
                .with_prompt("Select scan roots (directories containing your git repos)")
                .items(&labels)
                .defaults(&defaults)
                .interact()?;
            selections
                .into_iter()
                .map(|i| detected[i].clone())
                .collect()
        };

        // Workspace root
        let default_ws = shellexpand::tilde("~/loom").to_string();
        let ws_input: String = Input::new()
            .with_prompt("Workspace root directory")
            .default(default_ws)
            .interact_text()?;
        let workspace_root = std::path::PathBuf::from(shellexpand::tilde(&ws_input).as_ref());

        // Terminal detection
        let detected_terminal = init::detect_terminal();
        let terminal_default = detected_terminal.unwrap_or_else(|| "ghostty".to_string());
        let terminal: String = Input::new()
            .with_prompt("Terminal command")
            .default(terminal_default)
            .interact_text()?;

        // Branch prefix
        let branch_prefix: String = Input::new()
            .with_prompt("Branch prefix for worktrees")
            .default("loom".to_string())
            .interact_text()?;

        // Create config
        let config = init::create_config(
            scan_roots,
            workspace_root,
            Some(terminal),
            branch_prefix,
            vec!["claude-code".to_string()],
        )?;

        // Save and create directories
        init::finalize_init(&config)?;

        println!("\nloom initialized successfully!");

        Ok(())
    }

    pub fn run(self) -> anyhow::Result<()> {
        match self.command {
            Command::Init => {
                self.run_init()?;
            }
            Command::New { name, base, repos } => {
                println!("loom new {name} — not yet implemented");
                if let Some(b) = base {
                    println!("  --base {b}");
                }
                if let Some(r) = repos {
                    println!("  --repos {}", r.join(","));
                }
            }
            Command::Add { repo, workspace } => {
                println!("loom add {repo} — not yet implemented");
                if let Some(ws) = workspace {
                    println!("  --workspace {ws}");
                }
            }
            Command::Remove { repo, force } => {
                println!("loom remove {repo} (force={force}) — not yet implemented");
            }
            Command::List => {
                println!("loom list — not yet implemented");
            }
            Command::Status { name, fetch } => {
                let target = name.as_deref().unwrap_or("(detect from cwd)");
                println!("loom status {target} (fetch={fetch}) — not yet implemented");
            }
            Command::Save { force } => {
                println!("loom save (force={force}) — not yet implemented");
            }
            Command::Open { name } => {
                println!("loom open {name} — not yet implemented");
            }
            Command::Tui => {
                println!("loom tui — not yet implemented");
            }
            Command::Down { name, force } => {
                let target = name.as_deref().unwrap_or("(detect from cwd)");
                println!("loom down {target} (force={force}) — not yet implemented");
            }
            Command::Exec { cmd } => {
                if cmd.is_empty() {
                    anyhow::bail!("No command provided. Usage: loom exec <command> [args...]");
                }
                println!("loom exec {} — not yet implemented", cmd.join(" "));
            }
            Command::Shell { name } => {
                let target = name.as_deref().unwrap_or("(detect from cwd)");
                println!("loom shell {target} — not yet implemented");
            }
            Command::Completions { shell } => {
                use clap::CommandFactory;
                let mut cmd = Cli::command();
                clap_complete::generate(shell, &mut cmd, "loom", &mut std::io::stdout());
            }
        }
        Ok(())
    }
}

/// Verbosity level resolved from --quiet and --verbose flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Used in Phase 2 when commands check verbosity
pub enum Verbosity {
    /// --quiet: errors only
    Quiet,
    /// Default: info level
    Normal,
    /// -v: debug level
    Verbose,
    /// -vv+: trace level
    Trace,
}
