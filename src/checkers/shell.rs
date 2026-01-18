// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Shell/Bash language checker using ShellCheck.

use crate::checkers::Checker;
use crate::utils::types::{LintIssue, Severity};
use crate::{Language, Result};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

/// ShellCheck JSON output structure
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ShellCheckIssue {
    file: String,
    line: usize,
    column: usize,
    #[serde(rename = "endLine")]
    end_line: Option<usize>,
    #[serde(rename = "endColumn")]
    end_column: Option<usize>,
    level: String,
    code: u32,
    message: String,
    #[serde(default)]
    fix: Option<ShellCheckFix>,
}

#[derive(Debug, Deserialize)]
struct ShellCheckFix {
    #[serde(default)]
    replacements: Vec<ShellCheckReplacement>,
}

#[derive(Debug, Deserialize)]
struct ShellCheckReplacement {
    #[serde(default)]
    replacement: String,
}

/// Shell checker using ShellCheck.
pub struct ShellChecker;

impl ShellChecker {
    pub fn new() -> Self {
        Self
    }

    /// Parse ShellCheck JSON output.
    fn parse_shellcheck_output(&self, output: &str, path: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Try to parse JSON output
        if let Ok(sc_issues) = serde_json::from_str::<Vec<ShellCheckIssue>>(output) {
            for sc_issue in sc_issues {
                let severity = match sc_issue.level.as_str() {
                    "error" => Severity::Error,
                    "warning" => Severity::Warning,
                    "info" | "style" => Severity::Info,
                    _ => Severity::Warning,
                };

                let mut issue = LintIssue::new(
                    path.to_path_buf(),
                    sc_issue.line,
                    sc_issue.message.clone(),
                    severity,
                )
                .with_source("shellcheck".to_string())
                .with_code(format!("SC{}", sc_issue.code))
                .with_column(sc_issue.column);

                // Add fix suggestion if available
                if let Some(fix) = &sc_issue.fix {
                    if let Some(replacement) = fix.replacements.first() {
                        if !replacement.replacement.is_empty() {
                            issue = issue.with_suggestion(replacement.replacement.clone());
                        }
                    }
                }

                issues.push(issue);
            }
        }

        issues
    }
}

impl Default for ShellChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for ShellChecker {
    fn name(&self) -> &str {
        "shellcheck"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Shell]
    }

    fn check(&self, path: &Path) -> Result<Vec<LintIssue>> {
        let output = Command::new("shellcheck")
            .args(["--format=json"])
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::checker("shellcheck", path, format!("Failed to run: {}", e))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let issues = self.parse_shellcheck_output(&stdout, path);

        Ok(issues)
    }

    fn is_available(&self) -> bool {
        Command::new("shellcheck")
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
    fn test_parse_shellcheck_output() {
        let checker = ShellChecker::new();
        let output = r#"[
            {
                "file": "test.sh",
                "line": 5,
                "column": 1,
                "endLine": 5,
                "endColumn": 10,
                "level": "warning",
                "code": 2086,
                "message": "Double quote to prevent globbing and word splitting."
            },
            {
                "file": "test.sh",
                "line": 10,
                "column": 1,
                "endLine": 10,
                "endColumn": 5,
                "level": "error",
                "code": 1073,
                "message": "Couldn't parse this something expression."
            }
        ]"#;

        let issues = checker.parse_shellcheck_output(output, Path::new("test.sh"));

        assert_eq!(issues.len(), 2);

        let issue1 = &issues[0];
        assert_eq!(issue1.severity, Severity::Warning);
        assert_eq!(issue1.line, 5);
        assert_eq!(issue1.column, Some(1));
        assert_eq!(issue1.code, Some("SC2086".to_string()));

        let issue2 = &issues[1];
        assert_eq!(issue2.severity, Severity::Error);
        assert_eq!(issue2.line, 10);
    }

    #[test]
    fn test_parse_empty_output() {
        let checker = ShellChecker::new();
        let issues = checker.parse_shellcheck_output("[]", Path::new("test.sh"));
        assert!(issues.is_empty());
    }

    #[test]
    fn test_parse_info_level() {
        let checker = ShellChecker::new();
        let output = r#"[
            {
                "file": "test.sh",
                "line": 1,
                "column": 1,
                "level": "info",
                "code": 2034,
                "message": "var appears unused."
            }
        ]"#;

        let issues = checker.parse_shellcheck_output(output, Path::new("test.sh"));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Info);
    }
}
