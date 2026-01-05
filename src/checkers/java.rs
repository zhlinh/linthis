// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Java language checker using checkstyle.

use crate::checkers::Checker;
use crate::utils::types::{LintIssue, Severity};
use crate::{Language, Result};
use std::path::Path;
use std::process::Command;

/// Java checker using checkstyle.
pub struct JavaChecker;

impl JavaChecker {
    pub fn new() -> Self {
        Self
    }

    /// Find checkstyle configuration file in the project
    fn find_checkstyle_config(path: &Path) -> Option<std::path::PathBuf> {
        let mut current = if path.is_file() {
            path.parent()?.to_path_buf()
        } else {
            path.to_path_buf()
        };

        // Look for checkstyle configuration files
        let config_names = [
            ".linthis/configs/java/checkstyle.xml",  // Plugin config (highest priority)
            "checkstyle.xml",
            ".checkstyle.xml",
            "config/checkstyle/checkstyle.xml",
            "checkstyle-config.xml",
        ];

        loop {
            for config_name in &config_names {
                let config_path = current.join(config_name);
                if config_path.exists() {
                    return Some(config_path);
                }
            }

            if !current.pop() {
                break;
            }
        }

        None
    }

    /// Parse checkstyle output and extract issues.
    /// Default format: [SEVERITY] file:line:column: message
    fn parse_checkstyle_output(&self, output: &str, file_path: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        for line in output.lines() {
            if let Some(issue) = self.parse_checkstyle_line(line, file_path) {
                issues.push(issue);
            }
        }

        issues
    }

    fn parse_checkstyle_line(&self, line: &str, default_path: &Path) -> Option<LintIssue> {
        // checkstyle format varies, common pattern:
        // [ERROR] file:line:column: message [CheckName]
        // [WARN] file:line: message [CheckName]
        let line = line.trim();

        // Determine severity
        let (severity, rest) = if line.starts_with("[ERROR]") {
            (Severity::Error, line.strip_prefix("[ERROR]")?.trim())
        } else if line.starts_with("[WARN]") {
            (Severity::Warning, line.strip_prefix("[WARN]")?.trim())
        } else if line.starts_with("[INFO]") {
            (Severity::Info, line.strip_prefix("[INFO]")?.trim())
        } else {
            return None;
        };

        // Parse path:line[:column]: message
        let parts: Vec<&str> = rest.splitn(4, ':').collect();
        if parts.len() < 3 {
            return None;
        }

        let file_path_parsed = std::path::PathBuf::from(parts[0]);
        let line_num = parts[1].trim().parse::<usize>().ok()?;

        // Try to parse column, otherwise it's part of the message
        let (col, message) = if parts.len() >= 4 {
            if let Ok(c) = parts[2].trim().parse::<usize>() {
                (Some(c), parts[3].trim().to_string())
            } else {
                (None, format!("{}: {}", parts[2].trim(), parts[3].trim()))
            }
        } else {
            (None, parts[2].trim().to_string())
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
        .with_source("checkstyle".to_string());

        if let Some(c) = col {
            issue = issue.with_column(c);
        }

        Some(issue)
    }
}

impl Default for JavaChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for JavaChecker {
    fn name(&self) -> &str {
        "checkstyle"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Java]
    }

    fn check(&self, path: &Path) -> Result<Vec<LintIssue>> {
        // Find checkstyle configuration file
        let config_arg = if let Some(config_path) = Self::find_checkstyle_config(path) {
            vec!["-c".to_string(), config_path.to_string_lossy().to_string()]
        } else {
            // Use Google checks as default (built-in to checkstyle)
            vec!["-c".to_string(), "/google_checks.xml".to_string()]
        };

        // Try to use checkstyle if available
        let output = Command::new("checkstyle")
            .args(&config_arg)
            .arg(path)
            .output()
            .map_err(|e| crate::LintisError::Checker(format!("Failed to run checkstyle: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Checkstyle outputs to stdout for issues, stderr for errors
        let combined = format!("{}{}", stdout, stderr);
        let issues = self.parse_checkstyle_output(&combined, path);

        Ok(issues)
    }

    fn is_available(&self) -> bool {
        Command::new("checkstyle")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
