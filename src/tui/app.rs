// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! TUI application state and logic.

use std::path::PathBuf;

use crate::utils::types::{LintIssue, RunResult};
use crate::watch::{WatchConfig, WatchState};
use crate::Severity;

/// Application state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    /// Running normally
    Running,
    /// Help overlay is shown
    ShowingHelp,
    /// Quitting
    Quitting,
}

/// Which panel is focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPanel {
    /// File tree panel
    Files,
    /// Issues list panel
    Issues,
}

/// TUI Application
pub struct App {
    /// Current application state
    pub state: AppState,
    /// Watch configuration
    pub config: WatchConfig,
    /// Watch state (lint results)
    pub watch_state: WatchState,
    /// Currently focused panel
    pub focused_panel: FocusedPanel,
    /// Selected index in issues list
    pub issue_index: usize,
    /// Selected index in file tree
    pub file_index: usize,
    /// Scroll offset for issues
    pub issue_scroll: usize,
    /// Scroll offset for files
    pub file_scroll: usize,
    /// Whether a force re-run was requested
    pub force_rerun: bool,
    /// Status message to display
    pub status_message: Option<String>,
    /// Paths being watched
    pub watched_paths: Vec<PathBuf>,
}

impl App {
    /// Create a new App
    pub fn new(config: WatchConfig) -> Self {
        let watched_paths = config.paths.clone();
        Self {
            state: AppState::Running,
            config,
            watch_state: WatchState::new(),
            focused_panel: FocusedPanel::Issues,
            issue_index: 0,
            file_index: 0,
            issue_scroll: 0,
            file_scroll: 0,
            force_rerun: false,
            status_message: None,
            watched_paths,
        }
    }

    /// Check if app is running
    pub fn is_running(&self) -> bool {
        self.state != AppState::Quitting
    }

    /// Quit the application
    pub fn quit(&mut self) {
        self.state = AppState::Quitting;
    }

    /// Toggle help overlay
    pub fn toggle_help(&mut self) {
        self.state = match self.state {
            AppState::ShowingHelp => AppState::Running,
            _ => AppState::ShowingHelp,
        };
    }

    /// Switch focus between panels
    pub fn switch_focus(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Files => FocusedPanel::Issues,
            FocusedPanel::Issues => FocusedPanel::Files,
        };
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        match self.focused_panel {
            FocusedPanel::Issues => {
                if self.issue_index > 0 {
                    self.issue_index -= 1;
                    self.adjust_issue_scroll();
                }
            }
            FocusedPanel::Files => {
                if self.file_index > 0 {
                    self.file_index -= 1;
                    self.adjust_file_scroll();
                }
            }
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        match self.focused_panel {
            FocusedPanel::Issues => {
                let max = self.watch_state.issues().len().saturating_sub(1);
                if self.issue_index < max {
                    self.issue_index += 1;
                    self.adjust_issue_scroll();
                }
            }
            FocusedPanel::Files => {
                let max = self.watch_state.file_count().saturating_sub(1);
                if self.file_index < max {
                    self.file_index += 1;
                    self.adjust_file_scroll();
                }
            }
        }
    }

    /// Adjust scroll to keep selection visible (issues)
    fn adjust_issue_scroll(&mut self) {
        // Assume visible height of ~20 lines
        const VISIBLE: usize = 20;
        if self.issue_index < self.issue_scroll {
            self.issue_scroll = self.issue_index;
        } else if self.issue_index >= self.issue_scroll + VISIBLE {
            self.issue_scroll = self.issue_index - VISIBLE + 1;
        }
    }

    /// Adjust scroll to keep selection visible (files)
    fn adjust_file_scroll(&mut self) {
        const VISIBLE: usize = 15;
        if self.file_index < self.file_scroll {
            self.file_scroll = self.file_index;
        } else if self.file_index >= self.file_scroll + VISIBLE {
            self.file_scroll = self.file_index - VISIBLE + 1;
        }
    }

    /// Request a force re-run
    pub fn request_rerun(&mut self) {
        self.force_rerun = true;
        self.set_status("Re-running...");
    }

    /// Check and clear force rerun flag
    pub fn take_force_rerun(&mut self) -> bool {
        let val = self.force_rerun;
        self.force_rerun = false;
        val
    }

    /// Clear all results
    pub fn clear(&mut self) {
        self.watch_state.clear();
        self.issue_index = 0;
        self.file_index = 0;
        self.issue_scroll = 0;
        self.file_scroll = 0;
        self.set_status("Cleared");
    }

