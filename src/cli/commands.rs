// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! CLI command definitions using clap.
//!
//! This module contains the command-line interface definitions for linthis,
//! including the main CLI struct and all subcommand enums.

use clap::Parser;
use std::path::PathBuf;

/// Main CLI parser for linthis.
#[derive(Parser, Debug)]
#[command(name = "linthis")]
#[command(
    author,
    version,
    about = "A fast, cross-platform multi-language linter and formatter"
)]
pub struct Cli {
    /// Files or directories to include (can be specified multiple times)
    /// Examples: -i src -i lib, --include ./plugin
    #[arg(short = 'i', long = "include")]
    pub paths: Vec<PathBuf>,

    /// Only run lint checks, no formatting
    #[arg(short = 'c', long)]
    pub check_only: bool,

    /// Only format files, no lint checking
    #[arg(short = 'f', long)]
    pub format_only: bool,

    /// Check only staged files (git cached)
    #[arg(short = 's', long)]
    pub staged: bool,

    /// Specify languages to check (comma-separated: rust,python,typescript)
    #[arg(short, long, value_delimiter = ',')]
    pub lang: Option<Vec<String>>,

    /// Exclude patterns (glob patterns)
    #[arg(short, long)]
    pub exclude: Option<Vec<String>>,

    /// Disable default exclusions (.git, node_modules, target, etc.)
    #[arg(long)]
    pub no_default_excludes: bool,

    /// Disable .gitignore pattern exclusions
    #[arg(long)]
    pub no_gitignore: bool,

    /// Path to configuration file
    #[arg(long)]
    pub config: Option<std::path::PathBuf>,

    /// Initialize a new .linthis/config.toml configuration file
    #[arg(long)]
    pub init: bool,

    /// Generate default config files for all linters/formatters
    #[arg(long)]
    pub init_configs: bool,

    /// Format preset (google, standard, airbnb)
    #[arg(long)]
    pub preset: Option<String>,

    /// Output format: human, json, github-actions
    #[arg(short, long, default_value = "human")]
    pub output: String,

    /// Disable auto-saving results to .linthis/result/
    #[arg(long)]
    pub no_save_result: bool,

    /// Save results to custom file path (instead of default .linthis/result/)
    #[arg(long, value_name = "FILE")]
    pub output_file: Option<PathBuf>,

    /// Maximum number of result files to keep (default: 10, 0 = unlimited)
    #[arg(long, default_value = "10")]
    pub keep_results: usize,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Suppress non-error output
    #[arg(short, long)]
    pub quiet: bool,

    /// Run benchmark comparing ruff vs flake8+black for Python
    #[arg(long)]
    pub benchmark: bool,

    /// Interactive fix mode: review and fix issues one by one
    ///
    /// Usage:
    ///   --fix           Load last result and enter interactive mode
    ///   --fix last      Same as above (explicit)
    ///   --fix <FILE>    Load specific result file
    ///   -c --fix        Run check then enter interactive mode
    #[arg(short = 'F', long, value_name = "SOURCE", num_args = 0..=1, default_missing_value = "last")]
    pub fix: Option<String>,

    /// Skip loading plugins, use default configuration
    #[arg(long)]
    pub no_plugin: bool,

    /// Plugin subcommands (init, list, clean)
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Hook management tools
#[derive(Clone, Debug, clap::ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum HookTool {
    /// Prek (Rust-based, faster)
    Prek,
    /// Pre-commit (Python-based, standard)
    PreCommit,
    /// Traditional git hook
    Git,
}

/// Top-level subcommands
#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    /// Plugin management commands
    Plugin {
        #[command(subcommand)]
        action: PluginCommands,
    },
    /// Configuration management commands
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
    /// Git hook management commands
    Hook {
        #[command(subcommand)]
        action: HookCommands,
    },
    /// Initialize configuration file
    Init {
        /// Create global configuration (~/.linthis/config.toml)
        #[arg(short, long)]
        global: bool,

        /// Also install git hook after creating config
        #[arg(long)]
        with_hook: bool,

        /// Force overwrite existing files
        #[arg(long)]
        force: bool,
    },
}

