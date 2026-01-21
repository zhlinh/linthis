// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Convert Prettier config to linthis format.

use std::path::Path;

use crate::config::migrate::parsers::prettier;
use crate::config::migrate::{ConversionResult, DetectedConfig};

/// Convert Prettier configuration to linthis format
pub(crate) fn convert(
    detected: &DetectedConfig,
    project_root: &Path,
    dry_run: bool,
) -> Result<ConversionResult, String> {
    // Parse the Prettier config
    let config = prettier::parse(&detected.path)?;

    // Generate the converted config content
    let prettier_js = generate_prettier_js(&config);

    let config_dir = project_root.join(".linthis/configs/javascript");
    let config_path = config_dir.join("prettierrc.js");

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

    if !dry_run {
        // Create directory structure
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create directory: {}", e))?;

        // Write Prettier config
        std::fs::write(&config_path, prettier_js)
            .map_err(|e| format!("Failed to write config: {}", e))?;
        result.created_files.push(config_path);
    }

    Ok(result)
}

/// Generate Prettier JavaScript config file content
fn generate_prettier_js(config: &prettier::PrettierConfig) -> String {
    [
        "// Migrated from existing Prettier config by linthis",
        "// Review and adjust as needed",
        "",
        "module.exports = ",
        &config.to_js_object(),
        ";",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_prettier_js() {
        let config = prettier::PrettierConfig {
            tab_width: Some(4),
            semi: Some(true),
            single_quote: Some(true),
            ..Default::default()
        };

        let js = generate_prettier_js(&config);
        assert!(js.contains("tabWidth: 4"));
        assert!(js.contains("semi: true"));
        assert!(js.contains("singleQuote: true"));
    }
}
