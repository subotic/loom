//! Interactive tree-based multi-select widget for repo selection.
//!
//! Renders repos grouped by their org path as a collapsible tree.
//! Uses crossterm raw mode for keyboard interaction.

use std::io::{self, IsTerminal, Write};

use crossterm::{
    cursor, event,
    event::{Event, KeyCode, KeyEvent},
    style::Stylize,
    terminal,
};

use loom_core::registry::RepoEntry;

/// A node in the repo selection tree.
pub enum TreeNode {
    Folder {
        name: String,
        expanded: bool,
        children: Vec<TreeNode>,
    },
    Repo {
        /// Index into the original flat repo list
        index: usize,
        name: String,
        selected: bool,
    },
}

/// Build a tree from a flat list of RepoEntry based on the `org` field.
///
/// - `org=""` → top-level leaf
/// - `org="dasch-swiss"` → one folder level
/// - `org="github.com/dasch-swiss"` → two nested folder levels
pub fn build_tree(repos: &[RepoEntry]) -> Vec<TreeNode> {
    let mut root: Vec<TreeNode> = Vec::new();

    for (i, repo) in repos.iter().enumerate() {
        if repo.org.is_empty() {
            root.push(TreeNode::Repo {
                index: i,
                name: repo.name.clone(),
                selected: false,
            });
        } else {
            let parts: Vec<&str> = repo.org.split('/').collect();
            insert_into_tree(&mut root, &parts, i, &repo.name);
        }
    }

    root
}

fn insert_into_tree(children: &mut Vec<TreeNode>, path: &[&str], index: usize, name: &str) {
    if path.is_empty() {
        children.push(TreeNode::Repo {
            index,
            name: name.to_string(),
            selected: false,
        });
        return;
    }

    let folder_name = path[0];
    let rest = &path[1..];

    // Find existing folder
    let folder_pos = children
        .iter()
        .position(|n| matches!(n, TreeNode::Folder { name, .. } if name == folder_name));

    match folder_pos {
        Some(pos) => {
            if let TreeNode::Folder { children, .. } = &mut children[pos] {
                insert_into_tree(children, rest, index, name);
            }
        }
        None => {
            let mut new_folder = TreeNode::Folder {
                name: folder_name.to_string(),
                expanded: false,
                children: Vec::new(),
            };
            if let TreeNode::Folder { children, .. } = &mut new_folder {
                insert_into_tree(children, rest, index, name);
            }
            children.push(new_folder);
        }
    }
}

/// A flattened visible row for rendering.
struct VisibleRow {
    depth: usize,
    kind: RowKind,
    /// Path into the tree structure for cursor tracking
    tree_path: Vec<usize>,
}

enum RowKind {
    Folder {
        name: String,
        expanded: bool,
        selected_count: usize,
        total_count: usize,
    },
    Repo {
        name: String,
        selected: bool,
    },
}

/// Flatten the tree into visible rows (respecting collapsed folders).
fn flatten_visible(tree: &[TreeNode], depth: usize, path_prefix: &[usize]) -> Vec<VisibleRow> {
    let mut rows = Vec::new();
    for (i, node) in tree.iter().enumerate() {
        let mut tree_path = path_prefix.to_vec();
        tree_path.push(i);

        match node {
            TreeNode::Folder {
                name,
                expanded,
                children,
            } => {
                let (selected, total) = count_selected(children);
                rows.push(VisibleRow {
                    depth,
                    kind: RowKind::Folder {
                        name: name.clone(),
                        expanded: *expanded,
                        selected_count: selected,
                        total_count: total,
                    },
                    tree_path: tree_path.clone(),
                });
                if *expanded {
                    rows.extend(flatten_visible(children, depth + 1, &tree_path));
                }
            }
            TreeNode::Repo { name, selected, .. } => {
                rows.push(VisibleRow {
                    depth,
                    kind: RowKind::Repo {
                        name: name.clone(),
                        selected: *selected,
                    },
                    tree_path,
                });
            }
        }
    }
    rows
}

fn count_selected(children: &[TreeNode]) -> (usize, usize) {
    let mut selected = 0;
    let mut total = 0;
    for node in children {
        match node {
            TreeNode::Repo { selected: sel, .. } => {
                total += 1;
                if *sel {
                    selected += 1;
                }
            }
            TreeNode::Folder { children, .. } => {
                let (s, t) = count_selected(children);
                selected += s;
                total += t;
            }
        }
    }
    (selected, total)
}

/// Navigate into the tree using a path of indices.
fn get_node_mut<'a>(tree: &'a mut [TreeNode], path: &[usize]) -> Option<&'a mut TreeNode> {
    if path.is_empty() {
        return None;
    }
    let first = path[0];
    if first >= tree.len() {
        return None;
    }
    if path.len() == 1 {
        return Some(&mut tree[first]);
    }
    if let TreeNode::Folder { children, .. } = &mut tree[first] {
        get_node_mut(children, &path[1..])
    } else {
        None
    }
}

