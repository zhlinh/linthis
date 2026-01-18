// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Ruby language checker using RuboCop.

use crate::checkers::Checker;
use crate::utils::types::{LintIssue, Severity};
use crate::{Language, Result};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

/// RuboCop JSON output structure
#[derive(Debug, Deserialize)]
struct RuboCopOutput {
    files: Vec<RuboCopFile>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RuboCopFile {
    path: String,
    offenses: Vec<RuboCopOffense>,
}

#[derive(Debug, Deserialize)]
struct RuboCopOffense {
    severity: String,
    message: String,
    cop_name: String,
    location: RuboCopLocation,
}

#[derive(Debug, Deserialize)]
struct RuboCopLocation {
    start_line: usize,
    start_column: usize,
    #[allow(dead_code)]
    last_line: Option<usize>,
    #[allow(dead_code)]
    last_column: Option<usize>,
}

/// Ruby checker using RuboCop.
pub struct RubyChecker;

impl RubyChecker {
    pub fn new() -> Self {
        Self
    }

    /// Parse RuboCop JSON output.
    fn parse_rubocop_output(&self, output: &str, path: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Try to parse JSON output
        if let Ok(rubocop_output) = serde_json::from_str::<RuboCopOutput>(output) {
            for file in rubocop_output.files {
                for offense in file.offenses {
                    let severity = match offense.severity.as_str() {
                        "error" | "fatal" => Severity::Error,
                        "warning" => Severity::Warning,
                        "convention" | "refactor" | "info" => Severity::Info,
                        _ => Severity::Warning,
                    };

                    let issue = LintIssue::new(
                        path.to_path_buf(),
                        offense.location.start_line,
                        offense.message.clone(),
                        severity,
                    )
                    .with_source("rubocop".to_string())
                    .with_code(offense.cop_name.clone())
                    .with_column(offense.location.start_column);

                    issues.push(issue);
                }
            }
        }

        issues
    }
}

impl Default for RubyChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for RubyChecker {
    fn name(&self) -> &str {
        "rubocop"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Ruby]
    }

    fn check(&self, path: &Path) -> Result<Vec<LintIssue>> {
        let output = Command::new("rubocop")
            .args(["--format", "json"])
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::checker("rubocop", path, format!("Failed to run: {}", e))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let issues = self.parse_rubocop_output(&stdout, path);

        Ok(issues)
    }

    fn is_available(&self) -> bool {
        Command::new("rubocop")
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
    fn test_parse_rubocop_output() {
        let checker = RubyChecker::new();
        let output = r#"{
            "files": [
                {
                    "path": "test.rb",
                    "offenses": [
                        {
                            "severity": "convention",
                            "message": "Use 2 (not 4) spaces for indentation.",
                            "cop_name": "Layout/IndentationWidth",
                            "location": {
                                "start_line": 5,
                                "start_column": 1,
                                "last_line": 5,
                                "last_column": 4
                            }
                        },
                        {
                            "severity": "error",
                            "message": "unexpected token tRPAREN",
                            "cop_name": "Lint/Syntax",
                            "location": {
                                "start_line": 10,
                                "start_column": 15,
                                "last_line": 10,
                                "last_column": 16
                            }
                        }
                    ]
                }
            ]
        }"#;

        let issues = checker.parse_rubocop_output(output, Path::new("test.rb"));

        assert_eq!(issues.len(), 2);

        let issue1 = &issues[0];
        assert_eq!(issue1.severity, Severity::Info);
        assert_eq!(issue1.line, 5);
        assert_eq!(issue1.column, Some(1));
        assert_eq!(issue1.code, Some("Layout/IndentationWidth".to_string()));

        let issue2 = &issues[1];
        assert_eq!(issue2.severity, Severity::Error);
        assert_eq!(issue2.line, 10);
    }

    #[test]
    fn test_parse_empty_output() {
        let checker = RubyChecker::new();
        let output = r#"{"files": []}"#;
        let issues = checker.parse_rubocop_output(output, Path::new("test.rb"));
        assert!(issues.is_empty());
    }

    #[test]
    fn test_parse_warning_severity() {
        let checker = RubyChecker::new();
        let output = r#"{
            "files": [
                {
                    "path": "test.rb",
                    "offenses": [
                        {
                            "severity": "warning",
                            "message": "Useless assignment",
                            "cop_name": "Lint/UselessAssignment",
                            "location": {
                                "start_line": 1,
                                "start_column": 1
                            }
                        }
                    ]
                }
            ]
        }"#;

        let issues = checker.parse_rubocop_output(output, Path::new("test.rb"));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Warning);
    }
}
