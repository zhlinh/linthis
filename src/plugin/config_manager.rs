// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Configuration file manager for plugin add/remove operations.
//!
//! This module provides functionality to:
//! - Add plugins to project or global configuration
//! - Remove plugins from configuration
//! - List configured plugins
//! - Preserve TOML formatting using toml_edit

use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use toml_edit::{value, Array, DocumentMut, InlineTable, Item, Table};

/// Manages plugin configuration in .linthis.toml files
pub struct PluginConfigManager {
    config_path: PathBuf,
}

impl PluginConfigManager {
    /// Create a manager for project-level configuration (.linthis.toml in current directory)
    pub fn project() -> Result<Self> {
        let config_path = std::env::current_dir()
            .context("Failed to get current directory")?
            .join(".linthis.toml");
        Ok(Self { config_path })
    }

    /// Create a manager for global configuration (~/.linthis/config.toml)
    pub fn global() -> Result<Self> {
        let config_dir = directories::ProjectDirs::from("", "", "linthis")
            .ok_or_else(|| anyhow!("Cannot determine config directory"))?
            .config_dir()
            .to_path_buf();

        let config_path = config_dir.join("config.toml");
        Ok(Self { config_path })
    }

    /// Get the configuration file path
    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    /// Read configuration file as a TOML document
    fn read_config(&self) -> Result<DocumentMut> {
        if !self.config_path.exists() {
            // Return empty document if file doesn't exist
            return Ok(DocumentMut::new());
        }

        let content = std::fs::read_to_string(&self.config_path)
            .with_context(|| format!("Failed to read config file: {}", self.config_path.display()))?;

        content
            .parse::<DocumentMut>()
            .with_context(|| format!("Failed to parse TOML: {}", self.config_path.display()))
    }