/// Toggle all repos inside a subtree.
fn toggle_all_in(node: &mut TreeNode, target: bool) {
    match node {
        TreeNode::Repo { selected, .. } => *selected = target,
        TreeNode::Folder { children, .. } => {
            for child in children {
                toggle_all_in(child, target);
            }
        }
    }
}

/// Toggle all repos in a folder: select all if any unselected, else deselect all.
fn toggle_folder(children: &mut [TreeNode]) {
    let (sel, total) = count_selected(children);
    let target = sel < total;
    for child in children {
        toggle_all_in(child, target);
    }
}

/// Collect indices of all selected repos.
fn collect_selected(tree: &[TreeNode]) -> Vec<usize> {
    let mut result = Vec::new();
    for node in tree {
        match node {
            TreeNode::Repo {
                index, selected, ..
            } => {
                if *selected {
                    result.push(*index);
                }
            }
            TreeNode::Folder { children, .. } => {
                result.extend(collect_selected(children));
            }
        }
    }
    result
}

/// Run the interactive tree selection widget. Returns indices into the original repo list.
pub fn run(mut tree: Vec<TreeNode>) -> anyhow::Result<Vec<usize>> {
    // Non-TTY fallback
    if !io::stdin().is_terminal() {
        anyhow::bail!(
            "Interactive repo selection requires a terminal. Use --repos or --groups for non-interactive mode."
        );
    }

    let mut stdout = io::stdout();

    // Enter alternate screen + raw mode for clean terminal handling (works in tmux)
    crossterm::execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
    terminal::enable_raw_mode()?;

    let mut cursor_row: usize = 0;
    let mut scroll_offset: usize = 0;

    let result = (|| -> anyhow::Result<Vec<usize>> {
        loop {
            let rows = flatten_visible(&tree, 0, &[]);
            if rows.is_empty() {
                anyhow::bail!("No repositories found.");
            }

            // Clamp cursor
            if cursor_row >= rows.len() {
                cursor_row = rows.len() - 1;
            }

            // Get terminal height for viewport
            let (_, term_height) = terminal::size().unwrap_or((80, 24));
            let header_lines = 2u16; // title + blank line
            let footer_lines = 2u16; // blank line + help text
            let viewport_height =
                (term_height.saturating_sub(header_lines + footer_lines)) as usize;

            // Adjust scroll to keep cursor visible
            if cursor_row < scroll_offset {
                scroll_offset = cursor_row;
            }
            if viewport_height > 0 && cursor_row >= scroll_offset + viewport_height {
                scroll_offset = cursor_row - viewport_height + 1;
            }

            // Render
            crossterm::execute!(stdout, cursor::MoveTo(0, 0))?;

            // Header
            write!(stdout, "  Select repos for workspace:")?;
            crossterm::execute!(stdout, terminal::Clear(terminal::ClearType::UntilNewLine))?;
            write!(stdout, "\r\n")?;
            crossterm::execute!(stdout, terminal::Clear(terminal::ClearType::UntilNewLine))?;
            write!(stdout, "\r\n")?;

            // Visible rows within viewport
            let visible_end = (scroll_offset + viewport_height).min(rows.len());
            for (i, row) in rows
                .iter()
                .enumerate()
                .take(visible_end)
                .skip(scroll_offset)
            {
                let indent = "  ".repeat(row.depth + 1);
                let marker = if i == cursor_row { ">" } else { " " };

                match &row.kind {
                    RowKind::Folder {
                        name,
                        expanded,
                        selected_count,
                        total_count,
                    } => {
                        let arrow = if *expanded { "▾" } else { "▸" };
                        let line = format!(
                            "{marker} {indent}{arrow} {name} ({selected_count}/{total_count})"
                        );
                        if i == cursor_row {
                            write!(stdout, "{}", line.bold())?;
                        } else {
                            write!(stdout, "{}", line.dark_grey())?;
                        }
                    }
                    RowKind::Repo { name, selected } => {
                        let check = if *selected { "[x]" } else { "[ ]" };
                        let line = format!("{marker} {indent}{check} {name}");
                        if i == cursor_row {
                            write!(stdout, "{}", line.bold())?;
                        } else {
                            write!(stdout, "{line}")?;
                        }
                    }
                }
                crossterm::execute!(stdout, terminal::Clear(terminal::ClearType::UntilNewLine))?;
                write!(stdout, "\r\n")?;
            }

            // Clear any leftover lines from previous render
            for _ in visible_end..scroll_offset + viewport_height {
                crossterm::execute!(stdout, terminal::Clear(terminal::ClearType::UntilNewLine))?;
                write!(stdout, "\r\n")?;
            }

            // Footer with scroll indicator
            crossterm::execute!(stdout, terminal::Clear(terminal::ClearType::UntilNewLine))?;
            write!(stdout, "\r\n")?;
            let scroll_hint = if rows.len() > viewport_height {
                format!(" [{}-{}/{}]", scroll_offset + 1, visible_end, rows.len())
            } else {
                String::new()
            };
            write!(
                stdout,
                "  ↑↓:navigate  Enter:expand/collapse  Space:toggle  a:toggle-folder  c:confirm  Esc:cancel{scroll_hint}"
            )?;
            crossterm::execute!(stdout, terminal::Clear(terminal::ClearType::UntilNewLine))?;
            stdout.flush()?;

            // Read key event — handle both enhanced (Press/Release) and legacy protocols
            let ev = event::read()?;
            let code = match ev {
                Event::Key(KeyEvent {
                    code,
                    kind: event::KeyEventKind::Press,
                    ..
                }) => Some(code),
                // Legacy terminals (tmux) may not set kind — treat all Key events as press
                // when kind is not Release (crossterm 0.29 default for legacy is Press)
                _ => None,
            };

            if let Some(code) = code {
                match code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        cursor_row = cursor_row.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') if cursor_row + 1 < rows.len() => {
                        cursor_row += 1;
                    }
                    KeyCode::Char(' ') => {
                        let path = rows[cursor_row].tree_path.clone();
                        if let Some(node) = get_node_mut(&mut tree, &path) {
                            match node {
                                TreeNode::Repo { selected, .. } => {
                                    *selected = !*selected;
                                }
                                TreeNode::Folder { children, .. } => {
                                    toggle_folder(children);
                                }
                            }
                        }
                    }
                    KeyCode::Enter => {
                        let path = rows[cursor_row].tree_path.clone();
                        if let Some(TreeNode::Folder { expanded, .. }) =
                            get_node_mut(&mut tree, &path)
                        {
                            *expanded = !*expanded;
                        }
                    }
                    KeyCode::Char('a') => {
                        let path = rows[cursor_row].tree_path.clone();
                        if let Some(TreeNode::Folder { children, .. }) =
                            get_node_mut(&mut tree, &path)
                        {
                            toggle_folder(children);
                        }
                    }
                    KeyCode::Char('c') => {
                        break;
                    }
                    KeyCode::Esc => {
                        return Ok(vec![]);
                    }
                    _ => {}
                }
            }
        }

        Ok(collect_selected(&tree))
    })();

    // Always restore terminal state
    terminal::disable_raw_mode()?;
    crossterm::execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;

    result
}

