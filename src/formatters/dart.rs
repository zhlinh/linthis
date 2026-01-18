// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Dart language formatter using dart format.

use crate::formatters::Formatter;
use crate::utils::types::FormatResult;
use crate::{Language, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Dart formatter using dart format.
pub struct DartFormatter;

impl DartFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DartFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for DartFormatter {
    fn name(&self) -> &str {
        "dart-format"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Dart]
    }

    fn format(&self, path: &Path) -> Result<FormatResult> {
        // Read original content for comparison
        let original = fs::read_to_string(path).map_err(|e| {
            crate::LintisError::formatter("dart format", path, format!("Failed to read file: {}", e))
        })?;

        // Run dart format (in-place formatting)
        let output = Command::new("dart")
            .args(["format"])
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::formatter("dart format", path, format!("Failed to run: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(FormatResult::error(
                path.to_path_buf(),
                format!("dart format failed: {}", stderr),
            ));
        }

        // Read new content and compare
        let new_content = fs::read_to_string(path).map_err(|e| {
            crate::LintisError::formatter(
                "dart format",
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
        // Run dart format in check mode (--output=none --set-exit-if-changed)
        let output = Command::new("dart")
            .args(["format", "--output=none", "--set-exit-if-changed"])
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::formatter("dart format", path, format!("Failed to run: {}", e))
            })?;

        // Exit code 0 means file is formatted, non-zero means needs formatting
        Ok(!output.status.success())
    }

    fn is_available(&self) -> bool {
        Command::new("dart")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
