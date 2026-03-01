use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Cell, Clear, Paragraph, Row, Table, TableState, Wrap};

use super::app::{App, Screen, StatusLevel, WizardStep};

/// Main view dispatch (TEA view function).
pub fn view(app: &App, frame: &mut Frame) {
    let [main_area, status_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

    // Render main content based on screen
    match &app.screen {
        Screen::WorkspaceList => render_workspace_list(app, frame, main_area),
        Screen::WorkspaceDetail { .. } => render_workspace_detail(app, frame, main_area),
        Screen::NewWizard { .. } => render_new_wizard(app, frame, main_area),
        Screen::ConfirmDialog { message, .. } => {
            // Render list behind the dialog
            render_workspace_list(app, frame, main_area);
            render_confirm_dialog(frame, main_area, message);
        }
    }

    // Status bar
    render_status_bar(app, frame, status_area);
}

fn render_workspace_list(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::bordered().title(" loom — workspaces ");

    if app.workspaces.is_empty() {
        let text = Paragraph::new("No workspaces. Press 'n' to create one.")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(text, area);
        return;
    }

    let header = Row::new(vec!["NAME", "REPOS", "STATUS", "CREATED"])
        .style(Style::default().bold())
        .bottom_margin(1);

    let rows: Vec<Row> = app
        .workspaces
        .iter()
        .map(|ws| {
            let status_str = match &ws.status {
                crate::workspace::list::WorkspaceHealth::Clean => "clean".to_string(),
                crate::workspace::list::WorkspaceHealth::Dirty(n) => format!("{n} dirty"),
                crate::workspace::list::WorkspaceHealth::Broken(msg) => {
                    format!("broken: {msg}")
                }
            };
            let color = App::health_color(&ws.status);
            let date = ws.created.format("%Y-%m-%d").to_string();

            Row::new(vec![
                Cell::new(ws.name.clone()),
                Cell::new(ws.repo_count.to_string()),
                Cell::new(status_str).style(Style::default().fg(color)),
                Cell::new(date),
            ])
        })
        .collect();

    let widths = [
        Constraint::Min(20),
        Constraint::Length(6),
        Constraint::Min(12),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">> ");

    let mut state = TableState::default();
    state.select(Some(app.selected));
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_workspace_detail(app: &App, frame: &mut Frame, area: Rect) {
    let (name, path) = if let Screen::WorkspaceDetail { name, path } = &app.screen {
        (name.as_str(), path.display().to_string())
    } else {
        return;
    };

    let block = Block::bordered().title(format!(" loom — {name} "));

    let status = app.workspace_detail_status();

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [info_area, table_area] =
        Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(inner);

    // Info section
    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Path: ", Style::default().bold()),
            Span::raw(&path),
        ]),
        Line::from(vec![
            Span::styled("Repos: ", Style::default().bold()),
            Span::raw(
                status
                    .as_ref()
                    .map(|s| s.repos.len().to_string())
                    .unwrap_or_else(|| "?".to_string()),
            ),
        ]),
    ]);
    frame.render_widget(info, info_area);

    // Repo table
    if let Some(ref status) = status {
        let header = Row::new(vec!["REPO", "BRANCH", "STATUS", "AHEAD/BEHIND"])
            .style(Style::default().bold())
            .bottom_margin(1);

        let rows: Vec<Row> = status
            .repos
            .iter()
            .map(|repo| {
                if !repo.exists {
                    return Row::new(vec![
                        Cell::new(repo.name.clone()),
                        Cell::new(repo.branch.clone()),
                        Cell::new("missing").style(Style::default().fg(Color::Red)),
                        Cell::new("-"),
                    ]);
                }

                let status_str = if repo.is_dirty {
                    format!("{} changed", repo.change_count)
                } else {
                    "clean".to_string()
                };
                let color = if repo.is_dirty {
                    Color::Yellow
                } else {
                    Color::Green
                };

                let ab = if repo.ahead > 0 || repo.behind > 0 {
                    format!("+{} -{}", repo.ahead, repo.behind)
                } else {
                    "-".to_string()
                };

                Row::new(vec![
                    Cell::new(repo.name.clone()),
                    Cell::new(repo.branch.clone()),
                    Cell::new(status_str).style(Style::default().fg(color)),
                    Cell::new(ab),
                ])
            })
            .collect();

        let widths = [
            Constraint::Min(20),
            Constraint::Min(25),
            Constraint::Min(12),
            Constraint::Length(14),
        ];

        let table = Table::new(rows, widths).header(header);
        frame.render_widget(table, table_area);
    } else {
        let msg = Paragraph::new("Loading...").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, table_area);
    }
}

