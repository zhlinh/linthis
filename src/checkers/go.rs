// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Go language checker using go vet and staticcheck.

use crate::checkers::Checker;
use crate::utils::types::{LintIssue, Severity};
use crate::{Language, Result};
use std::path::Path;
use std::process::Command;

/// Go checker using go vet.
pub struct GoChecker;

impl GoChecker {
    pub fn new() -> Self {
        Self
    }

    /// Parse go vet output and extract issues.
    /// Format: file:line:column: message
    fn parse_go_vet_output(&self, output: &str, file_path: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        for line in output.lines() {
            if let Some(issue) = self.parse_go_vet_line(line, file_path) {
                issues.push(issue);
            }
        }

        issues
    }

    fn parse_go_vet_line(&self, line: &str, default_path: &Path) -> Option<LintIssue> {
        // go vet format: path:line:column: message
        // Example: main.go:10:5: unreachable code
        if !line.contains(':') {
            return None;
        }

        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() < 3 {
            return None;
        }

        let file_path = std::path::PathBuf::from(parts[0]);
        let line_num = parts[1].trim().parse::<usize>().ok()?;

        // Column and message handling
        let (col, message) = if parts.len() >= 4 {
            let col = parts[2].trim().parse::<usize>().ok();
            (col, parts[3].trim().to_string())
        } else {
            (None, parts[2].trim().to_string())
        };

        let mut issue = LintIssue::new(
            if file_path.exists() {
                file_path
            } else {
                default_path.to_path_buf()
            },
            line_num,
            message,
            Severity::Warning,
        )
        .with_source("go vet".to_string());

        if let Some(c) = col {
            issue = issue.with_column(c);
        }

        Some(issue)
    }
}

impl Default for GoChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for GoChecker {
    fn name(&self) -> &str {
        "go vet"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Go]
    }

    fn check(&self, path: &Path) -> Result<Vec<LintIssue>> {
        let output = Command::new("go")
            .args(["vet"])
            .arg(path)
            .output()
            .map_err(|e| crate::LintisError::Checker(format!("Failed to run go vet: {}", e)))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let issues = self.parse_go_vet_output(&stderr, path);

        Ok(issues)
    }

    fn is_available(&self) -> bool {
        Command::new("go")
            .arg("version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
