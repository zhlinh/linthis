// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Core types for linthis results and configuration.

use crate::Language;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Issue severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
        }
    }
}

/// A single lint issue found in a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintIssue {
    /// Relative path to the file
    pub file_path: PathBuf,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed, optional)
    pub column: Option<usize>,
    /// Issue severity
    pub severity: Severity,
    /// Rule/error code (e.g., "E0001", "W0612")
    pub code: Option<String>,
    /// Human-readable description
    pub message: String,
    /// Optional fix suggestion
    pub suggestion: Option<String>,
    /// Which linter produced this issue
    pub source: Option<String>,
    /// Programming language of the file
    pub language: Option<Language>,
    /// The source code line where the issue occurs (optional)
    pub code_line: Option<String>,
}

impl LintIssue {
    pub fn new(file_path: PathBuf, line: usize, message: String, severity: Severity) -> Self {
        Self {
            file_path,
            line,
            column: None,
            severity,
            code: None,
            message,
            suggestion: None,
            source: None,
            language: None,
            code_line: None,
        }
    }

    pub fn with_column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    pub fn with_code(mut self, code: String) -> Self {
        self.code = Some(code);
        self
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }

    pub fn with_language(mut self, language: Language) -> Self {
        self.language = Some(language);
        self
    }

    pub fn with_code_line(mut self, code_line: String) -> Self {
        self.code_line = Some(code_line);
        self
    }
}

/// Result of formatting a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatResult {
    /// Relative path to the file
    pub file_path: PathBuf,
    /// Whether the file was modified
    pub changed: bool,
    /// Unified diff of changes (optional)
    pub diff: Option<String>,
    /// Error message if formatting failed
    pub error: Option<String>,
}

impl FormatResult {
    pub fn unchanged(file_path: PathBuf) -> Self {
        Self {
            file_path,
            changed: false,
            diff: None,
            error: None,
        }
    }

    pub fn changed(file_path: PathBuf) -> Self {
        Self {
            file_path,
            changed: true,
            diff: None,
            error: None,
        }
    }

    pub fn with_diff(mut self, diff: String) -> Self {
        self.diff = Some(diff);
        self
    }

    pub fn error(file_path: PathBuf, error: String) -> Self {
        Self {
            file_path,
            changed: false,
            diff: None,
            error: Some(error),
        }
    }
}

/// Run mode indicator for output messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunModeKind {
    #[default]
    Both,
    CheckOnly,
    FormatOnly,
}

/// Aggregated result of a linthis run
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunResult {
    /// Total number of files processed
    pub total_files: usize,
    /// Number of files with lint issues
    pub files_with_issues: usize,
    /// Number of files that were formatted
    pub files_formatted: usize,
    /// All lint issues found (after formatting)
    pub issues: Vec<LintIssue>,
    /// Issues found before formatting (for comparison)
    pub issues_before_format: usize,
    /// Issues fixed by formatting
    pub issues_fixed: usize,
    /// All format results
    pub format_results: Vec<FormatResult>,
    /// Total execution time in milliseconds
    pub duration_ms: u64,
    /// Exit code: 0 = success, 1 = issues found, 2 = error
    pub exit_code: i32,
    /// Run mode for appropriate output messages
    pub run_mode: RunModeKind,
}

