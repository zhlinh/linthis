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
    about = "A fast, cross-platform multi-language linter and formatter",
    disable_version_flag = true
)]
#[non_exhaustive]
pub struct Cli {
    /// Print version
    #[arg(short = 'v', long = "version", action = clap::ArgAction::Version)]
    pub version: (),

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

    /// Check only locally modified files (staged + unstaged).
    ///
    /// Covers all files showing `modified:` in `git status` — both staged
    /// (index) and unstaged (working tree) changes in tracked files.
    /// Requires a git repository.
    #[arg(long, short = 'm', alias = "uncommitted")]
    pub modified: bool,

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
    #[arg(long)]
    pub verbose: bool,

    /// Suppress non-error output
    #[arg(short, long)]
    pub quiet: bool,

    /// Run benchmark comparing ruff vs flake8+black for Python
    #[arg(long)]
    pub benchmark: bool,

    /// Disable tool auto-install (overrides config)
    #[arg(long, default_value_t = false)]
    pub no_tool_auto_install: bool,

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

    /// AI auto-fix: check + fix issues automatically (equivalent to --fix --ai -y)
    ///
    /// Shorthand for running AI-powered fix mode without confirmation.
    /// Can specify provider with --provider.
    #[arg(long)]
    pub auto_fix: bool,

    /// Enter fix mode after check/format to fix issues
    ///
    /// Can be combined with --ai for AI-powered fixes
    #[arg(long)]
    pub fix: bool,

    /// Use AI for fix suggestions (requires --fix or --auto-fix)
    #[arg(long)]
    pub ai: bool,

    /// AI provider for fix (requires --ai or --auto-fix)
    ///
    /// Options: claude, claude-cli, codebuddy, codebuddy-cli, openai, local, mock
    #[arg(long)]
    pub provider: Option<String>,

    /// Automatically accept all fix suggestions (requires --fix)
    ///
    /// Warning: This will modify files automatically. Use with caution.
    #[arg(short = 'y', long = "yes", alias = "accept-all")]
    pub accept_all: bool,

    /// Hook event: enable compact output format for git hooks
    /// Shows summary at top, lists errors with file:line, and provides fix commands
    /// Optional value specifies hook type: pre-commit (default), pre-push, commit-msg
    #[arg(long = "hook-event", hide = true, value_name = "HOOK_TYPE", num_args = 0..=1, default_missing_value = "pre-commit")]
    pub hook_mode: Option<String>,

    /// Plugin subcommands (init, list, clean)
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Hook management tools
#[derive(Clone, Debug, PartialEq, clap::ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum HookTool {
    /// Traditional git hook
    Git,
    /// Git hook with AI agent fix fallback on failure
    GitWithAgent,
    /// AI coding agent integration (Claude Code, Cursor, etc.)
    Agent,
    /// Prek (Rust-based, faster)
    Prek,
    /// Prek with AI agent fix fallback on failure
    PrekWithAgent,
}

impl HookTool {
    /// Returns the base tool without the agent fix variant, if applicable
    pub fn base_tool(&self) -> &HookTool {
        match self {
            HookTool::GitWithAgent => &HookTool::Git,
            HookTool::PrekWithAgent => &HookTool::Prek,
            other => other,
        }
    }

    /// Returns true if this type includes an AI agent fix fallback
    pub fn has_agent_fix(&self) -> bool {
        matches!(
            self,
            HookTool::GitWithAgent | HookTool::PrekWithAgent
        )
    }

    /// Get the CLI string representation (kebab-case, matches clap ValueEnum)
    pub fn as_str(&self) -> &'static str {
        match self {
            HookTool::Git => "git",
            HookTool::GitWithAgent => "git-with-agent",
            HookTool::Agent => "agent",
            HookTool::Prek => "prek",
            HookTool::PrekWithAgent => "prek-with-agent",
        }
    }
}

/// AI agent CLI providers for automatic fix on hook failure (--type *-with-agent)
#[derive(Clone, Debug, clap::ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum AgentFixProvider {
    /// Anthropic Claude Code CLI (claude -p "prompt")
    Claude,
    /// OpenAI Codex CLI (codex exec "prompt")
    Codex,
    /// Google Gemini CLI (gemini -p "prompt")
    Gemini,
    /// Cursor AI agent CLI (cursor-agent chat "prompt")
    Cursor,
    /// Factory Droid CLI (droid exec --auto low "prompt")
    Droid,
    /// Augment Code Auggie CLI (auggie --print "prompt")
    Auggie,
    /// CodeBuddy CLI (codebuddy -p "prompt")
    Codebuddy,
    /// OpenClaw agent CLI (openclaw agent --message "prompt")
    Openclaw,
}

