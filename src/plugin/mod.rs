// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Plugin system for linthis configuration management.
//!
//! This module provides functionality for:
//! - Fetching configuration plugins from Git repositories
//! - Caching plugins locally for offline use
//! - Loading and merging plugin configurations
//! - Managing plugin lifecycle (init, list, clean, update)

pub mod auto_sync;
pub mod cache;
pub mod config_manager;
pub mod fetcher;
pub mod loader;
pub mod manifest;
pub mod registry;

use std::path::PathBuf;
use thiserror::Error;

/// Plugin-specific errors
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Git is not installed. Please install Git:\n  - Linux: sudo apt install git\n  - macOS: brew install git\n  - Windows: https://git-scm.com/download/win")]
    GitNotInstalled,

    #[error("Failed to clone plugin repository '{url}': {message}")]
    CloneFailed { url: String, message: String },

    #[error("Failed to update plugin '{name}': {message}")]
    UpdateFailed { name: String, message: String },

    #[error("Plugin not found in cache: {name}")]
    NotCached { name: String },

    #[error("Invalid plugin manifest at '{path}': {message}")]
    InvalidManifest { path: PathBuf, message: String },

    #[error(
        "Plugin '{name}' requires linthis version {required}, but current version is {current}"
    )]
    IncompatibleVersion {
        name: String,
        required: String,
        current: String,
    },

    #[error("Unknown plugin: '{name}'. Use a full Git URL or one of: official")]
    UnknownPlugin { name: String },

    #[error("Network error while fetching plugin: {message}")]
    NetworkError { message: String },

    #[error("Cache directory error: {message}")]
    CacheError { message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config file not found in plugin: {path}")]
    ConfigNotFound { path: PathBuf },
}

pub type Result<T> = std::result::Result<T, PluginError>;

/// Plugin source specification from config or CLI
#[derive(Debug, Clone)]
pub struct PluginSource {
    /// Short name for the plugin (e.g., "official", "company")
    pub name: String,
    /// Git repository URL (HTTPS or SSH)
    pub url: Option<String>,
    /// Git ref (tag, branch, commit hash)
    pub git_ref: Option<String>,
    /// Whether this plugin is enabled
    pub enabled: bool,
}

impl PluginSource {
    /// Create a new plugin source from a name (registry lookup) or URL
    pub fn new(name_or_url: &str) -> Self {
        if name_or_url.contains("://") || name_or_url.starts_with("git@") {
            // It's a URL
            Self {
                name: Self::name_from_url(name_or_url),
                url: Some(name_or_url.to_string()),
                git_ref: None,
                enabled: true,
            }
        } else {
            // It's a registry name
            Self {
                name: name_or_url.to_string(),
                url: None,
                git_ref: None,
                enabled: true,
            }
        }
    }

    /// Extract a short name from a URL
    fn name_from_url(url: &str) -> String {
        url.trim_end_matches(".git")
            .rsplit('/')
            .next()
            .unwrap_or("unknown")
            .to_string()
    }

    /// Create with a specific Git ref
    pub fn with_ref(mut self, git_ref: &str) -> Self {
        self.git_ref = Some(git_ref.to_string());
        self
    }
}

/// Log a plugin operation if verbose mode is enabled
pub fn log_plugin_operation(operation: &str, details: &str, verbose: bool) {
    if verbose {
        eprintln!("[plugin] {}: {}", operation, details);
    }
}

