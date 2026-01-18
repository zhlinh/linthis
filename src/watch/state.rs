// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Watch state management for tracking lint results and file statuses.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use crate::utils::types::{LintIssue, RunResult};
use crate::Language;

/// Status of a watched file
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    /// File has not been checked yet
    Pending,
    /// File is currently being checked
    Checking,
    /// File is clean (no issues)
    Clean,
    /// File has issues
    HasIssues(usize),
    /// File was formatted
    Formatted,
    /// File check resulted in an error
    Error,
}

impl FileStatus {
    /// Get a display symbol for the status
    pub fn symbol(&self) -> &'static str {
        match self {
            FileStatus::Pending => "○",
            FileStatus::Checking => "◐",
            FileStatus::Clean => "✓",
            FileStatus::HasIssues(_) => "✗",
            FileStatus::Formatted => "✎",
            FileStatus::Error => "⚠",
        }
    }

    /// Get a colored display symbol
    pub fn colored_symbol(&self) -> String {
        match self {
            FileStatus::Pending => "\x1b[90m○\x1b[0m".to_string(),
            FileStatus::Checking => "\x1b[33m◐\x1b[0m".to_string(),
            FileStatus::Clean => "\x1b[32m✓\x1b[0m".to_string(),
            FileStatus::HasIssues(_) => "\x1b[31m✗\x1b[0m".to_string(),
            FileStatus::Formatted => "\x1b[34m✎\x1b[0m".to_string(),
            FileStatus::Error => "\x1b[33m⚠\x1b[0m".to_string(),
        }
    }
}

/// Information about a watched file
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// File path
    pub path: PathBuf,
    /// Detected language
    pub language: Option<Language>,
    /// Current status
    pub status: FileStatus,
    /// Issues for this file
    pub issues: Vec<LintIssue>,
    /// Last check time
    pub last_checked: Option<Instant>,
}

impl FileInfo {
    /// Create a new FileInfo
    pub fn new(path: PathBuf) -> Self {
        let language = Language::from_path(&path);
        Self {
            path,
            language,
            status: FileStatus::Pending,
            issues: Vec::new(),
            last_checked: None,
        }
    }

    /// Update file info from lint results
    pub fn update_from_issues(&mut self, issues: Vec<LintIssue>) {
        self.issues = issues;
        self.status = if self.issues.is_empty() {
            FileStatus::Clean
        } else {
            FileStatus::HasIssues(self.issues.len())
        };
        self.last_checked = Some(Instant::now());
    }

    /// Mark as checking
    pub fn mark_checking(&mut self) {
        self.status = FileStatus::Checking;
    }

    /// Mark as formatted
    pub fn mark_formatted(&mut self) {
        self.status = FileStatus::Formatted;
    }

    /// Mark as error
    pub fn mark_error(&mut self) {
        self.status = FileStatus::Error;
        self.last_checked = Some(Instant::now());
    }
}

/// Watch state tracking all files and issues
#[derive(Debug)]
pub struct WatchState {
    /// Files being watched (path -> info)
    files: HashMap<PathBuf, FileInfo>,
    /// All current issues
    issues: Vec<LintIssue>,
    /// Recently changed files (for display)
    recent_changes: Vec<PathBuf>,
    /// Maximum recent changes to track
    max_recent: usize,
    /// Last update time
    last_update: Instant,
    /// Total lint runs
    total_runs: usize,
    /// Current status message
    status_message: String,
    /// Whether currently running
    is_running: bool,
}

impl Default for WatchState {
    fn default() -> Self {
        Self::new()
    }
}

