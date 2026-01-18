// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Scala language checker using scalafix.

use crate::checkers::Checker;
use crate::utils::types::{LintIssue, Severity};
use crate::{Language, Result};
use regex::Regex;
use std::path::Path;
use std::process::Command;

/// Scala checker using scalafix.
pub struct ScalaChecker;

impl ScalaChecker {
    pub fn new() -> Self {
        Self
    }

    /// Parse scalafix output.
    /// Format varies but typically: FILE:LINE:COL: [RULE] MESSAGE
    fn parse_scalafix_output(&self, output: &str, path: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Regex to match scalafix output format
        // Example: src/main/scala/Example.scala:10:5: error: [DisableSyntax.null] null is disabled
        let re = Regex::new(r"^(.+?):(\d+):(\d+):\s*(error|warning|info):\s*\[([^\]]+)\]\s*(.+)$")
            .unwrap();

        // Alternative format without severity: FILE:LINE:COL: [RULE] MESSAGE
        let re_simple = Regex::new(r"^(.+?):(\d+):(\d+):\s*\[([^\]]+)\]\s*(.+)$").unwrap();

        for line in output.lines() {
            if let Some(caps) = re.captures(line) {
                let line_num: usize = caps
                    .get(2)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(1);
                let col: usize = caps
                    .get(3)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(1);
                let severity_str = caps.get(4).map(|m| m.as_str()).unwrap_or("warning");
                let rule = caps.get(5).map(|m| m.as_str()).unwrap_or("");
                let message = caps.get(6).map(|m| m.as_str()).unwrap_or("");

                let severity = match severity_str {
                    "error" => Severity::Error,
                    "warning" => Severity::Warning,
                    _ => Severity::Info,
                };

                let issue = LintIssue::new(
                    path.to_path_buf(),
                    line_num,
                    message.to_string(),
                    severity,
                )
                .with_source("scalafix".to_string())
                .with_code(rule.to_string())
                .with_column(col);

                issues.push(issue);
            } else if let Some(caps) = re_simple.captures(line) {
                let line_num: usize = caps
                    .get(2)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(1);
                let col: usize = caps
                    .get(3)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(1);
                let rule = caps.get(4).map(|m| m.as_str()).unwrap_or("");
                let message = caps.get(5).map(|m| m.as_str()).unwrap_or("");

                let issue = LintIssue::new(
                    path.to_path_buf(),
                    line_num,
                    message.to_string(),
                    Severity::Warning,
                )
                .with_source("scalafix".to_string())
                .with_code(rule.to_string())
                .with_column(col);

                issues.push(issue);
            }
        }

        issues
    }
}

impl Default for ScalaChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for ScalaChecker {
    fn name(&self) -> &str {
        "scalafix"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Scala]
    }

    fn check(&self, path: &Path) -> Result<Vec<LintIssue>> {
        // Run scalafix with --check flag
        let output = Command::new("scalafix")
            .args(["--check"])
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::checker("scalafix", path, format!("Failed to run: {}", e))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Scalafix outputs to both stdout and stderr
        let combined = format!("{}\n{}", stdout, stderr);
        let issues = self.parse_scalafix_output(&combined, path);

        Ok(issues)
    }

    fn is_available(&self) -> bool {
        Command::new("scalafix")
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
    fn test_parse_scalafix_output() {
        let checker = ScalaChecker::new();
        let output = r#"src/main/scala/Example.scala:10:5: error: [DisableSyntax.null] null is disabled
src/main/scala/Example.scala:15:1: warning: [OrganizeImports] Imports are not sorted"#;

        let issues = checker.parse_scalafix_output(output, Path::new("Example.scala"));

        assert_eq!(issues.len(), 2);

        let issue1 = &issues[0];
        assert_eq!(issue1.severity, Severity::Error);
        assert_eq!(issue1.line, 10);
        assert_eq!(issue1.column, Some(5));
        assert_eq!(issue1.code, Some("DisableSyntax.null".to_string()));

        let issue2 = &issues[1];
        assert_eq!(issue2.severity, Severity::Warning);
        assert_eq!(issue2.line, 15);
    }

    #[test]
    fn test_parse_empty_output() {
        let checker = ScalaChecker::new();
        let issues = checker.parse_scalafix_output("", Path::new("Example.scala"));
        assert!(issues.is_empty());
    }

    #[test]
    fn test_parse_simple_format() {
        let checker = ScalaChecker::new();
        let output = "Example.scala:5:10: [UnusedImport] Unused import";

        let issues = checker.parse_scalafix_output(output, Path::new("Example.scala"));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Warning);
        assert_eq!(issues[0].line, 5);
        assert_eq!(issues[0].code, Some("UnusedImport".to_string()));
    }
}
