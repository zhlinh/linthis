// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Lua language checker using luacheck.

use crate::checkers::Checker;
use crate::utils::types::{LintIssue, Severity};
use crate::{Language, Result};
use regex::Regex;
use std::path::Path;
use std::process::Command;

/// Lua checker using luacheck.
pub struct LuaChecker;

impl LuaChecker {
    pub fn new() -> Self {
        Self
    }

    /// Parse luacheck plain text output.
    /// Format: FILE:LINE:COL: (WCODE) MESSAGE
    fn parse_luacheck_output(&self, output: &str) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Regex to match luacheck output format
        // Example: /path/file.lua:10:5: (W611) line contains only whitespace
        let re = Regex::new(r"^(.+?):(\d+):(\d+):\s*\(([EW]\d+)\)\s*(.+)$").unwrap();

        for line in output.lines() {
            if let Some(caps) = re.captures(line) {
                let file_path = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let line_num: usize = caps
                    .get(2)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(1);
                let col: usize = caps
                    .get(3)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(1);
                let code = caps.get(4).map(|m| m.as_str()).unwrap_or("");
                let message = caps.get(5).map(|m| m.as_str()).unwrap_or("");

                // Determine severity from code prefix
                let severity = if code.starts_with('E') {
                    Severity::Error
                } else {
                    Severity::Warning
                };

                let issue = LintIssue::new(
                    std::path::PathBuf::from(file_path),
                    line_num,
                    message.to_string(),
                    severity,
                )
                .with_source("luacheck".to_string())
                .with_code(code.to_string())
                .with_column(col);

                issues.push(issue);
            }
        }

        issues
    }
}

impl Default for LuaChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for LuaChecker {
    fn name(&self) -> &str {
        "luacheck"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Lua]
    }

    fn check(&self, path: &Path) -> Result<Vec<LintIssue>> {
        let output = Command::new("luacheck")
            .args(["--formatter", "plain", "--codes", "--no-color"])
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::checker("luacheck", path, format!("Failed to run: {}", e))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let issues = self.parse_luacheck_output(&stdout);

        Ok(issues)
    }

    fn is_available(&self) -> bool {
        Command::new("luacheck")
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
    fn test_parse_luacheck_output() {
        let checker = LuaChecker::new();
        let output = "/path/to/file.lua:10:5: (W611) line contains only whitespace\n/path/to/file.lua:15:1: (E001) syntax error";
        let issues = checker.parse_luacheck_output(output);

        assert_eq!(issues.len(), 2);

        let issue1 = &issues[0];
        assert_eq!(issue1.severity, Severity::Warning);
        assert_eq!(issue1.line, 10);
        assert_eq!(issue1.column, Some(5));
        assert_eq!(issue1.code, Some("W611".to_string()));

        let issue2 = &issues[1];
        assert_eq!(issue2.severity, Severity::Error);
        assert_eq!(issue2.line, 15);
    }

    #[test]
    fn test_parse_empty_output() {
        let checker = LuaChecker::new();
        let issues = checker.parse_luacheck_output("");
        assert!(issues.is_empty());
    }
}
