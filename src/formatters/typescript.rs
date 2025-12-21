// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! TypeScript/JavaScript language formatter using prettier.

use crate::formatters::Formatter;
use crate::utils::types::FormatResult;
use crate::{Language, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// TypeScript/JavaScript formatter using prettier.
pub struct TypeScriptFormatter;

impl TypeScriptFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TypeScriptFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for TypeScriptFormatter {
    fn name(&self) -> &str {
        "prettier"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::TypeScript, Language::JavaScript]
    }

    fn format(&self, path: &Path) -> Result<FormatResult> {
        // Read original content for comparison
        let original = fs::read_to_string(path)
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to read file: {}", e)))?;

        // Run prettier
        let output = Command::new("prettier")
            .args(["--write"])
            .arg(path)
            .output()
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to run prettier: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(FormatResult::error(
                path.to_path_buf(),
                format!("prettier failed: {}", stderr),
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
        // Run prettier in check mode
        let output = Command::new("prettier")
            .args(["--check"])
            .arg(path)
            .output()
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to run prettier: {}", e)))?;

        // Exit code 0 means file is formatted, non-zero means needs formatting
        Ok(!output.status.success())
    }

    fn is_available(&self) -> bool {
        Command::new("prettier")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
