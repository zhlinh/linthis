// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Configuration system for linthis with hierarchical precedence.
//!
//! This module provides configuration loading, parsing, and merging for linthis.
//! Configuration files support TOML, YAML, and JSON formats.
//!
//! ## Configuration Precedence
//!
//! Configuration is loaded and merged from multiple sources with the following precedence
//! (higher precedence overrides lower):
//!
//! 1. **CLI arguments** (highest) - Command-line flags
//! 2. **Project config** - `.linthis/config.toml` in project root
//! 3. **User config** - `~/.linthis/config.toml` for global settings
//! 4. **Built-in defaults** (lowest) - Sensible defaults
//!
//! ## Example Configuration
//!
//! ```toml
//! # .linthis/config.toml
//!
//! # Languages to check (empty = auto-detect)
//! languages = ["rust", "python"]
//!
//! # File patterns to exclude
//! excludes = ["vendor/**", "*.generated.*"]
//!
//! # Maximum cyclomatic complexity
//! max_complexity = 20
//!
//! # Rules configuration
//! [rules]
//! disable = ["E501"]
//!
//! # Language-specific settings
//! [rust]
//! max_complexity = 15
//!
//! [python]
//! excludes = ["*_test.py"]
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use linthis::config::Config;
//! use std::path::Path;
//!
//! // Load merged config from all sources
//! let config = Config::load_merged(Path::new("."));
//!
//! // Load from specific file
//! let config = Config::load(Path::new(".linthis/config.toml")).unwrap();
//!
//! // Generate default config
//! let default_toml = Config::generate_default_toml();
//! ```

pub mod cli;
pub mod migrate;
pub mod resolver;

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::rules::RulesConfig;

/// Main configuration structure for linthis.
///
/// Contains all configuration options for controlling lint behavior,
/// formatting, language-specific settings, and plugin configuration.
///
/// # Example
///
/// ```rust,no_run
/// use linthis::config::Config;
/// use std::path::Path;
///
/// // Load from project directory
/// let config = Config::load_merged(Path::new("."));
///
/// // Access configuration values
/// println!("Max complexity: {:?}", config.max_complexity);
/// println!("Excluded patterns: {:?}", config.excludes);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Languages to check (empty = auto-detect)
    #[serde(default)]
    pub languages: HashSet<String>,

    /// Paths/patterns to include (glob patterns)
    #[serde(default)]
    pub includes: Vec<String>,

    /// Paths/patterns to exclude (glob patterns)
    #[serde(default, alias = "exclude")]
    pub excludes: Vec<String>,

    /// Maximum cyclomatic complexity allowed
    #[serde(default)]
    pub max_complexity: Option<u32>,

    /// Format preset to use (google, standard, airbnb)
    #[serde(default)]
    pub preset: Option<String>,

    /// Verbose output
    #[serde(default)]
    pub verbose: Option<bool>,

    /// Source configuration (compatible with CodeCC .code.yml)
    #[serde(default)]
    pub source: Option<SourceConfig>,

    /// Language-specific overrides (flattened to root level)
    #[serde(default, flatten)]
    pub language_overrides: LanguageOverrides,

    /// Plugin configuration
    #[serde(default, alias = "plugin")]
    pub plugins: Option<PluginConfig>,

    /// Plugin auto-sync configuration
    #[serde(default)]
    pub plugin_auto_sync: Option<crate::plugin::AutoSyncConfig>,

    /// Self auto-update configuration
    #[serde(default)]
    pub self_auto_update: Option<crate::self_update::SelfUpdateConfig>,

    /// Performance settings
    #[serde(default)]
    pub performance: PerformanceConfig,

    /// Git hooks settings
    #[serde(default)]
    pub hooks: HooksConfig,

    /// Custom rules, rule disable, and severity overrides
    #[serde(default)]
    pub rules: RulesConfig,
}

/// Plugin configuration section
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginConfig {
    /// List of plugin sources in priority order (later overrides earlier)
    #[serde(default)]
    pub sources: Vec<PluginSourceConfig>,
}

