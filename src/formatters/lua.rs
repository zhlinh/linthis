// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Lua language formatter using stylua.

use crate::formatters::Formatter;
use crate::utils::types::FormatResult;
use crate::{Language, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Lua formatter using stylua.
pub struct LuaFormatter;

impl LuaFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LuaFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for LuaFormatter {
    fn name(&self) -> &str {
        "stylua"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Lua]
    }

    fn format(&self, path: &Path) -> Result<FormatResult> {
        // Read original content for comparison
        let original = fs::read_to_string(path).map_err(|e| {
            crate::LintisError::formatter("stylua", path, format!("Failed to read file: {}", e))
        })?;

        // Run stylua (formats in-place by default)
        let output = Command::new("stylua")
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::formatter("stylua", path, format!("Failed to run: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(FormatResult::error(
                path.to_path_buf(),
                format!("stylua failed: {}", stderr),
            ));
        }

        // Read new content and compare
        let new_content = fs::read_to_string(path).map_err(|e| {
            crate::LintisError::formatter(
                "stylua",
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
        // Run stylua with --check flag
        let output = Command::new("stylua")
            .args(["--check"])
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::formatter("stylua", path, format!("Failed to run: {}", e))
            })?;

        // Exit code 0 means file is formatted, non-zero means needs formatting
        Ok(!output.status.success())
    }

    fn is_available(&self) -> bool {
        Command::new("stylua")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
