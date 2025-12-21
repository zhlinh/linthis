// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Python language formatter using ruff.
//!
//! Ruff is an extremely fast Python formatter written in Rust, offering
//! 10-100x speed improvements over black with Black-compatible output.

use crate::formatters::Formatter;
use crate::utils::types::FormatResult;
use crate::{Language, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Python formatter using ruff.
pub struct PythonFormatter;

impl PythonFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PythonFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for PythonFormatter {
    fn name(&self) -> &str {
        "ruff"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Python]
    }

    fn format(&self, path: &Path) -> Result<FormatResult> {
        // Read original content for comparison
        let original = fs::read_to_string(path)
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to read file: {}", e)))?;

        // Run ruff format (uses project config or defaults)
        let output = Command::new("ruff")
            .args(["format"])
            .arg(path)
            .output()
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to run ruff: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(FormatResult::error(
                path.to_path_buf(),
                format!("ruff format failed: {}", stderr),
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
        // Run ruff format in check mode
        let output = Command::new("ruff")
            .args(["format", "--check"])
            .arg(path)
            .output()
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to run ruff: {}", e)))?;

        // Exit code 0 means file is formatted, non-zero means needs formatting
        Ok(!output.status.success())
    }

    fn is_available(&self) -> bool {
        Command::new("ruff")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