/// Plugin source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSourceConfig {
    /// Plugin name (required for registry lookup or display)
    pub name: String,
    /// Git repository URL (optional if name is in registry)
    #[serde(default)]
    pub url: Option<String>,
    /// Git ref (tag, branch, commit hash)
    #[serde(default, rename = "ref")]
    pub git_ref: Option<String>,
    /// Whether this plugin is enabled (default: true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Performance configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Large file threshold in bytes (default: 1MB = 1048576)
    #[serde(default = "default_large_file_threshold")]
    pub large_file_threshold: u64,
    /// Skip large files instead of warning
    #[serde(default)]
    pub skip_large_files: bool,
    /// Cache max age in days (default: 7)
    #[serde(default = "default_cache_max_age_days")]
    pub cache_max_age_days: u32,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            large_file_threshold: default_large_file_threshold(),
            skip_large_files: false,
            cache_max_age_days: default_cache_max_age_days(),
        }
    }
}

fn default_large_file_threshold() -> u64 {
    1048576 // 1MB
}

fn default_cache_max_age_days() -> u32 {
    7
}

/// Git hooks configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksConfig {
    /// Hook timeout in seconds (default: 60)
    #[serde(default = "default_hook_timeout")]
    pub timeout: u32,
    /// Enable parallel execution in hooks (default: true)
    #[serde(default = "default_hook_parallel")]
    pub parallel: bool,
    /// Commit message validation pattern (conventional commits by default)
    #[serde(default = "default_commit_msg_pattern")]
    pub commit_msg_pattern: String,
    /// Require ticket reference in commit messages (e.g., [JIRA-123])
    #[serde(default)]
    pub require_ticket: bool,
    /// Ticket pattern regex (e.g., r"\[\w+-\d+\]")
    #[serde(default)]
    pub ticket_pattern: Option<String>,
}

impl Default for HooksConfig {
    fn default() -> Self {
        Self {
            timeout: default_hook_timeout(),
            parallel: default_hook_parallel(),
            commit_msg_pattern: default_commit_msg_pattern(),
            require_ticket: false,
            ticket_pattern: None,
        }
    }
}

fn default_hook_timeout() -> u32 {
    60 // 60 seconds
}

fn default_hook_parallel() -> bool {
    true
}

fn default_commit_msg_pattern() -> String {
    r"^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\(.+\))?: .{1,72}".to_string()
}

/// Source path configuration (CodeCC compatibility)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceConfig {
    /// Test source patterns to exclude
    #[serde(default)]
    pub test_source: PathPatterns,

    /// Auto-generated source patterns to exclude
    #[serde(default)]
    pub auto_generate_source: PathPatterns,

    /// Third-party source patterns to exclude
    #[serde(default)]
    pub third_party_source: PathPatterns,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathPatterns {
    /// Regex patterns for file paths
    #[serde(default)]
    pub filepath_regex: Vec<String>,
}

/// Language-specific configuration overrides
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LanguageOverrides {
    #[serde(default)]
    pub rust: Option<LanguageConfig>,
    #[serde(default)]
    pub python: Option<LanguageConfig>,
    #[serde(default)]
    pub typescript: Option<LanguageConfig>,
    #[serde(default)]
    pub javascript: Option<LanguageConfig>,
    #[serde(default)]
    pub go: Option<LanguageConfig>,
    #[serde(default)]
    pub java: Option<LanguageConfig>,
    #[serde(default)]
    pub cpp: Option<CppLanguageConfig>,
    #[serde(default, alias = "objectivec")]
    pub oc: Option<CppLanguageConfig>,
}

/// Per-language configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LanguageConfig {
    /// Additional exclusion patterns for this language
    #[serde(default, alias = "exclude")]
    pub excludes: Vec<String>,
    /// Enable/disable this language
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Max complexity override
    #[serde(default)]
    pub max_complexity: Option<u32>,
    /// Language-specific rules configuration
    #[serde(default)]
    pub rules: Option<RulesConfig>,
}

