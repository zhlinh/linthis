// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Java language formatter using clang-format.

use crate::formatters::Formatter;
use crate::utils::types::FormatResult;
use crate::{Language, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Java formatter using clang-format.
pub struct JavaFormatter;

impl JavaFormatter {
    pub fn new() -> Self {
        Self
    }

    /// Find clang-format configuration file
    fn find_clang_format_config(path: &Path) -> Option<std::path::PathBuf> {
        let mut current = if path.is_file() {
            path.parent()?.to_path_buf()
        } else {
            path.to_path_buf()
        };

        let config_names = [
            ".linthis/configs/java/.clang-format",  // Plugin config (highest priority)
            ".clang-format",
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
}

impl Default for JavaFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for JavaFormatter {
    fn name(&self) -> &str {
        "clang-format"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Java]
    }

    fn format(&self, path: &Path) -> Result<FormatResult> {
        // Read original content for comparison
        let original = fs::read_to_string(path)
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to read file: {}", e)))?;

        // Build clang-format command
        let mut cmd = Command::new("clang-format");
        cmd.arg("-i"); // In-place formatting

        // Try to find clang-format config
        if let Some(config_path) = Self::find_clang_format_config(path) {
            cmd.arg(format!("--style=file:{}", config_path.display()));
        } else {
            // Fall back to Google style if no config found
            cmd.arg("--style=Google");
        }

        let output = cmd
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::Formatter(format!("Failed to run clang-format: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(FormatResult::error(
                path.to_path_buf(),
                format!("clang-format failed: {}", stderr),
            ));
        }

        // Read new content and compare
        let new_content = fs::read_to_string(path).map_err(|e| {
            crate::LintisError::Formatter(format!("Failed to read formatted file: {}", e))
        })?;

        if original == new_content {
            Ok(FormatResult::unchanged(path.to_path_buf()))
        } else {
            Ok(FormatResult::changed(path.to_path_buf()))
        }
    }

    fn check(&self, path: &Path) -> Result<bool> {
        // Read current content
        let current = fs::read_to_string(path)
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to read file: {}", e)))?;

        // Build clang-format command (without -i to output to stdout)
        let mut cmd = Command::new("clang-format");

        // Try to find clang-format config
        if let Some(config_path) = Self::find_clang_format_config(path) {
            cmd.arg(format!("--style=file:{}", config_path.display()));
        } else {
            // Fall back to Google style if no config found
            cmd.arg("--style=Google");
        }

        let output = cmd
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::Formatter(format!("Failed to run clang-format: {}", e))
            })?;

        let formatted = String::from_utf8_lossy(&output.stdout);

        // If they differ, file needs formatting
        Ok(current != formatted.as_ref())
    }

    fn is_available(&self) -> bool {
        Command::new("clang-format")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
