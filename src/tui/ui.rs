// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! UI rendering for the TUI.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

use super::app::{App, AppState};
use super::widgets::{file_tree, help, issue_list, status_bar};

/// Draw the entire TUI
pub fn draw(frame: &mut Frame, app: &App) {
    // Main layout: header, content, status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Status bar
        ])
        .split(frame.area());

    // Draw header
    draw_header(frame, app, chunks[0]);

    // Draw content (file tree + issues)
    draw_content(frame, app, chunks[1]);

    // Draw status bar
    status_bar::draw(frame, app, chunks[2]);

    // Draw help overlay if active
    if app.state == AppState::ShowingHelp {
        help::draw(frame);
    }
}

/// Draw the header bar
fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let watched = if app.watched_paths.len() == 1 {
        app.watched_paths[0].display().to_string()
    } else {
        format!("{} paths", app.watched_paths.len())
    };

    let file_count = app.watch_state.file_count();
    let header_text = format!(
        " linthis watch - {}  │  Watching: {} files",
        watched, file_count
    );

    let header = Paragraph::new(header_text)
        .style(Style::default().bg(Color::Blue).fg(Color::White));

    frame.render_widget(header, area);
}

/// Draw the main content area
fn draw_content(frame: &mut Frame, app: &App, area: Rect) {
    // Split into file tree (left) and issues (right)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // File tree
            Constraint::Percentage(70), // Issues
        ])
        .split(area);

    // Draw file tree
    file_tree::draw(frame, app, chunks[0]);

    // Draw issues list
    issue_list::draw(frame, app, chunks[1]);
}

/// Get style for focused/unfocused borders
pub fn border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

/// Format a file path for display
#[allow(dead_code)]
pub fn format_path(path: &std::path::Path, max_len: usize) -> String {
    let s = path.display().to_string();
    if s.len() <= max_len {
        s
    } else {
        // Truncate from the start with ellipsis
        format!("...{}", &s[s.len() - max_len + 3..])
    }
}

/// Get color for severity
pub fn severity_color(severity: crate::Severity) -> Color {
    match severity {
        crate::Severity::Error => Color::Red,
        crate::Severity::Warning => Color::Yellow,
        crate::Severity::Info => Color::Cyan,
    }
}

/// Get symbol for severity
pub fn severity_symbol(severity: crate::Severity) -> &'static str {
    match severity {
        crate::Severity::Error => "E",
        crate::Severity::Warning => "W",
        crate::Severity::Info => "I",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_path_short() {
        let path = std::path::Path::new("src/main.rs");
        assert_eq!(format_path(path, 50), "src/main.rs");
    }

    #[test]
    fn test_format_path_long() {
        let path = std::path::Path::new("very/long/path/to/some/file/main.rs");
        let formatted = format_path(path, 20);
        assert!(formatted.starts_with("..."));
        assert!(formatted.len() <= 20);
    }

    #[test]
    fn test_severity_color() {
        assert_eq!(severity_color(crate::Severity::Error), Color::Red);
        assert_eq!(severity_color(crate::Severity::Warning), Color::Yellow);
        assert_eq!(severity_color(crate::Severity::Info), Color::Cyan);
    }
}
