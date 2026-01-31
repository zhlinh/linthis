// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Configuration resolver for linthis with priority-based config lookup.
//!
//! This module implements the configuration priority system:
//!
//! 1. **Local manual configs** (highest) - ruff.toml, pyproject.toml, .eslintrc.js in project
//! 2. **CLI plugin configs** - from `--use-plugin` (referenced, not copied)
//! 3. **Project plugin configs** - from `.linthis.toml` plugins section
//! 4. **Global plugin configs** - from `~/.config/linthis/config.toml` plugins
//! 5. **Tool defaults** (lowest)

use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Source of a plugin configuration (determines priority)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfigSource {
    /// From `--use-plugin` CLI option (priority 2)
    CliPlugin,
    /// From project `.linthis.toml` plugins section (priority 3)
    ProjectPlugin,
    /// From global `~/.config/linthis/config.toml` plugins (priority 4)
    GlobalPlugin,
}

impl ConfigSource {
    /// Get the priority level (lower number = higher priority)
    pub fn priority(&self) -> u8 {
        match self {
            ConfigSource::CliPlugin => 2,
            ConfigSource::ProjectPlugin => 3,
            ConfigSource::GlobalPlugin => 4,
        }
    }
}

/// A resolved configuration from a plugin
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    /// Language this config applies to (e.g., "python", "typescript")
    pub language: String,
    /// Tool this config is for (e.g., "ruff", "eslint")
    pub tool: String,
    /// Path to the actual config file in plugin cache
    pub config_path: PathBuf,
    /// Source of this config (determines priority)
    pub source: ConfigSource,
    /// Plugin name for display purposes
    pub plugin_name: String,
}

impl ResolvedConfig {
    /// Create a new resolved config
    pub fn new(
        language: impl Into<String>,
        tool: impl Into<String>,
        config_path: PathBuf,
        source: ConfigSource,
        plugin_name: impl Into<String>,
    ) -> Self {
        Self {
            language: language.into(),
            tool: tool.into(),
            config_path,
            source,
            plugin_name: plugin_name.into(),
        }
    }
}

/// Configuration resolver that manages plugin configs with priority ordering.
///
/// The resolver holds references to plugin configs (not copies) and provides
/// methods to look up the appropriate config for a given language/tool,
/// respecting the priority order.
#[derive(Debug, Clone, Default)]
pub struct ConfigResolver {
    /// Plugin configs sorted by priority (higher priority first)
    configs: Vec<ResolvedConfig>,
}

impl ConfigResolver {
    /// Create a new empty config resolver
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new config resolver with the given configs
    pub fn with_configs(mut configs: Vec<ResolvedConfig>) -> Self {
        // Sort by priority (lower priority number = higher priority)
        configs.sort_by_key(|c| c.source.priority());
        Self { configs }
    }

    /// Add a resolved config to the resolver
    pub fn add_config(&mut self, config: ResolvedConfig) {
        self.configs.push(config);
        // Re-sort after adding
        self.configs.sort_by_key(|c| c.source.priority());
    }

    /// Add multiple resolved configs to the resolver
    pub fn add_configs(&mut self, configs: impl IntoIterator<Item = ResolvedConfig>) {
        self.configs.extend(configs);
        // Re-sort after adding
        self.configs.sort_by_key(|c| c.source.priority());
    }

    /// Get the config path for a language/tool, checking local configs first.
    ///
    /// Priority order:
    /// 1. Local manual config (searched from file's directory upward)
    /// 2. Plugin config (from resolver, sorted by source priority)
    /// 3. None (tool should use defaults)
    pub fn get_config(&self, lang: &str, tool: &str, file: &Path) -> Option<PathBuf> {
        // Priority 1: Check for local manual config first
        if let Some(local) = find_local_config(lang, tool, file) {
            return Some(local);
        }

        // Priority 2-4: Return first matching plugin config (already sorted by priority)
        self.configs
            .iter()
            .find(|c| c.language == lang && c.tool == tool)
            .map(|c| c.config_path.clone())
    }

