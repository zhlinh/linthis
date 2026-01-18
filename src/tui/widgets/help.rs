// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Help overlay widget for the TUI.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Draw the help overlay
pub fn draw(frame: &mut Frame) {
    // Center the popup
    let area = centered_rect(60, 70, frame.area());

    // Clear the background
    frame.render_widget(Clear, area);

    // Help content
    let help_lines = vec![
        Line::from(vec![Span::styled(
            "Keyboard Shortcuts",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Navigation", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  ↑/k     ", Style::default().fg(Color::Yellow)),
            Span::raw("Move up"),
        ]),
        Line::from(vec![
            Span::styled("  ↓/j     ", Style::default().fg(Color::Yellow)),
            Span::raw("Move down"),
        ]),
        Line::from(vec![
            Span::styled("  PgUp    ", Style::default().fg(Color::Yellow)),
            Span::raw("Page up"),
        ]),
        Line::from(vec![
            Span::styled("  PgDn    ", Style::default().fg(Color::Yellow)),
            Span::raw("Page down"),
        ]),
        Line::from(vec![
            Span::styled("  Home/g  ", Style::default().fg(Color::Yellow)),
            Span::raw("Go to start"),
        ]),
        Line::from(vec![
            Span::styled("  End/G   ", Style::default().fg(Color::Yellow)),
            Span::raw("Go to end"),
        ]),
        Line::from(vec![
            Span::styled("  Tab     ", Style::default().fg(Color::Yellow)),
            Span::raw("Switch panel focus"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Actions", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Enter   ", Style::default().fg(Color::Yellow)),
            Span::raw("Open file in editor"),
        ]),
        Line::from(vec![
            Span::styled("  r       ", Style::default().fg(Color::Yellow)),
            Span::raw("Force re-run linting"),
        ]),
        Line::from(vec![
            Span::styled("  c       ", Style::default().fg(Color::Yellow)),
            Span::raw("Clear all results"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("General", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  ?       ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle this help"),
        ]),
        Line::from(vec![
            Span::styled("  q       ", Style::default().fg(Color::Yellow)),
            Span::raw("Quit"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+C  ", Style::default().fg(Color::Yellow)),
            Span::raw("Quit"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press any key to close",
            Style::default().fg(Color::DarkGray),
        )]),
    ];

    let help = Paragraph::new(help_lines)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .style(Style::default().bg(Color::Rgb(20, 20, 30))),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(help, area);
}

/// Helper to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_centered_rect() {
        let area = Rect::new(0, 0, 100, 50);
        let centered = centered_rect(50, 50, area);

        // Should be roughly centered
        assert!(centered.x > 0);
        assert!(centered.y > 0);
        assert!(centered.width < 100);
        assert!(centered.height < 50);
    }
}