impl RunResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_issue(&mut self, issue: LintIssue) {
        self.issues.push(issue);
    }

    pub fn add_format_result(&mut self, result: FormatResult) {
        if result.changed {
            self.files_formatted += 1;
        }
        self.format_results.push(result);
    }

    /// Calculate exit code based on results
    pub fn calculate_exit_code(&mut self) {
        self.calculate_exit_code_with_warnings(false);
    }

    /// Calculate exit code based on results, with option to fail on warnings
    pub fn calculate_exit_code_with_warnings(&mut self, fail_on_warnings: bool) {
        let has_errors = self.issues.iter().any(|i| i.severity == Severity::Error);
        let has_warnings = self.issues.iter().any(|i| i.severity == Severity::Warning);
        let has_format_errors = self.format_results.iter().any(|r| r.error.is_some());

        if has_format_errors {
            self.exit_code = 2;
        } else if has_errors || (fail_on_warnings && has_warnings) {
            self.exit_code = 1;
        } else {
            self.exit_code = 0;
        }
    }

    /// Count files with issues
    pub fn count_files_with_issues(&mut self) {
        use std::collections::HashSet;
        let unique_files: HashSet<_> = self.issues.iter().map(|i| &i.file_path).collect();
        self.files_with_issues = unique_files.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Severity tests ====================

    #[test]
    fn test_severity_display_error() {
        assert_eq!(format!("{}", Severity::Error), "error");
    }

    #[test]
    fn test_severity_display_warning() {
        assert_eq!(format!("{}", Severity::Warning), "warning");
    }

    #[test]
    fn test_severity_display_info() {
        assert_eq!(format!("{}", Severity::Info), "info");
    }

    #[test]
    fn test_severity_equality() {
        assert_eq!(Severity::Error, Severity::Error);
        assert_ne!(Severity::Error, Severity::Warning);
    }

    // ==================== LintIssue tests ====================

    #[test]
    fn test_lint_issue_new() {
        let issue = LintIssue::new(
            PathBuf::from("test.cpp"),
            10,
            "Test message".to_string(),
            Severity::Warning,
        );

        assert_eq!(issue.file_path, PathBuf::from("test.cpp"));
        assert_eq!(issue.line, 10);
        assert_eq!(issue.message, "Test message");
        assert_eq!(issue.severity, Severity::Warning);
        assert!(issue.column.is_none());
        assert!(issue.code.is_none());
        assert!(issue.suggestion.is_none());
        assert!(issue.source.is_none());
        assert!(issue.language.is_none());
    }

    #[test]
    fn test_lint_issue_with_column() {
        let issue = LintIssue::new(
            PathBuf::from("test.cpp"),
            10,
            "msg".to_string(),
            Severity::Error,
        )
        .with_column(5);

        assert_eq!(issue.column, Some(5));
    }

    #[test]
    fn test_lint_issue_with_code() {
        let issue = LintIssue::new(
            PathBuf::from("test.cpp"),
            10,
            "msg".to_string(),
            Severity::Warning,
        )
        .with_code("E001".to_string());

        assert_eq!(issue.code, Some("E001".to_string()));
    }

    #[test]
    fn test_lint_issue_with_suggestion() {
        let issue = LintIssue::new(
            PathBuf::from("test.cpp"),
            10,
            "msg".to_string(),
            Severity::Info,
        )
        .with_suggestion("Use nullptr instead".to_string());

        assert_eq!(issue.suggestion, Some("Use nullptr instead".to_string()));
    }

    #[test]
    fn test_lint_issue_with_source() {
        let issue = LintIssue::new(
            PathBuf::from("test.cpp"),
            10,
            "msg".to_string(),
            Severity::Warning,
        )
        .with_source("cpplint".to_string());

        assert_eq!(issue.source, Some("cpplint".to_string()));
    }

    #[test]
    fn test_lint_issue_with_language() {
        let issue = LintIssue::new(
            PathBuf::from("test.cpp"),
            10,
            "msg".to_string(),
            Severity::Warning,
        )
        .with_language(Language::Cpp);

        assert_eq!(issue.language, Some(Language::Cpp));
    }

    #[test]
    fn test_lint_issue_builder_chaining() {
        let issue = LintIssue::new(
            PathBuf::from("test.cpp"),
            10,
            "Test error".to_string(),
            Severity::Error,
        )
        .with_column(5)
        .with_code("E001".to_string())
        .with_source("clang-tidy".to_string())
        .with_suggestion("Fix it".to_string())
        .with_language(Language::Cpp);

        assert_eq!(issue.column, Some(5));
        assert_eq!(issue.code, Some("E001".to_string()));
        assert_eq!(issue.source, Some("clang-tidy".to_string()));
        assert_eq!(issue.suggestion, Some("Fix it".to_string()));
        assert_eq!(issue.language, Some(Language::Cpp));
    }

    // ==================== FormatResult tests ====================

    #[test]
    fn test_format_result_unchanged() {
        let result = FormatResult::unchanged(PathBuf::from("test.cpp"));

        assert_eq!(result.file_path, PathBuf::from("test.cpp"));
        assert!(!result.changed);
        assert!(result.diff.is_none());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_format_result_changed() {
        let result = FormatResult::changed(PathBuf::from("test.cpp"));

        assert_eq!(result.file_path, PathBuf::from("test.cpp"));
        assert!(result.changed);
        assert!(result.diff.is_none());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_format_result_with_diff() {
        let result =
            FormatResult::changed(PathBuf::from("test.cpp")).with_diff("- old\n+ new".to_string());

        assert!(result.changed);
        assert_eq!(result.diff, Some("- old\n+ new".to_string()));
    }

    #[test]
    fn test_format_result_error() {
        let result = FormatResult::error(PathBuf::from("test.cpp"), "Format failed".to_string());

        assert_eq!(result.file_path, PathBuf::from("test.cpp"));
        assert!(!result.changed);
        assert!(result.diff.is_none());
        assert_eq!(result.error, Some("Format failed".to_string()));
    }

    // ==================== RunModeKind tests ====================

    #[test]
    fn test_run_mode_kind_default() {
        let mode = RunModeKind::default();
        assert_eq!(mode, RunModeKind::Both);
    }

    // ==================== RunResult tests ====================

    #[test]
    fn test_run_result_new() {
        let result = RunResult::new();

        assert_eq!(result.total_files, 0);
        assert_eq!(result.files_with_issues, 0);
        assert_eq!(result.files_formatted, 0);
        assert!(result.issues.is_empty());
        assert_eq!(result.issues_before_format, 0);
        assert_eq!(result.issues_fixed, 0);
        assert!(result.format_results.is_empty());
        assert_eq!(result.duration_ms, 0);
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.run_mode, RunModeKind::Both);
    }

    #[test]
    fn test_run_result_add_issue() {
        let mut result = RunResult::new();
        let issue = LintIssue::new(
            PathBuf::from("test.cpp"),
            10,
            "Test".to_string(),
            Severity::Warning,
        );

        result.add_issue(issue);

        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].file_path, PathBuf::from("test.cpp"));
    }

    #[test]
    fn test_run_result_add_format_result_changed() {
        let mut result = RunResult::new();
        let format_result = FormatResult::changed(PathBuf::from("test.cpp"));

        result.add_format_result(format_result);

        assert_eq!(result.files_formatted, 1);
        assert_eq!(result.format_results.len(), 1);
    }

    #[test]
    fn test_run_result_add_format_result_unchanged() {
        let mut result = RunResult::new();
        let format_result = FormatResult::unchanged(PathBuf::from("test.cpp"));

        result.add_format_result(format_result);

        assert_eq!(result.files_formatted, 0);
        assert_eq!(result.format_results.len(), 1);
    }

    #[test]
    fn test_run_result_calculate_exit_code_success() {
        let mut result = RunResult::new();
        result.calculate_exit_code();
        assert_eq!(result.exit_code, 0);
    }

    #[test]
    fn test_run_result_calculate_exit_code_with_error() {
        let mut result = RunResult::new();
        result.add_issue(LintIssue::new(
            PathBuf::from("test.cpp"),
            10,
            "Error".to_string(),
            Severity::Error,
        ));

        result.calculate_exit_code();
        assert_eq!(result.exit_code, 1);
    }

    #[test]
    fn test_run_result_calculate_exit_code_with_warning() {
        let mut result = RunResult::new();
        result.add_issue(LintIssue::new(
            PathBuf::from("test.cpp"),
            10,
            "Warning".to_string(),
            Severity::Warning,
        ));

        result.calculate_exit_code();
        assert_eq!(result.exit_code, 0); // Warnings don't cause exit code 1
    }

    #[test]
    fn test_run_result_calculate_exit_code_format_error() {
        let mut result = RunResult::new();
        result.add_format_result(FormatResult::error(
            PathBuf::from("test.cpp"),
            "Format failed".to_string(),
        ));

        result.calculate_exit_code();
        assert_eq!(result.exit_code, 2);
    }

    #[test]
    fn test_run_result_count_files_with_issues() {
        let mut result = RunResult::new();
        result.add_issue(LintIssue::new(
            PathBuf::from("test1.cpp"),
            10,
            "Issue 1".to_string(),
            Severity::Warning,
        ));
        result.add_issue(LintIssue::new(
            PathBuf::from("test1.cpp"),
            20,
            "Issue 2".to_string(),
            Severity::Warning,
        ));
        result.add_issue(LintIssue::new(
            PathBuf::from("test2.cpp"),
            5,
            "Issue 3".to_string(),
            Severity::Error,
        ));

        result.count_files_with_issues();

        // Only 2 unique files have issues
        assert_eq!(result.files_with_issues, 2);
    }

    #[test]
    fn test_run_result_count_files_with_issues_empty() {
        let mut result = RunResult::new();
        result.count_files_with_issues();
        assert_eq!(result.files_with_issues, 0);
    }
}
