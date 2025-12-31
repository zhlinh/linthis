// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Plugin configuration loader.
//!
//! Handles loading and merging configurations from plugins,
//! including path resolution and config file access.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::cache::PluginCache;
use super::config_manager::PluginConfigManager;
use super::fetcher::PluginFetcher;
use super::manifest::PluginManifest;
use super::registry::PluginRegistry;
use super::{log_plugin_operation, PluginError, PluginSource, Result};

/// Loaded configuration from a plugin
#[derive(Debug, Clone)]
pub struct LoadedConfig {
    /// Plugin name this config came from
    pub plugin_name: String,
    /// Language this config applies to
    pub language: String,
    /// Tool this config applies to (e.g., "clippy", "ruff")
    pub tool: String,
    /// Full path to the config file
    pub config_path: PathBuf,
}

/// Plugin loader handles fetching and loading plugin configurations
#[derive(Debug)]
pub struct PluginLoader {
    cache: PluginCache,
    fetcher: PluginFetcher,
    registry: PluginRegistry,
    verbose: bool,
}

impl PluginLoader {
    /// Create a new plugin loader
    pub fn new() -> Result<Self> {
        Ok(Self {
            cache: PluginCache::new()?,
            fetcher: PluginFetcher::new(),
            registry: PluginRegistry::new(),
            verbose: false,
        })
    }

    /// Create a loader with verbose logging
    pub fn with_verbose(verbose: bool) -> Result<Self> {
        Ok(Self {
            cache: PluginCache::new()?,
            fetcher: PluginFetcher::with_verbose(verbose),
            registry: PluginRegistry::new(),
            verbose,
        })
    }

    /// Create a loader with custom components (for testing)
    pub fn with_components(cache: PluginCache, registry: PluginRegistry, verbose: bool) -> Self {
        Self {
            cache,
            fetcher: PluginFetcher::with_verbose(verbose),
            registry,
            verbose,
        }
    }

    /// Load configurations from multiple plugin sources
    ///
    /// Plugins are loaded in order, with later plugins overriding earlier ones.
    pub fn load_configs(
        &self,
        sources: &[PluginSource],
        force_update: bool,
    ) -> Result<Vec<LoadedConfig>> {
        let mut all_configs: HashMap<(String, String), LoadedConfig> = HashMap::new();

        for source in sources {
            // Skip disabled plugins
            if !source.enabled {
                log_plugin_operation(
                    "skip",
                    &format!("Plugin '{}' is disabled", source.name),
                    self.verbose,
                );
                continue;
            }

            // Try to load plugin configs
            match self.load_plugin_configs(source, force_update) {
                Ok(configs) => {
                    for config in configs {
                        // Later plugins override earlier ones (same language + tool)
                        let key = (config.language.clone(), config.tool.clone());
                        log_plugin_operation(
                            "load",
                            &format!(
                                "Loaded {}/{} from {}",
                                config.language, config.tool, config.plugin_name
                            ),
                            self.verbose,
                        );
                        all_configs.insert(key, config);
                    }
                }
                Err(e) => {
                    // Log error but continue with other plugins
                    log_plugin_operation(
                        "error",
                        &format!("Failed to load plugin '{}': {}", source.name, e),
                        true, // Always log errors
                    );

                    // Try to use cached version if available
                    if let Ok(configs) = self.load_from_cache_only(source) {
                        log_plugin_operation(
                            "fallback",
                            &format!("Using cached version of '{}'", source.name),
                            self.verbose,
                        );
                        for config in configs {
                            let key = (config.language.clone(), config.tool.clone());
                            all_configs.insert(key, config);
                        }
                    }
                }
            }
        }

        Ok(all_configs.into_values().collect())
    }

    /// Resolve alias to URL by looking up in project and global configurations
    fn resolve_alias(&self, source: &PluginSource) -> Result<PluginSource> {
        // If source already has a URL, no need to resolve
        if source.url.is_some() {
            return Ok(source.clone());
        }

        // Try to resolve alias from project config
        if let Ok(manager) = PluginConfigManager::project() {
            if let Ok(Some((url, ref_))) = manager.get_plugin_by_alias(&source.name) {
                log_plugin_operation(
                    "resolve",
                    &format!("Resolved alias '{}' from project config", source.name),
                    self.verbose,
                );
                return Ok(PluginSource {
                    name: source.name.clone(),
                    url: Some(url),
                    git_ref: ref_.or_else(|| source.git_ref.clone()),
                    enabled: source.enabled,
                });
            }
        }

        // Try to resolve alias from global config
        if let Ok(manager) = PluginConfigManager::global() {
            if let Ok(Some((url, ref_))) = manager.get_plugin_by_alias(&source.name) {
                log_plugin_operation(
                    "resolve",
                    &format!("Resolved alias '{}' from global config", source.name),
                    self.verbose,
                );
                return Ok(PluginSource {
                    name: source.name.clone(),
                    url: Some(url),
                    git_ref: ref_.or_else(|| source.git_ref.clone()),
                    enabled: source.enabled,
                });
            }
        }

        // Alias not found in configs, return original source unchanged
        // It will be handled by the registry resolver next
        Ok(source.clone())
    }

