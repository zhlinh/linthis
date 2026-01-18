// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Convert Black and isort configs to ruff format.

use std::path::Path;

use crate::config::migrate::parsers::python::{self, BlackConfig, IsortConfig};
use crate::config::migrate::{ConversionResult, DetectedConfig, MigrationWarning, WarningSeverity};

/// Convert Black configuration to ruff format
pub(crate) fn convert_black(
    detected: &DetectedConfig,
    project_root: &Path,
    dry_run: bool,
) -> Result<ConversionResult, String> {
    // Parse the Black config
    let black_config = python::parse_black(&detected.path)?;

    // Check if isort is also present (we'll merge them)
    let isort_config = python::parse_isort(&detected.path).ok();

    // Generate the converted config content
    let ruff_toml = generate_ruff_toml(&black_config, isort_config.as_ref());

    let config_dir = project_root.join(".linthis/configs/python");
    let config_path = config_dir.join("ruff.toml");

    let mut result = ConversionResult {
        created_files: Vec::new(),
        changes: Vec::new(),
        warnings: Vec::new(),
    };

    // Record the change
    result.changes.push(format!(
        "Create {} (migrated from Black config in {})",
        config_path.display(),
        detected.path.display()
    ));

    // Add warning about Black replacement
    result.warnings.push(MigrationWarning {
        source: "python".to_string(),
        message: "Black config has been converted to ruff format. Consider removing Black from \
                 your dependencies as ruff provides the same formatting functionality."
            .to_string(),
        severity: WarningSeverity::Info,
    });

    // Warn about unsupported options
    if black_config.skip_string_normalization {
        result.warnings.push(MigrationWarning {
            source: "python".to_string(),
            message: "Black's 'skip-string-normalization' is not directly supported by ruff. \
                     The migrated config uses double quotes (ruff default)."
                .to_string(),
            severity: WarningSeverity::Warning,
        });
    }

    if !dry_run {
        // Create directory structure
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create directory: {}", e))?;

        // Write ruff config
        std::fs::write(&config_path, ruff_toml)
            .map_err(|e| format!("Failed to write config: {}", e))?;
        result.created_files.push(config_path);
    }

    Ok(result)
}

/// Convert isort configuration to ruff format
pub(crate) fn convert_isort(
    detected: &DetectedConfig,
    project_root: &Path,
    dry_run: bool,
) -> Result<ConversionResult, String> {
    // Parse the isort config
    let isort_config = python::parse_isort(&detected.path)?;

    // Check if Black is also present (we'll merge them)
    let black_config = python::parse_black(&detected.path).ok();

    // Generate the converted config content
    let ruff_toml = generate_ruff_toml(black_config.as_ref().unwrap_or(&BlackConfig::default()), Some(&isort_config));

    let config_dir = project_root.join(".linthis/configs/python");
    let config_path = config_dir.join("ruff.toml");

    let mut result = ConversionResult {
        created_files: Vec::new(),
        changes: Vec::new(),
        warnings: Vec::new(),
    };

    // Record the change
    result.changes.push(format!(
        "Create {} (migrated from isort config in {})",
        config_path.display(),
        detected.path.display()
    ));

    // Add warning about isort replacement
    result.warnings.push(MigrationWarning {
        source: "python".to_string(),
        message: "isort config has been converted to ruff format. Consider removing isort from \
                 your dependencies as ruff provides the same import sorting functionality."
            .to_string(),
        severity: WarningSeverity::Info,
    });

    if !dry_run {
        // Create directory structure
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create directory: {}", e))?;

        // Write ruff config
        std::fs::write(&config_path, ruff_toml)
            .map_err(|e| format!("Failed to write config: {}", e))?;
        result.created_files.push(config_path);
    }

    Ok(result)
}

/// Generate ruff.toml content from Black and isort configs
fn generate_ruff_toml(black: &BlackConfig, isort: Option<&IsortConfig>) -> String {
    let mut lines = Vec::new();
    lines.push("# Migrated from Black/isort config by linthis".to_string());
    lines.push("# Review and adjust as needed".to_string());
    lines.push(String::new());

    // Line length (prefer Black's setting)
    let line_length = black
        .line_length
        .or_else(|| isort.and_then(|i| i.line_length))
        .unwrap_or(88); // Black's default
    lines.push(format!("line-length = {}", line_length));

    // Target version from Black
    if !black.target_version.is_empty() {
        // Convert black's py38, py39 to ruff's format (uppercase)
        let ruff_version = black
            .target_version
            .first()
            .map(|v| v.to_uppercase())
            .unwrap_or_else(|| "PY38".to_string());
        lines.push(format!("target-version = \"{}\"", ruff_version));
    }

    lines.push(String::new());

    // Ruff lint section
    lines.push("[lint]".to_string());
    lines.push("# Enable common rules plus isort".to_string());
    lines.push("select = [\"E\", \"F\", \"W\", \"I\"]".to_string());
    lines.push(String::new());

    // Ruff format section (from Black settings)
    lines.push("[format]".to_string());
    lines.push("quote-style = \"double\"".to_string());
    lines.push("indent-style = \"space\"".to_string());

    if black.skip_magic_trailing_comma {
        lines.push("skip-magic-trailing-comma = true".to_string());
    }

    // isort section
    if let Some(isort) = isort {
        lines.push(String::new());
        lines.push("[lint.isort]".to_string());

        // Profile (black profile is common)
        if let Some(ref profile) = isort.profile {
            if profile == "black" {
                lines.push("# Using black-compatible import sorting".to_string());
            }
        }

        if !isort.known_first_party.is_empty() {
            let parties = isort
                .known_first_party
                .iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("known-first-party = [{}]", parties));
        }

        if !isort.known_third_party.is_empty() {
            let parties = isort
                .known_third_party
                .iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("known-third-party = [{}]", parties));
        }

        if isort.force_single_line {
            lines.push("force-single-line = true".to_string());
        }

        if isort.combine_as_imports {
            lines.push("combine-as-imports = true".to_string());
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ruff_toml_from_black() {
        let black = BlackConfig {
            line_length: Some(100),
            target_version: vec!["py39".to_string()],
            ..Default::default()
        };

        let toml = generate_ruff_toml(&black, None);
        assert!(toml.contains("line-length = 100"));
        assert!(toml.contains("target-version = \"PY39\""));
        assert!(toml.contains("quote-style = \"double\""));
    }

    #[test]
    fn test_generate_ruff_toml_with_isort() {
        let black = BlackConfig {
            line_length: Some(100),
            ..Default::default()
        };

        let isort = IsortConfig {
            profile: Some("black".to_string()),
            known_first_party: vec!["myapp".to_string(), "mylib".to_string()],
            force_single_line: true,
            ..Default::default()
        };

        let toml = generate_ruff_toml(&black, Some(&isort));
        assert!(toml.contains("line-length = 100"));
        assert!(toml.contains("[lint.isort]"));
        assert!(toml.contains("known-first-party = [\"myapp\", \"mylib\"]"));
        assert!(toml.contains("force-single-line = true"));
    }
}