fn render_new_wizard(app: &App, frame: &mut Frame, area: Rect) {
    let Screen::NewWizard {
        step,
        name,
        available_repos,
        groups,
        selected_group,
        selected,
        focused,
    } = &app.screen
    else {
        return;
    };

    let block = Block::bordered().title(" loom — new workspace ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    match step {
        WizardStep::EnterName => {
            let [prompt_area, input_area, hint_area] = Layout::vertical([
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .areas(inner);

            let prompt = Paragraph::new("Enter workspace name (lowercase, hyphens allowed):");
            frame.render_widget(prompt, prompt_area);

            let input_block = Block::bordered().title(" name ");
            let display_name = if name.is_empty() { "_" } else { name };
            let input = Paragraph::new(display_name.to_string())
                .block(input_block)
                .style(Style::default().fg(Color::Cyan));
            frame.render_widget(input, input_area);

            let hint = Paragraph::new("Press Enter to continue, Esc to cancel")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(hint, hint_area);
        }
        WizardStep::SelectGroups => {
            let [prompt_area, list_area, hint_area] = Layout::vertical([
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .areas(inner);

            let prompt = Paragraph::new(format!(
                "Select organization for '{}':",
                name
            ));
            frame.render_widget(prompt, prompt_area);

            let rows: Vec<Row> = groups
                .iter()
                .enumerate()
                .map(|(i, group)| {
                    let repo_count = available_repos.iter().filter(|r| r.org == *group).count();
                    let marker = if i == *focused { ">>" } else { "  " };
                    let style = if i == *focused {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default()
                    };
                    Row::new(vec![
                        Cell::new(marker),
                        Cell::new(group.clone()),
                        Cell::new(format!("({repo_count} repos)")),
                    ])
                    .style(style)
                })
                .collect();

            let widths = [
                Constraint::Length(3),
                Constraint::Min(20),
                Constraint::Length(12),
            ];
            let table = Table::new(rows, widths);
            frame.render_widget(table, list_area);

            let hint = Paragraph::new("Enter: select  Esc: back")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(hint, hint_area);
        }
        WizardStep::SelectRepos => {
            let [prompt_area, list_area, hint_area] = Layout::vertical([
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .areas(inner);

            let prompt = Paragraph::new(format!(
                "Select repos for '{}' (Space to toggle, Enter to confirm):",
                name
            ));
            frame.render_widget(prompt, prompt_area);

            // Show only repos from selected groups
            let visible_indices =
                App::filtered_repo_indices(available_repos, &groups[*selected_group]);
            let rows: Vec<Row> = visible_indices
                .iter()
                .enumerate()
                .map(|(display_idx, &repo_idx)| {
                    let repo = &available_repos[repo_idx];
                    let marker = if selected.contains(&repo_idx) {
                        "[x]"
                    } else {
                        "[ ]"
                    };
                    let style = if display_idx == *focused {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default()
                    };
                    Row::new(vec![
                        Cell::new(marker),
                        Cell::new(format!("{}/{}", repo.org, repo.name)),
                    ])
                    .style(style)
                })
                .collect();

            let widths = [Constraint::Length(4), Constraint::Min(20)];
            let table = Table::new(rows, widths);
            frame.render_widget(table, list_area);

            let hint = Paragraph::new("Space: toggle  Enter: confirm  Esc: back")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(hint, hint_area);
        }
        WizardStep::Confirm => {
            let selected_names: Vec<String> = selected
                .iter()
                .filter_map(|&i| {
                    available_repos
                        .get(i)
                        .map(|r| format!("{}/{}", r.org, r.name))
                })
                .collect();

            let text = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Workspace: ", Style::default().bold()),
                    Span::raw(name),
                ]),
                Line::from(vec![
                    Span::styled("  Repos:     ", Style::default().bold()),
                    Span::raw(selected_names.join(", ")),
                ]),
                Line::from(""),
                Line::from("  Press Enter to create, Esc to go back."),
            ];

            let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
            frame.render_widget(paragraph, inner);
        }
    }
}

fn render_confirm_dialog(frame: &mut Frame, area: Rect, message: &str) {
    let popup_area = area.centered(Constraint::Percentage(60), Constraint::Length(7));

    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Confirm ")
        .style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let text = Paragraph::new(vec![
        Line::from(""),
        Line::from(format!("  {message}")),
        Line::from(""),
        Line::from("  [Y]es  /  [N]o"),
    ]);
    frame.render_widget(text, inner);
}

fn render_status_bar(app: &App, frame: &mut Frame, area: Rect) {
    let (text, style) = if let Some(ref status) = app.status {
        let color = match status.level {
            StatusLevel::Info => Color::Green,
            StatusLevel::Error => Color::Red,
        };
        (
            status.text.clone(),
            Style::default().fg(Color::White).bg(color),
        )
    } else {
        // Show keybinding hints based on screen
        let hints = match &app.screen {
            Screen::WorkspaceList => "q:quit  n:new  Enter:detail  r:refresh",
            Screen::WorkspaceDetail { .. } => "Esc:back  d:teardown",
            Screen::NewWizard { step, .. } => match step {
                WizardStep::EnterName => "Enter:next  Esc:cancel",
                WizardStep::SelectGroups => "Enter:select  Esc:back",
                WizardStep::SelectRepos => "Space:toggle  Enter:confirm  Esc:back",
                WizardStep::Confirm => "Enter:create  Esc:back",
            },
            Screen::ConfirmDialog { .. } => "y:yes  n:no  Esc:cancel",
        };
        (hints.to_string(), Style::default().fg(Color::DarkGray))
    };

    let bar = Paragraph::new(format!(" {text}")).style(style);
    frame.render_widget(bar, area);
}
