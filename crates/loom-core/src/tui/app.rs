use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use crate::config::Config;
use crate::groups::GroupEntry;
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
        /// Config groups and org groups for the selection step.
        groups: Vec<GroupEntry>,
        /// Indices into `groups` that are selected.
        selected_groups: HashSet<usize>,
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
                    selected_groups,
                    groups,
                    focused,
                    ..
                } => {
                    let visible_count =
                        Self::filtered_repo_count(available_repos, selected_groups, groups);
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
                    selected_groups,
                    groups,
                    focused,
                    ..
                } => {
                    let visible_count =
                        Self::filtered_repo_count(available_repos, selected_groups, groups);
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
                            selected_groups: HashSet::new(),
                            selected: HashSet::new(),
                            focused: 0,
                        };
                    }
                    Screen::NewWizard {
                        step: WizardStep::SelectRepos,
                        name,
                        available_repos,
                        groups,
                        selected_groups,
                        ..
                    } => {
                        self.screen = Screen::NewWizard {
                            step: WizardStep::SelectGroups,
                            name,
                            available_repos,
                            groups,
                            selected_groups,
                            selected: HashSet::new(),
                            focused: 0,
                        };
                    }
                    Screen::NewWizard {
                        step: WizardStep::Confirm,
                        name,
                        available_repos,
                        groups,
                        selected_groups,
                        selected,
                        focused,
                    } => {
                        self.screen = Screen::NewWizard {
                            step: WizardStep::SelectRepos,
                            name,
                            available_repos,
                            groups,
                            selected_groups,
                            selected,
                            focused,
                        };
                    }
                    Screen::ConfirmDialog { .. } => {
                        self.refresh_workspaces();
                    }
                    Screen::WorkspaceList => { /* already home */ }
                }
            }

            // --- Start new workspace wizard ---
            Message::StartNewWizard => {
                let repos = crate::registry::discover_repos(
                    &self.config.registry.scan_roots,
                    Some(&self.config.workspace.root),
                );

                // Build combined group list: config groups + org groups.
                // Config groups appear in BTreeMap (alphabetical) order, not
                // config.toml source order — TOML key order is not preserved.
                let mut groups: Vec<GroupEntry> = Vec::new();
                for (name, repo_names) in &self.config.groups {
                    groups.push(GroupEntry::ConfigGroup {
                        name: name.clone(),
                        repo_names: repo_names.clone(),
                    });
                }
                let mut orgs: Vec<String> = repos.iter().map(|r| r.org.clone()).collect();
                orgs.dedup();
                for org in orgs {
                    groups.push(GroupEntry::OrgGroup { name: org });
                }

                self.screen = Screen::NewWizard {
                    step: WizardStep::EnterName,
                    name: String::new(),
                    available_repos: repos,
                    groups,
                    selected_groups: HashSet::new(),
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
                        selected_groups,
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
                                        selected_groups,
                                        selected,
                                        focused,
                                    };
                                    return;
                                }
                            }
                        } else {
                            name
                        };

                        let has_config_groups = groups
                            .iter()
                            .any(|g| matches!(g, GroupEntry::ConfigGroup { .. }));
                        let org_count = groups
                            .iter()
                            .filter(|g| matches!(g, GroupEntry::OrgGroup { .. }))
                            .count();

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
                                selected_groups: HashSet::new(),
                                selected,
                                focused: 0,
                            };
                        } else if !has_config_groups && org_count <= 1 {
                            // No config groups and at most one org — skip group selection
                            let all_selected: HashSet<usize> = (0..groups.len()).collect();
                            self.screen = Screen::NewWizard {
                                step: WizardStep::SelectRepos,
                                name,
                                available_repos,
                                groups,
                                selected_groups: all_selected,
                                selected,
                                focused: 0,
                            };
                        } else {
                            self.screen = Screen::NewWizard {
                                step: WizardStep::SelectGroups,
                                name,
                                available_repos,
                                groups,
                                selected_groups,
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
                        selected_groups,
                        ..
                    } => {
                        if selected_groups.is_empty() {
                            self.set_status(
                                "Select at least one group".to_string(),
                                StatusLevel::Error,
                            );
                            self.screen = Screen::NewWizard {
                                step: WizardStep::SelectGroups,
                                name,
                                available_repos,
                                groups,
                                selected_groups,
                                selected: HashSet::new(),
                                focused: 0,
                            };
                        } else {
                            // Pre-select repos from selected config groups
                            let filtered = Self::filtered_repo_indices(
                                &available_repos,
                                &selected_groups,
                                &groups,
                            );
                            let mut pre_selected = HashSet::new();
                            let mut warnings: Vec<String> = Vec::new();
                            for &gi in &selected_groups {
                                if let Some(GroupEntry::ConfigGroup {
                                    name: gname,
                                    repo_names,
                                }) = groups.get(gi)
                                {
                                    let mut matched_count = 0;
                                    for rn in repo_names {
                                        if let Some(pos) = filtered.iter().position(|&ri| {
                                            let r = &available_repos[ri];
                                            r.name == *rn || format!("{}/{}", r.org, r.name) == *rn
                                        }) {
                                            pre_selected.insert(filtered[pos]);
                                            matched_count += 1;
                                        }
                                    }
                                    if matched_count < repo_names.len() {
                                        warnings.push(format!(
                                            "Group '{}' matched {} of {} repos",
                                            gname,
                                            matched_count,
                                            repo_names.len()
                                        ));
                                    }
                                }
                            }
                            if !warnings.is_empty() {
                                self.set_status(warnings.join("; "), StatusLevel::Info);
                            }
                            self.screen = Screen::NewWizard {
                                step: WizardStep::SelectRepos,
                                name,
                                available_repos,
                                groups,
                                selected_groups,
                                selected: pre_selected,
                                focused: 0,
                            };
                        }
                    }
                    Screen::NewWizard {
                        step: WizardStep::SelectRepos,
                        name,
                        available_repos,
                        groups,
                        selected_groups,
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
                                selected_groups,
                                selected,
                                focused,
                            };
                        } else {
                            self.screen = Screen::NewWizard {
                                step: WizardStep::Confirm,
                                name,
                                available_repos,
                                groups,
                                selected_groups,
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
                match &mut self.screen {
                    Screen::NewWizard {
                        step: WizardStep::SelectGroups,
                        selected_groups,
                        ..
                    } => {
                        if selected_groups.contains(&idx) {
                            selected_groups.remove(&idx);
                        } else {
                            selected_groups.insert(idx);
                        }
                    }
                    // Only applies to SelectRepos — SelectGroups uses Enter-to-select
                    Screen::NewWizard {
                        step: WizardStep::SelectRepos,
                        available_repos,
                        groups,
                        selected_groups,
                        selected,
                        ..
                    } => {
                        let visible =
                            Self::filtered_repo_indices(available_repos, selected_groups, groups);
                        if let Some(&repo_idx) = visible.get(idx) {
                            if selected.contains(&repo_idx) {
                                selected.remove(&repo_idx);
                            } else {
                                selected.insert(repo_idx);
                            }
                        }
                    }
                    _ => {}
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

    /// Count repos visible after group filtering.
    fn filtered_repo_count(
        repos: &[RepoEntry],
        selected_groups: &HashSet<usize>,
        groups: &[GroupEntry],
    ) -> usize {
        Self::filtered_repo_indices(repos, selected_groups, groups).len()
    }

    /// Get the original indices of repos that belong to selected groups.
    pub fn filtered_repo_indices(
        repos: &[RepoEntry],
        selected_groups: &HashSet<usize>,
        groups: &[GroupEntry],
    ) -> Vec<usize> {
        // Collect selected org names from OrgGroup entries
        let selected_org_names: HashSet<&str> = selected_groups
            .iter()
            .filter_map(|&i| match groups.get(i) {
                Some(GroupEntry::OrgGroup { name }) => Some(name.as_str()),
                _ => None,
            })
            .collect();

        // Collect repo names from selected ConfigGroup entries
        let config_repo_names: HashSet<&str> = selected_groups
            .iter()
            .filter_map(|&i| match groups.get(i) {
                Some(GroupEntry::ConfigGroup { repo_names, .. }) => Some(repo_names),
                _ => None,
            })
            .flat_map(|names| names.iter().map(|s| s.as_str()))
            .collect();

        let mut seen = HashSet::new();
        repos
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                // Match by org group
                let by_org = selected_org_names.contains(r.org.as_str());
                // Match by config group repo names (bare name or org/name)
                let by_config = config_repo_names.contains(r.name.as_str())
                    || config_repo_names.contains(format!("{}/{}", r.org, r.name).as_str());
                by_org || by_config
            })
            .filter(|(i, _)| seen.insert(*i))
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
            groups: BTreeMap::new(),
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
            selected_groups: HashSet::new(),
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
            groups: vec![GroupEntry::OrgGroup {
                name: "test-org".to_string(),
            }],
            selected_groups: HashSet::new(),
            selected: HashSet::new(),
            focused: 0,
        };

        app.update(Message::WizardNextStep);

        // With one org group, should auto-skip to SelectRepos with a generated name
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
            selected_groups: HashSet::new(),
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

    fn make_repo(name: &str, org: &str) -> RepoEntry {
        RepoEntry {
            name: name.to_string(),
            org: org.to_string(),
            path: PathBuf::from(format!("/code/{}/{}", org, name)),
            remote_url: None,
        }
    }

    #[test]
    fn test_filtered_repo_indices_org_groups() {
        let repos = vec![
            make_repo("api", "dasch"),
            make_repo("das", "dasch"),
            make_repo("tools", "acme"),
        ];
        let groups = vec![
            GroupEntry::OrgGroup {
                name: "dasch".to_string(),
            },
            GroupEntry::OrgGroup {
                name: "acme".to_string(),
            },
        ];
        let selected = HashSet::from([0]); // select "dasch"
        let indices = App::filtered_repo_indices(&repos, &selected, &groups);
        assert_eq!(indices, vec![0, 1]); // api and das
    }

    #[test]
    fn test_filtered_repo_indices_config_groups() {
        let repos = vec![
            make_repo("api", "dasch"),
            make_repo("das", "dasch"),
            make_repo("sipi", "dasch"),
        ];
        let groups = vec![
            GroupEntry::ConfigGroup {
                name: "stack".to_string(),
                repo_names: vec!["api".to_string(), "sipi".to_string()],
            },
            GroupEntry::OrgGroup {
                name: "dasch".to_string(),
            },
        ];
        let selected = HashSet::from([0]); // select config group "stack"
        let indices = App::filtered_repo_indices(&repos, &selected, &groups);
        assert_eq!(indices, vec![0, 2]); // api and sipi
    }

    #[test]
    fn test_filtered_repo_indices_mixed_groups() {
        let repos = vec![
            make_repo("api", "dasch"),
            make_repo("das", "dasch"),
            make_repo("tools", "acme"),
        ];
        let groups = vec![
            GroupEntry::ConfigGroup {
                name: "stack".to_string(),
                repo_names: vec!["api".to_string(), "tools".to_string()],
            },
            GroupEntry::OrgGroup {
                name: "dasch".to_string(),
            },
        ];
        let selected = HashSet::from([0, 1]); // select both
        let indices = App::filtered_repo_indices(&repos, &selected, &groups);
        // api (config+org), das (org), tools (config) — deduplicated
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_skip_logic_with_config_groups() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = test_config(dir.path());
        config
            .groups
            .insert("my-stack".to_string(), vec!["api".to_string()]);
        let mut app = App::new(config);

        // Set up wizard with 1 config group + 1 org group
        app.screen = Screen::NewWizard {
            step: WizardStep::EnterName,
            name: "test-ws".to_string(),
            available_repos: vec![make_repo("api", "dasch")],
            groups: vec![
                GroupEntry::ConfigGroup {
                    name: "my-stack".to_string(),
                    repo_names: vec!["api".to_string()],
                },
                GroupEntry::OrgGroup {
                    name: "dasch".to_string(),
                },
            ],
            selected_groups: HashSet::new(),
            selected: HashSet::new(),
            focused: 0,
        };

        app.update(Message::WizardNextStep);

        // Should NOT skip SelectGroups because config groups exist
        if let Screen::NewWizard { step, .. } = &app.screen {
            assert_eq!(*step, WizardStep::SelectGroups);
        } else {
            panic!("Expected NewWizard screen");
        }
    }

    #[test]
    fn test_skip_logic_without_config_groups_single_org() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let mut app = App::new(config);

        // Set up wizard with 1 org group only
        app.screen = Screen::NewWizard {
            step: WizardStep::EnterName,
            name: "test-ws".to_string(),
            available_repos: vec![make_repo("api", "dasch")],
            groups: vec![GroupEntry::OrgGroup {
                name: "dasch".to_string(),
            }],
            selected_groups: HashSet::new(),
            selected: HashSet::new(),
            focused: 0,
        };

        app.update(Message::WizardNextStep);

        // Should skip to SelectRepos (no config groups, single org)
        if let Screen::NewWizard { step, .. } = &app.screen {
            assert_eq!(*step, WizardStep::SelectRepos);
        } else {
            panic!("Expected NewWizard screen");
        }
    }

    #[test]
    fn test_config_group_preselects_repos() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path());
        let mut app = App::new(config);

        let repos = vec![
            make_repo("api", "dasch"),
            make_repo("das", "dasch"),
            make_repo("sipi", "dasch"),
        ];
        let groups = vec![
            GroupEntry::ConfigGroup {
                name: "stack".to_string(),
                repo_names: vec!["api".to_string(), "sipi".to_string()],
            },
            GroupEntry::OrgGroup {
                name: "dasch".to_string(),
            },
        ];

        // Start at SelectGroups with both groups selected
        app.screen = Screen::NewWizard {
            step: WizardStep::SelectGroups,
            name: "test-ws".to_string(),
            available_repos: repos,
            groups,
            selected_groups: HashSet::from([0, 1]),
            selected: HashSet::new(),
            focused: 0,
        };

        app.update(Message::WizardNextStep);

        // Should advance to SelectRepos with api(0) and sipi(2) pre-selected
        if let Screen::NewWizard { step, selected, .. } = &app.screen {
            assert_eq!(*step, WizardStep::SelectRepos);
            assert!(selected.contains(&0), "api should be pre-selected");
            assert!(!selected.contains(&1), "das should NOT be pre-selected");
            assert!(selected.contains(&2), "sipi should be pre-selected");
        } else {
            panic!("Expected NewWizard screen");
        }
    }
}
