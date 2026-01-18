// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Dart language checker using dart analyze.

use crate::checkers::Checker;
use crate::utils::types::{LintIssue, Severity};
use crate::{Language, Result};
use std::path::Path;
use std::process::Command;

/// Dart checker using dart analyze.
pub struct DartChecker;

impl DartChecker {
    pub fn new() -> Self {
        Self
    }

    /// Parse dart analyze machine-readable output.
    /// Format: SEVERITY|TYPE|CODE|FILE|LINE|COL|LENGTH|MESSAGE
    fn parse_analyze_output(&self, output: &str) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        for line in output.lines() {
            if let Some(issue) = self.parse_analyze_line(line) {
                issues.push(issue);
            }
        }

        issues
    }

    fn parse_analyze_line(&self, line: &str) -> Option<LintIssue> {
        // Machine format: SEVERITY|TYPE|ERROR_CODE|FILE_PATH|LINE|COLUMN|LENGTH|ERROR_MESSAGE
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 8 {
            return None;
        }

        let severity_str = parts[0].trim();
        let _type_str = parts[1].trim(); // e.g., "LINT", "HINT", etc.
        let code = parts[2].trim();
        let file_path = parts[3].trim();
        let line_num = parts[4].trim().parse::<usize>().ok()?;
        let col = parts[5].trim().parse::<usize>().ok();
        // parts[6] is LENGTH which we don't use
        let message = parts[7..].join("|"); // Message might contain |

        let severity = match severity_str.to_uppercase().as_str() {
            "ERROR" => Severity::Error,
            "WARNING" => Severity::Warning,
            "INFO" | "HINT" => Severity::Info,
            _ => Severity::Info,
        };

        let mut issue = LintIssue::new(
            std::path::PathBuf::from(file_path),
            line_num,
            message.trim().to_string(),
            severity,
        )
        .with_source("dart-analyze".to_string())
        .with_code(code.to_string());

        if let Some(c) = col {
            issue = issue.with_column(c);
        }

        Some(issue)
    }
}

impl Default for DartChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for DartChecker {
    fn name(&self) -> &str {
        "dart-analyze"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Dart]
    }

    fn check(&self, path: &Path) -> Result<Vec<LintIssue>> {
        let output = Command::new("dart")
            .args(["analyze", "--format=machine"])
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::checker("dart analyze", path, format!("Failed to run: {}", e))
            })?;

        // dart analyze outputs to stderr in machine format
        let stderr = String::from_utf8_lossy(&output.stderr);
        let issues = self.parse_analyze_output(&stderr);

        Ok(issues)
    }

    fn is_available(&self) -> bool {
        Command::new("dart")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_analyze_output() {
        let checker = DartChecker::new();
        let output = "ERROR|LINT|undefined_identifier|/path/to/file.dart|10|5|3|Undefined name 'foo'";
        let issues = checker.parse_analyze_output(output);

        assert_eq!(issues.len(), 1);
        let issue = &issues[0];
        assert_eq!(issue.severity, Severity::Error);
        assert_eq!(issue.line, 10);
        assert_eq!(issue.column, Some(5));
        assert_eq!(issue.code, Some("undefined_identifier".to_string()));
        assert_eq!(issue.message, "Undefined name 'foo'");
    }

    #[test]
    fn test_parse_warning() {
        let checker = DartChecker::new();
        let output = "WARNING|HINT|unused_local_variable|/path/to/file.dart|5|3|1|The value of the local variable 'x' isn't used";
        let issues = checker.parse_analyze_output(output);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Warning);
    }

    #[test]
    fn test_parse_empty_output() {
        let checker = DartChecker::new();
        let issues = checker.parse_analyze_output("");
        assert!(issues.is_empty());
    }
}
