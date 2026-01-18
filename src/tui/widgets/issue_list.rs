// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Issue list widget for the TUI.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::tui::app::{App, FocusedPanel};
use crate::tui::ui::{border_style, severity_color, severity_symbol};

/// Draw the issue list widget
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_panel == FocusedPanel::Issues;

    let issues = app.watch_state.issues();

    // Create list items
    let items: Vec<ListItem> = issues
        .iter()
        .enumerate()
        .map(|(i, issue)| {
            let is_selected = i == app.issue_index;

            // Severity indicator
            let severity_style = Style::default()
                .fg(severity_color(issue.severity))
                .add_modifier(Modifier::BOLD);
            let severity = Span::styled(
                format!(" {} ", severity_symbol(issue.severity)),
                severity_style,
            );

            // File location
            let location = format!(
                "{}:{}",
                app.display_path(&issue.file_path),
                issue.line
            );
            let location_style = if is_selected && focused {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let location_span = Span::styled(location, location_style);

            // First line: severity + location
            let line1 = Line::from(vec![severity, location_span]);

            // Second line: message (indented)
            let msg_style = if is_selected && focused {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };

            // Truncate message if too long
            let msg = if issue.message.len() > 60 {
                format!("{}...", &issue.message[..57])
            } else {
                issue.message.clone()
            };
            let line2 = Line::from(vec![
                Span::raw("    "),
                Span::styled(msg, msg_style),
            ]);

            // Rule code if present
            let lines = if let Some(ref code) = issue.code {
                let code_style = Style::default().fg(Color::DarkGray);
                let line3 = Line::from(vec![
                    Span::raw("    "),
                    Span::styled(format!("[{}]", code), code_style),
                ]);
                vec![line1, line2, line3]
            } else {
                vec![line1, line2]
            };

            let item_style = if is_selected && focused {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            ListItem::new(lines).style(item_style)
        })
        .collect();

    // Title with counts
    let error_count = app.watch_state.error_count();
    let warning_count = app.watch_state.warning_count();
    let info_count = app.watch_state.info_count();

    let title = if issues.is_empty() {
        " Issues ".to_string()
    } else {
        format!(
            " Issues ({} E, {} W, {} I) ",
            error_count, warning_count, info_count
        )
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style(focused));

    // Create list widget
    let list = if issues.is_empty() {
        // Show "No issues" message
        List::new(vec![ListItem::new(Line::from(vec![
            Span::styled("  ✓ No issues found", Style::default().fg(Color::Green)),
        ]))])
        .block(block)
    } else {
        List::new(items).block(block)
    };

    // Render with scroll state
    let mut state = ListState::default();
    if !issues.is_empty() {
        state.select(Some(app.issue_index));
    }

    frame.render_stateful_widget(list, area, &mut state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_display() {
        assert_eq!(severity_symbol(crate::Severity::Error), "E");
        assert_eq!(severity_symbol(crate::Severity::Warning), "W");
        assert_eq!(severity_symbol(crate::Severity::Info), "I");
    }
}
