// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! PHP language formatter using php-cs-fixer.

use crate::formatters::Formatter;
use crate::utils::types::FormatResult;
use crate::{Language, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// PHP formatter using php-cs-fixer.
pub struct PhpFormatter;

impl PhpFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PhpFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for PhpFormatter {
    fn name(&self) -> &str {
        "php-cs-fixer"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Php]
    }

    fn format(&self, path: &Path) -> Result<FormatResult> {
        // Read original content for comparison
        let original = fs::read_to_string(path).map_err(|e| {
            crate::LintisError::formatter(
                "php-cs-fixer",
                path,
                format!("Failed to read file: {}", e),
            )
        })?;

        // Run php-cs-fixer fix
        let output = Command::new("php-cs-fixer")
            .args(["fix", "--quiet"])
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::formatter("php-cs-fixer", path, format!("Failed to run: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // php-cs-fixer may return non-zero for config issues
            if !stderr.is_empty() && stderr.contains("Error") {
                return Ok(FormatResult::error(
                    path.to_path_buf(),
                    format!("php-cs-fixer failed: {}", stderr),
                ));
            }
        }

        // Read new content and compare
        let new_content = fs::read_to_string(path).map_err(|e| {
            crate::LintisError::formatter(
                "php-cs-fixer",
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
        // Run php-cs-fixer with --dry-run to check if file needs formatting
        let output = Command::new("php-cs-fixer")
            .args(["fix", "--dry-run", "--diff"])
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::formatter("php-cs-fixer", path, format!("Failed to run: {}", e))
            })?;

        // If there's diff output, file needs formatting
        Ok(!output.stdout.is_empty())
    }

    fn is_available(&self) -> bool {
        Command::new("php-cs-fixer")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
