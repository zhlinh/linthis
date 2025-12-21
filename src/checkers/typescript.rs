// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! TypeScript/JavaScript language checker using eslint.

use crate::checkers::Checker;
use crate::utils::types::{LintIssue, Severity};
use crate::{Language, Result};
use std::path::Path;
use std::process::Command;

/// TypeScript/JavaScript checker using eslint.
pub struct TypeScriptChecker;

impl TypeScriptChecker {
    pub fn new() -> Self {
        Self
    }

    /// Parse eslint JSON output and extract issues.
    fn parse_eslint_output(&self, output: &str, file_path: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Try to parse as JSON
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
            if let Some(files) = json.as_array() {
                for file_result in files {
                    let messages = file_result.get("messages").and_then(|m| m.as_array());
                    let file = file_result
                        .get("filePath")
                        .and_then(|f| f.as_str())
                        .map(std::path::PathBuf::from)
                        .unwrap_or_else(|| file_path.to_path_buf());

                    if let Some(msgs) = messages {
                        for msg in msgs {
                            if let Some(issue) = self.parse_eslint_message(msg, &file) {
                                issues.push(issue);
                            }
                        }
                    }
                }
            }
        }

        issues
    }

    fn parse_eslint_message(&self, msg: &serde_json::Value, file_path: &Path) -> Option<LintIssue> {
        let line = msg.get("line").and_then(|l| l.as_u64()).unwrap_or(1) as usize;
        let column = msg
            .get("column")
            .and_then(|c| c.as_u64())
            .map(|c| c as usize);
        let message = msg.get("message").and_then(|m| m.as_str()).unwrap_or("");
        let rule_id = msg.get("ruleId").and_then(|r| r.as_str()).unwrap_or("");
        let severity_num = msg.get("severity").and_then(|s| s.as_u64()).unwrap_or(1);

        let severity = match severity_num {
            2 => Severity::Error,
            1 => Severity::Warning,
            _ => Severity::Info,
        };

        let mut issue =
            LintIssue::new(file_path.to_path_buf(), line, message.to_string(), severity)
                .with_source("eslint".to_string());

        if !rule_id.is_empty() {
            issue = issue.with_code(rule_id.to_string());
        }

        if let Some(c) = column {
            issue = issue.with_column(c);
        }

        Some(issue)
    }
}

impl Default for TypeScriptChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for TypeScriptChecker {
    fn name(&self) -> &str {
        "eslint"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::TypeScript, Language::JavaScript]
    }

    fn check(&self, path: &Path) -> Result<Vec<LintIssue>> {
        let output = Command::new("eslint")
            .args(["--format", "json", "--no-error-on-unmatched-pattern"])
            .arg(path)
            .output()
            .map_err(|e| crate::LintisError::Checker(format!("Failed to run eslint: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let issues = self.parse_eslint_output(&stdout, path);

        Ok(issues)
    }

    fn is_available(&self) -> bool {
        Command::new("eslint")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