    /// Set status message
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
    }

    /// Clear status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Update from lint results
    pub fn update_results(&mut self, result: &RunResult) {
        self.watch_state.update_from_result(result);

        // Reset selection if out of bounds
        let issue_count = self.watch_state.issues().len();
        if self.issue_index >= issue_count && issue_count > 0 {
            self.issue_index = issue_count - 1;
        }

        // Update status
        if self.watch_state.is_clean() {
            self.set_status("✓ All clear!");
        } else {
            let errors = self.watch_state.error_count();
            let warnings = self.watch_state.warning_count();
            self.set_status(format!("{} errors, {} warnings", errors, warnings));
        }
    }

    /// Get currently selected issue
    pub fn selected_issue(&self) -> Option<&LintIssue> {
        self.watch_state.issues().get(self.issue_index)
    }

    /// Get file path and line for opening in editor
    pub fn selected_location(&self) -> Option<(PathBuf, usize)> {
        self.selected_issue()
            .map(|issue| (issue.file_path.clone(), issue.line))
    }

    /// Open selected file in editor
    pub fn open_in_editor(&self) -> Option<std::process::Output> {
        let (path, line) = self.selected_location()?;

        // Try common editors in order of preference
        let editors = [
            ("code", vec!["-g".to_string(), format!("{}:{}", path.display(), line)]),
            ("cursor", vec!["-g".to_string(), format!("{}:{}", path.display(), line)]),
            ("vim", vec![format!("+{}", line), path.display().to_string()]),
            ("nvim", vec![format!("+{}", line), path.display().to_string()]),
            ("nano", vec![format!("+{}", line), path.display().to_string()]),
        ];

        for (editor, args) in editors {
            if let Ok(output) = std::process::Command::new(editor).args(&args).output() {
                return Some(output);
            }
        }

        None
    }

    /// Get issues grouped by severity for display
    pub fn issues_by_severity(&self) -> (Vec<&LintIssue>, Vec<&LintIssue>, Vec<&LintIssue>) {
        let issues = self.watch_state.issues();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut infos = Vec::new();

        for issue in issues {
            match issue.severity {
                Severity::Error => errors.push(issue),
                Severity::Warning => warnings.push(issue),
                Severity::Info => infos.push(issue),
            }
        }

        (errors, warnings, infos)
    }

    /// Get display path (relative to watched root if possible)
    pub fn display_path(&self, path: &std::path::Path) -> String {
        // Try to make path relative to first watched path
        if let Some(root) = self.watched_paths.first() {
            if let Ok(rel) = path.strip_prefix(root) {
                return rel.display().to_string();
            }
        }

        // Fall back to file name
        path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_transitions() {
        let config = WatchConfig::default();
        let mut app = App::new(config);

        assert_eq!(app.state, AppState::Running);
        assert!(app.is_running());

        app.toggle_help();
        assert_eq!(app.state, AppState::ShowingHelp);

        app.toggle_help();
        assert_eq!(app.state, AppState::Running);

        app.quit();
        assert_eq!(app.state, AppState::Quitting);
        assert!(!app.is_running());
    }

    #[test]
    fn test_focus_switching() {
        let config = WatchConfig::default();
        let mut app = App::new(config);

        assert_eq!(app.focused_panel, FocusedPanel::Issues);

        app.switch_focus();
        assert_eq!(app.focused_panel, FocusedPanel::Files);

        app.switch_focus();
        assert_eq!(app.focused_panel, FocusedPanel::Issues);
    }

    #[test]
    fn test_navigation() {
        let config = WatchConfig::default();
        let mut app = App::new(config);

        // Add some mock issues
        let mut result = RunResult::new();
        result.issues.push(LintIssue {
            file_path: PathBuf::from("test.rs"),
            line: 1,
            column: None,
            message: "Error 1".to_string(),
            code: Some("E001".to_string()),
            severity: Severity::Error,
            source: Some("test".to_string()),
            suggestion: None,
            language: None,
            code_line: None,
            context_before: Vec::new(),
            context_after: Vec::new(),
        });
        result.issues.push(LintIssue {
            file_path: PathBuf::from("test.rs"),
            line: 2,
            column: None,
            message: "Error 2".to_string(),
            code: Some("E002".to_string()),
            severity: Severity::Error,
            source: Some("test".to_string()),
            suggestion: None,
            language: None,
            code_line: None,
            context_before: Vec::new(),
            context_after: Vec::new(),
        });
        app.update_results(&result);

        assert_eq!(app.issue_index, 0);

        app.move_down();
        assert_eq!(app.issue_index, 1);

        app.move_down(); // Should not go past end
        assert_eq!(app.issue_index, 1);

        app.move_up();
        assert_eq!(app.issue_index, 0);

        app.move_up(); // Should not go negative
        assert_eq!(app.issue_index, 0);
    }

    #[test]
    fn test_force_rerun() {
        let config = WatchConfig::default();
        let mut app = App::new(config);

        assert!(!app.force_rerun);

        app.request_rerun();
        assert!(app.force_rerun);

        assert!(app.take_force_rerun());
        assert!(!app.force_rerun);
    }
}
