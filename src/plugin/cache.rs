// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Plugin cache management.
//!
//! Handles local storage of fetched plugins, including:
//! - Cache directory resolution (platform-specific)
//! - Plugin existence checks
//! - Cache listing and cleanup
//! - File locking for concurrent access

use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use super::manifest::PluginManifest;
use super::{PluginError, PluginSource, Result};

/// Cache metadata file name
const CACHE_METADATA_FILE: &str = ".linthis-cache.json";
/// Lock file name for concurrent access
const CACHE_LOCK_FILE: &str = ".linthis-cache.lock";

/// Metadata for a cached plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPlugin {
    /// Plugin name
    pub name: String,
    /// Git repository URL
    pub url: String,
    /// Git ref that was checked out
    pub git_ref: Option<String>,
    /// Timestamp when cached
    pub cached_at: DateTime<Utc>,
    /// Timestamp of last update check
    pub last_updated: DateTime<Utc>,
    /// Local cache path
    pub cache_path: PathBuf,
}

/// Plugin cache manager
#[derive(Debug)]
pub struct PluginCache {
    /// Root cache directory
    cache_dir: PathBuf,
}

impl PluginCache {
    /// Create a new plugin cache using platform-specific directories
    pub fn new() -> Result<Self> {
        let cache_dir = Self::get_cache_dir()?;
        Ok(Self { cache_dir })
    }