    /// Get the config path for a language/tool without checking local configs.
    /// This is useful when you only want plugin configs.
    ///
    /// Supports tool aliases (e.g., "ruff" matches "ruff-from-flake8").
    pub fn get_plugin_config(&self, lang: &str, tool: &str) -> Option<PathBuf> {
        // First try exact match
        if let Some(config) = self.configs
            .iter()
            .find(|c| c.language == lang && c.tool == tool)
        {
            return Some(config.config_path.clone());
        }

        // Try alias match (e.g., "ruff" matches "ruff-from-flake8")
        let aliases = get_tool_aliases(tool);
        for alias in aliases {
            if let Some(config) = self.configs
                .iter()
                .find(|c| c.language == lang && c.tool == alias)
            {
                return Some(config.config_path.clone());
            }
        }

        None
    }

    /// Get all configs for a given language
    pub fn get_configs_for_language(&self, lang: &str) -> Vec<&ResolvedConfig> {
        self.configs.iter().filter(|c| c.language == lang).collect()
    }

    /// Get all configs for a given tool
    pub fn get_configs_for_tool(&self, tool: &str) -> Vec<&ResolvedConfig> {
        self.configs.iter().filter(|c| c.tool == tool).collect()
    }

    /// Get the number of configs in the resolver
    pub fn len(&self) -> usize {
        self.configs.len()
    }

    /// Check if the resolver has no configs
    pub fn is_empty(&self) -> bool {
        self.configs.is_empty()
    }

    /// Get all configs as a slice
    pub fn configs(&self) -> &[ResolvedConfig] {
        &self.configs
    }
}

/// Find local manual config file by walking up from the file's directory.
///
/// This implements Priority 1: local manual configs take precedence over plugin configs.
pub fn find_local_config(lang: &str, tool: &str, file: &Path) -> Option<PathBuf> {
    let config_names = get_config_names(lang, tool);
    if config_names.is_empty() {
        return None;
    }

    let mut current = if file.is_file() {
        file.parent()?.to_path_buf()
    } else {
        file.to_path_buf()
    };

    loop {
        for config_name in &config_names {
            let config_path = current.join(config_name);
            if config_path.exists() {
                // Skip configs in .linthis/configs/ directory (those are plugin configs)
                if !is_plugin_config_path(&config_path) {
                    return Some(config_path);
                }
            }
        }

        if !current.pop() {
            break;
        }
    }

    None
}

/// Check if a path is inside a .linthis/configs/ directory (plugin config location)
fn is_plugin_config_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    path_str.contains(".linthis/configs/") || path_str.contains(".linthis\\configs\\")
}

/// Get tool aliases for matching plugin configs.
///
/// Some plugins use descriptive tool names like "ruff-from-flake8" instead of "ruff".
/// This function returns a list of aliases that should be checked.
fn get_tool_aliases(tool: &str) -> Vec<&'static str> {
    match tool {
        "ruff" => vec!["ruff-from-flake8"],
        "eslint" => vec!["eslint-from-tslint"],
        "clang-format" => vec!["clang-format-from-idea"],
        _ => vec![],
    }
}

/// Get the list of config file names to search for a given language/tool
fn get_config_names(lang: &str, tool: &str) -> Vec<&'static str> {
    match (lang, tool) {
        ("python", "ruff") => vec![
            "ruff.toml",
            ".ruff.toml",
            "pyproject.toml",
        ],
        ("typescript" | "javascript", "eslint") => vec![
            ".eslintrc.js",
            ".eslintrc.cjs",
            ".eslintrc.json",
            ".eslintrc.yml",
            ".eslintrc.yaml",
            "eslint.config.js",
            "eslint.config.mjs",
            "eslint.config.cjs",
        ],
        ("typescript" | "javascript", "prettier") => vec![
            ".prettierrc",
            ".prettierrc.json",
            ".prettierrc.yml",
            ".prettierrc.yaml",
            ".prettierrc.js",
            ".prettierrc.cjs",
            "prettier.config.js",
            "prettier.config.cjs",
        ],
        ("go", "golangci-lint") => vec![
            ".golangci.yml",
            ".golangci.yaml",
            ".golangci.toml",
            ".golangci.json",
        ],
        ("cpp" | "oc", "clang-tidy") => vec![
            ".clang-tidy",
        ],
        ("cpp" | "oc", "clang-format") => vec![
            ".clang-format",
            "_clang-format",
        ],
        ("rust", "clippy") => vec![
            "clippy.toml",
            ".clippy.toml",
        ],
        ("rust", "rustfmt") => vec![
            "rustfmt.toml",
            ".rustfmt.toml",
        ],
        ("java", "checkstyle") => vec![
            "checkstyle.xml",
        ],
        ("kotlin", "ktlint") => vec![
            ".editorconfig",
        ],
        ("swift", "swiftlint") => vec![
            ".swiftlint.yml",
            ".swiftlint.yaml",
        ],
        ("dart", "analysis_options") => vec![
            "analysis_options.yaml",
        ],
        ("lua", "luacheck") => vec![
            ".luacheckrc",
        ],
        ("lua", "stylua") => vec![
            "stylua.toml",
            ".stylua.toml",
        ],
        ("shell", "shellcheck") => vec![
            ".shellcheckrc",
        ],
        ("ruby", "rubocop") => vec![
            ".rubocop.yml",
        ],
        ("php", "phpcs") => vec![
            "phpcs.xml",
            "phpcs.xml.dist",
            ".phpcs.xml",
            ".phpcs.xml.dist",
        ],
        ("scala", "scalafmt") => vec![
            ".scalafmt.conf",
        ],
        _ => vec![],
    }
}

