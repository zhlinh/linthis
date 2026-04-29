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

    #[error("{}", format_unknown_plugin(name))]
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

/// Format the `UnknownPlugin` error message, adding a helpful hint when the
/// name appears to have been polluted by a shell-wrapping `git` command
/// (e.g. `[WARN] ... \n<actual-url>`).
fn format_unknown_plugin(name: &str) -> String {
    if name.contains('\n') || name.contains('\r') || name.contains("[WARN]") {
        let preview: String = name.chars().take(80).collect();
        format!(
            "Unknown plugin (with garbled name, possibly from a shell-wrapped git command). \
             Preview: {:?}. \
             Check if your shell wraps `git` to print warnings to stdout — that breaks linthis's \
             plugin URL parsing. Common fix: ensure git auth is configured (SSH key or cached \
             credential), so the wrapper doesn't emit warnings.",
            preview
        )
    } else {
        format!("Unknown plugin: '{name}'. Use a full Git URL or one of: official")
    }
}

/// Sanitize a `name_or_url` input that may have been polluted by an
/// upstream shell wrapper (e.g., an internal git proxy printing
/// `[WARN] ... \n<actual-url>`). Returns the cleaned input plus an
/// optional warning string for the caller to log.
///
/// Heuristic: if the input has multiple non-empty lines, take the LAST
/// line that contains either `://` or starts with `git@` (a real URL),
/// or any non-empty line if no URL-shaped line is found. This matches
/// the typical pattern of "warning text on first line, real URL on
/// last line".
fn sanitize_plugin_input(raw: &str) -> (String, Option<String>) {
    let trimmed = raw.trim();
    if !trimmed.contains('\n') && !trimmed.contains('\r') {
        return (trimmed.to_string(), None);
    }
    // Multi-line — try to recover the actual URL/name.
    let lines: Vec<&str> = trimmed
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    let recovered = lines
        .iter()
        .rev()
        .find(|l| l.contains("://") || l.starts_with("git@") || l.starts_with('/'))
        .copied()
        .or_else(|| lines.last().copied())
        .unwrap_or(trimmed)
        .to_string();
    let warning = format!(
        "[plugin] sanitized multi-line input (kept last URL-like line). \
         If this surprises you, check whether your shell wraps `git` to \
         emit warnings on stdout. Raw input was: {:?}",
        raw
    );
    (recovered, Some(warning))
}

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
        let (cleaned, warning) = sanitize_plugin_input(name_or_url);
        if let Some(w) = warning {
            eprintln!("{}", w);
        }
        let name_or_url = cleaned.as_str();
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

    // ==================== sanitize_plugin_input tests ====================

    #[test]
    fn sanitize_plugin_input_passthrough_on_clean_url() {
        let (out, warn) = sanitize_plugin_input("https://github.com/user/repo.git");
        assert_eq!(out, "https://github.com/user/repo.git");
        assert!(warn.is_none());
    }

    #[test]
    fn sanitize_plugin_input_trims_whitespace() {
        let (out, warn) = sanitize_plugin_input("  https://example.com/r.git  ");
        assert_eq!(out, "https://example.com/r.git");
        assert!(warn.is_none(), "no newline -> no warning");
    }

    #[test]
    fn sanitize_plugin_input_recovers_url_from_warn_prefix() {
        let raw =
            "[WARN] HTTPS auth required (HTTP 401), falling back to SSH\ngit@example.com:user/repo.git";
        let (out, warn) = sanitize_plugin_input(raw);
        assert_eq!(out, "git@example.com:user/repo.git");
        assert!(warn.is_some(), "should warn on cleanup");
        let w = warn.unwrap();
        assert!(w.contains("sanitized multi-line input"));
    }

    #[test]
    fn sanitize_plugin_input_picks_last_url_line() {
        let raw = "warning 1\nwarning 2\nhttps://github.com/u/r.git";
        let (out, _) = sanitize_plugin_input(raw);
        assert_eq!(out, "https://github.com/u/r.git");
    }

    #[test]
    fn sanitize_plugin_input_falls_back_to_last_line_when_no_url_shape() {
        let raw = "warning\nmy-plugin";
        let (out, _) = sanitize_plugin_input(raw);
        assert_eq!(out, "my-plugin");
    }

    #[test]
    fn plugin_source_new_recovers_from_polluted_url() {
        let raw = "[WARN] auth failed\ngit@github.com:u/r.git";
        let s = PluginSource::new(raw);
        assert_eq!(s.url.as_deref(), Some("git@github.com:u/r.git"));
        assert!(
            !s.name.contains("[WARN]"),
            "name should not have warn prefix: {}",
            s.name
        );
    }

    #[test]
    fn unknown_plugin_error_renders_garbled_name_with_hint() {
        let err = PluginError::UnknownPlugin {
            name: "[WARN] auth\ngit@x:y/z.git".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("garbled name"), "{msg}");
        assert!(msg.contains("shell wraps `git`"), "{msg}");
    }

    #[test]
    fn unknown_plugin_error_renders_clean_name_simply() {
        let err = PluginError::UnknownPlugin {
            name: "missing-plugin".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("'missing-plugin'"), "{msg}");
        assert!(!msg.contains("garbled"), "{msg}");
    }
}
