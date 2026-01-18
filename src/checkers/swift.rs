// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Swift language checker using swiftlint.

use crate::checkers::Checker;
use crate::utils::types::{LintIssue, Severity};
use crate::{Language, Result};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

/// SwiftLint JSON output structure
#[derive(Debug, Deserialize)]
struct SwiftLintIssue {
    character: Option<usize>,
    file: String,
    line: usize,
    reason: String,
    rule_id: String,
    severity: String,
    #[serde(rename = "type")]
    _issue_type: Option<String>,
}

/// Swift checker using swiftlint.
pub struct SwiftChecker;

impl SwiftChecker {
    pub fn new() -> Self {
        Self
    }

    /// Parse swiftlint JSON output.
    fn parse_swiftlint_output(&self, output: &str) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Parse JSON array of issues
        let swiftlint_issues: Vec<SwiftLintIssue> = match serde_json::from_str(output) {
            Ok(issues) => issues,
            Err(_) => return issues,
        };

        for sl_issue in swiftlint_issues {
            let severity = match sl_issue.severity.to_lowercase().as_str() {
                "error" => Severity::Error,
                "warning" => Severity::Warning,
                _ => Severity::Info,
            };

            let mut issue = LintIssue::new(
                std::path::PathBuf::from(&sl_issue.file),
                sl_issue.line,
                sl_issue.reason.clone(),
                severity,
            )
            .with_source("swiftlint".to_string())
            .with_code(sl_issue.rule_id.clone());

            if let Some(col) = sl_issue.character {
                issue = issue.with_column(col);
            }

            issues.push(issue);
        }

        issues
    }
}

impl Default for SwiftChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for SwiftChecker {
    fn name(&self) -> &str {
        "swiftlint"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Swift]
    }

    fn check(&self, path: &Path) -> Result<Vec<LintIssue>> {
        let output = Command::new("swiftlint")
            .args(["lint", "--reporter", "json", "--path"])
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::checker("swiftlint", path, format!("Failed to run: {}", e))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let issues = self.parse_swiftlint_output(&stdout);

        Ok(issues)
    }

    fn is_available(&self) -> bool {
        Command::new("swiftlint")
            .arg("version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_swiftlint_output() {
        let checker = SwiftChecker::new();
        let json = r#"[
            {
                "character": 10,
                "file": "/path/to/file.swift",
                "line": 5,
                "reason": "Line should be 120 characters or less",
                "rule_id": "line_length",
                "severity": "warning",
                "type": "Line Length"
            }
        ]"#;

        let issues = checker.parse_swiftlint_output(json);
        assert_eq!(issues.len(), 1);

        let issue = &issues[0];
        assert_eq!(issue.severity, Severity::Warning);
        assert_eq!(issue.line, 5);
        assert_eq!(issue.column, Some(10));
        assert_eq!(issue.code, Some("line_length".to_string()));
    }

    #[test]
    fn test_parse_empty_output() {
        let checker = SwiftChecker::new();
        let issues = checker.parse_swiftlint_output("[]");
        assert!(issues.is_empty());
    }
}