    /// Write configuration document to file
    fn write_config(&self, doc: &DocumentMut) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        std::fs::write(&self.config_path, doc.to_string())
            .with_context(|| format!("Failed to write config file: {}", self.config_path.display()))
    }

    /// Add a plugin to the configuration
    ///
    /// # Arguments
    /// * `alias` - Unique alias for the plugin
    /// * `url` - Git repository URL
    /// * `git_ref` - Optional git ref (branch, tag, or commit)
    ///
    /// # Errors
    /// Returns error if:
    /// - Alias already exists in this configuration
    /// - Failed to read/write configuration file
    pub fn add_plugin(&self, alias: &str, url: &str, git_ref: Option<&str>) -> Result<()> {
        let mut doc = self.read_config()?;

        // Ensure [plugin] table exists
        if !doc.contains_key("plugin") {
            doc["plugin"] = Item::Table(Table::new());
        }

        let plugin_table = doc["plugin"]
            .as_table_mut()
            .ok_or_else(|| anyhow!("'plugin' is not a table"))?;

        // Ensure sources array exists
        if !plugin_table.contains_key("sources") {
            plugin_table["sources"] = value(Array::new());
        }

        let sources = plugin_table["sources"]
            .as_array_mut()
            .ok_or_else(|| anyhow!("'plugin.sources' is not an array"))?;

        // Check if alias already exists
        if self.alias_exists(sources, alias) {
            return Err(anyhow!(
                "Plugin alias '{}' already exists in {}",
                alias,
                self.config_path.display()
            ));
        }

        // Create new plugin entry as inline table
        let mut plugin_entry = InlineTable::new();
        plugin_entry.insert("name", alias.into());
        plugin_entry.insert("url", url.into());
        if let Some(ref_) = git_ref {
            plugin_entry.insert("ref", ref_.into());
        }

        // Add to sources array
        sources.push(plugin_entry);

        // Write back to file
        self.write_config(&doc)?;
        Ok(())
    }

    /// Remove a plugin from configuration by alias
    ///
    /// # Returns
    /// - `Ok(true)` if plugin was found and removed
    /// - `Ok(false)` if plugin alias was not found
    /// - `Err` if failed to read/write configuration
    pub fn remove_plugin(&self, alias: &str) -> Result<bool> {
        let mut doc = self.read_config()?;

        let plugin_table = doc
            .get_mut("plugin")
            .and_then(|item| item.as_table_mut())
            .ok_or_else(|| anyhow!("No [plugin] section found in configuration"))?;

        let sources = plugin_table
            .get_mut("sources")
            .and_then(|item| item.as_array_mut())
            .ok_or_else(|| anyhow!("No plugin.sources array found in configuration"))?;

        let original_len = sources.len();

        // Remove matching plugin entries
        sources.retain(|item| {
            // Check inline table format: { name = "alias", url = "..." }
            if let Some(table) = item.as_inline_table() {
                if let Some(name) = table.get("name") {
                    return name.as_str() != Some(alias);
                }
            }
            // Keep items that don't match
            true
        });

        let removed = sources.len() < original_len;

        if removed {
            self.write_config(&doc)?;
        }

        Ok(removed)
    }

    /// Check if an alias already exists in the sources array
    fn alias_exists(&self, sources: &Array, alias: &str) -> bool {
        sources.iter().any(|item| {
            if let Some(table) = item.as_inline_table() {
                if let Some(name) = table.get("name") {
                    return name.as_str() == Some(alias);
                }
            }
            false
        })
    }

    /// List all configured plugins
    ///
    /// # Returns
    /// Vector of (alias, url, optional_ref) tuples
    pub fn list_plugins(&self) -> Result<Vec<(String, String, Option<String>)>> {
        let doc = self.read_config()?;
        let mut plugins = Vec::new();

        if let Some(sources) = doc
            .get("plugin")
            .and_then(|p| p.get("sources"))
            .and_then(|s| s.as_array())
        {
            for item in sources.iter() {
                if let Some(table) = item.as_inline_table() {
                    let name = table.get("name").and_then(|n| n.as_str());
                    let url = table.get("url").and_then(|u| u.as_str());
                    let ref_ = table.get("ref").and_then(|r| r.as_str());

                    if let (Some(name), Some(url)) = (name, url) {
                        plugins.push((
                            name.to_string(),
                            url.to_string(),
                            ref_.map(|s| s.to_string()),
                        ));
                    }
                }
            }
        }

        Ok(plugins)
    }

    /// Get plugin URL and ref by alias
    ///
    /// # Returns
    /// - `Ok(Some((url, optional_ref)))` if alias found
    /// - `Ok(None)` if alias not found
    /// - `Err` if failed to read configuration
    pub fn get_plugin_by_alias(&self, alias: &str) -> Result<Option<(String, Option<String>)>> {
        let plugins = self.list_plugins()?;
        Ok(plugins
            .into_iter()
            .find(|(name, _, _)| name == alias)
            .map(|(_, url, ref_)| (url, ref_)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_temp_manager() -> (PluginConfigManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".linthis.toml");
        let manager = PluginConfigManager { config_path };
        (manager, temp_dir)
    }

    #[test]
    fn test_add_plugin() {
        let (manager, _temp) = create_temp_manager();

        // Add first plugin
        manager
            .add_plugin("test", "https://example.com/test.git", None)
            .unwrap();

        // Verify it was added
        let plugins = manager.list_plugins().unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].0, "test");
        assert_eq!(plugins[0].1, "https://example.com/test.git");
    }

    #[test]
    fn test_add_plugin_with_ref() {
        let (manager, _temp) = create_temp_manager();

        manager
            .add_plugin("test", "https://example.com/test.git", Some("v1.0.0"))
            .unwrap();

        let plugins = manager.list_plugins().unwrap();
        assert_eq!(plugins[0].2, Some("v1.0.0".to_string()));
    }

    #[test]
    fn test_add_duplicate_alias() {
        let (manager, _temp) = create_temp_manager();

        manager
            .add_plugin("test", "https://example.com/test.git", None)
            .unwrap();

        // Try to add same alias again
        let result = manager.add_plugin("test", "https://example.com/other.git", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_remove_plugin() {
        let (manager, _temp) = create_temp_manager();

        manager
            .add_plugin("test", "https://example.com/test.git", None)
            .unwrap();

        // Remove plugin
        let removed = manager.remove_plugin("test").unwrap();
        assert!(removed);

        // Verify it's gone
        let plugins = manager.list_plugins().unwrap();
        assert_eq!(plugins.len(), 0);
    }

    #[test]
    fn test_remove_nonexistent() {
        let (manager, _temp) = create_temp_manager();

        manager
            .add_plugin("test", "https://example.com/test.git", None)
            .unwrap();

        // Try to remove non-existent plugin
        let removed = manager.remove_plugin("nonexistent").unwrap();
        assert!(!removed);
    }

    #[test]
    fn test_get_plugin_by_alias() {
        let (manager, _temp) = create_temp_manager();

        manager
            .add_plugin("test", "https://example.com/test.git", Some("v1.0.0"))
            .unwrap();

        let result = manager.get_plugin_by_alias("test").unwrap();
        assert!(result.is_some());
        let (url, ref_) = result.unwrap();
        assert_eq!(url, "https://example.com/test.git");
        assert_eq!(ref_, Some("v1.0.0".to_string()));
    }
}