// Re-export commonly used types
pub use auto_sync::{AutoSyncConfig, AutoSyncManager};
pub use cache::PluginCache;
pub use config_manager::PluginConfigManager;
pub use fetcher::PluginFetcher;
pub use loader::PluginLoader;
pub use manifest::PluginManifest;
pub use registry::PluginRegistry;

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== PluginSource::new tests ====================

    #[test]
    fn test_plugin_source_new_from_registry_name() {
        let source = PluginSource::new("official");
        assert_eq!(source.name, "official");
        assert!(source.url.is_none());
        assert!(source.git_ref.is_none());
        assert!(source.enabled);
    }

    #[test]
    fn test_plugin_source_new_from_https_url() {
        let source = PluginSource::new("https://github.com/zhlinh/linthis-plugin.git");
        assert_eq!(source.name, "linthis-plugin");
        assert_eq!(
            source.url,
            Some("https://github.com/zhlinh/linthis-plugin.git".to_string())
        );
        assert!(source.git_ref.is_none());
        assert!(source.enabled);
    }

    #[test]
    fn test_plugin_source_new_from_https_url_no_git_suffix() {
        let source = PluginSource::new("https://github.com/zhlinh/linthis-plugin");
        assert_eq!(source.name, "linthis-plugin");
        assert_eq!(
            source.url,
            Some("https://github.com/zhlinh/linthis-plugin".to_string())
        );
    }

    #[test]
    fn test_plugin_source_new_from_ssh_url() {
        let source = PluginSource::new("git@github.com:zhlinh/linthis-plugin.git");
        assert_eq!(source.name, "linthis-plugin");
        assert_eq!(
            source.url,
            Some("git@github.com:zhlinh/linthis-plugin.git".to_string())
        );
    }

    // ==================== PluginSource::with_ref tests ====================

    #[test]
    fn test_plugin_source_with_ref() {
        let source = PluginSource::new("official").with_ref("v1.0.0");
        assert_eq!(source.name, "official");
        assert_eq!(source.git_ref, Some("v1.0.0".to_string()));
    }

    #[test]
    fn test_plugin_source_with_ref_branch() {
        let source =
            PluginSource::new("https://github.com/zhlinh/linthis-plugin.git").with_ref("main");
        assert_eq!(source.git_ref, Some("main".to_string()));
    }

    #[test]
    fn test_plugin_source_with_ref_commit_hash() {
        let source = PluginSource::new("official").with_ref("abc1234def5678");
        assert_eq!(source.git_ref, Some("abc1234def5678".to_string()));
    }

    // ==================== PluginSource::name_from_url tests ====================

    #[test]
    fn test_name_from_url_github_https() {
        let name = PluginSource::name_from_url("https://github.com/zhlinh/linthis-plugin.git");
        assert_eq!(name, "linthis-plugin");
    }

    #[test]
    fn test_name_from_url_github_ssh() {
        let name = PluginSource::name_from_url("git@github.com:zhlinh/linthis-plugin.git");
        assert_eq!(name, "linthis-plugin");
    }

    #[test]
    fn test_name_from_url_no_git_suffix() {
        let name = PluginSource::name_from_url("https://gitlab.com/org/my-plugin");
        assert_eq!(name, "my-plugin");
    }

    #[test]
    fn test_name_from_url_simple_path() {
        let name = PluginSource::name_from_url("https://example.com/plugin.git");
        assert_eq!(name, "plugin");
    }

    // ==================== PluginError tests ====================

    #[test]
    fn test_plugin_error_display_git_not_installed() {
        let err = PluginError::GitNotInstalled;
        let msg = format!("{}", err);
        assert!(msg.contains("Git is not installed"));
    }

    #[test]
    fn test_plugin_error_display_clone_failed() {
        let err = PluginError::CloneFailed {
            url: "https://github.com/test/test.git".to_string(),
            message: "Connection refused".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Failed to clone"));
        assert!(msg.contains("Connection refused"));
    }

    #[test]
    fn test_plugin_error_display_not_cached() {
        let err = PluginError::NotCached {
            name: "test-plugin".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("not found in cache"));
        assert!(msg.contains("test-plugin"));
    }

    #[test]
    fn test_plugin_error_display_incompatible_version() {
        let err = PluginError::IncompatibleVersion {
            name: "test-plugin".to_string(),
            required: ">=1.0".to_string(),
            current: "0.5".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("test-plugin"));
        assert!(msg.contains(">=1.0"));
        assert!(msg.contains("0.5"));
    }

    #[test]
    fn test_plugin_error_display_unknown_plugin() {
        let err = PluginError::UnknownPlugin {
            name: "my-plugin".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Unknown plugin"));
        assert!(msg.contains("my-plugin"));
    }

    #[test]
    fn test_plugin_error_display_invalid_manifest() {
        let err = PluginError::InvalidManifest {
            path: PathBuf::from("/path/to/manifest.toml"),
            message: "missing field 'name'".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid plugin manifest"));
        assert!(msg.contains("manifest.toml"));
    }
}