/// C/C++/Objective-C language configuration with cpplint support
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CppLanguageConfig {
    /// Additional exclusion patterns for this language
    #[serde(default, alias = "exclude")]
    pub excludes: Vec<String>,
    /// Enable/disable this language
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Max complexity override
    #[serde(default)]
    pub max_complexity: Option<u32>,
    /// Cpplint line length (default: 80)
    #[serde(default)]
    pub linelength: Option<u32>,
    /// Cpplint filter rules (e.g., "-build/c++11,-build/header_guard")
    #[serde(default)]
    pub cpplint_filter: Option<String>,
    /// Clang-tidy checks to ignore (e.g., ["clang-analyzer-osx.cocoa.RetainCount"])
    #[serde(default)]
    pub clang_tidy_ignored_checks: Option<Vec<String>>,
    /// Language-specific rules configuration
    #[serde(default)]
    pub rules: Option<RulesConfig>,
}

impl LanguageOverrides {
    /// Merge another LanguageOverrides into this one
    pub fn merge(&mut self, other: LanguageOverrides) {
        macro_rules! merge_lang {
            ($field:ident) => {
                if other.$field.is_some() {
                    self.$field = other.$field;
                }
            };
        }

        merge_lang!(rust);
        merge_lang!(python);
        merge_lang!(typescript);
        merge_lang!(javascript);
        merge_lang!(go);
        merge_lang!(java);
        merge_lang!(cpp);
        merge_lang!(oc);
    }
}

/// Known configuration field names for suggestions
const KNOWN_FIELDS: &[&str] = &[
    "languages",
    "includes",
    "excludes",
    "max_complexity",
    "preset",
    "verbose",
    "quiet",
    "plugins",
    "self_auto_update",
    "plugin_auto_sync",
    "rules",
    "rust",
    "python",
    "go",
    "typescript",
    "javascript",
    "java",
    "cpp",
    "oc",
];

