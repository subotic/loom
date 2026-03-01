use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use crate::config::Config;
use crate::registry::RepoEntry;
use crate::workspace::list::{WorkspaceHealth, WorkspaceSummary};

/// Which screen the TUI is showing.
#[derive(Debug, Clone)]
pub enum Screen {
    WorkspaceList,
    WorkspaceDetail {
        name: String,
        path: PathBuf,
    },
    NewWizard {
        step: WizardStep,
        name: String,
        available_repos: Vec<RepoEntry>,
        /// Unique org names extracted from available_repos (sorted).
        groups: Vec<String>,
        /// Index into `groups` for the selected org.
        selected_group: usize,
        /// Indices into `available_repos` that are selected.
        selected: HashSet<usize>,
        focused: usize,
    },
    ConfirmDialog {
        message: String,
        action: PendingAction,
    },
}

/// Steps in the new-workspace wizard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WizardStep {
    EnterName,
    SelectGroups,
    SelectRepos,
    Confirm,
}

/// Action pending confirmation.
#[derive(Debug, Clone)]
pub enum PendingAction {
    TeardownWorkspace { name: String },
}

/// Messages that drive state transitions (TEA pattern).
#[derive(Debug)]
pub enum Message {
    // Navigation
    SelectNext,
    SelectPrev,
    Confirm,
    Cancel,
    Quit,

    // Workspace list
    OpenDetail,
    StartNewWizard,
    RefreshList,

    // Workspace detail
    TeardownWorkspace,

    // Wizard
    WizardCharInput(char),
    WizardBackspace,
    WizardNextStep,
    ToggleRepo(usize),

    // Confirm dialog
    ConfirmYes,
    ConfirmNo,

    // Status
    DismissStatus,
}

/// Severity of status bar messages.
#[derive(Debug, Clone)]
pub enum StatusLevel {
    Info,
    Error,
}

/// A status bar message with auto-dismiss.
#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub level: StatusLevel,
    pub created: Instant,
}

