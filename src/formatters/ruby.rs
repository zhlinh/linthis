// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Ruby language formatter using RuboCop.

use crate::formatters::Formatter;
use crate::utils::types::FormatResult;
use crate::{Language, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Ruby formatter using RuboCop with --autocorrect.
pub struct RubyFormatter;

impl RubyFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RubyFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for RubyFormatter {
    fn name(&self) -> &str {
        "rubocop"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Ruby]
    }

    fn format(&self, path: &Path) -> Result<FormatResult> {
        // Read original content for comparison
        let original = fs::read_to_string(path).map_err(|e| {
            crate::LintisError::formatter("rubocop", path, format!("Failed to read file: {}", e))
        })?;

        // Run rubocop with --autocorrect flag
        let output = Command::new("rubocop")
            .args(["--autocorrect", "--format", "quiet"])
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::formatter("rubocop", path, format!("Failed to run: {}", e))
            })?;

        // RuboCop returns non-zero if there were offenses (even after correction)
        // So we don't check status.success() here

        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Only return error if it's a real error, not just warnings
            if stderr.contains("Error:") || stderr.contains("fatal") {
                return Ok(FormatResult::error(
                    path.to_path_buf(),
                    format!("rubocop failed: {}", stderr),
                ));
            }
        }

        // Read new content and compare
        let new_content = fs::read_to_string(path).map_err(|e| {
            crate::LintisError::formatter(
                "rubocop",
                path,
                format!("Failed to read formatted file: {}", e),
            )
        })?;

        if original == new_content {
            Ok(FormatResult::unchanged(path.to_path_buf()))
        } else {
            Ok(FormatResult::changed(path.to_path_buf()))
        }
    }

    fn check(&self, path: &Path) -> Result<bool> {
        // Run rubocop without --autocorrect to check if file needs formatting
        let output = Command::new("rubocop")
            .args(["--format", "simple"])
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::formatter("rubocop", path, format!("Failed to run: {}", e))
            })?;

        // If exit code is non-zero, file has offenses (needs formatting)
        Ok(!output.status.success())
    }

    fn is_available(&self) -> bool {
        Command::new("rubocop")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
