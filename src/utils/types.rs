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
        let has_errors = self.issues.iter().any(|i| i.severity == Severity::Error);
        let has_format_errors = self.format_results.iter().any(|r| r.error.is_some());

        if has_format_errors {
            self.exit_code = 2;
        } else if has_errors {
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
