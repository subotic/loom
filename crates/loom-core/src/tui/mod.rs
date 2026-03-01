// Terminal UI: ratatui-based interactive workspace management
pub mod app;
pub mod views;

use std::time::Duration;

use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::config::Config;
use app::{App, Message, Screen, WizardStep};

/// Run the TUI application. This takes over the terminal.
pub fn run_tui(config: Config) -> Result<()> {
    let mut terminal = ratatui::init();

    let mut app = App::new(config);
    app.refresh_workspaces();

    let result = run_event_loop(&mut terminal, &mut app);

    ratatui::restore();
    result
}

fn run_event_loop(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| views::view(app, frame))?;

        app.tick();

        if app.should_quit {
            break;
        }

        // Poll for events with 100ms timeout (for tick/status dismiss)
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            // Only handle key press events (not release/repeat)
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if let Some(msg) = map_key_to_message(key.code, &app.screen) {
                app.update(msg);
            }
        }
    }

    Ok(())
}

/// Map a key press to a message based on current screen context.
fn map_key_to_message(key: KeyCode, screen: &Screen) -> Option<Message> {
    match screen {
        Screen::WorkspaceList => match key {
            KeyCode::Char('q') | KeyCode::Esc => Some(Message::Quit),
            KeyCode::Char('j') | KeyCode::Down => Some(Message::SelectNext),
            KeyCode::Char('k') | KeyCode::Up => Some(Message::SelectPrev),
            KeyCode::Enter => Some(Message::OpenDetail),
            KeyCode::Char('n') => Some(Message::StartNewWizard),
            KeyCode::Char('r') => Some(Message::RefreshList),
            _ => None,
        },
        Screen::WorkspaceDetail { .. } => match key {
            KeyCode::Char('q') => Some(Message::Quit),
            KeyCode::Esc => Some(Message::Cancel),
            KeyCode::Char('d') => Some(Message::TeardownWorkspace),
            _ => None,
        },
        Screen::NewWizard { step, .. } => match step {
            WizardStep::EnterName => match key {
                KeyCode::Esc => Some(Message::Cancel),
                KeyCode::Enter => Some(Message::WizardNextStep),
                KeyCode::Backspace => Some(Message::WizardBackspace),
                KeyCode::Char(ch) => Some(Message::WizardCharInput(ch)),
                _ => None,
            },
            WizardStep::SelectGroups => match key {
                KeyCode::Esc => Some(Message::Cancel),
                KeyCode::Enter => Some(Message::WizardNextStep),
                KeyCode::Char('j') | KeyCode::Down => Some(Message::SelectNext),
                KeyCode::Char('k') | KeyCode::Up => Some(Message::SelectPrev),
                _ => None,
            },
            WizardStep::SelectRepos => match key {
                KeyCode::Esc => Some(Message::Cancel),
                KeyCode::Enter => Some(Message::WizardNextStep),
                KeyCode::Char('j') | KeyCode::Down => Some(Message::SelectNext),
                KeyCode::Char('k') | KeyCode::Up => Some(Message::SelectPrev),
                KeyCode::Char(' ') => {
                    if let Screen::NewWizard { focused, .. } = screen {
                        Some(Message::ToggleRepo(*focused))
                    } else {
                        None
                    }
                }
                _ => None,
            },
            WizardStep::Confirm => match key {
                KeyCode::Esc => Some(Message::Cancel),
                KeyCode::Enter => Some(Message::WizardNextStep),
                _ => None,
            },
        },
        Screen::ConfirmDialog { .. } => match key {
            KeyCode::Char('y') | KeyCode::Enter => Some(Message::ConfirmYes),
            KeyCode::Char('n') | KeyCode::Esc => Some(Message::ConfirmNo),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_mapping_workspace_list() {
        let screen = Screen::WorkspaceList;

        assert!(matches!(
            map_key_to_message(KeyCode::Char('q'), &screen),
            Some(Message::Quit)
        ));
        assert!(matches!(
            map_key_to_message(KeyCode::Down, &screen),
            Some(Message::SelectNext)
        ));
        assert!(matches!(
            map_key_to_message(KeyCode::Up, &screen),
            Some(Message::SelectPrev)
        ));
        assert!(matches!(
            map_key_to_message(KeyCode::Enter, &screen),
            Some(Message::OpenDetail)
        ));
        assert!(matches!(
            map_key_to_message(KeyCode::Char('n'), &screen),
            Some(Message::StartNewWizard)
        ));
        assert!(map_key_to_message(KeyCode::Char('x'), &screen).is_none());
    }

    #[test]
    fn test_key_mapping_wizard_name() {
        let screen = Screen::NewWizard {
            step: WizardStep::EnterName,
            name: String::new(),
            available_repos: vec![],
            groups: vec![],
            selected_groups: std::collections::HashSet::new(),
            selected: std::collections::HashSet::new(),
            focused: 0,
        };

        assert!(matches!(
            map_key_to_message(KeyCode::Char('a'), &screen),
            Some(Message::WizardCharInput('a'))
        ));
        assert!(matches!(
            map_key_to_message(KeyCode::Backspace, &screen),
            Some(Message::WizardBackspace)
        ));
        assert!(matches!(
            map_key_to_message(KeyCode::Enter, &screen),
            Some(Message::WizardNextStep)
        ));
        assert!(matches!(
            map_key_to_message(KeyCode::Esc, &screen),
            Some(Message::Cancel)
        ));
    }

    #[test]
    fn test_key_mapping_confirm_dialog() {
        let screen = Screen::ConfirmDialog {
            message: "test?".to_string(),
            action: app::PendingAction::TeardownWorkspace {
                name: "test".to_string(),
            },
        };

        assert!(matches!(
            map_key_to_message(KeyCode::Char('y'), &screen),
            Some(Message::ConfirmYes)
        ));
        assert!(matches!(
            map_key_to_message(KeyCode::Char('n'), &screen),
            Some(Message::ConfirmNo)
        ));
        assert!(matches!(
            map_key_to_message(KeyCode::Esc, &screen),
            Some(Message::ConfirmNo)
        ));
    }
}
