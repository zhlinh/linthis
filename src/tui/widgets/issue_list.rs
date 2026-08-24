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
/// One issue as a list entry: severity + location, the message, and the rule
/// code when there is one.
fn issue_item(app: &App, issue: &crate::LintIssue, is_selected: bool, focused: bool) -> ListItem<'static> {
    let highlighted = is_selected && focused;

    let severity = Span::styled(
        format!(" {} ", severity_symbol(issue.severity)),
        Style::default()
            .fg(severity_color(issue.severity))
            .add_modifier(Modifier::BOLD),
    );
    let location_style = if highlighted {
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let location = Span::styled(
        format!("{}:{}", app.display_path(&issue.file_path), issue.line),
        location_style,
    );

    let msg_style = if highlighted {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::Gray)
    };
    // Keep an entry to two lines' worth of message.
    const MAX_MESSAGE: usize = 60;
    let msg = if issue.message.len() > MAX_MESSAGE {
        format!("{}...", &issue.message[..MAX_MESSAGE - 3])
    } else {
        issue.message.clone()
    };

    let mut lines = vec![
        Line::from(vec![severity, location]),
        Line::from(vec![Span::raw("    "), Span::styled(msg, msg_style)]),
    ];
    if let Some(code) = issue.code.as_ref() {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(format!("[{}]", code), Style::default().fg(Color::DarkGray)),
        ]));
    }

    let item_style = if highlighted {
        Style::default().bg(Color::DarkGray)
    } else {
        Style::default()
    };
    ListItem::new(lines).style(item_style)
}

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_panel == FocusedPanel::Issues;
    let issues = app.watch_state.issues();

    let items: Vec<ListItem> = issues
        .iter()
        .enumerate()
        .map(|(i, issue)| issue_item(app, issue, i == app.issue_index, focused))
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
        List::new(vec![ListItem::new(Line::from(vec![Span::styled(
            "  ✓ No issues found",
            Style::default().fg(Color::Green),
        )]))])
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