impl AgentFixProvider {
    /// Get the CLI string representation (matches clap ValueEnum, parseable by resolve_agent_fix_provider)
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentFixProvider::Claude => "claude",
            AgentFixProvider::Codex => "codex",
            AgentFixProvider::Gemini => "gemini",
            AgentFixProvider::Cursor => "cursor",
            AgentFixProvider::Droid => "droid",
            AgentFixProvider::Auggie => "auggie",
            AgentFixProvider::Codebuddy => "codebuddy",
            AgentFixProvider::Openclaw => "openclaw",
        }
    }
}

impl std::fmt::Display for AgentFixProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentFixProvider::Claude => write!(f, "Claude Code"),
            AgentFixProvider::Codex => write!(f, "Codex"),
            AgentFixProvider::Gemini => write!(f, "Gemini"),
            AgentFixProvider::Cursor => write!(f, "Cursor"),
            AgentFixProvider::Droid => write!(f, "Droid"),
            AgentFixProvider::Auggie => write!(f, "Auggie"),
            AgentFixProvider::Codebuddy => write!(f, "CodeBuddy"),
            AgentFixProvider::Openclaw => write!(f, "OpenClaw"),
        }
    }
}

/// AI coding agent providers (for skills/settings installation, --type agent)
#[derive(Clone, Debug, clap::ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum AgentProvider {
    /// Claude Code (CLAUDE.md / .claude/)
    Claude,
    /// OpenAI Codex (AGENTS.md / .codex/)
    Codex,
    /// Google Gemini (.gemini/)
    Gemini,
    /// Cursor (.cursor/)
    Cursor,
    /// Factory Droid (.droid/)
    Droid,
    /// Augment Code (.augment/)
    Auggie,
    /// CodeBuddy (.codebuddy/)
    Codebuddy,
    /// OpenClaw (.openclaw/)
    Openclaw,
}

impl std::fmt::Display for AgentProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentProvider::Claude => write!(f, "Claude Code"),
            AgentProvider::Codex => write!(f, "Codex"),
            AgentProvider::Gemini => write!(f, "Gemini"),
            AgentProvider::Cursor => write!(f, "Cursor"),
            AgentProvider::Droid => write!(f, "Droid"),
            AgentProvider::Auggie => write!(f, "Auggie"),
            AgentProvider::Codebuddy => write!(f, "CodeBuddy"),
            AgentProvider::Openclaw => write!(f, "OpenClaw"),
        }
    }
}

