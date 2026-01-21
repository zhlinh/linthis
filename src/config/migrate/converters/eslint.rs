// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Convert ESLint config to linthis format.

use std::path::Path;

use crate::config::migrate::parsers::eslint::{self, ESLintConfig};
use crate::config::migrate::{ConversionResult, DetectedConfig, MigrationWarning, WarningSeverity};

/// Convert ESLint configuration to linthis format
pub(crate) fn convert(
    detected: &DetectedConfig,
    project_root: &Path,
    dry_run: bool,
) -> Result<ConversionResult, String> {
    // Parse the ESLint config
    let config = eslint::parse(&detected.path)?;

    // Generate the converted config content
    let eslint_js = generate_eslint_js(&config);

    let config_dir = project_root.join(".linthis/configs/javascript");
    let config_path = config_dir.join(".eslintrc.js");

    let mut result = ConversionResult {
        created_files: Vec::new(),
        changes: Vec::new(),
        warnings: Vec::new(),
    };

    // Record the change
    result.changes.push(format!(
        "Create {} (migrated from {})",
        config_path.display(),
        detected.path.display()
    ));

    // Add excludes to linthis config if present
    if !config.ignores.is_empty() {
        result.changes.push(format!(
            "Add {} exclude pattern(s) to config.toml",
            config.ignores.len()
        ));
    }

    // Check for ESLint flat config (v9+)
    let filename = detected
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if filename.starts_with("eslint.config") {
        result.warnings.push(MigrationWarning {
            source: "eslint".to_string(),
            message: "ESLint flat config (v9+) detected. The migrated config uses legacy format. \
                     Consider manually updating to flat config format for ESLint v9+."
                .to_string(),
            severity: WarningSeverity::Warning,
        });
    }

    if !dry_run {
        // Create directory structure
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create directory: {}", e))?;

        // Write ESLint config
        std::fs::write(&config_path, eslint_js)
            .map_err(|e| format!("Failed to write config: {}", e))?;
        result.created_files.push(config_path);

        // Update config.toml excludes if needed
        if !config.ignores.is_empty() {
            update_config_excludes(project_root, &config.ignores)?;
        }
    }

    Ok(result)
}

/// Generate ESLint JavaScript config file content
fn generate_eslint_js(config: &ESLintConfig) -> String {
    let mut lines = vec![
        "// Migrated from existing ESLint config by linthis".to_string(),
        "// Review and adjust as needed".to_string(),
        String::new(),
        "module.exports = {".to_string(),
    ];

    // Add env
    if !config.env.is_empty() {
        lines.push("  env: {".to_string());
        for env in &config.env {
            lines.push(format!("    '{}': true,", env));
        }
        lines.push("  },".to_string());
    }

    // Add extends
    if !config.extends.is_empty() {
        let extends_str = config
            .extends
            .iter()
            .map(|e| format!("'{}'", e))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("  extends: [{}],", extends_str));
    }

    // Add plugins
    if !config.plugins.is_empty() {
        let plugins_str = config
            .plugins
            .iter()
            .map(|p| format!("'{}'", p))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("  plugins: [{}],", plugins_str));
    }

    // Add parser if specified
    if let Some(ref parser) = config.parser {
        lines.push(format!("  parser: '{}',", parser));
    }

    // Add rules
    if !config.rules.is_empty() {
        lines.push("  rules: {".to_string());
        for (name, value) in &config.rules {
            lines.push(format!("    '{}': {},", name, value.to_js_string()));
        }
        lines.push("  },".to_string());
    }

    lines.push("};".to_string());

    lines.join("\n")
}

/// Update linthis config.toml with exclude patterns
fn update_config_excludes(project_root: &Path, excludes: &[String]) -> Result<(), String> {
    let config_path = project_root.join(".linthis/config.toml");

    // Create config directory if needed
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    // Read existing config or create new
    let content = if config_path.exists() {
        std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config: {}", e))?
    } else {
        String::new()
    };

    // Parse as TOML
    let mut doc: toml_edit::DocumentMut = content
        .parse()
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    // Get or create excludes array
    let excludes_array = doc
        .entry("excludes")
        .or_insert_with(|| toml_edit::Item::Value(toml_edit::Value::Array(toml_edit::Array::new())))
        .as_array_mut()
        .ok_or("excludes is not an array")?;

    // Add new excludes
    for pattern in excludes {
        // Check if already exists
        let exists = excludes_array
            .iter()
            .any(|v| v.as_str() == Some(pattern.as_str()));
        if !exists {
            excludes_array.push(pattern.as_str());
        }
    }

    // Write back
    std::fs::write(&config_path, doc.to_string())
        .map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_eslint_js() {
        let config = ESLintConfig {
            extends: vec!["eslint:recommended".to_string()],
            rules: vec![
                (
                    "semi".to_string(),
                    eslint::RuleValue::Error,
                ),
            ],
            env: vec!["browser".to_string(), "node".to_string()],
            plugins: vec![],
            parser: None,
            ignores: vec![],
        };

        let js = generate_eslint_js(&config);
        assert!(js.contains("extends: ['eslint:recommended']"));
        assert!(js.contains("'semi': 'error'"));
        assert!(js.contains("'browser': true"));
    }
}