    /// Create a plugin cache with a custom directory (for testing)
    pub fn with_dir(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Get the platform-specific cache directory
    ///
    /// Returns:
    /// - Linux: ~/.cache/linthis/plugins
    /// - macOS: ~/Library/Caches/linthis/plugins
    /// - Windows: C:\Users\<user>\AppData\Local\linthis\cache\plugins
    fn get_cache_dir() -> Result<PathBuf> {
        let proj_dirs =
            ProjectDirs::from("", "", "linthis").ok_or_else(|| PluginError::CacheError {
                message: "Could not determine cache directory for this platform".to_string(),
            })?;

        let cache_dir = proj_dirs.cache_dir().join("plugins");
        Ok(cache_dir)
    }

    /// Get the cache root directory
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Convert a plugin URL to a cache path
    ///
    /// Example: https://github.com/zhlinh/linthis-config.git
    ///       -> ~/.cache/linthis/plugins/github.com/zhlinh/linthis-config
    pub fn url_to_cache_path(&self, url: &str) -> PathBuf {
        let clean_url = url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("git@")
            .replace(':', "/")
            .trim_end_matches(".git")
            .to_string();

        self.cache_dir.join(clean_url)
    }

    /// Check if a plugin is cached
    pub fn is_cached(&self, source: &PluginSource) -> bool {
        if let Some(ref url) = source.url {
            let cache_path = self.url_to_cache_path(url);
            cache_path.exists() && cache_path.join(super::manifest::MANIFEST_FILENAME).exists()
        } else {
            false
        }
    }

    /// Get the cache path for a plugin source
    pub fn get_cache_path(&self, source: &PluginSource) -> Option<PathBuf> {
        source.url.as_ref().map(|url| self.url_to_cache_path(url))
    }

    /// Load a cached plugin's manifest
    pub fn load_cached_plugin(&self, source: &PluginSource) -> Result<(PathBuf, PluginManifest)> {
        let cache_path = self
            .get_cache_path(source)
            .ok_or_else(|| PluginError::NotCached {
                name: source.name.clone(),
            })?;

        if !cache_path.exists() {
            return Err(PluginError::NotCached {
                name: source.name.clone(),
            });
        }

        let manifest = PluginManifest::load(&cache_path)?;
        Ok((cache_path, manifest))
    }

    /// List all cached plugins
    pub fn list_cached(&self) -> Result<Vec<CachedPlugin>> {
        let mut plugins = Vec::new();

        if !self.cache_dir.exists() {
            return Ok(plugins);
        }

        // Walk the cache directory looking for plugin manifests
        self.find_plugins_recursive(&self.cache_dir, &mut plugins)?;

        Ok(plugins)
    }

    /// Recursively find plugins in cache directory
    fn find_plugins_recursive(&self, dir: &Path, plugins: &mut Vec<CachedPlugin>) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        let manifest_path = dir.join(super::manifest::MANIFEST_FILENAME);
        if manifest_path.exists() {
            // Found a plugin
            if let Ok(manifest) = PluginManifest::load(dir) {
                let metadata = self.load_cache_metadata(dir);
                plugins.push(CachedPlugin {
                    name: manifest.plugin.name,
                    url: metadata.as_ref().map(|m| m.url.clone()).unwrap_or_default(),
                    git_ref: metadata.as_ref().and_then(|m| m.git_ref.clone()),
                    cached_at: metadata
                        .as_ref()
                        .map(|m| m.cached_at)
                        .unwrap_or_else(Utc::now),
                    last_updated: metadata
                        .as_ref()
                        .map(|m| m.last_updated)
                        .unwrap_or_else(Utc::now),
                    cache_path: dir.to_path_buf(),
                });
            }
            return Ok(());
        }

        // Continue searching subdirectories
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                self.find_plugins_recursive(&entry.path(), plugins)?;
            }
        }

        Ok(())
    }

    /// Load cache metadata for a plugin
    fn load_cache_metadata(&self, plugin_path: &Path) -> Option<CachedPlugin> {
        let metadata_path = plugin_path.join(CACHE_METADATA_FILE);
        if metadata_path.exists() {
            fs::read_to_string(&metadata_path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
        } else {
            None
        }
    }

    /// Save cache metadata for a plugin
    pub fn save_cache_metadata(&self, plugin: &CachedPlugin) -> Result<()> {
        let metadata_path = plugin.cache_path.join(CACHE_METADATA_FILE);
        let content =
            serde_json::to_string_pretty(plugin).map_err(|e| PluginError::CacheError {
                message: format!("Failed to serialize cache metadata: {}", e),
            })?;
        fs::write(metadata_path, content)?;
        Ok(())
    }

    /// Remove a cached plugin
    pub fn remove(&self, source: &PluginSource) -> Result<()> {
        if let Some(cache_path) = self.get_cache_path(source) {
            if cache_path.exists() {
                fs::remove_dir_all(&cache_path)?;
            }
        }
        Ok(())
    }

    /// Remove all cached plugins
    pub fn clear_all(&self) -> Result<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
        }
        Ok(())
    }

    /// Get total cache size in bytes
    pub fn cache_size(&self) -> Result<u64> {
        if !self.cache_dir.exists() {
            return Ok(0);
        }
        Self::dir_size(&self.cache_dir)
    }

    /// Calculate directory size recursively
    fn dir_size(path: &Path) -> Result<u64> {
        let mut size = 0;
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    size += Self::dir_size(&entry_path)?;
                } else {
                    size += entry.metadata()?.len();
                }
            }
        }
        Ok(size)
    }

    /// Acquire a lock for cache operations
    pub fn lock(&self) -> Result<CacheLock> {
        fs::create_dir_all(&self.cache_dir)?;
        let lock_path = self.cache_dir.join(CACHE_LOCK_FILE);
        let file = File::create(lock_path)?;
        file.lock_exclusive().map_err(|e| PluginError::CacheError {
            message: format!("Failed to acquire cache lock: {}", e),
        })?;
        Ok(CacheLock { file })
    }

    /// Ensure cache directory exists
    pub fn ensure_cache_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }
}

/// RAII lock guard for cache operations
pub struct CacheLock {
    file: File,
}

impl Drop for CacheLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// Format byte size for display
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_url_to_cache_path() {
        let temp_dir = TempDir::new().unwrap();
        let cache = PluginCache::with_dir(temp_dir.path().to_path_buf());

        let path = cache.url_to_cache_path("https://github.com/zhlinh/linthis-config.git");
        assert!(path.ends_with("github.com/zhlinh/linthis-config"));

        let ssh_path = cache.url_to_cache_path("git@github.com:zhlinh/linthis-config.git");
        assert!(ssh_path.ends_with("github.com/zhlinh/linthis-config"));
    }

    #[test]
    fn test_is_cached_empty() {
        let temp_dir = TempDir::new().unwrap();
        let cache = PluginCache::with_dir(temp_dir.path().to_path_buf());

        let source = PluginSource {
            name: "test".to_string(),
            url: Some("https://github.com/test/repo.git".to_string()),
            git_ref: None,
            enabled: true,
        };

        assert!(!cache.is_cached(&source));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1500), "1.46 KB");
        assert_eq!(format_size(1500000), "1.43 MB");
        assert_eq!(format_size(1500000000), "1.40 GB");
    }
}
