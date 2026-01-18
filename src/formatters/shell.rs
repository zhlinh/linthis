// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Shell/Bash language formatter using shfmt.

use crate::formatters::Formatter;
use crate::utils::types::FormatResult;
use crate::{Language, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Shell formatter using shfmt.
pub struct ShellFormatter;

impl ShellFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ShellFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for ShellFormatter {
    fn name(&self) -> &str {
        "shfmt"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Shell]
    }

    fn format(&self, path: &Path) -> Result<FormatResult> {
        // Read original content for comparison
        let original = fs::read_to_string(path).map_err(|e| {
            crate::LintisError::formatter("shfmt", path, format!("Failed to read file: {}", e))
        })?;

        // Run shfmt with -w flag to format in-place
        let output = Command::new("shfmt")
            .arg("-w")
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::formatter("shfmt", path, format!("Failed to run: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(FormatResult::error(
                path.to_path_buf(),
                format!("shfmt failed: {}", stderr),
            ));
        }

        // Read new content and compare
        let new_content = fs::read_to_string(path).map_err(|e| {
            crate::LintisError::formatter(
                "shfmt",
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
        // Run shfmt with -d flag (diff mode) to check formatting
        let output = Command::new("shfmt")
            .arg("-d")
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::formatter("shfmt", path, format!("Failed to run: {}", e))
            })?;

        // If diff output is non-empty, file needs formatting
        Ok(!output.stdout.is_empty())
    }

    fn is_available(&self) -> bool {
        Command::new("shfmt")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
