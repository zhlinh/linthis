// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! File tree widget for the TUI.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::tui::app::{App, FocusedPanel};
use crate::tui::ui::border_style;
use crate::watch::FileStatus;

/// Draw the file tree widget
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_panel == FocusedPanel::Files;

    // Get recent changes
    let recent = app.watch_state.recent_changes();

    // Create list items
    let mut items: Vec<ListItem> = Vec::new();

    // Section: Recently changed
    if !recent.is_empty() {
        items.push(ListItem::new(Line::from(vec![Span::styled(
            "Recently Changed:",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )])));

        for (i, path) in recent.iter().enumerate() {
            let is_selected = focused && i == app.file_index;

            // Get file status
            let status = app
                .watch_state
                .get_file(path)
                .map(|f| &f.status)
                .unwrap_or(&FileStatus::Pending);

            let (symbol, color) = status_display(status);

            let path_str = app.display_path(path);

            let item_style = if is_selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::styled(format!("  {} ", symbol), Style::default().fg(color)),
                Span::styled(
                    truncate_path(&path_str, 20),
                    Style::default().fg(if is_selected {
                        Color::White
                    } else {
                        Color::Gray
                    }),
                ),
            ]);

            items.push(ListItem::new(line).style(item_style));
        }

        // Spacer
        items.push(ListItem::new(Line::from("")));
    }

    // Section: All files summary
    let file_count = app.watch_state.file_count();
    let clean_count = app
        .watch_state
        .files()
        .filter(|f| f.status == FileStatus::Clean)
        .count();
    let issue_count = app
        .watch_state
        .files()
        .filter(|f| matches!(f.status, FileStatus::HasIssues(_)))
        .count();

    items.push(ListItem::new(Line::from(vec![Span::styled(
        "Summary:",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )])));

    items.push(ListItem::new(Line::from(vec![
        Span::raw("  Total: "),
        Span::styled(format!("{}", file_count), Style::default().fg(Color::White)),
    ])));

    items.push(ListItem::new(Line::from(vec![
        Span::styled("  ✓ ", Style::default().fg(Color::Green)),
        Span::styled(format!("{} clean", clean_count), Style::default().fg(Color::Gray)),
    ])));

    if issue_count > 0 {
        items.push(ListItem::new(Line::from(vec![
            Span::styled("  ✗ ", Style::default().fg(Color::Red)),
            Span::styled(
                format!("{} with issues", issue_count),
                Style::default().fg(Color::Gray),
            ),
        ])));
    }

    // Block with title
    let block = Block::default()
        .title(" Files ")
        .borders(Borders::ALL)
        .border_style(border_style(focused));

    let list = List::new(items).block(block);

    // Render with state
    let mut state = ListState::default();
    if focused && !recent.is_empty() {
        // Offset by 1 for header
        state.select(Some(app.file_index + 1));
    }

    frame.render_stateful_widget(list, area, &mut state);
}

/// Get display symbol and color for file status
fn status_display(status: &FileStatus) -> (&'static str, Color) {
    match status {
        FileStatus::Pending => ("○", Color::DarkGray),
        FileStatus::Checking => ("◐", Color::Yellow),
        FileStatus::Clean => ("✓", Color::Green),
        FileStatus::HasIssues(_) => ("✗", Color::Red),
        FileStatus::Formatted => ("✎", Color::Blue),
        FileStatus::Error => ("⚠", Color::Yellow),
    }
}

/// Truncate path to fit in given width
fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        format!("...{}", &path[path.len() - max_len + 3..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_display() {
        let (sym, _) = status_display(&FileStatus::Clean);
        assert_eq!(sym, "✓");

        let (sym, _) = status_display(&FileStatus::HasIssues(3));
        assert_eq!(sym, "✗");
    }

    #[test]
    fn test_truncate_path() {
        assert_eq!(truncate_path("short.rs", 20), "short.rs");
        assert_eq!(truncate_path("very/long/path/file.rs", 15), "...path/file.rs");
    }
}
