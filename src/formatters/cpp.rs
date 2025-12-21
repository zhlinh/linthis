// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! C/C++ language formatter using clang-format.

use crate::formatters::Formatter;
use crate::utils::types::FormatResult;
use crate::{Language, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// C/C++ formatter using clang-format.
pub struct CppFormatter;

impl CppFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CppFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for CppFormatter {
    fn name(&self) -> &str {
        "clang-format"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Cpp, Language::ObjectiveC]
    }

    fn format(&self, path: &Path) -> Result<FormatResult> {
        // Read original content for comparison
        let original = fs::read_to_string(path)
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to read file: {}", e)))?;

        // Run clang-format (-i modifies in place)
        let output = Command::new("clang-format")
            .args(["-i", "-style=Google"])
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

        // Run clang-format to get formatted output (without -i)
        let output = Command::new("clang-format")
            .args(["-style=Google"])
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
