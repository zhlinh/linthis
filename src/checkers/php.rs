// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! PHP language checker using PHP_CodeSniffer (phpcs).

use crate::checkers::Checker;
use crate::utils::types::{LintIssue, Severity};
use crate::{Language, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// phpcs JSON output structure
#[derive(Debug, Deserialize)]
struct PhpcsOutput {
    files: HashMap<String, PhpcsFile>,
}

#[derive(Debug, Deserialize)]
struct PhpcsFile {
    messages: Vec<PhpcsMessage>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PhpcsMessage {
    message: String,
    source: String,
    severity: u32,
    #[serde(rename = "type")]
    msg_type: String,
    line: usize,
    column: usize,
    #[serde(default)]
    fixable: bool,
}

/// PHP checker using phpcs.
pub struct PhpChecker;

impl PhpChecker {
    pub fn new() -> Self {
        Self
    }

    /// Parse phpcs JSON output.
    fn parse_phpcs_output(&self, output: &str, path: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Try to parse JSON output
        if let Ok(phpcs_output) = serde_json::from_str::<PhpcsOutput>(output) {
            for (_, file) in phpcs_output.files {
                for msg in file.messages {
                    let severity = match msg.msg_type.as_str() {
                        "ERROR" => Severity::Error,
                        "WARNING" => Severity::Warning,
                        _ => Severity::Info,
                    };

                    let mut issue = LintIssue::new(
                        path.to_path_buf(),
                        msg.line,
                        msg.message.clone(),
                        severity,
                    )
                    .with_source("phpcs".to_string())
                    .with_code(msg.source.clone())
                    .with_column(msg.column);

                    // Add fixable hint if applicable
                    if msg.fixable {
                        issue = issue.with_suggestion("Auto-fixable with phpcbf".to_string());
                    }

                    issues.push(issue);
                }
            }
        }

        issues
    }
}

impl Default for PhpChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for PhpChecker {
    fn name(&self) -> &str {
        "phpcs"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Php]
    }

    fn check(&self, path: &Path) -> Result<Vec<LintIssue>> {
        let output = Command::new("phpcs")
            .args(["--report=json"])
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::checker("phpcs", path, format!("Failed to run: {}", e))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let issues = self.parse_phpcs_output(&stdout, path);

        Ok(issues)
    }

    fn is_available(&self) -> bool {
        Command::new("phpcs")
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
    fn test_parse_phpcs_output() {
        let checker = PhpChecker::new();
        let output = r#"{
            "totals": {
                "errors": 1,
                "warnings": 1,
                "fixable": 1
            },
            "files": {
                "test.php": {
                    "errors": 1,
                    "warnings": 1,
                    "messages": [
                        {
                            "message": "Missing file doc comment",
                            "source": "PEAR.Commenting.FileComment.Missing",
                            "severity": 5,
                            "type": "ERROR",
                            "line": 1,
                            "column": 1,
                            "fixable": false
                        },
                        {
                            "message": "Line exceeds 80 characters",
                            "source": "Generic.Files.LineLength.TooLong",
                            "severity": 3,
                            "type": "WARNING",
                            "line": 10,
                            "column": 81,
                            "fixable": true
                        }
                    ]
                }
            }
        }"#;

        let issues = checker.parse_phpcs_output(output, Path::new("test.php"));

        assert_eq!(issues.len(), 2);

        let issue1 = &issues[0];
        assert_eq!(issue1.severity, Severity::Error);
        assert_eq!(issue1.line, 1);
        assert_eq!(issue1.column, Some(1));
        assert_eq!(
            issue1.code,
            Some("PEAR.Commenting.FileComment.Missing".to_string())
        );

        let issue2 = &issues[1];
        assert_eq!(issue2.severity, Severity::Warning);
        assert_eq!(issue2.line, 10);
        assert!(issue2.suggestion.is_some());
    }

    #[test]
    fn test_parse_empty_output() {
        let checker = PhpChecker::new();
        let output = r#"{"totals":{"errors":0,"warnings":0,"fixable":0},"files":{}}"#;
        let issues = checker.parse_phpcs_output(output, Path::new("test.php"));
        assert!(issues.is_empty());
    }
}