impl Config {
    /// Create a new empty configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from a file
    pub fn load(path: &Path) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            crate::LintisError::Config(format!(
                "Failed to read config file '{}': {}",
                path.display(),
                e
            ))
        })?;

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match ext {
            "yml" | "yaml" => serde_yaml::from_str(&content).map_err(|e| {
                crate::LintisError::Config(format_yaml_error(path, &e))
            }),
            "toml" => toml::from_str(&content).map_err(|e| {
                crate::LintisError::Config(format_toml_error(path, &content, &e))
            }),
            "json" => serde_json::from_str(&content).map_err(|e| {
                crate::LintisError::Config(format_json_error(path, &e))
            }),
            _ => Err(crate::LintisError::Config(format!(
                "Unsupported config format: '{}'\n\nSupported formats: toml, yaml, json",
                ext
            ))),
        }
    }

    /// Load built-in default configuration
    pub fn built_in_defaults() -> Self {
        Config {
            max_complexity: Some(20),
            ..Default::default()
        }
    }

    /// Load user-level configuration from ~/.linthis/config.toml
    pub fn load_user_config() -> Option<Self> {
        let home = dirs::home_dir()?;
        let config_path = home.join(".linthis").join("config.toml");
        if config_path.exists() {
            Self::load(&config_path).ok()
        } else {
            None
        }
    }

    /// Load project-level configuration from the given directory
    /// Searches for .linthis/config.toml in the start directory and parent directories
    pub fn load_project_config(start_dir: &Path) -> Option<Self> {
        let mut current = start_dir.to_path_buf();
        loop {
            let config_path = current.join(".linthis").join("config.toml");
            if config_path.exists() {
                if let Ok(config) = Self::load(&config_path) {
                    return Some(config);
                }
            }

            if !current.pop() {
                break;
            }
        }

        None
    }

    /// Merge another configuration into this one.
    /// Values from `other` override values in `self`.
    pub fn merge(&mut self, other: Config) {
        // Merge languages
        if !other.languages.is_empty() {
            self.languages = other.languages;
        }

        // Merge include patterns (append, don't replace)
        self.includes.extend(other.includes);

        // Merge exclude patterns (append, don't replace)
        self.excludes.extend(other.excludes);

        // Override scalar values
        if other.max_complexity.is_some() {
            self.max_complexity = other.max_complexity;
        }
        if other.preset.is_some() {
            self.preset = other.preset;
        }
        if other.verbose.is_some() {
            self.verbose = other.verbose;
        }
        if other.source.is_some() {
            self.source = other.source;
        }

        // Merge language overrides
        self.language_overrides.merge(other.language_overrides);

        if other.plugins.is_some() {
            self.plugins = other.plugins;
        }

        // Merge rules configuration
        self.rules.merge(other.rules);
    }

    /// Get plugin sources from config, converting to PluginSource type
    pub fn get_plugin_sources(&self) -> Vec<crate::plugin::PluginSource> {
        self.plugins
            .as_ref()
            .map(|p| {
                p.sources
                    .iter()
                    .map(|s| crate::plugin::PluginSource {
                        name: s.name.clone(),
                        url: s.url.clone(),
                        git_ref: s.git_ref.clone(),
                        enabled: s.enabled,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Load and merge configuration from all sources with proper precedence.
    /// Precedence: CLI > project > user > built-in
    pub fn load_merged(project_dir: &Path) -> Self {
        let mut config = Self::built_in_defaults();

        // Layer 2: User config
        if let Some(user_config) = Self::load_user_config() {
            config.merge(user_config);
        }

        // Layer 3: Project config
        if let Some(project_config) = Self::load_project_config(project_dir) {
            config.merge(project_config);
        }

        config
    }

    /// Generate a default configuration file content
    pub fn generate_default_toml() -> String {
        r#"# Linthis Configuration
# See https://github.com/zhlinh/linthis for documentation

# Languages to check (empty = auto-detect all supported languages)
# languages = ["rust", "python", "typescript"]

# Files or directories to include (glob patterns)
# includes = ["src/**", "lib/**"]

# Patterns to exclude (in addition to defaults)
excludes = []

# Maximum cyclomatic complexity allowed
max_complexity = 20

# Format preset: "google", "standard", or "airbnb"
# preset = "google"

# Plugin configuration
# [plugins]
# sources = [
#     { name = "official" },
#     { name = "myplugin", url = "https://github.com/zhlinh/linthis-plugin.git", ref = "main" }
# ]

# Rules configuration
# [rules]
# disable = ["E501", "whitespace/*"]  # Disable specific rules or prefixes
#
# [rules.severity]
# "W0612" = "error"  # Override severity (error, warning, info, off)
#
# [[rules.custom]]
# code = "custom/no-todo"
# pattern = "TODO|FIXME"
# message = "Found TODO/FIXME comment"
# severity = "warning"
# suggestion = "Address or convert to tracking issue"

# Language-specific overrides
# [rust]
# max_complexity = 15

# [python]
# excludes = ["*_test.py"]
"#
        .to_string()
    }

    /// Get the path for a new project config file
    pub fn project_config_path(project_dir: &Path) -> PathBuf {
        project_dir.join(".linthis").join("config.toml")
    }
}

/// Format a TOML error with helpful context and suggestions
fn format_toml_error(path: &Path, content: &str, err: &toml::de::Error) -> String {
    let mut msg = format!("Invalid TOML in '{}'", path.display());

    // Try to extract line information from the error message
    let err_str = err.to_string();

    // Check for unknown field errors
    if err_str.contains("unknown field") {
        if let Some(field) = extract_unknown_field(&err_str) {
            msg.push_str(&format!("\n\nUnknown field: '{}'", field));

            // Suggest similar field names
            if let Some(suggestion) = find_similar_field(&field, KNOWN_FIELDS) {
                msg.push_str(&format!("\n\nDid you mean: '{}'?", suggestion));
            } else {
                msg.push_str("\n\nValid top-level fields: languages, includes, excludes, preset, verbose, quiet, plugins");
            }
        }
    }

    // Try to extract line number from error message
    // TOML errors often include "at line X column Y" or similar patterns
    if let Some(line_info) = extract_line_from_error(&err_str) {
        msg.push_str(&format!("\n\nError at line {}", line_info));

        // Show the problematic line if we can parse the line number
        if let Ok(line_num) = line_info.parse::<usize>() {
            if let Some(line) = content.lines().nth(line_num.saturating_sub(1)) {
                msg.push_str(&format!(":\n  {} | {}", line_num, line.trim()));
            }
        }
    }

    // Add the original error message
    msg.push_str(&format!("\n\nDetails: {}", err));

    // Add hint for common issues
    msg.push_str(&format!("\n\nHint: {}", get_toml_hint(&err_str)));

    msg
}

/// Extract line number from error message
fn extract_line_from_error(err_str: &str) -> Option<String> {
    // Pattern: "at line X" or "line X"
    let patterns = ["at line ", "line "];
    for pattern in patterns {
        if let Some(start) = err_str.find(pattern) {
            let remaining = &err_str[start + pattern.len()..];
            let end = remaining
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(remaining.len());
            if end > 0 {
                return Some(remaining[..end].to_string());
            }
        }
    }
    None
}

/// Format a YAML error with helpful context
fn format_yaml_error(path: &Path, err: &serde_yaml::Error) -> String {
    let mut msg = format!("Invalid YAML in '{}'", path.display());

    if let Some(location) = err.location() {
        msg.push_str(&format!(
            "\n\nError at line {}, column {}",
            location.line(),
            location.column()
        ));
    }

    msg.push_str(&format!("\n\nDetails: {}", err));
    msg.push_str("\n\nHint: Check indentation and ensure proper YAML syntax");

    msg
}

/// Format a JSON error with helpful context
fn format_json_error(path: &Path, err: &serde_json::Error) -> String {
    let mut msg = format!("Invalid JSON in '{}'", path.display());

    msg.push_str(&format!(
        "\n\nError at line {}, column {}",
        err.line(),
        err.column()
    ));

    msg.push_str(&format!("\n\nDetails: {}", err));
    msg.push_str("\n\nHint: Check for missing commas, unclosed brackets, or trailing commas");

    msg
}

/// Extract the unknown field name from a TOML error message
fn extract_unknown_field(err_str: &str) -> Option<String> {
    // Pattern: "unknown field `fieldname`"
    if let Some(start) = err_str.find("unknown field `") {
        let remaining = &err_str[start + 15..];
        if let Some(end) = remaining.find('`') {
            return Some(remaining[..end].to_string());
        }
    }
    None
}

/// Find a similar field name using Levenshtein distance
fn find_similar_field(input: &str, candidates: &[&str]) -> Option<String> {
    let input_lower = input.to_lowercase();
    let mut best_match = None;
    let mut best_distance = usize::MAX;

    for &candidate in candidates {
        let distance = levenshtein_distance(&input_lower, &candidate.to_lowercase());
        // Only suggest if the distance is small (max 3 edits)
        if distance < best_distance && distance <= 3 {
            best_distance = distance;
            best_match = Some(candidate.to_string());
        }
    }

    best_match
}

/// Calculate Levenshtein distance between two strings
#[allow(clippy::needless_range_loop)]
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}

/// Get a helpful hint based on the error type
fn get_toml_hint(err_str: &str) -> &'static str {
    if err_str.contains("expected") && err_str.contains("found") {
        "Check the value type - strings need quotes, arrays use [], tables use [section]"
    } else if err_str.contains("missing field") {
        "Some required fields may be missing from your configuration"
    } else if err_str.contains("duplicate key") {
        "Remove the duplicate field definition"
    } else if err_str.contains("invalid type") {
        "Check that the value type matches what's expected (string, number, boolean, array, etc.)"
    } else if err_str.contains("unknown field") {
        "Check spelling or remove the unrecognized field"
    } else {
        "Run 'linthis init' to generate a valid configuration file"
    }
}

// Add dirs crate for home directory
// Note: You'll need to add `dirs = "5.0"` to Cargo.toml

/// Fallback for home directory if dirs crate is not available
mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var("HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_merge() {
        let mut base = Config {
            max_complexity: Some(20),
            excludes: vec!["*.log".to_string()],
            ..Default::default()
        };

        let override_config = Config {
            max_complexity: Some(15),
            excludes: vec!["*.tmp".to_string()],
            preset: Some("google".to_string()),
            ..Default::default()
        };

        base.merge(override_config);

        assert_eq!(base.max_complexity, Some(15));
        assert_eq!(
            base.excludes,
            vec!["*.log".to_string(), "*.tmp".to_string()]
        );
        assert_eq!(base.preset, Some("google".to_string()));
    }

    #[test]
    fn test_built_in_defaults() {
        let defaults = Config::built_in_defaults();
        assert_eq!(defaults.max_complexity, Some(20));
    }

    #[test]
    fn test_backward_compatibility() {
        // Test that old field names (exclude, plugin) still work via serde aliases
        let toml_with_old_names = r#"
            exclude = ["*.log", "target/**"]

            [plugin]
            sources = [
                { name = "test", enabled = true }
            ]
        "#;

        let config: Config = toml::from_str(toml_with_old_names).unwrap();
        assert_eq!(
            config.excludes,
            vec!["*.log".to_string(), "target/**".to_string()]
        );
        assert!(config.plugins.is_some());
        assert_eq!(config.plugins.as_ref().unwrap().sources.len(), 1);
        assert_eq!(config.plugins.as_ref().unwrap().sources[0].name, "test");
    }

    #[test]
    fn test_new_field_names() {
        // Test that new field names (includes, excludes, plugins) work
        let toml_with_new_names = r#"
            includes = ["src/**", "lib/**"]
            excludes = ["*.log", "target/**"]

            [plugins]
            sources = [
                { name = "test", enabled = true }
            ]
        "#;

        let config: Config = toml::from_str(toml_with_new_names).unwrap();
        assert_eq!(
            config.includes,
            vec!["src/**".to_string(), "lib/**".to_string()]
        );
        assert_eq!(
            config.excludes,
            vec!["*.log".to_string(), "target/**".to_string()]
        );
        assert!(config.plugins.is_some());
        assert_eq!(config.plugins.as_ref().unwrap().sources.len(), 1);
    }

    #[test]
    fn test_language_overrides_simplified_syntax() {
        // Test that simplified language syntax [rust] works (instead of [language_overrides.rust])
        let toml_with_simplified = r#"
            max_complexity = 20

            [rust]
            max_complexity = 15
            excludes = ["target/**"]

            [python]
            max_complexity = 10
            excludes = ["*_test.py"]
        "#;

        let config: Config = toml::from_str(toml_with_simplified).unwrap();
        assert_eq!(config.max_complexity, Some(20));

        // Check Rust overrides
        assert!(config.language_overrides.rust.is_some());
        let rust_config = config.language_overrides.rust.as_ref().unwrap();
        assert_eq!(rust_config.max_complexity, Some(15));
        assert_eq!(rust_config.excludes, vec!["target/**".to_string()]);

        // Check Python overrides
        assert!(config.language_overrides.python.is_some());
        let python_config = config.language_overrides.python.as_ref().unwrap();
        assert_eq!(python_config.max_complexity, Some(10));
        assert_eq!(python_config.excludes, vec!["*_test.py".to_string()]);
    }

    // ==================== LanguageOverrides tests ====================

    #[test]
    fn test_language_overrides_merge() {
        let mut base = LanguageOverrides {
            rust: Some(LanguageConfig {
                max_complexity: Some(15),
                ..Default::default()
            }),
            python: Some(LanguageConfig {
                max_complexity: Some(10),
                ..Default::default()
            }),
            ..Default::default()
        };

        let other = LanguageOverrides {
            rust: Some(LanguageConfig {
                max_complexity: Some(20),
                excludes: vec!["target/**".to_string()],
                ..Default::default()
            }),
            go: Some(LanguageConfig {
                max_complexity: Some(25),
                ..Default::default()
            }),
            ..Default::default()
        };

        base.merge(other);

        // Rust should be overridden
        assert!(base.rust.is_some());
        assert_eq!(base.rust.as_ref().unwrap().max_complexity, Some(20));

        // Python should be preserved (not in other)
        assert!(base.python.is_some());
        assert_eq!(base.python.as_ref().unwrap().max_complexity, Some(10));

        // Go should be added from other
        assert!(base.go.is_some());
        assert_eq!(base.go.as_ref().unwrap().max_complexity, Some(25));
    }

    #[test]
    fn test_language_overrides_merge_none_preserves() {
        let mut base = LanguageOverrides {
            rust: Some(LanguageConfig {
                max_complexity: Some(15),
                ..Default::default()
            }),
            ..Default::default()
        };

        let other = LanguageOverrides::default();
        base.merge(other);

        // Rust should be preserved
        assert!(base.rust.is_some());
        assert_eq!(base.rust.as_ref().unwrap().max_complexity, Some(15));
    }

    // ==================== Config new/default tests ====================

    #[test]
    fn test_config_new() {
        let config = Config::new();
        assert!(config.languages.is_empty());
        assert!(config.includes.is_empty());
        assert!(config.excludes.is_empty());
        assert!(config.max_complexity.is_none());
        assert!(config.preset.is_none());
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.languages.is_empty());
        assert!(config.plugins.is_none());
    }

    // ==================== generate_default_toml tests ====================

    #[test]
    fn test_generate_default_toml_is_valid() {
        let toml_content = Config::generate_default_toml();
        // Should be parseable as TOML
        let result: Result<Config, _> = toml::from_str(&toml_content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_default_toml_has_expected_values() {
        let toml_content = Config::generate_default_toml();
        let config: Config = toml::from_str(&toml_content).unwrap();
        assert_eq!(config.max_complexity, Some(20));
        assert!(config.excludes.is_empty());
    }

    // ==================== project_config_path tests ====================

    #[test]
    fn test_project_config_path() {
        let project_dir = Path::new("/home/user/project");
        let config_path = Config::project_config_path(project_dir);
        assert_eq!(
            config_path,
            PathBuf::from("/home/user/project/.linthis/config.toml")
        );
    }

    // ==================== Config merge edge cases ====================

    #[test]
    fn test_config_merge_languages() {
        let mut base = Config {
            languages: ["rust".to_string()].into_iter().collect(),
            ..Default::default()
        };

        let other = Config {
            languages: ["python".to_string(), "go".to_string()]
                .into_iter()
                .collect(),
            ..Default::default()
        };

        base.merge(other);

        // Languages should be replaced, not merged
        assert_eq!(base.languages.len(), 2);
        assert!(base.languages.contains("python"));
        assert!(base.languages.contains("go"));
        assert!(!base.languages.contains("rust"));
    }

    #[test]
    fn test_config_merge_empty_languages_preserves() {
        let mut base = Config {
            languages: ["rust".to_string()].into_iter().collect(),
            ..Default::default()
        };

        let other = Config {
            languages: HashSet::new(),
            ..Default::default()
        };

        base.merge(other);

        // Empty languages should not override
        assert_eq!(base.languages.len(), 1);
        assert!(base.languages.contains("rust"));
    }

    #[test]
    fn test_config_merge_includes_extends() {
        let mut base = Config {
            includes: vec!["src/**".to_string()],
            ..Default::default()
        };

        let other = Config {
            includes: vec!["lib/**".to_string()],
            ..Default::default()
        };

        base.merge(other);

        // Includes should be extended, not replaced
        assert_eq!(
            base.includes,
            vec!["src/**".to_string(), "lib/**".to_string()]
        );
    }

    #[test]
    fn test_config_merge_verbose() {
        let mut base = Config::default();
        let other = Config {
            verbose: Some(true),
            ..Default::default()
        };

        base.merge(other);
        assert_eq!(base.verbose, Some(true));
    }

    // ==================== PluginConfig tests ====================

    #[test]
    fn test_plugin_config_default() {
        let config = PluginConfig::default();
        assert!(config.sources.is_empty());
    }

    #[test]
    fn test_plugin_source_enabled_default() {
        let toml_str = r#"
            [plugins]
            sources = [
                { name = "test" }
            ]
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        let sources = &config.plugins.unwrap().sources;
        assert_eq!(sources.len(), 1);
        assert!(sources[0].enabled); // Should default to true
    }

    #[test]
    fn test_plugin_source_with_all_fields() {
        let toml_str = r#"
            [plugins]
            sources = [
                { name = "test", url = "https://example.com/repo.git", ref = "v1.0", enabled = false }
            ]
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        let sources = &config.plugins.unwrap().sources;
        assert_eq!(sources[0].name, "test");
        assert_eq!(
            sources[0].url,
            Some("https://example.com/repo.git".to_string())
        );
        assert_eq!(sources[0].git_ref, Some("v1.0".to_string()));
        assert!(!sources[0].enabled);
    }

    // ==================== SourceConfig tests ====================

    #[test]
    fn test_source_config_default() {
        let config = SourceConfig::default();
        assert!(config.test_source.filepath_regex.is_empty());
        assert!(config.auto_generate_source.filepath_regex.is_empty());
        assert!(config.third_party_source.filepath_regex.is_empty());
    }

    #[test]
    fn test_source_config_from_toml() {
        let toml_str = r#"
            [source.test_source]
            filepath_regex = [".*_test\\.py$", "test_.*\\.py$"]

            [source.third_party_source]
            filepath_regex = ["vendor/.*"]
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        let source = config.source.unwrap();
        assert_eq!(source.test_source.filepath_regex.len(), 2);
        assert_eq!(source.third_party_source.filepath_regex.len(), 1);
    }

    // ==================== CppLanguageConfig tests ====================

    #[test]
    fn test_cpp_language_config_from_toml() {
        let toml_str = r#"
            [cpp]
            linelength = 120
            cpplint_filter = "-build/c++11,-whitespace/tab"
            max_complexity = 25

            [oc]
            linelength = 150
            cpplint_filter = "-build/header_guard"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();

        let cpp = config.language_overrides.cpp.unwrap();
        assert_eq!(cpp.linelength, Some(120));
        assert_eq!(
            cpp.cpplint_filter,
            Some("-build/c++11,-whitespace/tab".to_string())
        );
        assert_eq!(cpp.max_complexity, Some(25));

        let oc = config.language_overrides.oc.unwrap();
        assert_eq!(oc.linelength, Some(150));
        assert_eq!(oc.cpplint_filter, Some("-build/header_guard".to_string()));
    }

    #[test]
    fn test_objectivec_alias() {
        // Test that 'objectivec' alias works for 'oc'
        let toml_str = r#"
            [objectivec]
            linelength = 200
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        let oc = config.language_overrides.oc.unwrap();
        assert_eq!(oc.linelength, Some(200));
    }

    // ==================== get_plugin_sources tests ====================

    #[test]
    fn test_get_plugin_sources_empty() {
        let config = Config::default();
        let sources = config.get_plugin_sources();
        assert!(sources.is_empty());
    }

    #[test]
    fn test_get_plugin_sources_with_plugins() {
        let config = Config {
            plugins: Some(PluginConfig {
                sources: vec![PluginSourceConfig {
                    name: "test".to_string(),
                    url: Some("https://example.com".to_string()),
                    git_ref: Some("main".to_string()),
                    enabled: true,
                }],
            }),
            ..Default::default()
        };

        let sources = config.get_plugin_sources();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].name, "test");
        assert_eq!(sources[0].url, Some("https://example.com".to_string()));
        assert_eq!(sources[0].git_ref, Some("main".to_string()));
        assert!(sources[0].enabled);
    }
}