impl WatchState {
    /// Create a new watch state
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            issues: Vec::new(),
            recent_changes: Vec::new(),
            max_recent: 5,
            last_update: Instant::now(),
            total_runs: 0,
            status_message: "Initializing...".to_string(),
            is_running: false,
        }
    }

    /// Update state from a RunResult
    pub fn update_from_result(&mut self, result: &RunResult) {
        self.last_update = Instant::now();
        self.total_runs += 1;
        self.is_running = false;

        // Clear previous issues
        self.issues.clear();

        // Group issues by file
        let mut issues_by_file: HashMap<PathBuf, Vec<LintIssue>> = HashMap::new();
        for issue in &result.issues {
            issues_by_file
                .entry(issue.file_path.clone())
                .or_default()
                .push(issue.clone());
        }

        // Update file statuses
        for file_info in self.files.values_mut() {
            if let Some(file_issues) = issues_by_file.remove(&file_info.path) {
                file_info.update_from_issues(file_issues);
            } else {
                // No issues for this file
                file_info.update_from_issues(Vec::new());
            }
        }

        // Collect all issues
        for issue in &result.issues {
            self.issues.push(issue.clone());
        }

        // Update status message
        self.status_message = if self.issues.is_empty() {
            "✓ Clean".to_string()
        } else {
            let errors = self.error_count();
            let warnings = self.warning_count();
            format!("{} errors, {} warnings", errors, warnings)
        };
    }

    /// Mark files as being checked
    pub fn mark_checking(&mut self, paths: &[PathBuf]) {
        self.is_running = true;
        self.status_message = "Checking...".to_string();

        for path in paths {
            if let Some(info) = self.files.get_mut(path) {
                info.mark_checking();
            } else {
                let mut info = FileInfo::new(path.clone());
                info.mark_checking();
                self.files.insert(path.clone(), info);
            }

            // Add to recent changes
            self.add_recent_change(path.clone());
        }
    }

    /// Add a file to recent changes
    fn add_recent_change(&mut self, path: PathBuf) {
        // Remove if already in list
        self.recent_changes.retain(|p| p != &path);

        // Add to front
        self.recent_changes.insert(0, path);

        // Trim to max size
        self.recent_changes.truncate(self.max_recent);
    }

    /// Get recent changes
    pub fn recent_changes(&self) -> &[PathBuf] {
        &self.recent_changes
    }

    /// Get all issues
    pub fn issues(&self) -> &[LintIssue] {
        &self.issues
    }

    /// Get issues for a specific file
    pub fn issues_for_file(&self, path: &PathBuf) -> Vec<&LintIssue> {
        self.issues.iter().filter(|i| &i.file_path == path).collect()
    }

    /// Get file info
    pub fn get_file(&self, path: &PathBuf) -> Option<&FileInfo> {
        self.files.get(path)
    }

    /// Get all files
    pub fn files(&self) -> impl Iterator<Item = &FileInfo> {
        self.files.values()
    }

    /// Get file count
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == crate::Severity::Error)
            .count()
    }

    /// Get warning count
    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == crate::Severity::Warning)
            .count()
    }

    /// Get info count
    pub fn info_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == crate::Severity::Info)
            .count()
    }

    /// Get status message
    pub fn status_message(&self) -> &str {
        &self.status_message
    }

    /// Check if currently running
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Get total runs
    pub fn total_runs(&self) -> usize {
        self.total_runs
    }

    /// Check if clean (no issues)
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }

    /// Get time since last update
    pub fn time_since_update(&self) -> std::time::Duration {
        self.last_update.elapsed()
    }

    /// Set status message
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = message.into();
    }

    /// Clear all state
    pub fn clear(&mut self) {
        self.files.clear();
        self.issues.clear();
        self.recent_changes.clear();
        self.status_message = "Cleared".to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Severity;

    #[test]
    fn test_file_status_symbols() {
        assert_eq!(FileStatus::Clean.symbol(), "✓");
        assert_eq!(FileStatus::HasIssues(5).symbol(), "✗");
        assert_eq!(FileStatus::Pending.symbol(), "○");
    }

    #[test]
    fn test_file_info_creation() {
        let info = FileInfo::new(PathBuf::from("test.rs"));
        assert_eq!(info.language, Some(Language::Rust));
        assert_eq!(info.status, FileStatus::Pending);
        assert!(info.issues.is_empty());
    }

    #[test]
    fn test_file_info_update() {
        let mut info = FileInfo::new(PathBuf::from("test.rs"));

        // Update with no issues
        info.update_from_issues(Vec::new());
        assert_eq!(info.status, FileStatus::Clean);

        // Update with issues
        let issue = LintIssue {
            file_path: PathBuf::from("test.rs"),
            line: 1,
            column: Some(1),
            message: "test".to_string(),
            code: Some("E001".to_string()),
            severity: Severity::Error,
            source: Some("test".to_string()),
            suggestion: None,
            language: Some(Language::Rust),
            code_line: None,
            context_before: Vec::new(),
            context_after: Vec::new(),
        };
        info.update_from_issues(vec![issue]);
        assert_eq!(info.status, FileStatus::HasIssues(1));
    }

    #[test]
    fn test_watch_state_recent_changes() {
        let mut state = WatchState::new();

        state.add_recent_change(PathBuf::from("a.rs"));
        state.add_recent_change(PathBuf::from("b.rs"));
        state.add_recent_change(PathBuf::from("c.rs"));

        assert_eq!(state.recent_changes.len(), 3);
        assert_eq!(state.recent_changes[0], PathBuf::from("c.rs")); // Most recent first

        // Adding same file should move it to front
        state.add_recent_change(PathBuf::from("a.rs"));
        assert_eq!(state.recent_changes[0], PathBuf::from("a.rs"));
        assert_eq!(state.recent_changes.len(), 3);
    }

    #[test]
    fn test_watch_state_counts() {
        let mut state = WatchState::new();

        let issues = vec![
            LintIssue {
                file_path: PathBuf::from("test.rs"),
                line: 1,
                column: None,
                message: "error".to_string(),
                code: Some("E001".to_string()),
                severity: Severity::Error,
                source: Some("test".to_string()),
                suggestion: None,
                language: Some(Language::Rust),
                code_line: None,
                context_before: Vec::new(),
                context_after: Vec::new(),
            },
            LintIssue {
                file_path: PathBuf::from("test.rs"),
                line: 2,
                column: None,
                message: "warning".to_string(),
                code: Some("W001".to_string()),
                severity: Severity::Warning,
                source: Some("test".to_string()),
                suggestion: None,
                language: Some(Language::Rust),
                code_line: None,
                context_before: Vec::new(),
                context_after: Vec::new(),
            },
        ];

        for issue in issues {
            state.issues.push(issue);
        }

        assert_eq!(state.error_count(), 1);
        assert_eq!(state.warning_count(), 1);
        assert!(!state.is_clean());
    }
}