/// Top-level application state.
pub struct App {
    pub screen: Screen,
    pub workspaces: Vec<WorkspaceSummary>,
    pub selected: usize,
    pub status: Option<StatusMessage>,
    pub should_quit: bool,
    pub config: Config,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            screen: Screen::WorkspaceList,
            workspaces: Vec::new(),
            selected: 0,
            status: None,
            should_quit: false,
            config,
        }
    }

    /// Load workspace list from disk.
    pub fn refresh_workspaces(&mut self) {
        match crate::workspace::list::list_workspaces(&self.config) {
            Ok(ws) => self.workspaces = ws,
            Err(e) => self.set_status(
                format!("Failed to load workspaces: {e}"),
                StatusLevel::Error,
            ),
        }
    }

    fn set_status(&mut self, text: String, level: StatusLevel) {
        self.status = Some(StatusMessage {
            text,
            level,
            created: Instant::now(),
        });
    }

    /// Auto-dismiss status messages after 5 seconds.
    pub fn tick(&mut self) {
        if let Some(ref status) = self.status
            && status.created.elapsed().as_secs() >= 5
        {
            self.status = None;
        }
    }

    /// Process a message and update state (TEA update function).
    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::Quit => {
                self.should_quit = true;
            }
            Message::DismissStatus => {
                self.status = None;
            }
            Message::RefreshList => {
                self.refresh_workspaces();
            }

            // --- Workspace List ---
            Message::SelectNext => match &mut self.screen {
                Screen::WorkspaceList => {
                    if !self.workspaces.is_empty() {
                        self.selected = (self.selected + 1) % self.workspaces.len();
                    }
                }
                Screen::WorkspaceDetail { .. } => {
                    // Could scroll repo list in future
                }
                Screen::NewWizard {
                    step: WizardStep::SelectGroups,
                    groups,
                    focused,
                    ..
                } => {
                    if !groups.is_empty() {
                        *focused = (*focused + 1) % groups.len();
                    }
                }
                Screen::NewWizard {
                    step: WizardStep::SelectRepos,
                    available_repos,
                    selected_group,
                    groups,
                    focused,
                    ..
                } => {
                    let visible_count =
                        Self::filtered_repo_count(available_repos, &groups[*selected_group]);
                    if visible_count > 0 {
                        *focused = (*focused + 1) % visible_count;
                    }
                }
                _ => {}
            },
            Message::SelectPrev => match &mut self.screen {
                Screen::WorkspaceList => {
                    if !self.workspaces.is_empty() {
                        self.selected = self
                            .selected
                            .checked_sub(1)
                            .unwrap_or(self.workspaces.len() - 1);
                    }
                }
                Screen::NewWizard {
                    step: WizardStep::SelectGroups,
                    groups,
                    focused,
                    ..
                } => {
                    if !groups.is_empty() {
                        *focused = focused.checked_sub(1).unwrap_or(groups.len() - 1);
                    }
                }
                Screen::NewWizard {
                    step: WizardStep::SelectRepos,
                    available_repos,
                    selected_group,
                    groups,
                    focused,
                    ..
                } => {
                    let visible_count =
                        Self::filtered_repo_count(available_repos, &groups[*selected_group]);
                    if visible_count > 0 {
                        *focused = focused.checked_sub(1).unwrap_or(visible_count - 1);
                    }
                }
                _ => {}
            },
            Message::OpenDetail | Message::Confirm => match &self.screen {
                Screen::WorkspaceList => {
                    if let Some(ws) = self.workspaces.get(self.selected) {
                        self.screen = Screen::WorkspaceDetail {
                            name: ws.name.clone(),
                            path: ws.path.clone(),
                        };
                    }
                }
                Screen::ConfirmDialog { .. } => {
                    self.update(Message::ConfirmYes);
                }
                _ => {}
            },
            Message::Cancel => {
                let screen = std::mem::replace(&mut self.screen, Screen::WorkspaceList);
                match screen {
                    Screen::WorkspaceDetail { .. } => {
                        self.refresh_workspaces();
                    }
                    Screen::NewWizard {
                        step: WizardStep::EnterName,
                        ..
                    } => {
                        // Already replaced with WorkspaceList
                    }
                    Screen::NewWizard {
                        step: WizardStep::SelectGroups,
                        name,
                        available_repos,
                        groups,
                        ..
                    } => {
                        self.screen = Screen::NewWizard {
                            step: WizardStep::EnterName,
                            name,
                            available_repos,
                            groups,
                            selected_group: 0,
                            selected: HashSet::new(),
                            focused: 0,
                        };
                    }
                    Screen::NewWizard {
                        step: WizardStep::SelectRepos,
                        name,
                        available_repos,
                        groups,
                        selected_group,
                        ..
                    } => {
                        self.screen = Screen::NewWizard {
                            step: WizardStep::SelectGroups,
                            name,
                            available_repos,
                            groups,
                            selected_group: 0,
                            selected: HashSet::new(),
                            focused: selected_group, // pre-focus the previously selected org
                        };
                    }
                    Screen::NewWizard {
                        step: WizardStep::Confirm,
                        name,
                        available_repos,
                        groups,
                        selected_group,
                        selected,
                        focused,
                    } => {
                        self.screen = Screen::NewWizard {
                            step: WizardStep::SelectRepos,
                            name,
                            available_repos,
                            groups,
                            selected_group,
                            selected,
                            focused,
                        };
                    }
                    Screen::ConfirmDialog { .. } => {
                        self.refresh_workspaces();
                    }
                    _ => {}
                }
            }

            // --- Start new workspace wizard ---
            Message::StartNewWizard => {
                let repos = crate::registry::discover_repos(
                    &self.config.registry.scan_roots,
                    Some(&self.config.workspace.root),
                );
                // Extract unique org names (repos already sorted by org, name)
                let mut groups: Vec<String> = repos.iter().map(|r| r.org.clone()).collect();
                groups.dedup();
                self.screen = Screen::NewWizard {
                    step: WizardStep::EnterName,
                    name: String::new(),
                    available_repos: repos,
                    groups,
                    selected_group: 0,
                    selected: HashSet::new(),
                    focused: 0,
                };
            }

            // --- Wizard input ---
            Message::WizardCharInput(ch) => {
                if let Screen::NewWizard {
                    step: WizardStep::EnterName,
                    name,
                    ..
                } = &mut self.screen
                {
                    // Only allow valid workspace name chars
                    if ch.is_ascii_alphanumeric() || ch == '-' {
                        name.push(ch);
                    }
                }
            }
            Message::WizardBackspace => {
                if let Screen::NewWizard {
                    step: WizardStep::EnterName,
                    name,
                    ..
                } = &mut self.screen
                {
                    name.pop();
                }
            }
            Message::WizardNextStep => {
                let screen = std::mem::replace(&mut self.screen, Screen::WorkspaceList);
                match screen {
                    Screen::NewWizard {
                        step: WizardStep::EnterName,
                        name,
                        available_repos,
                        groups,
                        selected_group,
                        selected,
                        focused,
                    } => {
                        // Generate random name if empty
                        let name = if name.is_empty() {
                            match crate::names::generate_unique_workspace_name(
                                &self.config.workspace.root,
                                crate::names::MAX_NAME_RETRIES,
                            ) {
                                Ok(generated) => generated,
                                Err(e) => {
                                    self.set_status(
                                        format!("Failed to generate name: {e}"),
                                        StatusLevel::Error,
                                    );
                                    // Restore wizard with the original (empty) name so the user can retry
                                    self.screen = Screen::NewWizard {
                                        step: WizardStep::EnterName,
                                        name,
                                        available_repos,
                                        groups,
                                        selected_group,
                                        selected,
                                        focused,
                                    };
                                    return;
                                }
                            }
                        } else {
                            name
                        };

                        if groups.is_empty() {
                            // No orgs discovered — cannot proceed
                            self.set_status(
                                "No repositories found. Check scan_roots in config.".to_string(),
                                StatusLevel::Error,
                            );
                            self.screen = Screen::NewWizard {
                                step: WizardStep::EnterName,
                                name,
                                available_repos,
                                groups,
                                selected_group: 0,
                                selected,
                                focused: 0,
                            };
                        } else if groups.len() == 1 {
                            // Only one org — skip group selection, auto-select it
                            self.screen = Screen::NewWizard {
                                step: WizardStep::SelectRepos,
                                name,
                                available_repos,
                                groups,
                                selected_group: 0,
                                selected,
                                focused: 0,
                            };
                        } else {
                            self.screen = Screen::NewWizard {
                                step: WizardStep::SelectGroups,
                                name,
                                available_repos,
                                groups,
                                selected_group,
                                selected,
                                focused: 0,
                            };
                        }
                    }
                    Screen::NewWizard {
                        step: WizardStep::SelectGroups,
                        name,
                        available_repos,
                        groups,
                        focused,
                        ..
                    } => {
                        self.screen = Screen::NewWizard {
                            step: WizardStep::SelectRepos,
                            name,
                            available_repos,
                            groups,
                            selected_group: focused,
                            selected: HashSet::new(),
                            focused: 0,
                        };
                    }
                    Screen::NewWizard {
                        step: WizardStep::SelectRepos,
                        name,
                        available_repos,
                        groups,
                        selected_group,
                        selected,
                        focused,
                    } => {
                        if selected.is_empty() {
                            self.set_status(
                                "Select at least one repo".to_string(),
                                StatusLevel::Error,
                            );
                            self.screen = Screen::NewWizard {
                                step: WizardStep::SelectRepos,
                                name,
                                available_repos,
                                groups,
                                selected_group,
                                selected,
                                focused,
                            };
                        } else {
                            self.screen = Screen::NewWizard {
                                step: WizardStep::Confirm,
                                name,
                                available_repos,
                                groups,
                                selected_group,
                                selected,
                                focused,
                            };
                        }
                    }
                    Screen::NewWizard {
                        step: WizardStep::Confirm,
                        name,
                        available_repos,
                        selected,
                        ..
                    } => {
                        // Execute workspace creation
                        let repos: Vec<RepoEntry> = selected
                            .iter()
                            .filter_map(|&i| available_repos.get(i).cloned())
                            .collect();
                        match crate::workspace::new::create_workspace(
                            &self.config,
                            crate::workspace::new::NewWorkspaceOpts {
                                name: name.clone(),
                                repos,
                                base_branch: None,
                                preset: None,
                            },
                        ) {
                            Ok(result) => {
                                self.set_status(
                                    format!(
                                        "Created '{}' with {} repo(s)",
                                        result.name, result.repos_added
                                    ),
                                    StatusLevel::Info,
                                );
                                self.screen = Screen::WorkspaceList;
                                self.refresh_workspaces();
                            }
                            Err(e) => {
                                self.set_status(
                                    format!("Failed to create workspace: {e}"),
                                    StatusLevel::Error,
                                );
                                self.screen = Screen::WorkspaceList;
                            }
                        }
                    }
                    other => {
                        self.screen = other;
                    }
                }
            }
            Message::ToggleRepo(idx) => {
                // Only applies to SelectRepos — SelectGroups uses Enter-to-select
                if let Screen::NewWizard {
                    step: WizardStep::SelectRepos,
                    available_repos,
                    groups,
                    selected_group,
                    selected,
                    ..
                } = &mut self.screen
                {
                    let visible =
                        Self::filtered_repo_indices(available_repos, &groups[*selected_group]);
                    if let Some(&repo_idx) = visible.get(idx) {
                        if selected.contains(&repo_idx) {
                            selected.remove(&repo_idx);
                        } else {
                            selected.insert(repo_idx);
                        }
                    }
                }
            }

            // --- Workspace detail actions ---
            Message::TeardownWorkspace => {
                if let Screen::WorkspaceDetail { name, .. } = &self.screen {
                    let ws_name = name.clone();
                    self.screen = Screen::ConfirmDialog {
                        message: format!(
                            "Tear down workspace '{ws_name}'? This removes all worktrees."
                        ),
                        action: PendingAction::TeardownWorkspace { name: ws_name },
                    };
                }
            }

            // --- Confirm dialog ---
            Message::ConfirmYes => {
                let screen = std::mem::replace(&mut self.screen, Screen::WorkspaceList);
                if let Screen::ConfirmDialog { action, .. } = screen {
                    match action {
                        PendingAction::TeardownWorkspace { name } => {
                            self.execute_teardown(&name);
                        }
                    }
                }
                self.refresh_workspaces();
            }
            Message::ConfirmNo => {
                // Go back to where we came from
                self.screen = Screen::WorkspaceList;
                self.refresh_workspaces();
            }
        }
    }

    /// Count repos visible after org filtering.
    fn filtered_repo_count(repos: &[RepoEntry], org: &str) -> usize {
        repos.iter().filter(|r| r.org == org).count()
    }

    /// Get the original indices of repos that belong to the selected org.
    pub fn filtered_repo_indices(repos: &[RepoEntry], org: &str) -> Vec<usize> {
        repos
            .iter()
            .enumerate()
            .filter(|(_, r)| r.org == org)
            .map(|(i, _)| i)
            .collect()
    }

    fn execute_teardown(&mut self, name: &str) {
        let cwd = match std::env::current_dir() {
            Ok(c) => c,
            Err(e) => {
                self.set_status(format!("Cannot get cwd: {e}"), StatusLevel::Error);
                return;
            }
        };

        let (ws_path, mut manifest) =
            match crate::workspace::resolve_workspace(Some(name), &cwd, &self.config) {
                Ok(r) => r,
                Err(e) => {
                    self.set_status(format!("Cannot find workspace: {e}"), StatusLevel::Error);
                    return;
                }
            };

        let all_repos: Vec<String> = manifest.repos.iter().map(|r| r.name.clone()).collect();
        match crate::workspace::down::teardown_workspace(
            &self.config,
            &ws_path,
            &mut manifest,
            &all_repos,
            false,
        ) {
            Ok(result) => {
                let msg = format!(
                    "Torn down '{}': {} removed, {} failed",
                    name,
                    result.removed.len(),
                    result.failed.len()
                );
                self.set_status(msg, StatusLevel::Info);
            }
            Err(e) => {
                self.set_status(format!("Teardown failed: {e}"), StatusLevel::Error);
            }
        }
    }

    /// Get detail info for the currently viewed workspace.
    pub fn workspace_detail_status(&self) -> Option<crate::workspace::status::WorkspaceStatus> {
        if let Screen::WorkspaceDetail { name, .. } = &self.screen {
            let cwd = std::env::current_dir().ok()?;
            let (ws_path, manifest) =
                crate::workspace::resolve_workspace(Some(name), &cwd, &self.config).ok()?;
            crate::workspace::status::workspace_status(&manifest, &ws_path, false).ok()
        } else {
            None
        }
    }

    /// Color for workspace health status.
    pub fn health_color(health: &WorkspaceHealth) -> ratatui::style::Color {
        use ratatui::style::Color;
        match health {
            WorkspaceHealth::Clean => Color::Green,
            WorkspaceHealth::Dirty(_) => Color::Yellow,
            WorkspaceHealth::Broken(_) => Color::Red,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentsConfig, DefaultsConfig, RegistryConfig, WorkspaceConfig};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn test_config(dir: &std::path::Path) -> Config {
        let ws_root = dir.join("loom");
        std::fs::create_dir_all(ws_root.join(".loom")).unwrap();
        Config {
            registry: RegistryConfig { scan_roots: vec![] },
            workspace: WorkspaceConfig { root: ws_root },
            sync: None,
            terminal: None,
            defaults: DefaultsConfig::default(),
            repos: BTreeMap::new(),
            specs: None,
            agents: AgentsConfig::default(),
        }
    }

    #[test]
    fn test_app_initial_state() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let app = App::new(config);

        assert!(!app.should_quit);
        assert!(app.workspaces.is_empty());
        assert_eq!(app.selected, 0);
        assert!(matches!(app.screen, Screen::WorkspaceList));
    }

    #[test]
    fn test_quit_message() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let mut app = App::new(config);

        app.update(Message::Quit);
        assert!(app.should_quit);
    }

    #[test]
    fn test_select_navigation_empty() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let mut app = App::new(config);

        // Should not panic on empty list
        app.update(Message::SelectNext);
        assert_eq!(app.selected, 0);
        app.update(Message::SelectPrev);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_status_dismiss() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let mut app = App::new(config);

        app.set_status("test".to_string(), StatusLevel::Info);
        assert!(app.status.is_some());

        app.update(Message::DismissStatus);
        assert!(app.status.is_none());
    }

    #[test]
    fn test_wizard_name_input() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let mut app = App::new(config);

        app.screen = Screen::NewWizard {
            step: WizardStep::EnterName,
            name: String::new(),
            available_repos: vec![],
            groups: vec![],
            selected_group: 0,
            selected: HashSet::new(),
            focused: 0,
        };

        app.update(Message::WizardCharInput('m'));
        app.update(Message::WizardCharInput('y'));
        app.update(Message::WizardCharInput('-'));
        app.update(Message::WizardCharInput('w'));
        app.update(Message::WizardCharInput('s'));

        if let Screen::NewWizard { name, .. } = &app.screen {
            assert_eq!(name, "my-ws");
        } else {
            panic!("Expected NewWizard screen");
        }

        app.update(Message::WizardBackspace);
        if let Screen::NewWizard { name, .. } = &app.screen {
            assert_eq!(name, "my-w");
        } else {
            panic!("Expected NewWizard screen");
        }
    }

    #[test]
    fn test_wizard_generates_random_name_when_empty() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let mut app = App::new(config);

        app.screen = Screen::NewWizard {
            step: WizardStep::EnterName,
            name: String::new(),
            available_repos: vec![],
            groups: vec!["test-org".to_string()],
            selected_group: 0,
            selected: HashSet::new(),
            focused: 0,
        };

        app.update(Message::WizardNextStep);

        // With one group, should auto-skip to SelectRepos with a generated name
        if let Screen::NewWizard { name, step, .. } = &app.screen {
            assert!(!name.is_empty(), "Name should be generated");
            assert_eq!(name.split('-').count(), 3, "Name should be adj-mod-noun");
            assert_eq!(*step, WizardStep::SelectRepos);
        } else {
            panic!("Expected NewWizard screen");
        }
    }

    #[test]
    fn test_wizard_empty_groups_shows_error() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let mut app = App::new(config);

        app.screen = Screen::NewWizard {
            step: WizardStep::EnterName,
            name: "test-ws".to_string(),
            available_repos: vec![],
            groups: vec![],
            selected_group: 0,
            selected: HashSet::new(),
            focused: 0,
        };

        app.update(Message::WizardNextStep);

        // With no groups, should stay on EnterName and show error
        if let Screen::NewWizard { step, .. } = &app.screen {
            assert_eq!(*step, WizardStep::EnterName);
        } else {
            panic!("Expected NewWizard screen");
        }
        assert!(app.status.is_some(), "Should show error status");
    }

    #[test]
    fn test_cancel_from_detail_returns_to_list() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let mut app = App::new(config);

        app.screen = Screen::WorkspaceDetail {
            name: "test".to_string(),
            path: PathBuf::from("/tmp/test"),
        };

        app.update(Message::Cancel);
        assert!(matches!(app.screen, Screen::WorkspaceList));
    }
}
