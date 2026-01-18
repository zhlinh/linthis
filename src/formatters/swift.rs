// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Swift language formatter using swift-format.

use crate::formatters::Formatter;
use crate::utils::types::FormatResult;
use crate::{Language, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Swift formatter using swift-format.
pub struct SwiftFormatter;

impl SwiftFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SwiftFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for SwiftFormatter {
    fn name(&self) -> &str {
        "swift-format"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Swift]
    }

    fn format(&self, path: &Path) -> Result<FormatResult> {
        // Read original content for comparison
        let original = fs::read_to_string(path).map_err(|e| {
            crate::LintisError::formatter(
                "swift-format",
                path,
                format!("Failed to read file: {}", e),
            )
        })?;

        // Run swift-format in-place
        let output = Command::new("swift-format")
            .args(["--in-place"])
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::formatter("swift-format", path, format!("Failed to run: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(FormatResult::error(
                path.to_path_buf(),
                format!("swift-format failed: {}", stderr),
            ));
        }

        // Read new content and compare
        let new_content = fs::read_to_string(path).map_err(|e| {
            crate::LintisError::formatter(
                "swift-format",
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
        // swift-format doesn't have a built-in check mode, so we compare output
        let original = fs::read_to_string(path).map_err(|e| {
            crate::LintisError::formatter(
                "swift-format",
                path,
                format!("Failed to read file: {}", e),
            )
        })?;

        // Run swift-format and capture output (without --in-place)
        let output = Command::new("swift-format")
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::formatter("swift-format", path, format!("Failed to run: {}", e))
            })?;

        if !output.status.success() {
            // If swift-format fails, consider it as needing formatting
            return Ok(true);
        }

        let formatted = String::from_utf8_lossy(&output.stdout);
        Ok(original != formatted.as_ref())
    }

    fn is_available(&self) -> bool {
        Command::new("swift-format")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
