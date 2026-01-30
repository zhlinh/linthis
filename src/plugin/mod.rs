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
//! Plugins allow extending linthis with community or custom lint configurations,
//! additional rules, and shared presets. Plugins are Git repositories containing
//! a `linthis-plugin.toml` manifest.
//!
//! ## Features
//!
//! - **Git-based distribution**: Plugins are fetched from Git repositories
//! - **Local caching**: Plugins are cached for offline use and faster startup
//! - **Version pinning**: Pin plugins to specific tags, branches, or commits
//! - **Auto-sync**: Optionally auto-update plugins on schedule
//! - **Registry lookup**: Use short names for well-known plugins
//!
//! ## Plugin Configuration
//!
//! ```toml
//! # .linthis/config.toml
//! [plugins]
//! sources = [
//!     { name = "official" },                                    # Registry lookup
//!     { name = "company", url = "https://github.com/co/plugin.git" },  # Direct URL
//!     { name = "pinned", url = "...", ref = "v1.0.0" },         # Pinned version
//! ]
//! ```
//!
//! ## Plugin Manifest
//!
//! Each plugin must contain a `linthis-plugin.toml` manifest:
//!
//! ```toml
//! [plugin]
//! name = "my-plugin"
//! version = "1.0.0"
//! description = "Custom lint rules for my project"
//! min_linthis_version = "0.1.0"
//!
//! [config]
//! path = "config.toml"  # Plugin's configuration file
//! ```
//!
//! ## CLI Commands
//!
//! ```bash
//! # Initialize plugins from config
//! linthis plugin init
//!
//! # List installed plugins
//! linthis plugin list
//!
//! # Update all plugins
//! linthis plugin update
//!
//! # Clean plugin cache
//! linthis plugin clean
//! ```
//!
//! ## Module Structure
//!
//! - [`fetcher`]: Git clone/pull operations
//! - [`cache`]: Local plugin caching
//! - [`loader`]: Plugin configuration loading
//! - [`manifest`]: Plugin manifest parsing
//! - [`registry`]: Well-known plugin registry
//! - [`auto_sync`]: Automatic plugin updates

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

    #[error("Configuration error: {message}")]
    ConfigError { message: String },

    #[error("Validation error: {message}")]
    ValidationError { message: String },

    #[error("Cannot determine home directory")]
    HomeDirectoryError,

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml_edit::TomlError),
}

pub type Result<T> = std::result::Result<T, PluginError>;

impl From<crate::LintisError> for PluginError {
    fn from(err: crate::LintisError) -> Self {
        match err {
            crate::LintisError::Io(e) => PluginError::Io(e),
            crate::LintisError::Config(msg) => PluginError::ConfigError { message: msg },
            _ => PluginError::ConfigError {
                message: err.to_string(),
            },
        }
    }
}

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
    /// Create a new plugin source from a name (registry lookup), URL, or local path
    pub fn new(name_or_url: &str) -> Self {
        if name_or_url.contains("://") || name_or_url.starts_with("git@") {
            // It's a URL
            Self {
                name: Self::name_from_url(name_or_url),
                url: Some(name_or_url.to_string()),
                git_ref: None,
                enabled: true,
            }
        } else if name_or_url.starts_with('/')
            || name_or_url.starts_with("./")
            || name_or_url.starts_with("../")
        {
            // It's a local path
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

    /// Check if this is a local path source
    pub fn is_local_path(&self) -> bool {
        if let Some(ref url) = self.url {
            url.starts_with('/') || url.starts_with("./") || url.starts_with("../")
        } else {
            false
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

    #[test]
    fn test_plugin_source_new_from_local_path_absolute() {
        let source = PluginSource::new("/path/to/local/plugin");
        assert_eq!(source.name, "plugin");
        assert_eq!(source.url, Some("/path/to/local/plugin".to_string()));
        assert!(source.is_local_path());
    }

    #[test]
    fn test_plugin_source_new_from_local_path_relative() {
        let source = PluginSource::new("./my-plugin");
        assert_eq!(source.name, "my-plugin");
        assert_eq!(source.url, Some("./my-plugin".to_string()));
        assert!(source.is_local_path());
    }

    #[test]
    fn test_plugin_source_new_from_local_path_parent() {
        let source = PluginSource::new("../parent-plugin");
        assert_eq!(source.name, "parent-plugin");
        assert_eq!(source.url, Some("../parent-plugin".to_string()));
        assert!(source.is_local_path());
    }

    #[test]
    fn test_plugin_source_is_not_local_path() {
        let source = PluginSource::new("https://github.com/org/plugin.git");
        assert!(!source.is_local_path());

        let source2 = PluginSource::new("official");
        assert!(!source2.is_local_path());
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