    /// Load configurations from a single plugin source
    fn load_plugin_configs(
        &self,
        source: &PluginSource,
        force_update: bool,
    ) -> Result<Vec<LoadedConfig>> {
        // First, try to resolve alias from configuration files
        let resolved_from_alias = self.resolve_alias(source)?;

        // Then, resolve registry name to URL if still needed
        let resolved_source = self.registry.resolve(&resolved_from_alias)?;

        // Fetch/update the plugin
        let cached = self
            .fetcher
            .fetch(&resolved_source, &self.cache, force_update)?;

        // Load manifest
        let manifest = PluginManifest::load(&cached.cache_path)?;

        // Extract all configs
        self.extract_configs(&manifest, &cached.cache_path)
    }

    /// Load configs from cache only (no network)
    fn load_from_cache_only(&self, source: &PluginSource) -> Result<Vec<LoadedConfig>> {
        // Resolve registry name to URL if needed
        let resolved_source = self.registry.resolve(source)?;

        // Load from cache
        let (cache_path, manifest) = self.cache.load_cached_plugin(&resolved_source)?;

        // Extract configs
        self.extract_configs(&manifest, &cache_path)
    }

    /// Extract configurations from a loaded manifest
    fn extract_configs(
        &self,
        manifest: &PluginManifest,
        plugin_path: &Path,
    ) -> Result<Vec<LoadedConfig>> {
        let mut configs = Vec::new();

        for (language, tools) in &manifest.configs {
            for (tool, config_rel_path) in tools {
                let config_path = plugin_path.join(config_rel_path);

                if !config_path.exists() {
                    return Err(PluginError::ConfigNotFound { path: config_path });
                }

                configs.push(LoadedConfig {
                    plugin_name: manifest.plugin.name.clone(),
                    language: language.clone(),
                    tool: tool.clone(),
                    config_path,
                });
            }
        }

        Ok(configs)
    }

    /// Get config file content for a specific language and tool
    pub fn get_config_content(
        &self,
        sources: &[PluginSource],
        language: &str,
        tool: &str,
    ) -> Result<Option<String>> {
        let configs = self.load_configs(sources, false)?;

        for config in configs {
            if config.language == language && config.tool == tool {
                let content = fs::read_to_string(&config.config_path)?;
                return Ok(Some(content));
            }
        }

        Ok(None)
    }

    /// Get config file path for a specific language and tool
    pub fn get_config_path(
        &self,
        sources: &[PluginSource],
        language: &str,
        tool: &str,
    ) -> Result<Option<PathBuf>> {
        let configs = self.load_configs(sources, false)?;

        for config in configs {
            if config.language == language && config.tool == tool {
                return Ok(Some(config.config_path));
            }
        }

        Ok(None)
    }

    /// Get the plugin cache
    pub fn cache(&self) -> &PluginCache {
        &self.cache
    }

    /// Get the plugin registry
    pub fn registry(&self) -> &PluginRegistry {
        &self.registry
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new().expect("Failed to create plugin loader")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_plugin(dir: &Path, name: &str) -> PathBuf {
        let plugin_dir = dir.join(name);
        fs::create_dir_all(&plugin_dir).unwrap();

        // Create manifest
        let manifest = format!(
            r#"
[plugin]
name = "{}"
version = "1.0.0"

[configs.rust]
clippy = "rust/clippy.toml"
"#,
            name
        );
        fs::write(plugin_dir.join("linthis-plugin.toml"), manifest).unwrap();

        // Create config file
        fs::create_dir_all(plugin_dir.join("rust")).unwrap();
        fs::write(plugin_dir.join("rust/clippy.toml"), "# clippy config").unwrap();

        plugin_dir
    }

    #[test]
    fn test_extract_configs() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_path = create_test_plugin(temp_dir.path(), "test-plugin");

        let manifest = PluginManifest::load(&plugin_path).unwrap();
        let cache = PluginCache::with_dir(temp_dir.path().to_path_buf());
        let loader = PluginLoader::with_components(cache, PluginRegistry::new(), false);

        let configs = loader.extract_configs(&manifest, &plugin_path).unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].language, "rust");
        assert_eq!(configs[0].tool, "clippy");
    }
}
