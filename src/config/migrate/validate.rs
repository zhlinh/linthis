// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Configuration validation and suggestion generation.

use std::path::Path;

use super::{MigrationResult, MigrationWarning, WarningSeverity};

/// Validation result
pub struct ValidationResult {
    pub warnings: Vec<MigrationWarning>,
    pub suggestions: Vec<String>,
}

/// Validate the migration and generate suggestions
pub fn validate_migration(
    result: &MigrationResult,
    project_root: &Path,
) -> Result<ValidationResult, String> {
    let mut validation = ValidationResult {
        warnings: Vec::new(),
        suggestions: Vec::new(),
    };

    // Check for conflicting configurations
    check_conflicting_configs(project_root, &mut validation)?;

    // Check for deprecated options
    check_deprecated_options(result, &mut validation);

    // Generate best practice suggestions
    generate_suggestions(project_root, result, &mut validation)?;

    Ok(validation)
}

/// Check for conflicting configurations
fn check_conflicting_configs(
    project_root: &Path,
    validation: &mut ValidationResult,
) -> Result<(), String> {
    // Check for multiple ESLint configs
    let eslint_configs = [
        ".eslintrc.js",
        ".eslintrc.json",
        ".eslintrc.yml",
        ".eslintrc",
        "eslint.config.js",
    ];
    let found_eslint: Vec<&str> = eslint_configs
        .iter()
        .filter(|c| project_root.join(c).exists())
        .copied()
        .collect();

    let linthis_eslint = project_root.join(".linthis/configs/javascript/.eslintrc.js");
    if found_eslint.len() > 1 && linthis_eslint.exists() {
        validation.warnings.push(MigrationWarning {
            source: "eslint".to_string(),
            message: format!(
                "Multiple ESLint configs found ({}) plus migrated config. \
                 ESLint will use the first one found in its search order.",
                found_eslint.join(", ")
            ),
            severity: WarningSeverity::Warning,
        });
    }

    // Check for Black config conflicting with ruff format
    let pyproject = project_root.join("pyproject.toml");
    let linthis_ruff = project_root.join(".linthis/configs/python/ruff.toml");
    if pyproject.exists() && linthis_ruff.exists() {
        if let Ok(content) = std::fs::read_to_string(&pyproject) {
            if content.contains("[tool.black]") {
                validation.warnings.push(MigrationWarning {
                    source: "python".to_string(),
                    message: "Both Black (pyproject.toml) and ruff (migrated) are configured. \
                             Consider disabling Black to use ruff exclusively for formatting."
                        .to_string(),
                    severity: WarningSeverity::Warning,
                });
            }
        }
    }

    Ok(())
}

/// Check for deprecated options in migrated configs
fn check_deprecated_options(result: &MigrationResult, validation: &mut ValidationResult) {
    // Check for deprecated ESLint config format
    for change in &result.config_changes {
        if change.contains("eslint.config") {
            validation.suggestions.push(
                "ESLint flat config (eslint.config.js) was detected. Consider migrating to the \
                 new flat config format for better compatibility with ESLint v9+."
                    .to_string(),
            );
        }
    }
}

/// Generate best practice suggestions
fn generate_suggestions(
    project_root: &Path,
    result: &MigrationResult,
    validation: &mut ValidationResult,
) -> Result<(), String> {
    let linthis_config = project_root.join(".linthis/config.toml");

    // Suggest creating main config if missing
    if !linthis_config.exists() {
        validation.suggestions.push(
            "No .linthis/config.toml found. Run 'linthis init' to create a main configuration file."
                .to_string(),
        );
    }

    // Check for TypeScript projects without typescript-eslint
    let tsconfig = project_root.join("tsconfig.json");
    let eslint_config = project_root.join(".linthis/configs/typescript/.eslintrc.js");
    if tsconfig.exists() && eslint_config.exists() {
        if let Ok(content) = std::fs::read_to_string(&eslint_config) {
            if !content.contains("@typescript-eslint") {
                validation.suggestions.push(
                    "TypeScript project detected but @typescript-eslint is not configured. \
                     Consider adding '@typescript-eslint/eslint-plugin' for better TypeScript linting."
                        .to_string(),
                );
            }
        }
    }

    // Suggest running doctor after migration
    if !result.created_files.is_empty() || !result.config_changes.is_empty() {
        validation.suggestions.push(
            "Run 'linthis doctor' to verify all required tools are installed and configured."
                .to_string(),
        );
    }

    // Suggest removing old dependencies for Python
    let has_python_migration = result
        .created_files
        .iter()
        .any(|p| p.to_string_lossy().contains("python"));
    if has_python_migration {
        validation.suggestions.push(
            "If using ruff for Python formatting, consider removing Black and isort from your \
             dependencies to simplify your toolchain."
                .to_string(),
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_suggests_linthis_init() {
        let dir = TempDir::new().unwrap();
        let result = MigrationResult::default();

        let validation = validate_migration(&result, dir.path()).unwrap();
        assert!(validation
            .suggestions
            .iter()
            .any(|s| s.contains("linthis init")));
    }

    #[test]
    fn test_validate_warns_about_multiple_eslint_configs() {
        let dir = TempDir::new().unwrap();

        // Create multiple ESLint configs
        std::fs::write(dir.path().join(".eslintrc.js"), "").unwrap();
        std::fs::write(dir.path().join(".eslintrc.json"), "{}").unwrap();

        // Create linthis config
        let linthis_dir = dir.path().join(".linthis/configs/javascript");
        std::fs::create_dir_all(&linthis_dir).unwrap();
        std::fs::write(linthis_dir.join(".eslintrc.js"), "").unwrap();

        let result = MigrationResult::default();
        let validation = validate_migration(&result, dir.path()).unwrap();

        assert!(validation
            .warnings
            .iter()
            .any(|w| w.message.contains("Multiple ESLint configs")));
    }
}
