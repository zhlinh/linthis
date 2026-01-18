// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Status bar widget for the TUI.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::tui::app::App;

/// Draw the status bar widget
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans = Vec::new();

    // Status indicator
    let (status_symbol, status_color) = if app.watch_state.is_running() {
        ("◐", Color::Yellow)
    } else if app.watch_state.is_clean() {
        ("✓", Color::Green)
    } else if app.watch_state.error_count() > 0 {
        ("✗", Color::Red)
    } else {
        ("!", Color::Yellow)
    };

    spans.push(Span::styled(
        format!(" {} ", status_symbol),
        Style::default().fg(status_color).add_modifier(Modifier::BOLD),
    ));

    // Status message
    let status_msg = app
        .status_message
        .as_deref()
        .unwrap_or(app.watch_state.status_message());

    spans.push(Span::styled(
        format!("{}  ", status_msg),
        Style::default().fg(Color::White),
    ));

    // Separator
    spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));

    // Issue counts
    let error_count = app.watch_state.error_count();
    let warning_count = app.watch_state.warning_count();
    let info_count = app.watch_state.info_count();

    // Errors
    spans.push(Span::styled(
        format!("  E:{}", error_count),
        Style::default().fg(if error_count > 0 {
            Color::Red
        } else {
            Color::DarkGray
        }),
    ));

    // Warnings
    spans.push(Span::styled(
        format!("  W:{}", warning_count),
        Style::default().fg(if warning_count > 0 {
            Color::Yellow
        } else {
            Color::DarkGray
        }),
    ));

    // Info
    spans.push(Span::styled(
        format!("  I:{}  ", info_count),
        Style::default().fg(if info_count > 0 {
            Color::Cyan
        } else {
            Color::DarkGray
        }),
    ));

    // Separator
    spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));

    // Run count
    let runs = app.watch_state.total_runs();
    spans.push(Span::styled(
        format!("  Runs: {}  ", runs),
        Style::default().fg(Color::DarkGray),
    ));

    // Separator
    spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));

    // Keyboard shortcuts hint
    spans.push(Span::styled(
        "  q:Quit  r:Rerun  ?:Help ",
        Style::default().fg(Color::DarkGray),
    ));

    let status = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::Rgb(30, 30, 30)));

    frame.render_widget(status, area);
}

#[cfg(test)]
mod tests {
    // Status bar is primarily visual, so tests are minimal
    // The main functionality is tested through integration tests
}