/// Builder for creating a ConfigResolver with configs from multiple sources
#[derive(Debug, Default)]
pub struct ConfigResolverBuilder {
    configs: Vec<ResolvedConfig>,
}

impl ConfigResolverBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add configs from CLI plugins (--use-plugin)
    pub fn with_cli_plugins(mut self, plugin_configs: Vec<(String, String, PathBuf, String)>) -> Self {
        for (lang, tool, path, plugin_name) in plugin_configs {
            self.configs.push(ResolvedConfig::new(
                lang,
                tool,
                path,
                ConfigSource::CliPlugin,
                plugin_name,
            ));
        }
        self
    }

    /// Add configs from project plugins (.linthis.toml)
    pub fn with_project_plugins(mut self, plugin_configs: Vec<(String, String, PathBuf, String)>) -> Self {
        for (lang, tool, path, plugin_name) in plugin_configs {
            self.configs.push(ResolvedConfig::new(
                lang,
                tool,
                path,
                ConfigSource::ProjectPlugin,
                plugin_name,
            ));
        }
        self
    }

    /// Add configs from global plugins (~/.config/linthis/)
    pub fn with_global_plugins(mut self, plugin_configs: Vec<(String, String, PathBuf, String)>) -> Self {
        for (lang, tool, path, plugin_name) in plugin_configs {
            self.configs.push(ResolvedConfig::new(
                lang,
                tool,
                path,
                ConfigSource::GlobalPlugin,
                plugin_name,
            ));
        }
        self
    }

    /// Build the ConfigResolver
    pub fn build(self) -> ConfigResolver {
        ConfigResolver::with_configs(self.configs)
    }
}

/// Thread-safe shared ConfigResolver
pub type SharedConfigResolver = Arc<ConfigResolver>;

