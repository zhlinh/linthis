// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Go language formatter using gofmt.

use crate::formatters::Formatter;
use crate::utils::types::FormatResult;
use crate::{Language, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Go formatter using gofmt.
pub struct GoFormatter;

impl GoFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GoFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for GoFormatter {
    fn name(&self) -> &str {
        "gofmt"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Go]
    }

    fn format(&self, path: &Path) -> Result<FormatResult> {
        // Read original content for comparison
        let original = fs::read_to_string(path)
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to read file: {}", e)))?;

        // Run gofmt (writes to stdout by default, use -w to write to file)
        let output = Command::new("gofmt")
            .args(["-w"])
            .arg(path)
            .output()
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to run gofmt: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(FormatResult::error(
                path.to_path_buf(),
                format!("gofmt failed: {}", stderr),
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
        // Run gofmt in check mode (-l lists files that need formatting)
        let output = Command::new("gofmt")
            .args(["-l"])
            .arg(path)
            .output()
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to run gofmt: {}", e)))?;

        // If output is non-empty, file needs formatting
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(!stdout.trim().is_empty())
    }

    fn is_available(&self) -> bool {
        Command::new("gofmt")
            .arg("-h")
            .output()
            .map(|_| true) // gofmt -h returns error but we just check if command exists
            .unwrap_or(false)
    }
}
