// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Kotlin language checker using ktlint.

use crate::checkers::Checker;
use crate::utils::types::{LintIssue, Severity};
use crate::{Language, Result};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

/// ktlint JSON output error structure
#[derive(Debug, Deserialize)]
struct KtlintError {
    line: usize,
    col: usize,
    message: String,
    rule: String,
}

/// ktlint JSON output file structure
#[derive(Debug, Deserialize)]
struct KtlintFileResult {
    file: String,
    errors: Vec<KtlintError>,
}

/// Kotlin checker using ktlint.
pub struct KotlinChecker;

impl KotlinChecker {
    pub fn new() -> Self {
        Self
    }

    /// Parse ktlint JSON output.
    fn parse_ktlint_output(&self, output: &str) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Parse JSON array of file results
        let file_results: Vec<KtlintFileResult> = match serde_json::from_str(output) {
            Ok(results) => results,
            Err(_) => return issues,
        };

        for file_result in file_results {
            for error in file_result.errors {
                let issue = LintIssue::new(
                    std::path::PathBuf::from(&file_result.file),
                    error.line,
                    error.message.clone(),
                    Severity::Warning, // ktlint issues are typically style warnings
                )
                .with_source("ktlint".to_string())
                .with_code(error.rule.clone())
                .with_column(error.col);

                issues.push(issue);
            }
        }

        issues
    }
}

impl Default for KotlinChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for KotlinChecker {
    fn name(&self) -> &str {
        "ktlint"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Kotlin]
    }

    fn check(&self, path: &Path) -> Result<Vec<LintIssue>> {
        let output = Command::new("ktlint")
            .args(["--reporter=json"])
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::checker("ktlint", path, format!("Failed to run: {}", e))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let issues = self.parse_ktlint_output(&stdout);

        Ok(issues)
    }

    fn is_available(&self) -> bool {
        Command::new("ktlint")
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
    fn test_parse_ktlint_output() {
        let checker = KotlinChecker::new();
        let json = r#"[
            {
                "file": "/path/to/file.kt",
                "errors": [
                    {
                        "line": 1,
                        "col": 1,
                        "message": "Needless blank line(s)",
                        "rule": "no-blank-line-before-rbrace"
                    }
                ]
            }
        ]"#;

        let issues = checker.parse_ktlint_output(json);
        assert_eq!(issues.len(), 1);

        let issue = &issues[0];
        assert_eq!(issue.severity, Severity::Warning);
        assert_eq!(issue.line, 1);
        assert_eq!(issue.column, Some(1));
        assert_eq!(issue.code, Some("no-blank-line-before-rbrace".to_string()));
    }

    #[test]
    fn test_parse_empty_output() {
        let checker = KotlinChecker::new();
        let issues = checker.parse_ktlint_output("[]");
        assert!(issues.is_empty());
    }
}
