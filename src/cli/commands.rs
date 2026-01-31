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

    /// Check only files changed since a git ref (branch, tag, or commit)
    #[arg(long, value_name = "REF")]
    pub since: Option<String>,

    /// Check only uncommitted files (staged + unstaged)
    #[arg(long)]
    pub uncommitted: bool,

    /// Ignore cache and force re-checking all files
    #[arg(long)]
    pub no_cache: bool,

    /// Clear the cache before running
    #[arg(long)]
    pub clear_cache: bool,

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

    /// Skip loading plugins, use default configuration
    #[arg(long)]
    pub no_plugin: bool,

    /// Use specific plugin(s) directly, bypassing config files
    /// Useful for debugging plugins or CI integration
    ///
    /// Formats:
    ///   --use-plugin https://github.com/org/plugin.git
    ///   --use-plugin https://github.com/org/plugin.git@v1.0
    ///   --use-plugin /path/to/local/plugin
    ///   --use-plugin plugin1,plugin2  (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub use_plugin: Option<Vec<String>>,

    /// Hook mode: enable compact output format for git hooks
    /// Shows summary at top, lists errors with file:line, and provides fix commands
    /// Optional value specifies hook type: pre-commit (default), pre-push, commit-msg
    #[arg(long, hide = true, value_name = "HOOK_TYPE", num_args = 0..=1, default_missing_value = "pre-commit")]
    pub hook_mode: Option<String>,

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

/// Git hook event types
#[derive(Clone, Debug, Default, clap::ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum HookEvent {
    /// Pre-commit hook (runs before commit is created)
    #[default]
    PreCommit,
    /// Pre-push hook (runs before push to remote)
    PrePush,
    /// Commit-msg hook (validates commit message format)
    CommitMsg,
}

