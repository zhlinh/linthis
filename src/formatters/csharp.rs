// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! C# language formatter using dotnet format.

use crate::formatters::Formatter;
use crate::utils::types::FormatResult;
use crate::{Language, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// C# formatter using dotnet format.
pub struct CSharpFormatter;

impl CSharpFormatter {
    pub fn new() -> Self {
        Self
    }

    /// Find the project/solution file for the given source file.
    fn find_project_file(&self, path: &Path) -> Option<std::path::PathBuf> {
        let mut current = path.parent()?;

        loop {
            // Check for .csproj files
            if let Ok(entries) = std::fs::read_dir(current) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let entry_path = entry.path();
                    if let Some(ext) = entry_path.extension() {
                        if ext == "csproj" || ext == "sln" {
                            return Some(entry_path);
                        }
                    }
                }
            }

            // Move up to parent directory
            current = current.parent()?;
        }
    }
}

impl Default for CSharpFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for CSharpFormatter {
    fn name(&self) -> &str {
        "dotnet-format"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::CSharp]
    }

    fn format(&self, path: &Path) -> Result<FormatResult> {
        // Read original content for comparison
        let original = fs::read_to_string(path).map_err(|e| {
            crate::LintisError::formatter(
                "dotnet-format",
                path,
                format!("Failed to read file: {}", e),
            )
        })?;

        // Find project file for context
        let project_file = self.find_project_file(path);

        let output = if let Some(ref proj) = project_file {
            // Run dotnet format on the project
            Command::new("dotnet")
                .args(["format"])
                .arg(proj)
                .output()
                .map_err(|e| {
                    crate::LintisError::formatter(
                        "dotnet-format",
                        path,
                        format!("Failed to run: {}", e),
                    )
                })?
        } else {
            // Try to run on single file (may fail without project context)
            Command::new("dotnet")
                .args(["format", "--include"])
                .arg(path)
                .output()
                .map_err(|e| {
                    crate::LintisError::formatter(
                        "dotnet-format",
                        path,
                        format!("Failed to run: {}", e),
                    )
                })?
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Only return error for actual failures
            if stderr.contains("error") || stderr.contains("Error") {
                return Ok(FormatResult::error(
                    path.to_path_buf(),
                    format!("dotnet-format failed: {}", stderr),
                ));
            }
        }

        // Read new content and compare
        let new_content = fs::read_to_string(path).map_err(|e| {
            crate::LintisError::formatter(
                "dotnet-format",
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
        // Find project file for context
        let project_file = self.find_project_file(path);

        let output = if let Some(ref proj) = project_file {
            // Run dotnet format with --verify-no-changes
            Command::new("dotnet")
                .args(["format", "--verify-no-changes"])
                .arg(proj)
                .output()
                .map_err(|e| {
                    crate::LintisError::formatter(
                        "dotnet-format",
                        path,
                        format!("Failed to run: {}", e),
                    )
                })?
        } else {
            // Try to run on single file
            Command::new("dotnet")
                .args(["format", "--verify-no-changes", "--include"])
                .arg(path)
                .output()
                .map_err(|e| {
                    crate::LintisError::formatter(
                        "dotnet-format",
                        path,
                        format!("Failed to run: {}", e),
                    )
                })?
        };

        // Exit code 0 means file is formatted, non-zero means needs formatting
        Ok(!output.status.success())
    }

    fn is_available(&self) -> bool {
        Command::new("dotnet")
            .args(["format", "--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
