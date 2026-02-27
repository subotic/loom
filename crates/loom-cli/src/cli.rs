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

    fn run_list() -> anyhow::Result<()> {
        use loom_core::config::ensure_config_loaded;
        use loom_core::workspace::list::{WorkspaceHealth, list_workspaces};

        let config = ensure_config_loaded()?;
        let summaries = list_workspaces(&config)?;

        if summaries.is_empty() {
            println!("No workspaces. Run `loom new <name>` to create one.");
            return Ok(());
        }

        println!("{:<20} {:<6} {:<12} CREATED", "NAME", "REPOS", "STATUS");
        for ws in &summaries {
            let status_str = match &ws.status {
                WorkspaceHealth::Clean => "clean".to_string(),
                WorkspaceHealth::Dirty(n) => format!("{n} dirty"),
                WorkspaceHealth::Broken(msg) => format!("broken: {msg}"),
            };
            let date = ws.created.format("%Y-%m-%d");
            println!(
                "{:<20} {:<6} {:<12} {}",
                ws.name, ws.repo_count, status_str, date
            );
        }

        Ok(())
    }

    fn run_status(name: Option<String>, fetch: bool) -> anyhow::Result<()> {
        use loom_core::config::ensure_config_loaded;
        use loom_core::workspace;
        use loom_core::workspace::status::workspace_status;

        let config = ensure_config_loaded()?;
        let cwd = std::env::current_dir()?;

        // Resolve workspace: by name, by cwd, or fall back to list
        let (ws_path, manifest) = match workspace::resolve_workspace(name.as_deref(), &cwd, &config)
        {
            Ok(result) => result,
            Err(_) if name.is_none() => {
                // Outside workspace with no name → delegate to list
                return Self::run_list();
            }
            Err(e) => return Err(e),
        };

        let status = workspace_status(&manifest, &ws_path, fetch)?;

        println!("Workspace: {}", status.name);
        println!("Path: {}", status.path.display());
        if let Some(ref base) = status.base_branch {
            println!("Base branch: {base}");
        }
        println!();

        if status.repos.is_empty() {
            println!("  No repos in this workspace.");
            return Ok(());
        }

        println!(
            "  {:<20} {:<25} {:<10} AHEAD/BEHIND",
            "REPO", "BRANCH", "STATUS"
        );
        for repo in &status.repos {
            if !repo.exists {
                println!("  {:<20} {:<25} (missing)", repo.name, repo.branch);
                continue;
            }

            let status_str = if repo.is_dirty {
                format!("{} changed", repo.change_count)
            } else {
                "clean".to_string()
            };

            let ab_str = if repo.ahead > 0 || repo.behind > 0 {
                format!("+{} -{}", repo.ahead, repo.behind)
            } else {
                "-".to_string()
            };

            println!(
                "  {:<20} {:<25} {:<10} {}",
                repo.name, repo.branch, status_str, ab_str
            );
        }

        if !fetch {
            println!("\n  (ahead/behind based on last fetch — use --fetch for current data)");
        }

        Ok(())
    }

    fn run_add(repo_name: String, workspace_name: Option<String>) -> anyhow::Result<()> {
        use loom_core::config::ensure_config_loaded;
        use loom_core::registry;
        use loom_core::workspace;
        use loom_core::workspace::add::add_repo;

        let config = ensure_config_loaded()?;
        let cwd = std::env::current_dir()?;

        let (ws_path, mut manifest) =
            workspace::resolve_workspace(workspace_name.as_deref(), &cwd, &config)?;

        // Find the repo in registry
        let all_repos =
            registry::discover_repos(&config.registry.scan_roots, Some(&config.workspace.root));
        let repo = all_repos
            .iter()
            .find(|r| r.name == repo_name || format!("{}/{}", r.org, r.name) == repo_name)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Repository '{}' not found in registry. Available: {}",
                    repo_name,
                    all_repos
                        .iter()
                        .map(|r| format!("{}/{}", r.org, r.name))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;

        add_repo(&config, &ws_path, &mut manifest, repo)?;
        println!("Added '{}' to workspace '{}'.", repo_name, manifest.name);

        Ok(())
    }

    fn run_remove(repo_name: String, force: bool) -> anyhow::Result<()> {
        use loom_core::config::ensure_config_loaded;
        use loom_core::workspace;
        use loom_core::workspace::remove::remove_repo;

        let config = ensure_config_loaded()?;
        let cwd = std::env::current_dir()?;

        let (ws_path, mut manifest) = workspace::resolve_workspace(None, &cwd, &config)?;

        remove_repo(&config, &ws_path, &mut manifest, &repo_name, force)?;
        println!(
            "Removed '{}' from workspace '{}'.",
            repo_name, manifest.name
        );

        Ok(())
    }

    fn run_down(name: Option<String>, force: bool) -> anyhow::Result<()> {
        use dialoguer::Confirm;
        use loom_core::config::ensure_config_loaded;
        use loom_core::workspace;
        use loom_core::workspace::down::{check_workspace, teardown_workspace};

        let config = ensure_config_loaded()?;
        let cwd = std::env::current_dir()?;

        let (ws_path, mut manifest) = workspace::resolve_workspace(name.as_deref(), &cwd, &config)?;

        let check = check_workspace(&manifest);

        // Show summary
        if !check.clean_repos.is_empty() {
            println!(
                "Clean repos (will remove): {}",
                check.clean_repos.join(", ")
            );
        }
        if !check.dirty_repos.is_empty() {
            println!("Dirty repos:");
            for (name, count) in &check.dirty_repos {
                println!("  {} ({} changes)", name, count);
            }
        }
        if !check.missing_repos.is_empty() {
            println!(
                "Missing repos (already gone): {}",
                check.missing_repos.join(", ")
            );
        }

        // Determine repos to remove
        let mut repos_to_remove: Vec<String> = check.clean_repos.clone();
        repos_to_remove.extend(check.missing_repos.clone());

        if !check.dirty_repos.is_empty() {
            if force {
                repos_to_remove.extend(check.dirty_repos.iter().map(|(n, _)| n.clone()));
            } else {
                let confirm = Confirm::new()
                    .with_prompt("Remove dirty repos too? (uncommitted changes will be lost)")
                    .default(false)
                    .interact()?;
                if confirm {
                    repos_to_remove.extend(check.dirty_repos.iter().map(|(n, _)| n.clone()));
                }
            }
        }

        if repos_to_remove.is_empty() {
            println!("Nothing to remove.");
            return Ok(());
        }

        let result = teardown_workspace(&config, &ws_path, &mut manifest, &repos_to_remove, force)?;

        println!("Removed {} repo(s).", result.removed.len());
        if !result.failed.is_empty() {
            for (name, err) in &result.failed {
                eprintln!("  Failed to remove {}: {}", name, err);
            }
        }
        if result.remaining.is_empty() {
            println!("Workspace '{}' torn down.", manifest.name);
        } else {
            println!(
                "Partial teardown. Remaining: {}",
                result.remaining.join(", ")
            );
        }

        Ok(())
    }

    fn run_exec(cmd: Vec<String>) -> anyhow::Result<()> {
        use loom_core::config::ensure_config_loaded;
        use loom_core::workspace;
        use loom_core::workspace::exec::exec_in_workspace;

        if cmd.is_empty() {
            anyhow::bail!("No command provided. Usage: loom exec <command> [args...]");
        }

        let config = ensure_config_loaded()?;
        let cwd = std::env::current_dir()?;

        let (_ws_path, manifest) = workspace::resolve_workspace(None, &cwd, &config)?;

        let result = exec_in_workspace(&manifest, &cmd)?;

        // Summary
        let failed: Vec<_> = result.results.iter().filter(|r| !r.success).collect();

        if !failed.is_empty() {
            eprintln!("\n{} repo(s) failed:", failed.len());
            for r in &failed {
                eprintln!("  {} (exit code {})", r.repo_name, r.exit_code);
            }
            std::process::exit(1);
        }

        Ok(())
    }

    fn run_shell(name: Option<String>) -> anyhow::Result<()> {
        use loom_core::config::ensure_config_loaded;
        use loom_core::workspace;
        use loom_core::workspace::shell::open_terminal;

        let config = ensure_config_loaded()?;
        let cwd = std::env::current_dir()?;

        let (ws_path, _manifest) = workspace::resolve_workspace(name.as_deref(), &cwd, &config)?;

        let terminal = config
            .terminal
            .as_ref()
            .map(|t| t.command.as_str())
            .unwrap_or("ghostty");

        open_terminal(terminal, &ws_path)?;
        println!("Opened terminal at {}", ws_path.display());

        Ok(())
    }

    fn run_new(
        name: String,
        base: Option<String>,
        repos_filter: Option<Vec<String>>,
    ) -> anyhow::Result<()> {
        use dialoguer::MultiSelect;
        use loom_core::config::ensure_config_loaded;
        use loom_core::registry;
        use loom_core::workspace::new::{NewWorkspaceOpts, create_workspace};

        let config = ensure_config_loaded()?;

        // Discover all repos
        let all_repos =
            registry::discover_repos(&config.registry.scan_roots, Some(&config.workspace.root));
        if all_repos.is_empty() {
            anyhow::bail!(
                "No repositories found in scan roots. Check your config scan_roots paths."
            );
        }

        // Select repos: --repos flag (non-interactive) or dialoguer MultiSelect (interactive)
        let selected_repos = match repos_filter {
            Some(names) => {
                // Non-interactive: match by name
                let mut matched: Vec<loom_core::registry::RepoEntry> = Vec::new();
                for req_name in &names {
                    let found = all_repos.iter().find(|r| {
                        r.name == *req_name || format!("{}/{}", r.org, r.name) == *req_name
                    });
                    match found {
                        Some(r) => matched.push(r.clone()),
                        None => anyhow::bail!(
                            "Repository '{}' not found. Available: {}",
                            req_name,
                            all_repos
                                .iter()
                                .map(|r| format!("{}/{}", r.org, r.name))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    }
                }
                matched
            }
            None => {
                // Interactive: MultiSelect
                let labels: Vec<String> = all_repos
                    .iter()
                    .map(|r| format!("{}/{}", r.org, r.name))
                    .collect();
                let selections = MultiSelect::new()
                    .with_prompt("Select repositories for this workspace")
                    .items(&labels)
                    .interact()?;
                if selections.is_empty() {
                    anyhow::bail!("No repositories selected.");
                }
                selections
                    .into_iter()
                    .map(|i| all_repos[i].clone())
                    .collect()
            }
        };

        let result = create_workspace(
            &config,
            NewWorkspaceOpts {
                name,
                repos: selected_repos,
                base_branch: base,
            },
        )?;

        // Report results
        println!(
            "Workspace '{}' created at {}",
            result.name,
            result.path.display()
        );
        println!("  {} repo(s) added", result.repos_added);

        if !result.repos_failed.is_empty() {
            eprintln!("  {} repo(s) failed:", result.repos_failed.len());
            for (name, err) in &result.repos_failed {
                eprintln!("    {}: {}", name, err);
            }
        }

        Ok(())
    }

    pub fn run(self) -> anyhow::Result<()> {
        match self.command {
            Command::Init => {
                self.run_init()?;
            }
            Command::New { name, base, repos } => {
                Self::run_new(name, base, repos)?;
            }
            Command::Add { repo, workspace } => {
                Self::run_add(repo, workspace)?;
            }
            Command::Remove { repo, force } => {
                Self::run_remove(repo, force)?;
            }
            Command::List => {
                Self::run_list()?;
            }
            Command::Status { name, fetch } => {
                Self::run_status(name, fetch)?;
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
                Self::run_down(name, force)?;
            }
            Command::Exec { cmd } => {
                Self::run_exec(cmd)?;
            }
            Command::Shell { name } => {
                Self::run_shell(name)?;
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
