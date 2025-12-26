// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Plugin registry for resolving short names to Git URLs.
//!
//! The registry provides a mapping of well-known plugin names
//! to their Git repository URLs.

use std::collections::HashMap;

use super::{PluginError, PluginSource, Result};

/// Built-in plugin registry entry
#[derive(Debug, Clone)]
pub struct RegistryEntry {
    /// Git repository URL
    pub url: String,
    /// Description of the plugin
    pub description: String,
    /// Default Git ref (branch/tag)
    pub default_ref: Option<String>,
}

/// Plugin registry for resolving short names to URLs
#[derive(Debug, Clone)]
pub struct PluginRegistry {
    entries: HashMap<String, RegistryEntry>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    /// Create a new registry with built-in plugins
    pub fn new() -> Self {
        Self {
            entries: builtin_registry(),
        }
    }

    /// Resolve a plugin source, looking up registry if needed
    pub fn resolve(&self, source: &PluginSource) -> Result<PluginSource> {
        // If URL is already provided, return as-is
        if source.url.is_some() {
            return Ok(source.clone());
        }

        // Look up in registry
        match self.entries.get(&source.name) {
            Some(entry) => Ok(PluginSource {
                name: source.name.clone(),
                url: Some(entry.url.clone()),
                git_ref: source.git_ref.clone().or_else(|| entry.default_ref.clone()),
                enabled: source.enabled,
            }),
            None => Err(PluginError::UnknownPlugin {
                name: source.name.clone(),
            }),
        }
    }

    /// Check if a name is in the registry
    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// Get registry entry by name
    pub fn get(&self, name: &str) -> Option<&RegistryEntry> {
        self.entries.get(name)
    }

    /// List all available plugin names
    pub fn list_names(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }

    /// List all registry entries
    pub fn list_all(&self) -> &HashMap<String, RegistryEntry> {
        &self.entries
    }
}

/// Create the built-in plugin registry
fn builtin_registry() -> HashMap<String, RegistryEntry> {
    let mut registry = HashMap::new();

    // Official linthis configuration plugin
    registry.insert(
        "official".to_string(),
        RegistryEntry {
            url: "https://github.com/zhlinh/linthis-config.git".to_string(),
            description: "Official linthis configuration with community best practices".to_string(),
            default_ref: Some("main".to_string()),
        },
    );

    registry
}

/// Convenience function to get the built-in registry
pub fn get_builtin_registry() -> PluginRegistry {
    PluginRegistry::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_registry_has_official() {
        let registry = PluginRegistry::new();
        assert!(registry.contains("official"));

        let entry = registry.get("official").unwrap();
        assert!(entry.url.contains("linthis-config"));
    }

    #[test]
    fn test_resolve_registry_name() {
        let registry = PluginRegistry::new();
        let source = PluginSource::new("official");

        let resolved = registry.resolve(&source).unwrap();
        assert!(resolved.url.is_some());
        assert!(resolved.url.unwrap().contains("linthis-config"));
    }

    #[test]
    fn test_resolve_url_passthrough() {
        let registry = PluginRegistry::new();
        let source = PluginSource::new("https://github.com/user/config.git");

        let resolved = registry.resolve(&source).unwrap();
        assert_eq!(
            resolved.url,
            Some("https://github.com/user/config.git".to_string())
        );
    }

    #[test]
    fn test_resolve_unknown_name() {
        let registry = PluginRegistry::new();
        let source = PluginSource::new("unknown-plugin");

        let result = registry.resolve(&source);
        assert!(matches!(result, Err(PluginError::UnknownPlugin { .. })));
    }
}
