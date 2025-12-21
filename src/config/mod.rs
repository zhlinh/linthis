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
//! Configuration is loaded and merged from multiple sources with the following precedence
//! (higher precedence overrides lower):
//!
//! 1. CLI arguments (highest)
//! 2. Project config (.linthis.toml in project root)
//! 3. User config (~/.linthis/config.toml)
//! 4. Built-in defaults (lowest)

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Languages to check (empty = auto-detect)
    #[serde(default)]
    pub languages: HashSet<String>,

    /// Paths/patterns to exclude (glob patterns)
    #[serde(default)]
    pub exclude: Vec<String>,

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

    /// Language-specific overrides
    #[serde(default)]
    pub language_overrides: Option<LanguageOverrides>,
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
    pub cpp: Option<LanguageConfig>,
}

/// Per-language configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LanguageConfig {
    /// Additional exclusion patterns for this language
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Enable/disable this language
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Max complexity override
    #[serde(default)]
    pub max_complexity: Option<u32>,
}

impl Config {
    /// Create a new empty configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from a file
    pub fn load(path: &Path) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::LintisError::Config(format!("Failed to read config: {}", e)))?;

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match ext {
            "yml" | "yaml" => serde_yaml::from_str(&content)
                .map_err(|e| crate::LintisError::Config(format!("Failed to parse YAML: {}", e))),
            "toml" => toml::from_str(&content)
                .map_err(|e| crate::LintisError::Config(format!("Failed to parse TOML: {}", e))),
            "json" => serde_json::from_str(&content)
                .map_err(|e| crate::LintisError::Config(format!("Failed to parse JSON: {}", e))),
            _ => Err(crate::LintisError::Config(format!(
                "Unsupported config format: {}",
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
        let config_paths = [
            home.join(".linthis").join("config.toml"),
            home.join(".linthis").join("config.yml"),
            home.join(".linthis.toml"),
        ];

        for path in &config_paths {
            if path.exists() {
                if let Ok(config) = Self::load(path) {
                    return Some(config);
                }
            }
        }

        None
    }

    /// Load project-level configuration from the given directory
    pub fn load_project_config(start_dir: &Path) -> Option<Self> {
        let config_names = [
            ".linthis.toml",
            ".linthis.yml",
            ".linthis.yaml",
            "linthis.toml",
            ".code.yml", // CodeCC compatibility
        ];

        let mut current = start_dir.to_path_buf();
        loop {
            for name in &config_names {
                let config_path = current.join(name);
                if config_path.exists() {
                    if let Ok(config) = Self::load(&config_path) {
                        return Some(config);
                    }
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

        // Merge exclude patterns (append, don't replace)
        self.exclude.extend(other.exclude);

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
        if other.language_overrides.is_some() {
            self.language_overrides = other.language_overrides;
        }
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
        r#"# Lintis Configuration
# See https://github.com/your-org/linthis for documentation

# Languages to check (empty = auto-detect all supported languages)
# languages = ["rust", "python", "typescript"]

# Patterns to exclude (in addition to defaults)
exclude = []

# Maximum cyclomatic complexity allowed
max_complexity = 20

# Format preset: "google", "standard", or "airbnb"
# preset = "google"

# Language-specific overrides
# [language_overrides.rust]
# max_complexity = 15

# [language_overrides.python]
# exclude = ["*_test.py"]
"#
        .to_string()
    }

    /// Get the path for a new project config file
    pub fn project_config_path(project_dir: &Path) -> PathBuf {
        project_dir.join(".linthis.toml")
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
            exclude: vec!["*.log".to_string()],
            ..Default::default()
        };

        let override_config = Config {
            max_complexity: Some(15),
            exclude: vec!["*.tmp".to_string()],
            preset: Some("google".to_string()),
            ..Default::default()
        };

        base.merge(override_config);

        assert_eq!(base.max_complexity, Some(15));
        assert_eq!(base.exclude, vec!["*.log".to_string(), "*.tmp".to_string()]);
        assert_eq!(base.preset, Some("google".to_string()));
    }

    #[test]
    fn test_built_in_defaults() {
        let defaults = Config::built_in_defaults();
        assert_eq!(defaults.max_complexity, Some(20));
    }
}