/// Git hook event types
#[derive(Clone, Debug, Default, PartialEq, clap::ValueEnum)]
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

    /// Get the CLI string representation (kebab-case, matches clap ValueEnum)
    pub fn as_str(&self) -> &'static str {
        self.hook_filename()
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
        #[arg(long)]
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
        #[arg(long)]
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
        #[arg(long)]
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
    /// Validate commit message format (Conventional Commits)
    ///
    /// Accepts either a path to the commit message file (as used by git's
    /// commit-msg hook) or the message string directly.
    ///
    /// Example usage:
    ///   linthis cmsg "feat: add new feature"
    ///   linthis cmsg "fix(api): handle null response"
    ///   linthis cmsg .git/COMMIT_EDITMSG
    ///   linthis cmsg .git/COMMIT_EDITMSG --auto-fix   # AI rewrite on failure
    Cmsg {
        /// Path to commit message file, or the commit message string directly.
        msg_or_file: String,

        /// Automatically rewrite invalid commit messages using AI
        ///
        /// When the message fails validation, AI rewrites it to conform to
        /// Conventional Commits format and writes it back (if source is a file).
        #[arg(long)]
        auto_fix: bool,

        /// AI provider for auto-fix
        #[arg(long, requires = "auto_fix")]
        provider: Option<String>,
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
        #[arg(long)]
        verbose: bool,
    },
    /// Format files and manage format backups
    ///
    /// Run formatters on files with automatic backup before changes.
    /// Supports undo to restore files to their pre-format state.
    ///
    /// Example usage:
    ///   linthis format                       # Format all files
    ///   linthis format -s                    # Format staged files
    ///   linthis format -i src/main.rs        # Format specific file
    ///   linthis format --undo                # Undo last format
    ///   linthis format --list-backups        # List available backups
    Format {
        /// Files or directories to include
        #[arg(short = 'i', long = "include")]
        paths: Vec<PathBuf>,

        /// Format only staged files (git cached)
        #[arg(short = 's', long)]
        staged: bool,

        /// Format only locally modified files (staged + unstaged)
        ///
        /// Covers all files showing `modified:` in `git status` — both staged
        /// (index) and unstaged (working tree) changes in tracked files.
        #[arg(short = 'm', long, alias = "uncommitted")]
        modified: bool,

        /// Exclude patterns (glob patterns)
        #[arg(short, long)]
        exclude: Option<Vec<String>>,

        /// Restore files from the last backup (undo previous format)
        ///
        /// Backups are automatically created before each format operation.
        /// Use this to revert changes if the format produced unwanted results.
        #[arg(long)]
        undo: bool,

        /// Backup name to restore (use with --undo, defaults to latest)
        #[arg(default_value = "last")]
        source: String,

        /// List available backups
        #[arg(long)]
        list_backups: bool,

        /// Verbose output
        #[arg(long)]
        verbose: bool,

        /// Suppress non-error output
        #[arg(short, long)]
        quiet: bool,
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

        /// AI auto-fix: apply all AI suggestions automatically (equivalent to --ai -y)
        #[arg(long = "auto")]
        auto_fix: bool,

        /// Enable AI-powered fix suggestions
        #[arg(long)]
        ai: bool,

        /// AI provider: claude (default), claude-cli, codebuddy, codebuddy-cli, openai, local, mock
        ///
        /// Priority: CLI > env var (LINTHIS_AI_PROVIDER) > config file ([ai] section) > default
        #[arg(long)]
        provider: Option<String>,

        /// Model name (defaults to provider's default)
        #[arg(long)]
        model: Option<String>,

        /// Maximum suggestions per issue (default: 3)
        #[arg(long, default_value = "3")]
        max_suggestions: usize,

        /// Automatically accept all AI suggestions without confirmation
        ///
        /// Warning: This will modify files automatically. Use with caution.
        #[arg(short = 'y', long = "yes", alias = "accept-all")]
        accept_all: bool,

        /// Number of parallel jobs for AI analysis (default: 4)
        ///
        /// Use -j 4 or --jobs 4 for parallel processing with 4 threads.
        /// Use -j 1 for sequential processing.
        #[arg(short = 'j', long, default_value = "4", requires = "ai")]
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
        #[arg(long)]
        verbose: bool,

        /// Suppress non-error output
        #[arg(short, long)]
        quiet: bool,

        /// Restore files from the last backup (undo previous fix)
        ///
        /// Backups are automatically created before each fix operation.
        /// Use this to revert changes if the fix produced unwanted results.
        #[arg(long)]
        undo: bool,

        /// List available backups
        #[arg(long)]
        list_backups: bool,
    },
    /// AI-powered code review
    ///
    /// Analyze git diffs using AI to find code quality, security, and architecture issues.
    /// Supports background execution via pre-push hooks, auto-fix with PR/MR creation,
    /// and configurable notifications.
    ///
    /// Example usage:
    ///   linthis review                      # Review current branch vs remote
    ///   linthis review --auto-fix           # Review + auto-fix + create PR
    ///   linthis review -r alice -r bob      # Specify reviewers
    ///   linthis review --base main          # Diff against main branch
    ///   linthis review --background         # Run in background
    ///   linthis review --status             # Check background review status
    Review {
        /// Run review in background (async, non-blocking)
        #[arg(long, short = 'b')]
        background: bool,

        /// Enable AI auto-fix and create PR/MR with fixes
        #[arg(long)]
        auto_fix: bool,

        /// Auto-fix mode: pr, commit, apply (default: apply)
        #[arg(long)]
        auto_fix_mode: Option<String>,

        /// Specify reviewer(s) for PR/MR (repeatable)
        #[arg(long = "reviewer", short = 'r')]
        reviewers: Option<Vec<String>>,

        /// AI provider to use
        #[arg(long)]
        provider: Option<String>,

        /// Base branch/commit for diff comparison
        #[arg(long)]
        base: Option<String>,

        /// HEAD ref to review (default: HEAD)
        #[arg(long, default_value = "HEAD")]
        head: String,

        /// Generate report only, do not create PR/MR
        #[arg(long)]
        no_pr: bool,

        /// Notification channels to use (repeatable)
        #[arg(long)]
        notify: Option<Vec<String>>,

        /// Check status of background reviews
        #[arg(long)]
        status: bool,

        /// Preview auto-fix actions without pushing or creating PR
        #[arg(long)]
        dry_run: bool,

        /// Remove old review artifacts
        #[arg(long)]
        clean: bool,

        /// Output format: markdown or json
        #[arg(short, long, default_value = "markdown")]
        output: String,
    },
}