/// Hook subcommands
#[derive(clap::Subcommand, Debug)]
pub enum HookCommands {
    /// Install git pre-commit hook
    Install {
        /// Hook type to install
        #[arg(long = "type", value_name = "TYPE")]
        hook_type: Option<HookTool>,

        /// Hook only runs check (no formatting)
        #[arg(short = 'c', long = "check-only")]
        check_only: bool,

        /// Hook only runs format (no linting)
        #[arg(short = 'f', long = "format-only")]
        format_only: bool,

        /// Force overwrite existing hook
        #[arg(long)]
        force: bool,

        /// Non-interactive mode (use defaults, no prompts)
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Uninstall git pre-commit hook
    Uninstall {
        /// Non-interactive mode
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Show git hook status
    Status,
    /// Check for hook conflicts
    Check,
}

/// Plugin subcommands
#[derive(clap::Subcommand, Debug)]
pub enum PluginCommands {
    /// Initialize a new plugin
    Init {
        /// Plugin name
        name: String,
    },
    /// List configured or cached plugins
    List {
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
        /// List global plugins (~/.linthis/config.toml)
        #[arg(short, long)]
        global: bool,
        /// List cached (downloaded) plugins instead of configured
        #[arg(short, long)]
        cached: bool,
    },
    /// Clean cached plugins
    Clean {
        /// Remove all cached plugins
        #[arg(long)]
        all: bool,
    },
    /// Sync (download/update) configured plugins to latest version
    Sync {
        /// Sync global plugins (~/.linthis/config.toml)
        #[arg(short, long)]
        global: bool,
    },
    /// Validate a plugin manifest
    Validate {
        /// Path to plugin directory
        path: PathBuf,
    },
    /// Add a plugin to configuration
    Add {
        /// Plugin alias (unique name for the plugin)
        alias: String,
        /// Plugin Git repository URL
        url: String,
        /// Git reference (branch, tag, or commit)
        #[arg(long = "ref")]
        git_ref: Option<String>,
        /// Add to global configuration (~/.linthis/config.toml)
        #[arg(short, long)]
        global: bool,
    },
    /// Remove a plugin from configuration (by alias)
    Remove {
        /// Plugin alias to remove
        alias: String,
        /// Remove from global configuration
        #[arg(short, long)]
        global: bool,
    },
    /// Apply (copy) plugin configs to current project
    Apply {
        /// Plugin alias to apply configs from
        alias: Option<String>,
        /// Apply configs from global plugins
        #[arg(short, long)]
        global: bool,
        /// Languages to apply configs for (e.g., cpp, oc, swift)
        #[arg(short, long)]
        language: Option<Vec<String>>,
    },
}

/// Config subcommands
#[derive(clap::Subcommand, Debug)]
pub enum ConfigCommands {
    /// Add value to an array field (includes, excludes, languages)
    Add {
        /// Field name (includes, excludes, languages)
        field: ConfigField,
        /// Value to add
        value: String,
        /// Modify global configuration (~/.linthis/config.toml)
        #[arg(short, long)]
        global: bool,
    },
    /// Remove value from an array field
    Remove {
        /// Field name (includes, excludes, languages)
        field: ConfigField,
        /// Value to remove
        value: String,
        /// Modify global configuration
        #[arg(short, long)]
        global: bool,
    },
    /// Clear all values from an array field
    Clear {
        /// Field name (includes, excludes, languages)
        field: ConfigField,
        /// Modify global configuration
        #[arg(short, long)]
        global: bool,
    },
    /// Set a scalar field value (max_complexity, preset, verbose)
    Set {
        /// Field name (max_complexity, preset, verbose)
        field: String,
        /// Field value
        value: String,
        /// Modify global configuration
        #[arg(short, long)]
        global: bool,
    },
    /// Unset a scalar field (restore to default)
    Unset {
        /// Field name
        field: String,
        /// Modify global configuration
        #[arg(short, long)]
        global: bool,
    },
    /// Get the value of a field
    Get {
        /// Field name
        field: String,
        /// Get from global configuration
        #[arg(short, long)]
        global: bool,
    },
    /// List all configuration values
    List {
        /// Show detailed information (including source)
        #[arg(short, long)]
        verbose: bool,
        /// List global configuration
        #[arg(short, long)]
        global: bool,
    },
}

/// Configuration field types for CLI operations
#[derive(clap::ValueEnum, Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum ConfigField {
    #[value(name = "includes")]
    Includes,
    #[value(name = "excludes")]
    Excludes,
    #[value(name = "languages")]
    Languages,
}

impl ConfigField {
    /// Get the string representation of the field name
    pub fn as_str(&self) -> &'static str {
        match self {
            ConfigField::Includes => "includes",
            ConfigField::Excludes => "excludes",
            ConfigField::Languages => "languages",
        }
    }
}