/// Create a shared (Arc) ConfigResolver
pub fn shared_resolver(resolver: ConfigResolver) -> SharedConfigResolver {
    Arc::new(resolver)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_config_source_priority() {
        assert!(ConfigSource::CliPlugin.priority() < ConfigSource::ProjectPlugin.priority());
        assert!(ConfigSource::ProjectPlugin.priority() < ConfigSource::GlobalPlugin.priority());
    }

    #[test]
    fn test_config_resolver_sorting() {
        let resolver = ConfigResolver::with_configs(vec![
            ResolvedConfig::new("python", "ruff", PathBuf::from("/global/ruff.toml"), ConfigSource::GlobalPlugin, "global"),
            ResolvedConfig::new("python", "ruff", PathBuf::from("/cli/ruff.toml"), ConfigSource::CliPlugin, "cli"),
            ResolvedConfig::new("python", "ruff", PathBuf::from("/project/ruff.toml"), ConfigSource::ProjectPlugin, "project"),
        ]);

        // Should be sorted: CLI (2) < Project (3) < Global (4)
        let configs = resolver.configs();
        assert_eq!(configs[0].source, ConfigSource::CliPlugin);
        assert_eq!(configs[1].source, ConfigSource::ProjectPlugin);
        assert_eq!(configs[2].source, ConfigSource::GlobalPlugin);
    }

    #[test]
    fn test_get_plugin_config() {
        let resolver = ConfigResolver::with_configs(vec![
            ResolvedConfig::new("python", "ruff", PathBuf::from("/cli/ruff.toml"), ConfigSource::CliPlugin, "cli"),
            ResolvedConfig::new("python", "ruff", PathBuf::from("/project/ruff.toml"), ConfigSource::ProjectPlugin, "project"),
        ]);

        // Should return CLI plugin config (higher priority)
        let config = resolver.get_plugin_config("python", "ruff");
        assert_eq!(config, Some(PathBuf::from("/cli/ruff.toml")));
    }

    #[test]
    fn test_get_plugin_config_not_found() {
        let resolver = ConfigResolver::new();
        let config = resolver.get_plugin_config("python", "ruff");
        assert_eq!(config, None);
    }

    #[test]
    fn test_local_config_has_priority() {
        // Create temp directory with local ruff.toml
        let temp = tempdir().unwrap();
        let local_config = temp.path().join("ruff.toml");
        fs::write(&local_config, "# local config").unwrap();

        let test_file = temp.path().join("test.py");
        fs::write(&test_file, "# test").unwrap();

        // Create resolver with plugin config
        let resolver = ConfigResolver::with_configs(vec![
            ResolvedConfig::new("python", "ruff", PathBuf::from("/plugin/ruff.toml"), ConfigSource::CliPlugin, "plugin"),
        ]);

        // Should return local config (priority 1) over plugin config (priority 2)
        let config = resolver.get_config("python", "ruff", &test_file);
        assert_eq!(config, Some(local_config));
    }

    #[test]
    fn test_plugin_config_used_when_no_local() {
        // Create temp directory WITHOUT local config
        let temp = tempdir().unwrap();
        let test_file = temp.path().join("test.py");
        fs::write(&test_file, "# test").unwrap();

        // Create resolver with plugin config
        let plugin_path = PathBuf::from("/plugin/ruff.toml");
        let resolver = ConfigResolver::with_configs(vec![
            ResolvedConfig::new("python", "ruff", plugin_path.clone(), ConfigSource::CliPlugin, "plugin"),
        ]);

        // Should return plugin config when no local config exists
        let config = resolver.get_config("python", "ruff", &test_file);
        assert_eq!(config, Some(plugin_path));
    }

    #[test]
    fn test_is_plugin_config_path() {
        assert!(is_plugin_config_path(Path::new("/project/.linthis/configs/python/ruff.toml")));
        assert!(is_plugin_config_path(Path::new("C:\\project\\.linthis\\configs\\python\\ruff.toml")));
        assert!(!is_plugin_config_path(Path::new("/project/ruff.toml")));
        assert!(!is_plugin_config_path(Path::new("/project/.linthis/config.toml")));
    }

    #[test]
    fn test_get_config_names_python() {
        let names = get_config_names("python", "ruff");
        assert!(names.contains(&"ruff.toml"));
        assert!(names.contains(&".ruff.toml"));
        assert!(names.contains(&"pyproject.toml"));
    }

    #[test]
    fn test_get_config_names_typescript() {
        let names = get_config_names("typescript", "eslint");
        assert!(names.contains(&".eslintrc.js"));
        assert!(names.contains(&"eslint.config.js"));
    }

    #[test]
    fn test_builder() {
        let resolver = ConfigResolverBuilder::new()
            .with_cli_plugins(vec![
                ("python".to_string(), "ruff".to_string(), PathBuf::from("/cli/ruff.toml"), "cli-plugin".to_string()),
            ])
            .with_project_plugins(vec![
                ("python".to_string(), "ruff".to_string(), PathBuf::from("/project/ruff.toml"), "project-plugin".to_string()),
            ])
            .build();

        assert_eq!(resolver.len(), 2);

        // CLI plugin should come first (higher priority)
        let configs = resolver.configs();
        assert_eq!(configs[0].source, ConfigSource::CliPlugin);
        assert_eq!(configs[1].source, ConfigSource::ProjectPlugin);
    }

    #[test]
    fn test_resolver_len_and_is_empty() {
        let resolver = ConfigResolver::new();
        assert!(resolver.is_empty());
        assert_eq!(resolver.len(), 0);

        let resolver = ConfigResolver::with_configs(vec![
            ResolvedConfig::new("python", "ruff", PathBuf::from("/ruff.toml"), ConfigSource::CliPlugin, "test"),
        ]);
        assert!(!resolver.is_empty());
        assert_eq!(resolver.len(), 1);
    }
}
