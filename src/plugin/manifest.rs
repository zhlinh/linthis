// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Plugin manifest parsing and validation.
//!
//! The manifest file (`linthis-plugin.toml`) defines plugin metadata
//! and configuration file mappings for each language.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::{PluginError, Result};

/// Plugin manifest file name
pub const MANIFEST_FILENAME: &str = "linthis-plugin.toml";

/// Plugin manifest structure (linthis-plugin.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin metadata section
    pub plugin: PluginMetadata,
    /// Configuration mappings by language
    #[serde(default)]
    pub configs: HashMap<String, HashMap<String, String>>,
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin name
    pub name: String,
    /// Plugin version (semver)
    pub version: String,
    /// Short description
    #[serde(default)]
    pub description: String,
    /// Minimum linthis version required (e.g., ">=0.2.0")
    #[serde(default)]
    pub linthis_version: Option<String>,
    /// Supported languages
    #[serde(default)]
    pub languages: Vec<String>,
    /// License identifier
    #[serde(default)]
    pub license: Option<String>,
    /// Plugin authors
    #[serde(default)]
    pub authors: Vec<Author>,
}

/// Plugin author information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
}

impl PluginManifest {
    /// Load manifest from a plugin directory
    pub fn load(plugin_path: &Path) -> Result<Self> {
        let manifest_path = plugin_path.join(MANIFEST_FILENAME);

        if !manifest_path.exists() {
            return Err(PluginError::InvalidManifest {
                path: manifest_path,
                message: "Manifest file not found".to_string(),
            });
        }

        let content = fs::read_to_string(&manifest_path)?;
        Self::parse(&content, &manifest_path)
    }

    /// Parse manifest from TOML content
    pub fn parse(content: &str, path: &Path) -> Result<Self> {
        // First try standard format
        if let Ok(manifest) = toml::from_str::<PluginManifest>(content) {
            if !manifest.configs.is_empty() {
                return Ok(manifest);
            }
        }

        // Try to parse extended format with ["language.xxx"] sections
        Self::parse_extended_format(content, path)
    }

    /// Parse extended manifest format with ["language.xxx".tools.yyy] sections
    fn parse_extended_format(content: &str, path: &Path) -> Result<Self> {
        let value: toml::Value = toml::from_str(content).map_err(|e| PluginError::InvalidManifest {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;

        // Parse plugin metadata
        let plugin_table = value.get("plugin").ok_or_else(|| PluginError::InvalidManifest {
            path: path.to_path_buf(),
            message: "Missing [plugin] section".to_string(),
        })?;

        let plugin: PluginMetadata = plugin_table
            .clone()
            .try_into()
            .map_err(|e: toml::de::Error| PluginError::InvalidManifest {
                path: path.to_path_buf(),
                message: format!("Invalid plugin metadata: {}", e),
            })?;

        // Parse ["language.xxx"] sections
        let mut configs: HashMap<String, HashMap<String, String>> = HashMap::new();

        if let Some(table) = value.as_table() {
            for (key, section) in table {
                // Match keys like "language.cpp", "language.python", etc.
                if let Some(lang) = key.strip_prefix("language.") {
                    if let Some(tools_section) = section.get("tools") {
                        if let Some(tools_table) = tools_section.as_table() {
                            let lang_configs = configs.entry(lang.to_string()).or_default();

                            for (tool_name, tool_config) in tools_table {
                                // Get files array
                                if let Some(files) = tool_config.get("files") {
                                    if let Some(files_array) = files.as_array() {
                                        // Use first file as the config path
                                        if let Some(first_file) = files_array.first() {
                                            if let Some(file_path) = first_file.as_str() {
                                                // Prepend language directory to path
                                                let full_path = format!("{}/{}", lang, file_path);
                                                lang_configs.insert(tool_name.clone(), full_path);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(Self { plugin, configs })
    }

    /// Validate manifest contents
    pub fn validate(&self, plugin_path: &Path) -> Result<()> {
        // Check required fields
        if self.plugin.name.is_empty() {
            return Err(PluginError::InvalidManifest {
                path: plugin_path.join(MANIFEST_FILENAME),
                message: "Plugin name is required".to_string(),
            });
        }

        if self.plugin.version.is_empty() {
            return Err(PluginError::InvalidManifest {
                path: plugin_path.join(MANIFEST_FILENAME),
                message: "Plugin version is required".to_string(),
            });
        }

        // Validate config file paths exist
        for (lang, tools) in &self.configs {
            for (tool, config_path) in tools {
                let full_path = plugin_path.join(config_path);
                if !full_path.exists() {
                    return Err(PluginError::InvalidManifest {
                        path: plugin_path.join(MANIFEST_FILENAME),
                        message: format!(
                            "Config file not found: {} (for {}/{})",
                            config_path, lang, tool
                        ),
                    });
                }
            }
        }

        Ok(())
    }

    /// Get config path for a specific language and tool
    pub fn get_config_path(&self, language: &str, tool: &str) -> Option<&String> {
        self.configs.get(language).and_then(|tools| tools.get(tool))
    }

    /// Get all config paths for a language
    pub fn get_language_configs(&self, language: &str) -> Option<&HashMap<String, String>> {
        self.configs.get(language)
    }

    /// Check if plugin supports a language
    pub fn supports_language(&self, language: &str) -> bool {
        self.configs.contains_key(language)
    }

    /// Create a minimal manifest for scaffolding
    pub fn scaffold(name: &str) -> Self {
        Self {
            plugin: PluginMetadata {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                description: format!("{} configuration plugin for linthis", name),
                linthis_version: Some(">=0.2.0".to_string()),
                languages: vec![
                    "rust".to_string(),
                    "python".to_string(),
                    "typescript".to_string(),
                ],
                license: Some("MIT".to_string()),
                authors: vec![Author {
                    name: "Your Name".to_string(),
                    email: Some("you@example.com".to_string()),
                }],
            },
            configs: HashMap::new(),
        }
    }

    /// Serialize manifest to TOML string
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self).map_err(|e| PluginError::InvalidManifest {
            path: std::path::PathBuf::from(MANIFEST_FILENAME),
            message: format!("Failed to serialize manifest: {}", e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_manifest() {
        let content = r#"
[plugin]
name = "test-plugin"
version = "1.0.0"
"#;
        let manifest = PluginManifest::parse(content, Path::new("test")).unwrap();
        assert_eq!(manifest.plugin.name, "test-plugin");
        assert_eq!(manifest.plugin.version, "1.0.0");
    }

    #[test]
    fn test_parse_full_manifest() {
        let content = r#"
[plugin]
name = "official"
version = "1.0.0"
description = "Official linthis configurations"
linthis_version = ">=0.2.0"
languages = ["rust", "python"]
license = "MIT"

[[plugin.authors]]
name = "Test Author"
email = "test@example.com"

[configs.rust]
clippy = "rust/clippy.toml"
rustfmt = "rust/rustfmt.toml"

[configs.python]
ruff = "python/ruff.toml"
"#;
        let manifest = PluginManifest::parse(content, Path::new("test")).unwrap();
        assert_eq!(manifest.plugin.name, "official");
        assert_eq!(manifest.plugin.languages, vec!["rust", "python"]);
        assert_eq!(
            manifest.get_config_path("rust", "clippy"),
            Some(&"rust/clippy.toml".to_string())
        );
    }

    #[test]
    fn test_scaffold_manifest() {
        let manifest = PluginManifest::scaffold("my-config");
        assert_eq!(manifest.plugin.name, "my-config");
        assert_eq!(manifest.plugin.version, "0.1.0");
    }
}
