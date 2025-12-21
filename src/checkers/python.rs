// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Python language checker using ruff.
//!
//! Ruff is an extremely fast Python linter written in Rust, offering
//! 10-100x speed improvements over flake8 with 800+ built-in rules.

use crate::checkers::Checker;
use crate::utils::types::{LintIssue, Severity};
use crate::{Language, Result};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

/// Ruff JSON output location structure
#[derive(Debug, Deserialize)]
struct RuffLocation {
    row: usize,
    column: usize,
}

/// Ruff JSON output fix edit structure
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RuffEdit {
    content: String,
    location: RuffLocation,
    end_location: RuffLocation,
}

/// Ruff JSON output fix structure
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RuffFix {
    message: String,
    applicability: String,
    edits: Vec<RuffEdit>,
}

/// Ruff JSON output issue structure
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RuffIssue {
    filename: String,
    code: String,
    message: String,
    location: RuffLocation,
    end_location: RuffLocation,
    fix: Option<RuffFix>,
    url: Option<String>,
}

/// Python checker using ruff.
pub struct PythonChecker;

impl PythonChecker {
    pub fn new() -> Self {
        Self
    }

    /// Parse ruff JSON output and extract issues.
    fn parse_ruff_json_output(&self, output: &str, _file_path: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Parse JSON array of issues
        let ruff_issues: Vec<RuffIssue> = match serde_json::from_str(output) {
            Ok(issues) => issues,
            Err(_) => return issues, // Return empty on parse error
        };

        for ruff_issue in ruff_issues {
            let severity = self.map_code_to_severity(&ruff_issue.code);

            let mut issue = LintIssue::new(
                std::path::PathBuf::from(&ruff_issue.filename),
                ruff_issue.location.row,
                ruff_issue.message.clone(),
                severity,
            )
            .with_source("ruff".to_string())
            .with_code(ruff_issue.code.clone())
            .with_column(ruff_issue.location.column);

            // Add fix suggestion if available
            if let Some(fix) = &ruff_issue.fix {
                issue = issue.with_suggestion(fix.message.clone());
            }

            issues.push(issue);
        }

        issues
    }

    /// Map ruff error code prefix to severity level.
    ///
    /// Ruff code prefixes:
    /// - E (pycodestyle Error) -> Error
    /// - F (Pyflakes) -> Error
    /// - W (pycodestyle Warning) -> Warning
    /// - C (Convention/mccabe) -> Info
    /// - R (Refactor) -> Info
    /// - I (isort) -> Info
    /// - N (pep8-naming) -> Warning
    /// - D (pydocstyle) -> Info
    /// - UP (pyupgrade) -> Info
    /// - B (flake8-bugbear) -> Warning
    /// - S (flake8-bandit/security) -> Warning
    /// - A (flake8-builtins) -> Warning
    /// - Others -> Info
    fn map_code_to_severity(&self, code: &str) -> Severity {
        if code.is_empty() {
            return Severity::Info;
        }

        // Get the letter prefix (could be 1-2 chars like "UP", "PL", etc.)
        let prefix: String = code.chars().take_while(|c| c.is_ascii_alphabetic()).collect();

        match prefix.as_str() {
            // Errors
            "E" | "F" => Severity::Error,
            // Warnings
            "W" | "N" | "B" | "S" | "A" | "PL" | "PLW" | "PLR" | "PLE" | "C90" => Severity::Warning,
            // Info (conventions, refactoring suggestions)
            "C" | "R" | "I" | "D" | "UP" | "YTT" | "ANN" | "BLE" | "FBT" | "COM" | "DTZ"
            | "EM" | "EXE" | "FA" | "ISC" | "ICN" | "LOG" | "G" | "INP" | "PIE" | "T20" | "PYI"
            | "PT" | "Q" | "RSE" | "RET" | "SLF" | "SLOT" | "SIM" | "TID" | "TCH" | "INT"
            | "ARG" | "PTH" | "TD" | "FIX" | "ERA" | "PD" | "PGH" | "TRY" | "FLY" | "NPY"
            | "AIR" | "PERF" | "FURB" | "RUF" => Severity::Info,
            // Default to Info for unknown codes
            _ => Severity::Info,
        }
    }
}

impl Default for PythonChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for PythonChecker {
    fn name(&self) -> &str {
        "ruff"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Python]
    }

    fn check(&self, path: &Path) -> Result<Vec<LintIssue>> {
        let output = Command::new("ruff")
            .args(["check", "--output-format", "json"])
            .arg(path)
            .output()
            .map_err(|e| crate::LintisError::Checker(format!("Failed to run ruff: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let issues = self.parse_ruff_json_output(&stdout, path);

        Ok(issues)
    }

    fn is_available(&self) -> bool {
        Command::new("ruff")
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
    fn test_severity_mapping() {
        let checker = PythonChecker::new();

        // Errors
        assert_eq!(checker.map_code_to_severity("E501"), Severity::Error);
        assert_eq!(checker.map_code_to_severity("F401"), Severity::Error);

        // Warnings
        assert_eq!(checker.map_code_to_severity("W503"), Severity::Warning);
        assert_eq!(checker.map_code_to_severity("N801"), Severity::Warning);
        assert_eq!(checker.map_code_to_severity("B006"), Severity::Warning);
        assert_eq!(checker.map_code_to_severity("S101"), Severity::Warning);

        // Info
        assert_eq!(checker.map_code_to_severity("I001"), Severity::Info);
        assert_eq!(checker.map_code_to_severity("D100"), Severity::Info);
        assert_eq!(checker.map_code_to_severity("UP035"), Severity::Info);
        assert_eq!(checker.map_code_to_severity("RUF001"), Severity::Info);
    }

    #[test]
    fn test_parse_ruff_json_output() {
        let checker = PythonChecker::new();
        let json = r#"[
            {
                "cell": null,
                "code": "F401",
                "end_location": {"column": 10, "row": 1},
                "filename": "test.py",
                "fix": {
                    "applicability": "safe",
                    "edits": [{"content": "", "end_location": {"column": 10, "row": 1}, "location": {"column": 0, "row": 1}}],
                    "message": "Remove unused import: `os`"
                },
                "location": {"column": 8, "row": 1},
                "message": "`os` imported but unused",
                "noqa_row": 1,
                "url": "https://docs.astral.sh/ruff/rules/unused-import"
            }
        ]"#;

        let issues = checker.parse_ruff_json_output(json, Path::new("test.py"));
        assert_eq!(issues.len(), 1);

        let issue = &issues[0];
        assert_eq!(issue.code, Some("F401".to_string()));
        assert_eq!(issue.message, "`os` imported but unused");
        assert_eq!(issue.line, 1);
        assert_eq!(issue.column, Some(8));
        assert_eq!(issue.severity, Severity::Error);
        assert_eq!(issue.source, Some("ruff".to_string()));
        assert_eq!(issue.suggestion, Some("Remove unused import: `os`".to_string()));
    }

    #[test]
    fn test_parse_empty_output() {
        let checker = PythonChecker::new();
        let issues = checker.parse_ruff_json_output("[]", Path::new("test.py"));
        assert!(issues.is_empty());
    }

    #[test]
    fn test_parse_invalid_json() {
        let checker = PythonChecker::new();
        let issues = checker.parse_ruff_json_output("not valid json", Path::new("test.py"));
        assert!(issues.is_empty());
    }
}
