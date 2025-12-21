// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Rust language checker using clippy.

use crate::checkers::Checker;
use crate::utils::types::{LintIssue, Severity};
use crate::{Language, Result};
use std::path::Path;
use std::process::Command;

/// Rust checker using cargo clippy.
pub struct RustChecker;

impl RustChecker {
    pub fn new() -> Self {
        Self
    }

    /// Parse clippy output and extract issues.
    fn parse_clippy_output(&self, output: &str, file_path: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Clippy output format: file:line:column: severity: message
        for line in output.lines() {
            if let Some(issue) = self.parse_clippy_line(line, file_path) {
                issues.push(issue);
            }
        }

        issues
    }

    fn parse_clippy_line(&self, line: &str, default_path: &Path) -> Option<LintIssue> {
        // Simple parsing - clippy outputs: path:line:col: severity: message
        // Example: src/main.rs:10:5: warning: unused variable `x`
        if !line.contains(": warning:") && !line.contains(": error:") {
            return None;
        }

        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() < 4 {
            return None;
        }

        let file_path = std::path::PathBuf::from(parts[0]);
        let line_num = parts[1].trim().parse::<usize>().ok()?;
        let col = parts[2].trim().parse::<usize>().ok();

        let rest = parts[3];
        let (severity, message) = if rest.contains("warning:") {
            let msg = rest.trim_start_matches(" warning:").trim();
            (Severity::Warning, msg.to_string())
        } else if rest.contains("error:") {
            let msg = rest.trim_start_matches(" error:").trim();
            (Severity::Error, msg.to_string())
        } else {
            return None;
        };

        let mut issue = LintIssue::new(
            if file_path.exists() {
                file_path
            } else {
                default_path.to_path_buf()
            },
            line_num,
            message,
            severity,
        )
        .with_source("clippy".to_string());

        if let Some(c) = col {
            issue = issue.with_column(c);
        }

        Some(issue)
    }
}

impl Default for RustChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for RustChecker {
    fn name(&self) -> &str {
        "clippy"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Rust]
    }

    fn check(&self, path: &Path) -> Result<Vec<LintIssue>> {
        // For single files, we use rustc with clippy lints
        // For full projects, we'd use cargo clippy
        let output = Command::new("rustc")
            .args([
                "--edition=2021",
                "-W",
                "clippy::all",
                "--emit=metadata",
                "-o",
                "/dev/null",
            ])
            .arg(path)
            .output()
            .map_err(|e| crate::LintisError::Checker(format!("Failed to run rustc: {}", e)))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let issues = self.parse_clippy_output(&stderr, path);

        Ok(issues)
    }

    fn is_available(&self) -> bool {
        Command::new("rustc")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
