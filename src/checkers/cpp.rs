// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! C/C++ language checker using cpplint or cppcheck.

use crate::checkers::Checker;
use crate::utils::types::{LintIssue, Severity};
use crate::{Language, Result};
use std::path::Path;
use std::process::Command;

/// C/C++ checker using cpplint.
pub struct CppChecker;

impl CppChecker {
    pub fn new() -> Self {
        Self
    }

    /// Parse cpplint output and extract issues.
    /// Format: file:line: message [category] [confidence]
    fn parse_cpplint_output(&self, output: &str, file_path: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        for line in output.lines() {
            if let Some(issue) = self.parse_cpplint_line(line, file_path) {
                issues.push(issue);
            }
        }

        issues
    }

    fn parse_cpplint_line(&self, line: &str, default_path: &Path) -> Option<LintIssue> {
        // cpplint format: file:line: message [category] [confidence]
        // Example: test.cpp:10: Missing space after comma [whitespace/comma] [3]
        if !line.contains(':')
            || line.starts_with("Done processing")
            || line.starts_with("Total errors")
        {
            return None;
        }

        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() < 3 {
            return None;
        }

        let file_path_parsed = std::path::PathBuf::from(parts[0]);
        let line_num = parts[1].trim().parse::<usize>().ok()?;
        let rest = parts[2].trim();

        // Parse message and extract category
        let (message, code) = if let Some(bracket_start) = rest.find('[') {
            let msg = rest[..bracket_start].trim();
            let category = rest[bracket_start..].trim_matches(|c| c == '[' || c == ']');
            // Extract just the first bracketed category
            let cat = category.split("] [").next().unwrap_or(category);
            (msg.to_string(), Some(cat.to_string()))
        } else {
            (rest.to_string(), None)
        };

        // Determine severity based on confidence level (1-5, higher = more confident)
        // or category
        let severity = if message.to_lowercase().contains("error") {
            Severity::Error
        } else {
            Severity::Warning
        };

        let mut issue = LintIssue::new(
            if file_path_parsed.exists() {
                file_path_parsed
            } else {
                default_path.to_path_buf()
            },
            line_num,
            message,
            severity,
        )
        .with_source("cpplint".to_string());

        if let Some(c) = code {
            issue = issue.with_code(c);
        }

        Some(issue)
    }
}

impl Default for CppChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for CppChecker {
    fn name(&self) -> &str {
        "cpplint"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Cpp, Language::ObjectiveC]
    }

    fn check(&self, path: &Path) -> Result<Vec<LintIssue>> {
        let output = Command::new("cpplint")
            .arg(path)
            .output()
            .map_err(|e| crate::LintisError::Checker(format!("Failed to run cpplint: {}", e)))?;

        // cpplint outputs to stderr
        let stderr = String::from_utf8_lossy(&output.stderr);
        let issues = self.parse_cpplint_output(&stderr, path);

        Ok(issues)
    }

    fn is_available(&self) -> bool {
        Command::new("cpplint")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