// Uses std::io::IsTerminal (stable since Rust 1.70) for TTY detection.

#[cfg(test)]
mod tests {
    use super::*;

    fn make_repo(name: &str, org: &str) -> RepoEntry {
        RepoEntry {
            name: name.to_string(),
            org: org.to_string(),
            path: std::path::PathBuf::from(format!("/code/{org}/{name}")),
            remote_url: None,
        }
    }

    #[test]
    fn test_build_tree_flat() {
        let repos = vec![make_repo("repo-a", ""), make_repo("repo-b", "")];
        let tree = build_tree(&repos);
        assert_eq!(tree.len(), 2);
        assert!(matches!(&tree[0], TreeNode::Repo { name, .. } if name == "repo-a"));
    }

    #[test]
    fn test_build_tree_grouped() {
        let repos = vec![
            make_repo("dsp-api", "dasch-swiss"),
            make_repo("dsp-das", "dasch-swiss"),
            make_repo("loom", "subotic"),
        ];
        let tree = build_tree(&repos);
        assert_eq!(tree.len(), 2); // two folders
        assert!(matches!(&tree[0], TreeNode::Folder { name, children, .. }
            if name == "dasch-swiss" && children.len() == 2));
    }

    #[test]
    fn test_build_tree_folders_start_collapsed() {
        let repos = vec![
            make_repo("dsp-api", "dasch-swiss"),
            make_repo("loom", "subotic"),
        ];
        let tree = build_tree(&repos);
        for node in &tree {
            if let TreeNode::Folder { expanded, .. } = node {
                assert!(!expanded, "folders should start collapsed");
            }
        }
    }

    #[test]
    fn test_build_tree_deep() {
        let repos = vec![make_repo("dsp-api", "github.com/dasch-swiss")];
        let tree = build_tree(&repos);
        // github.com folder > dasch-swiss folder > dsp-api repo
        assert_eq!(tree.len(), 1);
        if let TreeNode::Folder { name, children, .. } = &tree[0] {
            assert_eq!(name, "github.com");
            assert_eq!(children.len(), 1);
            if let TreeNode::Folder { name, children, .. } = &children[0] {
                assert_eq!(name, "dasch-swiss");
                assert_eq!(children.len(), 1);
                assert!(matches!(&children[0], TreeNode::Repo { name, .. } if name == "dsp-api"));
            }
        }
    }

    #[test]
    fn test_collect_selected() {
        let tree = vec![
            TreeNode::Repo {
                index: 0,
                name: "a".to_string(),
                selected: true,
            },
            TreeNode::Repo {
                index: 1,
                name: "b".to_string(),
                selected: false,
            },
            TreeNode::Folder {
                name: "org".to_string(),
                expanded: false,
                children: vec![TreeNode::Repo {
                    index: 2,
                    name: "c".to_string(),
                    selected: true,
                }],
            },
        ];
        let selected = collect_selected(&tree);
        assert_eq!(selected, vec![0, 2]);
    }
}