/// Hook subcommands
#[derive(clap::Subcommand, Debug)]
pub enum HookCommands {
    /// Install git hooks or AI agent skills
    ///
    /// Hook types (--type):
    ///
    ///   {git,prek}             Traditional shell hook
    ///
    ///   {git,prek}-with-agent  Shell hook + AI auto-fix on failure
    ///
    ///   agent                             AI coding agent lint skill (Claude, Cursor, etc.)
    Install {
        /// Hook tool(s) to use — comma-separated or repeated
        /// (e.g. --type git,agent or --type git --type agent)
        ///
        /// Shell hooks:          git, prek
        /// Shell hook + AI fix:  git-with-agent, prek-with-agent
        /// AI agent skills:      agent
        ///
        /// If both x and x-with-agent are given, x-with-agent wins.
        /// If omitted, an interactive menu is shown (or git when -y is set).
        #[arg(long = "type", value_name = "TYPE", value_delimiter = ',', num_args = 0..)]
        hook_types: Vec<HookTool>,

        /// Git hook event(s) — comma-separated or repeated
        /// (e.g. --event pre-commit,pre-push)
        ///
        /// If omitted, an interactive menu is shown (or pre-commit when -y is set,
        /// or all three events when --type agent is the only type with -y).
        #[arg(long = "event", value_name = "EVENT", value_delimiter = ',', num_args = 0..)]
        hook_events: Vec<HookEvent>,

        /// Force overwrite existing hook
        #[arg(long)]
        force: bool,

        /// Non-interactive mode (use defaults, no prompts)
        #[arg(short = 'y', long)]
        yes: bool,

        /// Install globally:
        ///
        /// - For --type agent: installs skills into user home directory (~/.claude/, ~/.cursor/, etc.)
        ///
        /// - For other types: installs hook into ~/.config/git/hooks/ and sets core.hooksPath
        ///   (Strategy B: local hook takes priority; global runs linthis only when local has no linthis)
        #[arg(short = 'g', long)]
        global: bool,

        /// AI provider: claude, codex, gemini, cursor, droid, auggie, codebuddy
        ///
        /// For --type agent: installs skill/settings files for the provider
        ///
        /// For --type *-with-agent: uses the provider's headless CLI to auto-fix
        #[arg(long)]
        provider: Option<String>,

        /// Extra arguments for the linthis command in the hook script
        ///
        /// Default: "-c -f" (check + format).
        /// Examples: "-c" (check only), "-f" (format only),
        /// "-c -f --auto-fix --provider claude" (AI auto-fix)
        #[arg(long, allow_hyphen_values = true)]
        args: Option<String>,

        /// Extra arguments passed to the AI agent CLI (e.g. "--model opus")
        #[arg(long, allow_hyphen_values = true)]
        provider_args: Option<String>,
    },
    /// Uninstall git hook
    Uninstall {
        /// Hook tool(s) to uninstall — comma-separated or repeated
        /// (e.g. --type git,agent or --type git --type agent)
        #[arg(long = "type", value_name = "TYPE", value_delimiter = ',', num_args = 0..)]
        hook_types: Vec<HookTool>,

        /// Git hook event(s) to uninstall — comma-separated or repeated
        /// (e.g. --event pre-commit,pre-push)
        #[arg(long = "event", value_name = "EVENT", value_delimiter = ',', num_args = 0..)]
        hook_events: Vec<HookEvent>,

        /// Uninstall all hooks (all types × all events)
        #[arg(long, conflicts_with_all = ["all_types", "all_events"])]
        all: bool,

        /// Uninstall all hook types for the specified --event(s).
        /// Requires at least one --event. Equivalent to specifying every --type.
        #[arg(long, conflicts_with = "all", requires = "hook_events")]
        all_types: bool,

        /// Uninstall all hook events for the specified --type(s).
        /// Requires at least one --type. Equivalent to specifying every --event.
        #[arg(long, conflicts_with = "all", requires = "hook_types")]
        all_events: bool,

        /// Non-interactive mode
        #[arg(short = 'y', long)]
        yes: bool,

        /// Uninstall from global location:
        /// - For --type agent: removes skills from user home directory
        /// - For other types: removes hook from ~/.config/git/hooks/ and unsets core.hooksPath if empty
        #[arg(short = 'g', long)]
        global: bool,
    },
    /// Show git hook status
    Status,
    /// Check for hook conflicts
    Check,
    /// Validate commit message format (used by commit-msg hook)
    ///
    /// Deprecated: use `linthis cmsg` instead.
    #[command(hide = true)]
    CommitMsgCheck {
        /// Path to commit message file, or the commit message string directly.
        msg_or_file: String,
    },
    /// Execute hook logic at runtime (used internally by thin wrapper scripts)
    ///
    /// This command is invoked by the thin wrapper scripts installed in .git/hooks/.
    /// It generates and executes the full hook script dynamically from the current
    /// linthis binary, so hook logic is always up-to-date after binary upgrades.
    #[command(hide = true)]
    Run {
        /// Git hook event
        #[arg(long)]
        event: HookEvent,
        /// Hook tool type
        #[arg(long = "type", value_name = "TYPE")]
        hook_type: HookTool,
        /// AI provider (for *-with-agent types)
        #[arg(long)]
        provider: Option<String>,
        /// Extra arguments passed to the AI agent CLI
        #[arg(long, allow_hyphen_values = true)]
        provider_args: Option<String>,
        /// Run against global hooks
        #[arg(short = 'g', long)]
        global: bool,
        /// Passthrough arguments from the hook invocation (e.g. commit message file path)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        hook_args: Vec<String>,
    },
    /// Re-sync installed hooks (thin wrapper scripts + agent skill components)
    ///
    /// Re-generates hook scripts and refreshes agent skills (CLAUDE.md, skill files, etc.)
    /// from the current linthis binary. Useful after upgrading linthis or plugins.
    ///
    /// Reads installed hook parameters from ~/.linthis/installed-hooks.toml.
    Sync {
        /// Re-sync global hooks instead of local project hooks
        #[arg(short = 'g', long)]
        global: bool,
        /// Non-interactive (skip confirmation prompts)
        #[arg(short = 'y', long)]
        yes: bool,
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
        #[arg(long)]
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
        /// Plugin alias to sync (syncs all if omitted)
        #[arg(value_name = "ALIAS")]
        plugin: Option<String>,
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
        #[arg(long)]
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
        #[arg(long)]
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
    /// Show full lint results in human-readable format
    ///
    /// Display saved lint results (JSON) in the same human-readable format
    /// as `linthis -c` output. Useful for viewing full error details from
    /// hook output or reviewing previous runs.
    ///
    /// Example usage:
    ///   linthis report show                 # Show last result
    ///   linthis report show result.json     # Show specific file
    ///   linthis report show -n 10           # Limit to 10 issues
    ///   linthis report show --compact       # Compact format (no code context)
    ///   linthis report show --errors-only   # Only show errors
    Show {
        /// Source of lint results: "last" (default) or a result file path
        #[arg(default_value = "last")]
        source: String,

        /// Limit number of issues to display (0 = unlimited)
        #[arg(short = 'n', long, default_value = "0")]
        limit: usize,

        /// Compact format: show only file:line message without code context
        #[arg(long)]
        compact: bool,

        /// Only show errors (skip warnings and info)
        #[arg(long)]
        errors_only: bool,

        /// Only show warnings (skip errors and info)
        #[arg(long)]
        warnings_only: bool,

        /// Output format: human (default), json
        #[arg(short, long, default_value = "human")]
        format: String,
    },
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
