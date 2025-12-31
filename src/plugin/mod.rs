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
pub use cache::PluginCache;
pub use config_manager::PluginConfigManager;
pub use fetcher::PluginFetcher;
pub use loader::PluginLoader;
pub use manifest::PluginManifest;
pub use registry::PluginRegistry;