impl HookEvent {
    /// Get the git hook file name for this event
    pub fn hook_filename(&self) -> &'static str {
        match self {
            HookEvent::PreCommit => "pre-commit",
            HookEvent::PrePush => "pre-push",
            HookEvent::CommitMsg => "commit-msg",
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            HookEvent::PreCommit => "pre-commit (runs before commit)",
            HookEvent::PrePush => "pre-push (runs before push)",
            HookEvent::CommitMsg => "commit-msg (validates commit message)",
        }
    }
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
    /// Cache management commands
    Cache {
        #[command(subcommand)]
        action: CacheCommands,
    },
    /// Security vulnerability scanning
    ///
    /// Scan project dependencies for known security vulnerabilities.
    /// Supports Rust (cargo-audit), JavaScript (npm audit), Python (pip-audit),
    /// Go (govulncheck), and Java (dependency-check).
    ///
    /// Example usage:
    ///   linthis security                    # Scan current directory
    ///   linthis security --severity high    # Only show high+ severity
    ///   linthis security --fix              # Show fix suggestions
    ///   linthis security --format json      # Output as JSON
    Security {
        /// Path to scan (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Minimum severity to report (critical, high, medium, low)
        #[arg(long, short = 's')]
        severity: Option<String>,

        /// Include dev dependencies
        #[arg(long)]
        include_dev: bool,

        /// Show fix suggestions
        #[arg(long)]
        fix: bool,

        /// Vulnerability IDs to ignore (can be specified multiple times)
        #[arg(long, short = 'i')]
        ignore: Option<Vec<String>>,

        /// Output format: human, json, sarif
        #[arg(short, long, default_value = "human")]
        format: String,

        /// Generate SBOM (Software Bill of Materials)
        #[arg(long)]
        sbom: bool,

        /// Exit with error if vulnerabilities meet severity threshold
        #[arg(long)]
        fail_on: Option<String>,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// License compliance checking
    ///
    /// Scan project dependencies for license information and check compliance
    /// against configurable policies.
    ///
    /// Example usage:
    ///   linthis license                     # Scan current directory
    ///   linthis license --policy strict     # Use strict policy
    ///   linthis license --format json       # Output as JSON
    ///   linthis license --sbom              # Generate SBOM
    License {
        /// Path to scan (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Policy preset: default, strict, permissive
        #[arg(long, short = 'p', default_value = "default")]
        policy: String,

        /// Custom policy file path
        #[arg(long)]
        policy_file: Option<PathBuf>,

        /// Include dev dependencies
        #[arg(long)]
        include_dev: bool,

        /// Output format: human, json, spdx
        #[arg(short, long, default_value = "human")]
        format: String,

        /// Generate SBOM (Software Bill of Materials)
        #[arg(long)]
        sbom: bool,

        /// Exit with error if policy violations found
        #[arg(long)]
        fail_on_violation: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// Code complexity analysis
    ///
    /// Analyze code complexity metrics across your codebase including cyclomatic
    /// complexity, cognitive complexity, nesting depth, and function length.
    ///
    /// Example usage:
    ///   linthis complexity                  # Analyze current directory
    ///   linthis complexity src/             # Analyze specific directory
    ///   linthis complexity --format html    # Generate HTML report
    ///   linthis complexity --threshold 15   # Custom complexity threshold
    Complexity {
        /// Path to analyze (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// File patterns to include (glob patterns)
        #[arg(long, short = 'i')]
        include: Option<Vec<String>>,

        /// File patterns to exclude (glob patterns)
        #[arg(long, short = 'e')]
        exclude: Option<Vec<String>>,

        /// Complexity threshold for warnings (cyclomatic)
        #[arg(long, short = 't')]
        threshold: Option<u32>,

        /// Threshold preset: default, strict, lenient
        #[arg(long, default_value = "default")]
        preset: String,

        /// Output format: human, json, markdown, html
        #[arg(short, long, default_value = "human")]
        format: String,

        /// Include trend analysis from previous runs
        #[arg(long)]
        with_trends: bool,

        /// Number of historical runs for trends (default: 10)
        #[arg(short = 'n', long, default_value = "10")]
        trend_count: usize,

        /// Only show functions exceeding threshold
        #[arg(long)]
        only_high: bool,

        /// Sort output by: cyclomatic, cognitive, lines, name
        #[arg(long, default_value = "cyclomatic")]
        sort: String,

        /// Disable parallel processing
        #[arg(long)]
        no_parallel: bool,

        /// Exit with error if any file exceeds threshold
        #[arg(long)]
        fail_on_high: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
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
    /// Check tool availability and configuration health
    Doctor {
        /// Check all languages instead of only detected ones
        #[arg(long)]
        all: bool,

        /// Output format: human, json
        #[arg(short, long, default_value = "human")]
        output: String,
    },
    /// Start the Language Server Protocol (LSP) server
    ///
    /// The LSP server provides real-time linting diagnostics to editors
    /// like VS Code, Neovim, and other LSP-compatible clients.
    ///
    /// Example usage:
    ///   linthis lsp              # Start in stdio mode (default)
    ///   linthis lsp --mode tcp   # Start in TCP mode on port 9257
    ///   linthis lsp --use-plugin https://github.com/org/plugin.git
    Lsp {
        /// Communication mode: stdio (default) or tcp
        #[arg(long, default_value = "stdio")]
        mode: String,

        /// TCP port (only used when mode is tcp)
        #[arg(long, default_value = "9257")]
        port: u16,

        /// Use specific plugin(s) directly, bypassing config files
        ///
        /// Example: --use-plugin https://github.com/zhlinh/linthis-plugin-template
        #[arg(long, value_delimiter = ',')]
        use_plugin: Option<Vec<String>>,
    },
    /// Generate reports and analyze lint results
    ///
    /// Generate HTML reports, view statistics, analyze trends over time,
    /// and check code consistency across your codebase.
    ///
    /// Example usage:
    ///   linthis report stats              # Show statistics from last run
    ///   linthis report html               # Generate HTML report
    ///   linthis report html --with-trends # Include trend analysis
    ///   linthis report trends -n 20       # Analyze last 20 runs
    ///   linthis report consistency        # Check code consistency
    Report {
        #[command(subcommand)]
        action: ReportCommands,
    },
    /// Watch files for changes and auto-lint
    ///
    /// Monitors directories for file changes and automatically runs lint checks
    /// when files are modified. Supports a rich TUI interface or simple stdout mode.
    ///
    /// Example usage:
    ///   linthis watch                    # Watch current directory with TUI
    ///   linthis watch src/               # Watch specific directory
    ///   linthis watch --no-tui           # Watch with simple stdout output
    ///   linthis watch -c                 # Watch with check-only mode
    ///   linthis watch --notify           # Enable desktop notifications
    Watch {
        /// Paths to watch (defaults to current directory)
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,

        /// Only check, don't format
        #[arg(short = 'c', long)]
        check_only: bool,

        /// Only format, don't check
        #[arg(short = 'f', long)]
        format_only: bool,

        /// Debounce delay in milliseconds
        #[arg(long, default_value = "300")]
        debounce: u64,

        /// Enable desktop notifications
        #[arg(long)]
        notify: bool,

        /// Disable TUI (use simple stdout output)
        #[arg(long)]
        no_tui: bool,

        /// Clear screen before each run
        #[arg(long)]
        clear: bool,

        /// Specify languages to check (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        lang: Option<Vec<String>>,

        /// Exclude patterns (glob patterns)
        #[arg(short, long)]
        exclude: Option<Vec<String>>,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// Interactive fix mode for reviewing and fixing lint issues
    ///
    /// Review and fix issues one by one, with optional AI-powered suggestions.
    /// Supports loading from previous results or running check first.
    ///
    /// Example usage:
    ///   linthis fix                          # Load last result, interactive mode
    ///   linthis fix result.json              # Load specific result file
    ///   linthis fix -c                       # Run check first, then fix
    ///   linthis fix --ai                     # AI-assisted batch fix mode
    ///   linthis fix --ai -i src/main.rs --line 10  # AI fix for specific location
    Fix {
        /// Source of lint results: "last" (default) or a result file path
        #[arg(default_value = "last")]
        source: String,

        /// Run lint check first, then enter fix mode
        #[arg(short = 'c', long)]
        check: bool,

        /// Run format only first, then enter fix mode
        #[arg(short = 'f', long)]
        format_only: bool,

        /// Enable AI-powered fix suggestions
        #[arg(long)]
        ai: bool,

        /// AI provider: claude (default), claude-cli, openai, local, mock
        ///
        /// Priority: command line > LINTHIS_AI_PROVIDER env var > default (claude)
        #[arg(long, requires = "ai")]
        provider: Option<String>,

        /// Model name (defaults to provider's default)
        #[arg(long, requires = "ai")]
        model: Option<String>,

        /// Maximum suggestions per issue (default: 3)
        #[arg(long, default_value = "3", requires = "ai")]
        max_suggestions: usize,

        /// Automatically apply AI suggestions without confirmation
        ///
        /// Warning: This will modify files automatically. Use with caution.
        #[arg(long, requires = "ai")]
        auto_apply: bool,

        /// Number of parallel jobs for AI analysis (default: 8)
        ///
        /// Use -j 4 or --jobs 4 for parallel processing with 4 threads.
        /// Use -j 1 for sequential processing.
        #[arg(short = 'j', long, default_value = "8", requires = "ai")]
        jobs: usize,

        /// Target specific file for AI fix (use with --ai)
        #[arg(short = 'i', long = "include", requires = "ai")]
        file: Option<PathBuf>,

        /// Target specific line number (requires -i/--include)
        #[arg(long, requires = "file")]
        line: Option<u32>,

        /// Issue message for context (optional, use with --line)
        #[arg(long, requires = "line")]
        message: Option<String>,

        /// Rule ID for context (optional, use with --line)
        #[arg(long, requires = "line")]
        rule: Option<String>,

        /// Output format for AI mode: human, json, diff
        #[arg(short = 'o', long, default_value = "human")]
        output: String,

        /// Include code context in AI output
        #[arg(long)]
        with_context: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,

        /// Suppress non-error output
        #[arg(short, long)]
        quiet: bool,
    },
}

/// Hook subcommands
#[derive(clap::Subcommand, Debug)]
pub enum HookCommands {
    /// Install git hook (pre-commit, pre-push, or commit-msg)
    Install {
        /// Hook tool to use (git, prek, or pre-commit)
        #[arg(long = "type", value_name = "TYPE")]
        hook_type: Option<HookTool>,

        /// Git hook event type (pre-commit, pre-push, commit-msg)
        #[arg(long = "hook", value_name = "HOOK", default_value = "pre-commit")]
        hook_event: HookEvent,

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
    /// Uninstall git hook
    Uninstall {
        /// Git hook event type to uninstall (pre-commit, pre-push, commit-msg)
        #[arg(long = "hook", value_name = "HOOK")]
        hook_event: Option<HookEvent>,

        /// Uninstall all hooks
        #[arg(long)]
        all: bool,

        /// Non-interactive mode
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Show git hook status
    Status,
    /// Check for hook conflicts
    Check,
    /// Validate commit message format (used by commit-msg hook)
    CommitMsgCheck {
        /// Path to the commit message file
        msg_file: PathBuf,
    },
}

/// Plugin subcommands
#[derive(clap::Subcommand, Debug)]
pub enum PluginCommands {
    /// Create a new plugin from template
    ///
    /// Creates a plugin directory with example configs for supported languages.
    /// The generated plugin includes linting and formatting configs that can be
    /// customized for your team's coding standards.
    ///
    /// Example:
    ///   linthis plugin new my-company-standards
    ///   linthis plugin new my-plugin --languages rust,python,go
    New {
        /// Plugin name (will be used as directory name)
        name: String,
        /// Only include specific languages (comma-separated)
        /// Example: --languages rust,python,go
        #[arg(short, long, value_delimiter = ',')]
        languages: Option<Vec<String>>,
        /// Overwrite existing directory
        #[arg(long)]
        force: bool,
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
    /// Migrate existing linter/formatter configs to linthis format
    Migrate {
        /// Only migrate from specific tool (eslint, prettier, black, isort)
        #[arg(long = "from")]
        from_tool: Option<String>,
        /// Preview changes without applying them
        #[arg(long)]
        dry_run: bool,
        /// Create backup of original config files
        #[arg(long)]
        backup: bool,
        /// Show detailed output
        #[arg(short, long)]
        verbose: bool,
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

/// Cache subcommands
#[derive(clap::Subcommand, Debug)]
pub enum CacheCommands {
    /// Clear the lint cache
    Clear,
    /// Show cache statistics
    Status,
}

/// Report subcommands
#[derive(clap::Subcommand, Debug)]
pub enum ReportCommands {
    /// Generate an HTML report from lint results
    ///
    /// Creates a self-contained HTML file with charts, statistics,
    /// and detailed issue listings. Optionally includes trend analysis.
    Html {
        /// Source of lint results: "last" (default), "current", or a file path
        #[arg(default_value = "last")]
        source: String,

        /// Output file path (default: .linthis/reports/report-{timestamp}.html)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Include historical trend analysis in the report
        #[arg(long)]
        with_trends: bool,

        /// Number of historical runs to include in trends (default: 10)
        #[arg(short = 'n', long, default_value = "10")]
        trend_count: usize,
    },
    /// Show statistics from lint results
    ///
    /// Displays issue counts by severity, language, tool, and rule.
    /// Also shows top problematic files and summary metrics.
    Stats {
        /// Source of lint results: "last" (default), "current", or a file path
        #[arg(default_value = "last")]
        source: String,

        /// Output format: human (default), json
        #[arg(short, long, default_value = "human")]
        format: String,
    },
    /// Analyze code quality trends over time
    ///
    /// Examines historical lint results to identify trends in code quality.
    /// Shows whether issues are improving, stable, or degrading.
    Trends {
        /// Number of historical runs to analyze (default: 10)
        #[arg(short = 'n', long, default_value = "10")]
        count: usize,

        /// Output format: human (default), json
        #[arg(short, long, default_value = "human")]
        format: String,
    },
    /// Analyze team code style consistency
    ///
    /// Identifies repeated patterns, outlier files, and systematic issues
    /// to help improve code consistency across the codebase.
    Consistency {
        /// Source of lint results: "last" (default), "current", or a file path
        #[arg(default_value = "last")]
        source: String,

        /// Output format: human (default), json
        #[arg(short, long, default_value = "human")]
        format: String,
    },
}
