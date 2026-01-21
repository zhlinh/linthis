// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Configuration migration system for linthis.
//!
//! This module provides functionality to:
//! - Detect existing linter/formatter configurations in a project
//! - Parse configurations from ESLint, Prettier, Black, and isort
//! - Convert configurations to linthis format
//! - Validate migrated configurations

pub mod converters;
pub mod detect;
pub mod parsers;
pub mod validate;

use std::path::{Path, PathBuf};

/// Supported tools for migration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    ESLint,
    Prettier,
    Black,
    Isort,
}

impl Tool {
    /// Get the string representation of the tool
    pub fn as_str(&self) -> &'static str {
        match self {
            Tool::ESLint => "eslint",
            Tool::Prettier => "prettier",
            Tool::Black => "black",
            Tool::Isort => "isort",
        }
    }

    /// Parse tool from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "eslint" => Some(Tool::ESLint),
            "prettier" => Some(Tool::Prettier),
            "black" => Some(Tool::Black),
            "isort" => Some(Tool::Isort),
            _ => None,
        }
    }
}

/// Detected configuration with source info
#[derive(Debug)]
pub struct DetectedConfig {
    pub tool: Tool,
    pub path: PathBuf,
    pub language: String,
}

/// Warning severity levels
#[derive(Debug, Clone, Copy)]
pub enum WarningSeverity {
    Info,
    Warning,
    Error,
}

/// Warning generated during migration
#[derive(Debug)]
pub struct MigrationWarning {
    pub source: String,
    pub message: String,
    pub severity: WarningSeverity,
}

/// Result of a migration operation
#[derive(Debug, Default)]
pub struct MigrationResult {
    /// Files that were created
    pub created_files: Vec<PathBuf>,
    /// Files that were backed up
    pub backed_up_files: Vec<PathBuf>,
    /// Configuration changes applied
    pub config_changes: Vec<String>,
    /// Warnings generated during migration
    pub warnings: Vec<MigrationWarning>,
    /// Validation suggestions
    pub suggestions: Vec<String>,
}

/// Migration options
#[derive(Debug, Default)]
pub struct MigrationOptions {
    pub dry_run: bool,
    pub backup: bool,
    pub tool_filter: Option<Tool>,
    pub verbose: bool,
}

/// Main migration function
pub fn migrate_configs(
    project_root: &Path,
    options: &MigrationOptions,
) -> Result<MigrationResult, String> {
    let mut result = MigrationResult::default();

    // Step 1: Detect existing configurations
    let detected = detect::detect_configs(project_root, options.tool_filter)?;

    if detected.is_empty() {
        result.warnings.push(MigrationWarning {
            source: "detect".to_string(),
            message: "No supported configuration files found".to_string(),
            severity: WarningSeverity::Info,
        });
        return Ok(result);
    }

    // Step 2: Parse and convert each configuration
    for config in &detected {
        match convert_config(config, project_root, options) {
            Ok(conversion) => {
                if options.dry_run {
                    result.config_changes.extend(conversion.changes);
                } else {
                    // Backup if requested
                    if options.backup {
                        if let Some(backup_path) = backup_original(&config.path)? {
                            result.backed_up_files.push(backup_path);
                        }
                    }
                    // Apply changes
                    result.created_files.extend(conversion.created_files);
                    result.config_changes.extend(conversion.changes);
                }
                result.warnings.extend(conversion.warnings);
            }
            Err(e) => {
                result.warnings.push(MigrationWarning {
                    source: config.tool.as_str().to_string(),
                    message: format!("Failed to convert {}: {}", config.path.display(), e),
                    severity: WarningSeverity::Error,
                });
            }
        }
    }

    // Step 3: Validate and generate suggestions
    let validation = validate::validate_migration(&result, project_root)?;
    result.warnings.extend(validation.warnings);
    result.suggestions.extend(validation.suggestions);

    Ok(result)
}

/// Conversion result for a single config
pub(crate) struct ConversionResult {
    pub created_files: Vec<PathBuf>,
    pub changes: Vec<String>,
    pub warnings: Vec<MigrationWarning>,
}

/// Convert a detected config to linthis format
fn convert_config(
    config: &DetectedConfig,
    project_root: &Path,
    options: &MigrationOptions,
) -> Result<ConversionResult, String> {
    match config.tool {
        Tool::ESLint => converters::eslint::convert(config, project_root, options.dry_run),
        Tool::Prettier => converters::prettier::convert(config, project_root, options.dry_run),
        Tool::Black => converters::python::convert_black(config, project_root, options.dry_run),
        Tool::Isort => converters::python::convert_isort(config, project_root, options.dry_run),
    }
}

/// Backup original config file
fn backup_original(path: &Path) -> Result<Option<PathBuf>, String> {
    if path.exists() {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let backup_path = if ext.is_empty() {
            path.with_extension("backup")
        } else {
            path.with_extension(format!("{}.backup", ext))
        };
        std::fs::copy(path, &backup_path)
            .map_err(|e| format!("Failed to backup {}: {}", path.display(), e))?;
        Ok(Some(backup_path))
    } else {
        Ok(None)
    }
}
