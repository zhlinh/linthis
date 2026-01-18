// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Event handling for the TUI.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, AppState};

/// Application events
#[derive(Debug, Clone)]
pub enum Event {
    /// Keyboard input
    Key(KeyEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// Tick (for periodic updates)
    Tick,
    /// Lint results ready
    LintComplete,
    /// Error occurred
    Error(String),
}

/// Event handler that runs in a separate thread
pub struct EventHandler {
    /// Receiver for events
    rx: Receiver<Event>,
    /// Sender for events (to allow external events)
    tx: Sender<Event>,
}

impl EventHandler {
    /// Create a new event handler
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel();

        let event_tx = tx.clone();
        thread::spawn(move || {
            loop {
                // Poll for events with timeout
                if event::poll(tick_rate).unwrap_or(false) {
                    if let Ok(evt) = event::read() {
                        let event = match evt {
                            CrosstermEvent::Key(key) => Event::Key(key),
                            CrosstermEvent::Resize(w, h) => Event::Resize(w, h),
                            _ => continue,
                        };
                        if event_tx.send(event).is_err() {
                            break;
                        }
                    }
                } else {
                    // Send tick on timeout
                    if event_tx.send(Event::Tick).is_err() {
                        break;
                    }
                }
            }
        });

        Self { rx, tx }
    }

    /// Get the next event (blocking)
    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }

    /// Try to get the next event (non-blocking)
    pub fn try_next(&self) -> Option<Event> {
        self.rx.try_recv().ok()
    }

    /// Get sender for sending custom events
    pub fn sender(&self) -> Sender<Event> {
        self.tx.clone()
    }
}

/// Handle a key event and update app state
pub fn handle_key_event(app: &mut App, key: KeyEvent) {
    // Check for quit first (always available)
    if key.code == KeyCode::Char('q')
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
    {
        app.quit();
        return;
    }

    // Handle based on current state
    match app.state {
        AppState::ShowingHelp => {
            // Any key closes help
            app.toggle_help();
        }
        AppState::Running => {
            handle_running_key(app, key);
        }
        AppState::Quitting => {}
    }
}

/// Handle key events in running state
fn handle_running_key(app: &mut App, key: KeyEvent) {
    match key.code {
        // Navigation
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),

        // Page navigation
        KeyCode::PageUp => {
            for _ in 0..10 {
                app.move_up();
            }
        }
        KeyCode::PageDown => {
            for _ in 0..10 {
                app.move_down();
            }
        }

        // Home/End
        KeyCode::Home | KeyCode::Char('g') => {
            app.issue_index = 0;
            app.issue_scroll = 0;
        }
        KeyCode::End | KeyCode::Char('G') => {
            let count = app.watch_state.issues().len();
            if count > 0 {
                app.issue_index = count - 1;
            }
        }

        // Panel focus
        KeyCode::Tab => app.switch_focus(),

        // Actions
        KeyCode::Enter => {
            // Open in editor
            if app.open_in_editor().is_some() {
                app.set_status("Opened in editor");
            }
        }
        KeyCode::Char('r') => app.request_rerun(),
        KeyCode::Char('c') => app.clear(),
        KeyCode::Char('?') => app.toggle_help(),

        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watch::WatchConfig;

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn make_key_with_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn test_quit_with_q() {
        let config = WatchConfig::default();
        let mut app = App::new(config);

        handle_key_event(&mut app, make_key(KeyCode::Char('q')));
        assert_eq!(app.state, AppState::Quitting);
    }

    #[test]
    fn test_quit_with_ctrl_c() {
        let config = WatchConfig::default();
        let mut app = App::new(config);

        handle_key_event(
            &mut app,
            make_key_with_mod(KeyCode::Char('c'), KeyModifiers::CONTROL),
        );
        assert_eq!(app.state, AppState::Quitting);
    }

    #[test]
    fn test_help_toggle() {
        let config = WatchConfig::default();
        let mut app = App::new(config);

        handle_key_event(&mut app, make_key(KeyCode::Char('?')));
        assert_eq!(app.state, AppState::ShowingHelp);

        // Any key closes help
        handle_key_event(&mut app, make_key(KeyCode::Char('a')));
        assert_eq!(app.state, AppState::Running);
    }

    #[test]
    fn test_navigation_keys() {
        let config = WatchConfig::default();
        let mut app = App::new(config);

        // Add issues for navigation
        let mut result = crate::utils::types::RunResult::new();
        for i in 0..5 {
            result.issues.push(crate::utils::types::LintIssue {
                file_path: std::path::PathBuf::from("test.rs"),
                line: i + 1,
                column: None,
                message: format!("Issue {}", i),
                code: Some(format!("E00{}", i)),
                severity: crate::Severity::Error,
                source: Some("test".to_string()),
                suggestion: None,
                language: None,
                code_line: None,
                context_before: Vec::new(),
                context_after: Vec::new(),
            });
        }
        app.update_results(&result);

        // Test j/k navigation
        handle_key_event(&mut app, make_key(KeyCode::Char('j')));
        assert_eq!(app.issue_index, 1);

        handle_key_event(&mut app, make_key(KeyCode::Char('k')));
        assert_eq!(app.issue_index, 0);

        // Test arrow keys
        handle_key_event(&mut app, make_key(KeyCode::Down));
        assert_eq!(app.issue_index, 1);

        handle_key_event(&mut app, make_key(KeyCode::Up));
        assert_eq!(app.issue_index, 0);
    }

    #[test]
    fn test_tab_switches_focus() {
        let config = WatchConfig::default();
        let mut app = App::new(config);

        use super::super::app::FocusedPanel;

        assert_eq!(app.focused_panel, FocusedPanel::Issues);

        handle_key_event(&mut app, make_key(KeyCode::Tab));
        assert_eq!(app.focused_panel, FocusedPanel::Files);

        handle_key_event(&mut app, make_key(KeyCode::Tab));
        assert_eq!(app.focused_panel, FocusedPanel::Issues);
    }

    #[test]
    fn test_rerun_request() {
        let config = WatchConfig::default();
        let mut app = App::new(config);

        handle_key_event(&mut app, make_key(KeyCode::Char('r')));
        assert!(app.force_rerun);
    }
}
