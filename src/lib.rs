// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! # linthis
//!
//! A fast, cross-platform multi-language linting aggregator that unifies
//! various language-specific linters under a single configuration.
//!
//! ## Quick Start
//!
//! ```bash
//! # Run linting and formatting on current directory
//! linthis
//!
//! # Check specific files (lint only)
//! linthis -c src/main.rs
//!
//! # Format only
//! linthis -f src/
//!
//! # Output as JSON
//! linthis --output json
//!
//! # Skip cache for fresh results
//! linthis --no-cache
//! ```
//!
//! ## Features
//!
//! - **Multi-language support**: Rust, Python, TypeScript, JavaScript, Go, Java,
//!   C++, Objective-C, Swift, Kotlin, Lua, Dart, Shell, Ruby, PHP, Scala, C#
//! - **Unified configuration**: Single `.linthis/config.toml` for all languages
//! - **Custom regex rules**: Define project-specific lint rules
//! - **Plugin system**: Extend with community or custom plugins
//! - **Git hooks integration**: Pre-commit and pre-push hooks
//! - **Intelligent caching**: Skip unchanged files for faster runs
//! - **Parallel processing**: Leverage multiple cores for speed
//!
//! ## Library Usage
//!
//! ```rust,no_run
//! use linthis::{run, RunOptions, RunMode, Language};
//! use std::path::PathBuf;
//!
//! // Run with default options (current directory)
//! let options = RunOptions::default();
//! let result = run(&options).expect("Lint run failed");
//!
//! println!("Found {} issues in {} files", result.issues.len(), result.total_files);
//!
//! // Custom options
//! let options = RunOptions {
//!     paths: vec![PathBuf::from("src/")],
//!     mode: RunMode::CheckOnly,
//!     languages: vec![Language::Rust],
//!     verbose: true,
//!     ..Default::default()
//! };
//! let result = run(&options).expect("Lint run failed");
//! ```
//!
//! ## Configuration
//!
//! Configuration files are loaded with the following precedence:
//!
//! 1. CLI arguments (highest)
//! 2. Project config (`.linthis/config.toml`)
//! 3. User config (`~/.linthis/config.toml`)
//! 4. Built-in defaults (lowest)
//!
//! Example configuration:
//!
//! ```toml
//! # .linthis/config.toml
//! excludes = ["vendor/**", "*.generated.*"]
//! max_complexity = 20
//!
//! [rules]
//! disable = ["E501", "whitespace/*"]
//!
//! [[rules.custom]]
//! code = "custom/no-todo"
//! pattern = "TODO|FIXME"
//! message = "Found TODO comment"
//! severity = "warning"
//!
//! [rust]
//! max_complexity = 15
//! ```
//!
//! ## Modules
//!
//! - [`config`]: Configuration loading and merging
//! - [`checkers`]: Language-specific lint checkers
//! - [`formatters`]: Language-specific code formatters
//! - [`rules`]: Custom rules and rule filtering
//! - [`plugin`]: Plugin system for extensions
//! - [`cache`]: Result caching for performance
//! - [`reports`]: Output formatting (text, JSON, SARIF)
//!
//! ## Error Handling
//!
//! The library uses [`LintisError`] for all error types, which includes:
//!
//! - I/O errors (file access)
//! - Configuration errors (invalid TOML/YAML)
//! - Checker errors (tool execution failures)
//! - Tool availability errors (missing linters)

pub mod benchmark;
pub mod cache;
pub mod checkers;
pub mod config;
pub mod hooks;
/// Thin re-export of CLI command types for use in tests and downstream crates.
pub mod cli {
    #[path = "commands.rs"]
    pub mod commands;
}
pub mod agent_stream;
pub mod ai;
pub mod complexity;
pub mod fixers;
pub mod formatters;
pub mod gitignore;
pub mod interactive;
pub mod license;
pub mod lsp;
pub mod plugin;
pub mod presets;
pub mod reports;
pub mod review;
pub mod rules;
pub mod security;
pub mod self_update;
pub mod templates;
pub mod tools;
pub mod tui;
pub mod utils;
pub mod vcs;
pub mod watch;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use thiserror::Error;

use rayon::prelude::*;

use config::resolver::{ConfigResolver, SharedConfigResolver};

/// Global progress counter for parallel checking
static PROGRESS_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Track which tool warnings have been shown (to avoid duplicate warnings)
static WARNED_TOOLS: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// Track unavailable tools for reporting in RunResult
static UNAVAILABLE_TOOLS: Mutex<Option<Vec<utils::types::UnavailableTool>>> = Mutex::new(None);

/// Track tools where auto-install was attempted but failed
static AUTO_INSTALL_FAILED: Mutex<Option<HashSet<String>>> = Mutex::new(None);

use cache::LintCache;
use checkers::{
    Checker, CppChecker, DartChecker, GoChecker, JavaChecker, KotlinChecker, LuaChecker,
    PythonChecker, RustChecker, SwiftChecker, TypeScriptChecker,
};
use config::Config;
use formatters::{
    CppFormatter, DartFormatter, Formatter, GoFormatter, JavaFormatter, KotlinFormatter,
    LuaFormatter, PythonFormatter, RustFormatter, SwiftFormatter, TypeScriptFormatter,
};
use rules::{CustomRulesChecker, RuleFilter};
use utils::types::RunResult;
use utils::walker::{walk_paths, WalkerConfig};

#[derive(Error, Debug)]
pub enum LintisError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("{tool} error for '{file}': {message}")]
    Checker {
        tool: String,
        file: PathBuf,
        message: String,
    },

    #[error("{tool} error for '{file}': {message}")]
    Formatter {
        tool: String,
        file: PathBuf,
        message: String,
    },

    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("Cache error ({operation}): {message}")]
    Cache { operation: String, message: String },

    #[error("Tool not available: {tool} ({language}). {install_hint}")]
    ToolNotAvailable {
        tool: String,
        language: String,
        install_hint: String,
    },

    #[error("{0}")]
    Generic(String),
}

impl LintisError {
    /// Create a checker error with tool, file, and message context.
    pub fn checker(tool: &str, file: &Path, message: impl Into<String>) -> Self {
        LintisError::Checker {
            tool: tool.to_string(),
            file: file.to_path_buf(),
            message: message.into(),
        }
    }

    /// Create a formatter error with tool, file, and message context.
    pub fn formatter(tool: &str, file: &Path, message: impl Into<String>) -> Self {
        LintisError::Formatter {
            tool: tool.to_string(),
            file: file.to_path_buf(),
            message: message.into(),
        }
    }

    /// Create a cache error with operation and message context.
    pub fn cache(operation: &str, message: impl Into<String>) -> Self {
        LintisError::Cache {
            operation: operation.to_string(),
            message: message.into(),
        }
    }

    /// Create a tool not available error with installation hint.
    pub fn tool_not_available(tool: &str, language: &str, hint: &str) -> Self {
        LintisError::ToolNotAvailable {
            tool: tool.to_string(),
            language: language.to_string(),
            install_hint: hint.to_string(),
        }
    }

    /// Get numeric error code for programmatic handling.
    pub fn error_code(&self) -> i32 {
        match self {
            LintisError::Io(_) => 10,
            LintisError::Config(_) => 20,
            LintisError::Checker { .. } => 30,
            LintisError::Formatter { .. } => 40,
            LintisError::UnsupportedLanguage(_) => 50,
            LintisError::Cache { .. } => 60,
            LintisError::ToolNotAvailable { .. } => 70,
            LintisError::Generic(_) => 80,
        }
    }

    /// Check if error allows graceful degradation.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            LintisError::ToolNotAvailable { .. }
                | LintisError::Cache { .. }
                | LintisError::Generic(_)
        )
    }
}

impl From<serde_json::Error> for LintisError {
    fn from(err: serde_json::Error) -> Self {
        LintisError::cache("json_parse", err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, LintisError>;

impl From<plugin::PluginError> for LintisError {
    fn from(err: plugin::PluginError) -> Self {
        match err {
            plugin::PluginError::Io(e) => LintisError::Io(e),
            plugin::PluginError::ConfigError { message } => LintisError::Config(message),
            plugin::PluginError::ConfigNotFound { path } => {
                LintisError::Config(format!("Config file not found: {}", path.display()))
            }
            _ => LintisError::Config(err.to_string()),
        }
    }
}

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Cpp,
    ObjectiveC,
    Java,
    Python,
    Rust,
    Go,
    JavaScript,
    TypeScript,
    Dart,
    Swift,
    Kotlin,
    Lua,
    Shell,
    Ruby,
    Php,
    Scala,
    CSharp,
}

impl Language {
    pub fn from_extension(ext: &str) -> Option<Self> {
        let lower = ext.to_lowercase();
        Self::from_extension_compiled(&lower).or_else(|| Self::from_extension_scripting(&lower))
    }

    /// Match compiled language extensions.
    fn from_extension_compiled(ext: &str) -> Option<Self> {
        match ext {
            "c" | "cc" | "cpp" | "cxx" | "hpp" | "hxx" => Some(Language::Cpp),
            "h" => None, // Handle .h files specially in from_path()
            "m" | "mm" => Some(Language::ObjectiveC),
            "java" => Some(Language::Java),
            "rs" => Some(Language::Rust),
            "go" => Some(Language::Go),
            "dart" => Some(Language::Dart),
            "swift" => Some(Language::Swift),
            "kt" | "kts" => Some(Language::Kotlin),
            "scala" | "sc" => Some(Language::Scala),
            "cs" | "csx" => Some(Language::CSharp),
            _ => None,
        }
    }

    /// Match scripting/interpreted language extensions.
    fn from_extension_scripting(ext: &str) -> Option<Self> {
        match ext {
            "py" | "pyw" => Some(Language::Python),
            "js" | "jsx" | "mjs" | "cjs" => Some(Language::JavaScript),
            "ts" | "tsx" | "mts" | "cts" => Some(Language::TypeScript),
            "lua" => Some(Language::Lua),
            "sh" | "bash" | "zsh" | "ksh" => Some(Language::Shell),
            "rb" | "rake" | "gemspec" => Some(Language::Ruby),
            "php" | "phtml" => Some(Language::Php),
            _ => None,
        }
    }

    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension().and_then(|e| e.to_str())?;

        // Special handling for .h files - smart detection
        if ext.eq_ignore_ascii_case("h") {
            return Some(Self::detect_header_language(path));
        }

        Self::from_extension(ext)
    }

    /// Smart detection for .h header files to determine if it's C++/C or Objective-C
    fn detect_header_language(path: &Path) -> Self {
        // 1. Check for corresponding implementation files
        if let Some(lang) = Self::detect_by_sibling_impl(path) {
            return lang;
        }

        // 2. Check file content for language-specific patterns
        if let Some(lang) = Self::detect_by_content(path) {
            return lang;
        }

        // 3. Check directory for other files to infer language
        if let Some(lang) = Self::detect_by_directory_context(path) {
            return lang;
        }

        // 4. Default to C++
        Language::Cpp
    }

    /// Check for sibling .m/.mm (Objective-C) or .cpp/.cc/.cxx/.c (C++) files.
    fn detect_by_sibling_impl(path: &Path) -> Option<Self> {
        let parent = path.parent()?;
        let stem = path.file_stem().and_then(|s| s.to_str())?;

        for ext in &["m", "mm"] {
            if parent.join(format!("{}.{}", stem, ext)).exists() {
                return Some(Language::ObjectiveC);
            }
        }
        for ext in &["cpp", "cc", "cxx", "c"] {
            if parent.join(format!("{}.{}", stem, ext)).exists() {
                return Some(Language::Cpp);
            }
        }
        None
    }

    /// Detect language from file content patterns.
    fn detect_by_content(path: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;

        if Self::content_has_objc_patterns(&content) {
            return Some(Language::ObjectiveC);
        }
        if Self::contains_ns_type(&content) {
            return Some(Language::ObjectiveC);
        }

        let cpp_patterns = ["namespace ", "template<", "template <"];
        for pattern in cpp_patterns {
            if content.contains(pattern) {
                return Some(Language::Cpp);
            }
        }
        None
    }

    /// Check if content contains Objective-C specific patterns.
    fn content_has_objc_patterns(content: &str) -> bool {
        const OBJC_PATTERNS: &[&str] = &[
            "#import",
            "@import",
            "@interface",
            "@implementation",
            "@protocol",
            "@property",
            "@synthesize",
            "@dynamic",
            "@selector",
            "@class",
            "@end",
            "NS_ASSUME_NONNULL_BEGIN",
            "NS_ENUM",
            "NS_OPTIONS",
            "nullable",
            "nonnull",
            "+ (",
            "- (",
            " @\"",
            " @[",
        ];
        OBJC_PATTERNS.iter().any(|p| content.contains(p))
    }

    /// Check directory siblings for language context clues.
    fn detect_by_directory_context(path: &Path) -> Option<Self> {
        let parent = path.parent()?;
        let entries = std::fs::read_dir(parent).ok()?;
        let mut has_objc = false;
        let mut has_cpp = false;
        for entry in entries.filter_map(|e| e.ok()) {
            if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                match ext {
                    "m" | "mm" => has_objc = true,
                    "cpp" | "cc" | "cxx" => has_cpp = true,
                    _ => {}
                }
            }
        }
        if has_objc && !has_cpp {
            return Some(Language::ObjectiveC);
        }
        None
    }

    /// Check if content contains Foundation types (NS followed by uppercase letter).
    /// Examples: NSString, NSArray, NSDictionary, NSObject, NSURL, etc.
    fn contains_ns_type(content: &str) -> bool {
        let bytes = content.as_bytes();
        let len = bytes.len();

        // Look for "NS" followed by an uppercase letter A-Z
        for i in 0..len.saturating_sub(2) {
            if bytes[i] == b'N' && bytes[i + 1] == b'S' {
                let next_char = bytes[i + 2];
                // Check if next char is uppercase A-Z (ASCII 65-90)
                if next_char.is_ascii_uppercase() {
                    // Make sure it's not part of a longer identifier before "NS"
                    // (i.e., NS should be at word boundary)
                    if i == 0 || !Self::is_identifier_char(bytes[i - 1]) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if a byte is a valid identifier character (alphanumeric or underscore)
    fn is_identifier_char(b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_'
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "cpp" | "c++" | "cxx" => Some(Language::Cpp),
            "oc" | "objc" | "objective-c" | "objectivec" => Some(Language::ObjectiveC),
            "java" => Some(Language::Java),
            "python" | "py" => Some(Language::Python),
            "rust" | "rs" => Some(Language::Rust),
            "go" | "golang" => Some(Language::Go),
            "javascript" | "js" => Some(Language::JavaScript),
            "typescript" | "ts" => Some(Language::TypeScript),
            "dart" => Some(Language::Dart),
            "swift" => Some(Language::Swift),
            "kotlin" | "kt" => Some(Language::Kotlin),
            "lua" => Some(Language::Lua),
            "shell" | "sh" | "bash" | "zsh" => Some(Language::Shell),
            "ruby" | "rb" => Some(Language::Ruby),
            "php" => Some(Language::Php),
            "scala" => Some(Language::Scala),
            "csharp" | "c#" | "cs" => Some(Language::CSharp),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Language::Cpp => "cpp",
            Language::ObjectiveC => "oc",
            Language::Java => "java",
            Language::Python => "python",
            Language::Rust => "rust",
            Language::Go => "go",
            Language::JavaScript => "javascript",
            Language::TypeScript => "typescript",
            Language::Dart => "dart",
            Language::Swift => "swift",
            Language::Kotlin => "kotlin",
            Language::Lua => "lua",
            Language::Shell => "shell",
            Language::Ruby => "ruby",
            Language::Php => "php",
            Language::Scala => "scala",
            Language::CSharp => "csharp",
        }
    }

    /// Get all file extensions for this language.
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Language::Cpp => &["c", "cc", "cpp", "cxx", "h", "hpp", "hxx"],
            Language::ObjectiveC => &["m", "mm"],
            Language::Java => &["java"],
            Language::Python => &["py", "pyw"],
            Language::Rust => &["rs"],
            Language::Go => &["go"],
            Language::JavaScript => &["js", "jsx", "mjs", "cjs"],
            Language::TypeScript => &["ts", "tsx", "mts", "cts"],
            Language::Dart => &["dart"],
            Language::Swift => &["swift"],
            Language::Kotlin => &["kt", "kts"],
            Language::Lua => &["lua"],
            Language::Shell => &["sh", "bash", "zsh", "ksh"],
            Language::Ruby => &["rb", "rake", "gemspec"],
            Language::Php => &["php", "phtml"],
            Language::Scala => &["scala", "sc"],
            Language::CSharp => &["cs", "csx"],
        }
    }
}

/// Run mode for linthis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    /// Run both lint and format (default)
    Both,
    /// Run only lint checks
    CheckOnly,
    /// Run only formatting
    FormatOnly,
}

/// Controls how missing tools are auto-installed during linting
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ToolInstallMode {
    /// Silently install without prompting (default)
    #[default]
    Auto,
    /// Prompt user before installing; falls back to Disabled in non-TTY
    Prompt,
    /// Never auto-install; show warning only
    Disabled,
}

/// Progress information for callbacks
#[derive(Debug, Clone)]
pub struct Progress {
    /// Current step name
    pub step: String,
    /// Current file being processed (if any)
    pub current_file: Option<String>,
    /// Current file index (1-based)
    pub current: usize,
    /// Total number of files
    pub total: usize,
}

/// Options for running linthis
#[derive(Clone)]
pub struct RunOptions {
    /// Paths to check (files or directories)
    pub paths: Vec<PathBuf>,
    /// Run mode
    pub mode: RunMode,
    /// Languages to check (empty = auto-detect)
    pub languages: Vec<Language>,
    /// Exclusion patterns
    pub exclude_patterns: Vec<String>,
    /// Verbose output
    pub verbose: bool,
    /// Quiet mode (no progress output)
    pub quiet: bool,
    /// Active plugins (name only, for display)
    pub plugins: Vec<String>,
    /// Disable cache (force re-check all files)
    pub no_cache: bool,
    /// Config resolver for plugin configs (priority-based lookup)
    pub config_resolver: Option<SharedConfigResolver>,
    /// How to handle missing tools during linting
    pub tool_install_mode: ToolInstallMode,
    /// Hook event context (if running inside a git hook)
    pub hook_event: Option<String>,
}

impl std::fmt::Debug for RunOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunOptions")
            .field("paths", &self.paths)
            .field("mode", &self.mode)
            .field("languages", &self.languages)
            .field("exclude_patterns", &self.exclude_patterns)
            .field("verbose", &self.verbose)
            .field("quiet", &self.quiet)
            .field("plugins", &self.plugins)
            .field("no_cache", &self.no_cache)
            .field(
                "config_resolver",
                &self
                    .config_resolver
                    .as_ref()
                    .map(|r| format!("{} configs", r.len())),
            )
            .finish()
    }
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            paths: vec![PathBuf::from(".")],
            mode: RunMode::Both,
            languages: Vec::new(),
            exclude_patterns: Vec::new(),
            verbose: false,
            quiet: false,
            plugins: Vec::new(),
            no_cache: false,
            config_resolver: None,
            tool_install_mode: ToolInstallMode::Prompt,
            hook_event: None,
        }
    }
}

/// Get the checker for a given language.
pub fn get_checker(lang: Language) -> Option<Box<dyn Checker>> {
    match lang {
        Language::Rust => Some(Box::new(RustChecker::new())),
        Language::Python => Some(Box::new(PythonChecker::new())),
        Language::TypeScript | Language::JavaScript => Some(Box::new(TypeScriptChecker::new())),
        Language::Go => Some(Box::new(GoChecker::new())),
        Language::Java => Some(Box::new(JavaChecker::new())),
        Language::Cpp | Language::ObjectiveC => Some(Box::new(CppChecker::new())),
        Language::Dart => Some(Box::new(DartChecker::new())),
        Language::Swift => Some(Box::new(SwiftChecker::new())),
        Language::Kotlin => Some(Box::new(KotlinChecker::new())),
        Language::Lua => Some(Box::new(LuaChecker::new())),
        Language::Shell => Some(Box::new(checkers::ShellChecker::new())),
        Language::Ruby => Some(Box::new(checkers::RubyChecker::new())),
        Language::Php => Some(Box::new(checkers::PhpChecker::new())),
        Language::Scala => Some(Box::new(checkers::ScalaChecker::new())),
        Language::CSharp => Some(Box::new(checkers::CSharpChecker::new())),
    }
}

/// Check if the formatter for a given language is available.
pub fn get_formatter_availability(lang: Language) -> bool {
    get_formatter(lang)
        .map(|f| f.is_available())
        .unwrap_or(false)
}

/// Get the formatter for a given language.
fn get_formatter(lang: Language) -> Option<Box<dyn Formatter>> {
    match lang {
        Language::Rust => Some(Box::new(RustFormatter::new())),
        Language::Python => Some(Box::new(PythonFormatter::new())),
        Language::TypeScript | Language::JavaScript => Some(Box::new(TypeScriptFormatter::new())),
        Language::Go => Some(Box::new(GoFormatter::new())),
        Language::Java => Some(Box::new(JavaFormatter::new())),
        Language::Cpp | Language::ObjectiveC => Some(Box::new(CppFormatter::new())),
        Language::Dart => Some(Box::new(DartFormatter::new())),
        Language::Swift => Some(Box::new(SwiftFormatter::new())),
        Language::Kotlin => Some(Box::new(KotlinFormatter::new())),
        Language::Lua => Some(Box::new(LuaFormatter::new())),
        Language::Shell => Some(Box::new(formatters::ShellFormatter::new())),
        Language::Ruby => Some(Box::new(formatters::RubyFormatter::new())),
        Language::Php => Some(Box::new(formatters::PhpFormatter::new())),
        Language::Scala => Some(Box::new(formatters::ScalaFormatter::new())),
        Language::CSharp => Some(Box::new(formatters::CSharpFormatter::new())),
    }
}

/// Check if a command is available on the system PATH.
pub fn is_command_available(cmd: &str) -> bool {
    std::process::Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Platform-aware install hint: returns (macos, windows, linux) variants.
fn platform_hint(macos: &str, windows: &str, linux: &str) -> String {
    if cfg!(target_os = "macos") {
        macos.to_string()
    } else if cfg!(target_os = "windows") {
        windows.to_string()
    } else {
        linux.to_string()
    }
}

/// Platform-aware install hint with only macos vs other.
fn platform_hint_macos_or(macos: &str, other: &str) -> String {
    if cfg!(target_os = "macos") {
        macos.to_string()
    } else {
        other.to_string()
    }
}

/// Generate a Python tool install hint, preferring uv > pipx > pip.
/// Falls back to platform-specific uv install instructions if none are available.
pub fn python_tool_install_hint(tool: &str) -> String {
    if is_command_available("uv") {
        format!("Install: uv tool install {}", tool)
    } else if is_command_available("pipx") {
        format!("Install: pipx install {}", tool)
    } else if is_command_available("pip") || is_command_available("pip3") {
        format!("Install: pip install {}", tool)
    } else {
        platform_hint(
            &format!(
                "Install: brew install uv && uv tool install {}",
                tool
            ),
            &format!(
                "Install: powershell -c \"irm https://astral.sh/uv/install.ps1 | iex\" && uv tool install {}",
                tool
            ),
            &format!(
                "Install: curl -LsSf https://astral.sh/uv/install.sh | sh && uv tool install {}",
                tool
            ),
        )
    }
}

/// Get installation instructions for a language's linter (platform-specific)
fn get_checker_install_hint(lang: Language) -> String {
    match lang {
        Language::Rust => "Install: rustup component add clippy".to_string(),
        Language::Python => python_tool_install_hint("ruff"),
        Language::Go => platform_hint(
            "Install: brew install golangci-lint\n         Or: go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest",
            "Install: go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest\n         Or: choco install golangci-lint",
            "Install: go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest\n         Or: sudo apt install golangci-lint (Ubuntu/Debian)",
        ),
        Language::TypeScript | Language::JavaScript => "Install: npm install -g eslint".to_string(),
        Language::Java => platform_hint(
            "Install: brew install checkstyle",
            "Install: choco install checkstyle\n         Or download from: https://checkstyle.org/",
            "Install: sudo apt install checkstyle (Ubuntu/Debian)\n         Or download from: https://checkstyle.org/",
        ),
        Language::Cpp | Language::ObjectiveC => platform_hint(
            "Install: brew install llvm  (clang-tidy)\n         Or: brew install cpplint\n         Or: uv tool install cpplint  /  pipx install cpplint",
            "Install: choco install llvm  (clang-tidy)\n         Or: uv tool install cpplint  /  pipx install cpplint",
            "Install: sudo apt install clang-tidy  (Ubuntu/Debian)\n         Or: sudo dnf install clang-tools-extra  (Fedora)\n         Or: uv tool install cpplint  /  pipx install cpplint",
        ),
        Language::Dart => "Install: Dart SDK (includes dart analyze)\n         https://dart.dev/get-dart".to_string(),
        Language::Swift => platform_hint_macos_or(
            "Install: brew install swiftlint",
            "Install: https://github.com/realm/SwiftLint",
        ),
        Language::Kotlin => platform_hint_macos_or(
            "Install: brew install ktlint",
            "Install: https://github.com/pinterest/ktlint",
        ),
        Language::Lua => get_checker_install_hint_lua(),
        Language::Shell => platform_hint(
            "Install: brew install shellcheck",
            "Install: choco install shellcheck\n         Or: scoop install shellcheck",
            "Install: sudo apt install shellcheck (Ubuntu/Debian)",
        ),
        Language::Ruby => get_checker_install_hint_ruby(),
        Language::Php => get_checker_install_hint_php(),
        Language::Scala => platform_hint_macos_or(
            "Install: brew install scalafix\n         Or: cs install scalafix",
            "Install: cs install scalafix\n         https://scalacenter.github.io/scalafix/",
        ),
        Language::CSharp => "Install: dotnet tool install -g dotnet-format".to_string(),
    }
}

/// Lua checker install hint.
///
/// On macOS we lead with `brew install luacheck` because the LuaRocks route
/// routinely fails on modern Lua versions: when Homebrew's default Lua is
/// 5.5+, `luarocks install luacheck` bails on its `argparse` /
/// `luafilesystem` dependencies whose rockspecs haven't been marked
/// Lua-5.5 compatible (see https://github.com/lunarmodules/luacheck/issues).
/// Homebrew ships a pre-built luacheck that sidesteps the whole problem.
///
/// On other platforms we keep the luarocks path as the primary install,
/// since the Homebrew alternative doesn't apply.
fn get_checker_install_hint_lua() -> String {
    if cfg!(target_os = "macos") {
        if is_command_available("brew") {
            return "Install: brew install luacheck  \
                    (preferred on macOS; luarocks + Lua 5.5 often fails on argparse)"
                .to_string();
        }
        // No brew yet — tell them to get brew first.
        return "Install: 1) Install Homebrew from https://brew.sh/\n         \
                2) brew install luacheck"
            .to_string();
    }

    if is_command_available("luarocks") {
        "Install: luarocks install luacheck".to_string()
    } else {
        platform_hint(
            "Install: brew install luacheck",
            "Install: 1) Install Lua from https://www.lua.org/download.html\n         2) Install LuaRocks from https://luarocks.org/\n         3) luarocks install luacheck",
            "Install: sudo apt install luarocks && luarocks install luacheck (Ubuntu/Debian)",
        )
    }
}

/// Ruby checker install hint with gem detection.
fn get_checker_install_hint_ruby() -> String {
    if is_command_available("gem") {
        "Install: gem install rubocop".to_string()
    } else {
        "Install: 1) Install Ruby from https://www.ruby-lang.org/\n         2) gem install rubocop"
            .to_string()
    }
}

/// PHP checker install hint with composer detection.
fn get_checker_install_hint_php() -> String {
    if is_command_available("composer") {
        "Install: composer global require squizlabs/php_codesniffer".to_string()
    } else {
        "Install: 1) Install Composer from https://getcomposer.org/\n         2) composer global require squizlabs/php_codesniffer".to_string()
    }
}

/// Get installation instructions for a language's formatter (platform-specific)
fn get_formatter_install_hint(lang: Language) -> String {
    match lang {
        Language::Rust => "Install: rustup component add rustfmt".to_string(),
        Language::Python => python_tool_install_hint("ruff"),
        Language::Go => "Install: Go formatter (gofmt) is included with Go".to_string(),
        Language::TypeScript | Language::JavaScript => "Install: npm install -g prettier".to_string(),
        Language::Java => platform_hint(
            "Install: brew install google-java-format",
            "Install: Download from https://github.com/google/google-java-format/releases",
            "Install: Download from https://github.com/google/google-java-format/releases\n         Or use your package manager",
        ),
        Language::Cpp | Language::ObjectiveC => platform_hint(
            "Install: brew install clang-format\n         Or: brew install llvm",
            "Install: choco install llvm (includes clang-format)",
            "Install: sudo apt install clang-format (Ubuntu/Debian)",
        ),
        Language::Dart => "Install: Dart SDK (includes dart format)\n         https://dart.dev/get-dart".to_string(),
        Language::Swift => platform_hint_macos_or(
            "Install: brew install swift-format",
            "Install: https://github.com/apple/swift-format",
        ),
        Language::Kotlin => platform_hint_macos_or(
            "Install: brew install ktlint",
            "Install: https://github.com/pinterest/ktlint",
        ),
        Language::Lua => platform_hint(
            "Install: brew install stylua\n         Or: cargo install stylua",
            "Install: scoop install stylua\n         Or: choco install stylua\n         Or: cargo install stylua",
            "Install: cargo install stylua",
        ),
        Language::Shell => platform_hint(
            "Install: brew install shfmt",
            "Install: choco install shfmt\n         Or: scoop install shfmt",
            "Install: sudo apt install shfmt (Ubuntu/Debian)\n         Or: go install mvdan.cc/sh/v3/cmd/shfmt@latest",
        ),
        Language::Ruby => "Install: gem install rubocop".to_string(),
        Language::Php => "Install: composer global require friendsofphp/php-cs-fixer".to_string(),
        Language::Scala => platform_hint_macos_or(
            "Install: brew install scalafmt\n         Or: cs install scalafmt",
            "Install: cs install scalafmt\n         https://scalameta.org/scalafmt/",
        ),
        Language::CSharp => "Install: dotnet tool install -g dotnet-format".to_string(),
    }
}

/// Get auto-install commands for a missing tool.
/// Returns a list of candidate commands to try in order (first that succeeds wins).
/// Each command is split into [program, arg1, arg2, ...].
fn get_auto_install_commands(lang: Language, is_checker: bool) -> Vec<Vec<String>> {
    if is_checker {
        get_checker_install_commands(lang)
    } else {
        get_formatter_install_commands(lang)
    }
}

/// Get auto-install commands for a checker tool.
fn get_checker_install_commands(lang: Language) -> Vec<Vec<String>> {
    macro_rules! cmd {
        ($($s:expr),+) => { vec![$($s.to_string()),+] }
    }

    match lang {
        Language::Rust => vec![cmd!["rustup", "component", "add", "clippy"]],
        Language::Python => get_auto_install_ruff(),
        Language::Go => get_auto_install_go_checker(),
        Language::TypeScript | Language::JavaScript => vec![cmd!["npm", "install", "-g", "eslint"]],
        Language::Java => platform_install_cmd("checkstyle", "checkstyle", "checkstyle"),
        Language::Cpp | Language::ObjectiveC => get_auto_install_cpp_checker(),
        Language::Dart => vec![],
        Language::Swift => macos_only_brew("swiftlint"),
        Language::Kotlin => get_auto_install_kotlin(),
        Language::Lua => get_auto_install_luacheck(),
        Language::Shell => platform_install_cmd("shellcheck", "shellcheck", "shellcheck"),
        Language::Ruby => vec![cmd!["gem", "install", "rubocop"]],
        Language::Php => vec![cmd![
            "composer",
            "global",
            "require",
            "squizlabs/php_codesniffer"
        ]],
        Language::Scala => {
            if cfg!(target_os = "macos") {
                vec![cmd!["brew", "install", "scalafix"]]
            } else {
                vec![cmd!["cs", "install", "scalafix"]]
            }
        }
        Language::CSharp => vec![cmd!["dotnet", "tool", "install", "-g", "dotnet-format"]],
    }
}

/// Get auto-install commands for a formatter tool.
fn get_formatter_install_commands(lang: Language) -> Vec<Vec<String>> {
    macro_rules! cmd {
        ($($s:expr),+) => { vec![$($s.to_string()),+] }
    }

    match lang {
        Language::Rust => vec![cmd!["rustup", "component", "add", "rustfmt"]],
        Language::Python => get_auto_install_ruff(),
        Language::Go => vec![],
        Language::TypeScript | Language::JavaScript => {
            vec![cmd!["npm", "install", "-g", "prettier"]]
        }
        Language::Java => get_auto_install_java_formatter(),
        Language::Cpp | Language::ObjectiveC => {
            platform_install_cmd("clang-format", "llvm", "clang-format")
        }
        Language::Dart => vec![],
        Language::Swift => macos_only_brew("swift-format"),
        Language::Kotlin => get_auto_install_kotlin(),
        Language::Lua => get_auto_install_stylua(),
        Language::Shell => get_auto_install_shfmt(),
        Language::Ruby => vec![cmd!["gem", "install", "rubocop"]],
        Language::Php => vec![cmd![
            "composer",
            "global",
            "require",
            "friendsofphp/php-cs-fixer"
        ]],
        Language::Scala => {
            if cfg!(target_os = "macos") {
                vec![cmd!["brew", "install", "scalafmt"]]
            } else {
                vec![cmd!["cs", "install", "scalafmt"]]
            }
        }
        Language::CSharp => vec![cmd!["dotnet", "tool", "install", "-g", "dotnet-format"]],
    }
}

/// Install a Python CLI tool in an isolated venv, falling back to system pip.
/// Order: uv tool install > pipx install > uv pip --system > pip3 > pip
fn python_tool_install_cmds(package: &str) -> Vec<Vec<String>> {
    let p = package.to_string();
    vec![
        vec!["uv".into(), "tool".into(), "install".into(), p.clone()],
        vec!["pipx".into(), "install".into(), p.clone()],
        vec![
            "uv".into(),
            "pip".into(),
            "install".into(),
            "--system".into(),
            p.clone(),
        ],
        vec!["pip3".into(), "install".into(), p.clone()],
        vec!["pip".into(), "install".into(), p],
    ]
}

/// Platform-aware package manager install command.
fn platform_install_cmd(brew_pkg: &str, choco_pkg: &str, apt_pkg: &str) -> Vec<Vec<String>> {
    if cfg!(target_os = "macos") {
        vec![vec!["brew".into(), "install".into(), brew_pkg.into()]]
    } else if cfg!(target_os = "windows") {
        vec![vec!["choco".into(), "install".into(), choco_pkg.into()]]
    } else {
        vec![vec![
            "sudo".into(),
            "apt-get".into(),
            "install".into(),
            "-y".into(),
            apt_pkg.into(),
        ]]
    }
}

/// Go checker auto-install commands.
fn get_auto_install_go_checker() -> Vec<Vec<String>> {
    if cfg!(target_os = "macos") {
        vec![vec![
            "brew".into(),
            "install".into(),
            "golangci-lint".into(),
        ]]
    } else {
        vec![vec![
            "go".into(),
            "install".into(),
            "github.com/golangci/golangci-lint/cmd/golangci-lint@latest".into(),
        ]]
    }
}

/// Java formatter auto-install commands.
fn get_auto_install_java_formatter() -> Vec<Vec<String>> {
    if cfg!(target_os = "macos") {
        vec![vec![
            "brew".into(),
            "install".into(),
            "google-java-format".into(),
        ]]
    } else {
        vec![]
    }
}

/// luacheck auto-install commands.
///
/// macOS: `brew install luacheck` is reliable; LuaRocks' `argparse`
/// dependency chain breaks under Homebrew's Lua 5.5. Other platforms
/// fall back to luarocks.
fn get_auto_install_luacheck() -> Vec<Vec<String>> {
    if cfg!(target_os = "macos") {
        vec![vec!["brew".into(), "install".into(), "luacheck".into()]]
    } else {
        vec![vec!["luarocks".into(), "install".into(), "luacheck".into()]]
    }
}

/// Kotlin auto-install commands.
fn get_auto_install_kotlin() -> Vec<Vec<String>> {
    if cfg!(target_os = "macos") {
        vec![vec!["brew".into(), "install".into(), "ktlint".into()]]
    } else if cfg!(target_os = "windows") {
        vec![vec!["choco".into(), "install".into(), "ktlint".into()]]
    } else {
        vec![]
    }
}

/// shfmt auto-install commands.
fn get_auto_install_shfmt() -> Vec<Vec<String>> {
    if cfg!(target_os = "macos") {
        vec![vec!["brew".into(), "install".into(), "shfmt".into()]]
    } else if cfg!(target_os = "windows") {
        vec![vec!["choco".into(), "install".into(), "shfmt".into()]]
    } else {
        vec![
            vec![
                "sudo".into(),
                "apt-get".into(),
                "install".into(),
                "-y".into(),
                "shfmt".into(),
            ],
            vec![
                "go".into(),
                "install".into(),
                "mvdan.cc/sh/v3/cmd/shfmt@latest".into(),
            ],
        ]
    }
}

/// ruff auto-install: brew (macOS) > uv tool > pipx > pip fallbacks
fn get_auto_install_ruff() -> Vec<Vec<String>> {
    macro_rules! cmd {
        ($($s:expr),+) => { vec![$($s.to_string()),+] }
    }
    if cfg!(target_os = "macos") {
        let mut cmds = vec![cmd!["brew", "install", "ruff"]];
        cmds.extend(python_tool_install_cmds("ruff"));
        cmds
    } else {
        python_tool_install_cmds("ruff")
    }
}

/// C++ checker auto-install.
///
/// Tries clang-tidy (bundled in llvm/clang) first as the preferred linter,
/// then cpplint as a lightweight fallback.
/// cpplint is a Python tool — prefer isolated installs (uv tool / pipx) over
/// system pip to avoid environment conflicts.
fn get_auto_install_cpp_checker() -> Vec<Vec<String>> {
    macro_rules! cmd {
        ($($s:expr),+) => { vec![$($s.to_string()),+] }
    }
    if cfg!(target_os = "macos") {
        let mut cmds = vec![
            cmd!["brew", "install", "llvm"],    // brings clang-tidy
            cmd!["brew", "install", "cpplint"], // Homebrew has a cpplint formula
        ];
        cmds.extend(python_tool_install_cmds("cpplint"));
        cmds
    } else if cfg!(target_os = "windows") {
        let mut cmds = vec![cmd!["choco", "install", "llvm"]];
        cmds.extend(python_tool_install_cmds("cpplint"));
        cmds
    } else {
        // Linux: apt clang-tidy is the most reliable; cpplint via isolated Python as fallback
        let mut cmds = vec![
            cmd!["sudo", "apt-get", "install", "-y", "clang-tidy"],
            cmd!["sudo", "dnf", "install", "-y", "clang-tools-extra"],
            cmd!["sudo", "pacman", "-S", "--noconfirm", "clang"],
        ];
        cmds.extend(python_tool_install_cmds("cpplint"));
        cmds
    }
}

/// stylua auto-install: brew (macOS) > cargo; scoop/choco/cargo (Windows); cargo (Linux).
///
/// `brew install stylua` and package-manager installs use pre-built binaries,
/// avoiding the 2–5 min compile time of `cargo install stylua`.
fn get_auto_install_stylua() -> Vec<Vec<String>> {
    macro_rules! cmd {
        ($($s:expr),+) => { vec![$($s.to_string()),+] }
    }
    if cfg!(target_os = "macos") {
        vec![
            cmd!["brew", "install", "stylua"],
            cmd!["cargo", "install", "stylua"],
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            cmd!["scoop", "install", "stylua"],
            cmd!["choco", "install", "stylua"],
            cmd!["cargo", "install", "stylua"],
        ]
    } else {
        // Linux: no standard package manager formula; cargo is the universal fallback
        vec![cmd!["cargo", "install", "stylua"]]
    }
}

/// Return brew install command on macOS, empty otherwise.
fn macos_only_brew(package: &str) -> Vec<Vec<String>> {
    if cfg!(target_os = "macos") {
        vec![vec!["brew".into(), "install".into(), package.into()]]
    } else {
        vec![]
    }
}

/// Try to install a tool by running the given command.
/// Returns Ok(()) on success, Err(message) on failure.
fn try_install_tool(command: &[String]) -> std::result::Result<(), String> {
    if command.is_empty() {
        return Err("empty install command".to_string());
    }
    let output = std::process::Command::new(&command[0])
        .args(&command[1..])
        .output()
        .map_err(|e| format!("failed to run `{}`: {}", command.join(" "), e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "`{}` failed (exit {}): {}",
            command.join(" "),
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ))
    }
}

type InstalledTool = (Language, bool);
type FailedTool = (Language, bool, String);

/// Check if a tool is already available on the system.
fn is_tool_available(lang: Language, is_checker: bool) -> bool {
    if is_checker {
        get_checker(lang).map(|c| c.is_available()).unwrap_or(true)
    } else {
        get_formatter(lang)
            .map(|f| f.is_available())
            .unwrap_or(true)
    }
}

/// Prompt the user to install a missing tool. Returns true if user approves.
fn prompt_tool_install(lang: Language, tool_name: &str) -> bool {
    use std::io::Write;
    eprint!("\r\x1b[K");
    eprint!(
        "\x1b[33m?\x1b[0m Missing {} tool \x1b[1m{}\x1b[0m — install now? [Y/n] ",
        lang.name(),
        tool_name
    );
    let _ = std::io::stderr().flush();
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    let trimmed = input.trim().to_ascii_lowercase();
    trimmed.is_empty() || trimmed == "y" || trimmed == "yes"
}

/// Attempt to install a tool using candidate commands. Returns Ok on success, Err with last error.
fn attempt_install(commands: &[Vec<String>]) -> std::result::Result<(), String> {
    let mut last_err = String::new();
    for cmd in commands {
        match try_install_tool(cmd) {
            Ok(()) => return Ok(()),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

/// Check if install mode should skip installation for this context.
fn should_skip_install(install_mode: &ToolInstallMode) -> bool {
    matches!(install_mode, ToolInstallMode::Disabled)
        || (matches!(install_mode, ToolInstallMode::Prompt)
            && !std::io::IsTerminal::is_terminal(&std::io::stdin()))
}

/// Pre-flight: auto-install missing tools for detected languages.
/// Returns (installed_tools, failed_tools).
fn pre_flight_install(
    file_langs: &[(&std::path::PathBuf, Language)],
    run_mode: &RunMode,
    install_mode: &ToolInstallMode,
    quiet: bool,
) -> (Vec<InstalledTool>, Vec<FailedTool>) {
    use std::collections::HashSet;

    if should_skip_install(install_mode) {
        return (Vec::new(), Vec::new());
    }

    let mut seen: HashSet<(Language, bool)> = HashSet::new();
    let mut installed: Vec<InstalledTool> = Vec::new();
    let mut failed: Vec<FailedTool> = Vec::new();

    let check_checker = matches!(run_mode, RunMode::Both | RunMode::CheckOnly);
    let check_formatter = matches!(run_mode, RunMode::Both | RunMode::FormatOnly);

    for (_, lang) in file_langs {
        let lang = *lang;
        for is_checker in [true, false] {
            if (is_checker && !check_checker) || (!is_checker && !check_formatter) {
                continue;
            }
            if !seen.insert((lang, is_checker)) || is_tool_available(lang, is_checker) {
                continue;
            }

            let tool_name = get_tool_name(lang, is_checker);
            let commands = get_auto_install_commands(lang, is_checker);
            if commands.is_empty() {
                continue;
            }

            if matches!(install_mode, ToolInstallMode::Prompt)
                && !prompt_tool_install(lang, &tool_name)
            {
                continue;
            }

            if !quiet {
                eprint!("\r\x1b[K");
                eprintln!(
                    "\x1b[36mInstalling\x1b[0m: {} (missing {} tool for {})",
                    tool_name,
                    if is_checker { "linter" } else { "formatter" },
                    lang.name()
                );
            }

            match attempt_install(&commands) {
                Ok(()) => {
                    if !quiet {
                        eprintln!("\x1b[32mInstalled\x1b[0m:  {}", tool_name);
                    }
                    installed.push((lang, is_checker));
                }
                Err(last_err) => {
                    failed.push((lang, is_checker, last_err));
                }
            }
        }
    }

    (installed, failed)
}

/// Warn about missing tool (once per tool) and record for reporting
fn warn_missing_tool(tool_type: &str, lang: Language, is_checker: bool) {
    let tool_key = format!("{}-{}", tool_type, lang.name());
    if should_warn_tool(&tool_key) {
        let hint = if is_checker {
            get_checker_install_hint(lang)
        } else {
            get_formatter_install_hint(lang)
        };

        // Record the unavailable tool, checking if auto-install was attempted
        let tool_name = get_tool_name(lang, is_checker);
        let auto_failed = {
            let set = AUTO_INSTALL_FAILED.lock().unwrap();
            set.as_ref()
                .map(|s| s.contains(&format!("{}-{}", tool_name, lang.name())))
                .unwrap_or(false)
        };
        let mut tool_info =
            utils::types::UnavailableTool::new(&tool_name, lang.name(), tool_type, &hint);
        if auto_failed {
            tool_info = tool_info.with_auto_install_failed();
        }
        record_unavailable_tool(tool_info);

        eprintln!();
        eprintln!(
            "\x1b[33mWarning\x1b[0m: No {} {} available for {} files",
            lang.name(),
            tool_type,
            lang.name()
        );
        eprintln!("  {}", hint);
        eprintln!();
    }
}

/// Get the name of the checker tool for a language.
fn get_checker_tool_name(lang: Language) -> &'static str {
    match lang {
        Language::Rust => "clippy",
        Language::Python => "ruff",
        Language::Go => "golangci-lint",
        Language::TypeScript | Language::JavaScript => "eslint",
        Language::Java => "checkstyle",
        Language::Cpp | Language::ObjectiveC => "cpplint",
        Language::Dart => "dart-analyze",
        Language::Swift => "swiftlint",
        Language::Kotlin => "ktlint",
        Language::Lua => "luacheck",
        Language::Shell => "shellcheck",
        Language::Ruby => "rubocop",
        Language::Php => "phpcs",
        Language::Scala => "scalafix",
        Language::CSharp => "dotnet-format",
    }
}

/// Get the name of the formatter tool for a language.
fn get_formatter_tool_name(lang: Language) -> &'static str {
    match lang {
        Language::Rust => "rustfmt",
        Language::Python => "ruff",
        Language::Go => "gofmt",
        Language::TypeScript | Language::JavaScript => "prettier",
        Language::Java => "google-java-format",
        Language::Cpp | Language::ObjectiveC => "clang-format",
        Language::Dart => "dart-format",
        Language::Swift => "swift-format",
        Language::Kotlin => "ktlint",
        Language::Lua => "stylua",
        Language::Shell => "shfmt",
        Language::Ruby => "rubocop",
        Language::Php => "php-cs-fixer",
        Language::Scala => "scalafmt",
        Language::CSharp => "dotnet-format",
    }
}

/// Get the name of the tool for a language
fn get_tool_name(lang: Language, is_checker: bool) -> String {
    if is_checker {
        get_checker_tool_name(lang).to_string()
    } else {
        get_formatter_tool_name(lang).to_string()
    }
}

/// Record an unavailable tool for later reporting
fn record_unavailable_tool(tool: utils::types::UnavailableTool) {
    let mut tools = UNAVAILABLE_TOOLS.lock().unwrap();
    if tools.is_none() {
        *tools = Some(Vec::new());
    }
    if let Some(ref mut list) = *tools {
        // Only add if not already present (by tool name + language)
        let exists = list
            .iter()
            .any(|t| t.tool == tool.tool && t.language == tool.language);
        if !exists {
            list.push(tool);
        }
    }
}

/// Collect and clear all recorded unavailable tools
fn collect_unavailable_tools() -> Vec<utils::types::UnavailableTool> {
    let mut tools = UNAVAILABLE_TOOLS.lock().unwrap();
    tools.take().unwrap_or_default()
}

/// Check if we've already warned about a tool
fn should_warn_tool(tool_name: &str) -> bool {
    let mut warned = WARNED_TOOLS.lock().unwrap();
    if warned.is_none() {
        *warned = Some(HashSet::new());
    }
    let set = warned.as_mut().unwrap();
    if set.contains(tool_name) {
        false
    } else {
        set.insert(tool_name.to_string());
        true
    }
}

/// Run checker on a file and return issues.
fn run_checker_on_file(
    file: &Path,
    lang: Language,
    verbose: bool,
    config_resolver: Option<&ConfigResolver>,
) -> Vec<utils::types::LintIssue> {
    let mut issues = Vec::new();
    if let Some(checker) = get_checker(lang) {
        if checker.is_available() {
            // Get config from resolver if available
            let config_path =
                config_resolver.and_then(|r| r.get_plugin_config(lang.name(), checker.name()));

            let result = if let Some(ref cfg) = config_path {
                checker.check_with_config(file, Some(cfg.as_path()))
            } else {
                checker.check(file)
            };

            match result {
                Ok(file_issues) => {
                    // Set language for each issue
                    for mut issue in file_issues {
                        issue.language = Some(lang);
                        issues.push(issue);
                    }
                }
                Err(e) => {
                    if verbose {
                        eprintln!("Check error for {}: {}", file.display(), e);
                    }
                }
            }
        } else {
            // Show warning once per tool (not per file)
            warn_missing_tool("linter", lang, true);
        }
    }
    issues
}

/// Spinner characters for progress indication
const SPINNER_CHARS: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Format elapsed time as human-readable string
fn format_elapsed(start: Instant) -> String {
    let secs = start.elapsed().as_secs();
    if secs >= 60 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}s", secs)
    }
}

/// Get spinner character based on counter
fn get_spinner(count: usize) -> char {
    SPINNER_CHARS[count % SPINNER_CHARS.len()]
}

/// Print progress message with spinner and elapsed time (respects quiet mode)
fn print_progress_with_time(msg: &str, quiet: bool, start: Instant, count: usize) {
    if !quiet {
        use std::io::Write;
        let spinner = get_spinner(count);
        let time_suffix = format!(" ({})", format_elapsed(start));
        // Clear line and print with spinner
        eprint!("\r\x1b[K\x1b[36m{}\x1b[0m {}{}", spinner, msg, time_suffix);
        let _ = std::io::stderr().flush();
    }
}

/// Simple print progress for clearing lines
fn print_progress(msg: &str, quiet: bool) {
    if !quiet {
        use std::io::Write;
        eprint!("\r\x1b[K{}", msg);
        let _ = std::io::stderr().flush();
    }
}

/// Normalize a path to absolute for reliable comparison.
fn normalize_path(p: &Path) -> PathBuf {
    if let Ok(canonical) = p.canonicalize() {
        return canonical;
    }
    if p.is_absolute() {
        return p.to_path_buf();
    }
    if let Ok(current_dir) = std::env::current_dir() {
        let joined = current_dir.join(p);
        joined.canonicalize().unwrap_or(joined)
    } else {
        let s = p.to_string_lossy();
        let s = s.strip_prefix("./").unwrap_or(&s);
        PathBuf::from(s)
    }
}

/// Print the "Found N files" scanning status line.
fn print_found_files_status(file_count: usize) {
    use std::io::Write;
    eprint!(
        "\r\x1b[K\x1b[36m{}\x1b[0m Found {} files, checking...",
        SPINNER_CHARS[1], file_count
    );
    let _ = std::io::stderr().flush();
}

/// Handle pre-flight install results: print warnings for failures and record them.
fn handle_install_failures(failed: &[FailedTool], file_count: usize, quiet: bool) {
    if failed.is_empty() || quiet {
        return;
    }
    eprint!("\r\x1b[K");
    for (lang, is_checker, err) in failed {
        let tool_name = get_tool_name(*lang, *is_checker);
        let hint = if *is_checker {
            get_checker_install_hint(*lang)
        } else {
            get_formatter_install_hint(*lang)
        };
        let tool_key = format!("{}-{}", tool_name, lang.name());
        {
            let mut set = AUTO_INSTALL_FAILED.lock().unwrap();
            set.get_or_insert_with(HashSet::new).insert(tool_key);
        }
        eprintln!(
            "\x1b[33mWarning\x1b[0m: Auto-install failed for {} — {}\n  Manual install: {}",
            tool_name, err, hint
        );
    }
    print_found_files_status(file_count);
}

/// Load the lint cache if caching is enabled.
fn load_cache(
    no_cache: bool,
    mode: &RunMode,
    verbose: bool,
    cache_max_age_days: Option<u32>,
) -> Option<Mutex<LintCache>> {
    if no_cache || *mode == RunMode::FormatOnly {
        return None;
    }
    let project_root = utils::get_project_root();
    match LintCache::load(&project_root) {
        Ok(mut c) => {
            c.prune(cache_max_age_days);
            c.reset_stats();
            Some(Mutex::new(c))
        }
        Err(e) => {
            if verbose {
                eprintln!("Cache load failed: {}, starting fresh", e);
            }
            Some(Mutex::new(LintCache::new()))
        }
    }
}

/// Check a single file with cache support. Returns issues found.
fn check_file_with_cache(
    file: &Path,
    lang: Language,
    verbose: bool,
    config_resolver: Option<&ConfigResolver>,
    cache: &Option<Mutex<LintCache>>,
    project_root: &Path,
) -> Vec<utils::types::LintIssue> {
    if let Some(ref cache_mutex) = cache {
        let mut cache_guard = cache_mutex.lock().unwrap();
        if let Some(cached_issues) = cache_guard.check_file(lang.name(), file, project_root) {
            return cached_issues;
        }
    }

    let issues = run_checker_on_file(file, lang, verbose, config_resolver);

    if let Some(ref cache_mutex) = cache {
        let mut cache_guard = cache_mutex.lock().unwrap();
        let _ = cache_guard.update_file(lang.name(), file, project_root, &issues);
    }

    issues
}

/// Format a single file. Returns the format result if available.
fn format_single_file(file: &Path, lang: Language, verbose: bool) -> Option<FormatResult> {
    let formatter = get_formatter(lang)?;
    if !formatter.is_available() {
        warn_missing_tool("formatter", lang, false);
        return None;
    }
    match formatter.format(file) {
        Ok(fr) => Some(fr),
        Err(e) => {
            if verbose {
                eprintln!("Format error for {}: {}", file.display(), e);
            }
            None
        }
    }
}

/// Run the "Both" mode: lint -> format -> re-lint.
fn run_both_mode(
    file_langs: &[(&PathBuf, Language)],
    options: &RunOptions,
    cache: &Option<Mutex<LintCache>>,
    project_root: &Path,
    result: &mut RunResult,
) {
    let total_files = file_langs.len();

    // Step 1: First lint pass
    if options.verbose {
        eprintln!("Step 1: Checking for issues...");
    }
    PROGRESS_COUNTER.store(0, Ordering::Relaxed);
    let step1_start = Instant::now();

    let check_results: Vec<(PathBuf, Vec<_>)> = file_langs
        .par_iter()
        .map(|(file, lang)| {
            let count = PROGRESS_COUNTER.fetch_add(1, Ordering::Relaxed);
            if !options.quiet && !options.verbose {
                let percentage = ((count + 1) as f64 / total_files as f64 * 100.0) as usize;
                print_progress_with_time(
                    &format!(
                        "[1/3] Checking {}/{} ({}%)...",
                        count + 1,
                        total_files,
                        percentage
                    ),
                    false,
                    step1_start,
                    count,
                );
            }
            let issues = check_file_with_cache(
                file,
                *lang,
                options.verbose,
                options.config_resolver.as_deref(),
                cache,
                project_root,
            );
            ((*file).clone(), issues)
        })
        .collect();

    let mut issues_before = Vec::new();
    let mut files_with_issues: HashSet<PathBuf> = HashSet::new();
    for (file, file_issues) in check_results {
        if !file_issues.is_empty() {
            files_with_issues.insert(file);
        }
        issues_before.extend(file_issues);
    }
    result.issues_before_format = issues_before.len();

    // Step 2: Format files with issues
    run_both_format_step(file_langs, &files_with_issues, options, result);

    // Step 3: Re-check formatted files
    let formatted_files = result
        .format_results
        .iter()
        .filter(|fr| fr.changed)
        .map(|fr| fr.file_path.clone())
        .collect::<HashSet<PathBuf>>();

    run_both_recheck_step(file_langs, &formatted_files, options, result);

    // Keep original issues for files that weren't formatted
    for (file, _) in file_langs {
        if files_with_issues.contains(*file) && !formatted_files.contains(*file) {
            let normalized_file = normalize_path(file);
            for issue in &issues_before {
                let normalized_issue_path = normalize_path(&issue.file_path);
                if normalized_issue_path == normalized_file {
                    result.add_issue(issue.clone());
                }
            }
        }
    }

    print_progress("", options.quiet || options.verbose);

    if !formatted_files.is_empty() && result.issues_before_format > result.issues.len() {
        result.issues_fixed = result.issues_before_format - result.issues.len();
    }
}

/// Step 2 of Both mode: format files with issues.
fn run_both_format_step(
    file_langs: &[(&PathBuf, Language)],
    files_with_issues: &HashSet<PathBuf>,
    options: &RunOptions,
    result: &mut RunResult,
) {
    if options.verbose {
        eprintln!(
            "Step 2: Formatting {} files with issues...",
            files_with_issues.len()
        );
    }
    let files_to_format: Vec<_> = file_langs
        .iter()
        .filter(|(f, _)| files_with_issues.contains(*f))
        .collect();
    let format_total = files_to_format.len();
    PROGRESS_COUNTER.store(0, Ordering::Relaxed);
    let step2_start = Instant::now();

    let format_results: Vec<(PathBuf, Option<FormatResult>)> = files_to_format
        .par_iter()
        .map(|(file, lang)| {
            let count = PROGRESS_COUNTER.fetch_add(1, Ordering::Relaxed);
            if !options.quiet && !options.verbose {
                let percentage = ((count + 1) as f64 / format_total as f64 * 100.0) as usize;
                print_progress_with_time(
                    &format!(
                        "[2/3] Formatting {}/{} ({}%)...",
                        count + 1,
                        format_total,
                        percentage
                    ),
                    false,
                    step2_start,
                    count,
                );
            }
            let fr = format_single_file(file, *lang, options.verbose);
            ((*file).clone(), fr)
        })
        .collect();

    for (_file, format_result) in format_results {
        if let Some(fr) = format_result {
            result.add_format_result(fr);
        }
    }
}

/// Step 3 of Both mode: re-check formatted files.
fn run_both_recheck_step(
    file_langs: &[(&PathBuf, Language)],
    formatted_files: &HashSet<PathBuf>,
    options: &RunOptions,
    result: &mut RunResult,
) {
    if options.verbose {
        eprintln!(
            "Step 3: Rechecking {} formatted files...",
            formatted_files.len()
        );
    }
    let recheck_total = formatted_files.len();
    PROGRESS_COUNTER.store(0, Ordering::Relaxed);
    let step3_start = Instant::now();

    let recheck_issues: Vec<_> = file_langs
        .par_iter()
        .flat_map(|(file, lang)| {
            if !formatted_files.contains(*file) {
                return vec![];
            }
            let count = PROGRESS_COUNTER.fetch_add(1, Ordering::Relaxed);
            if !options.quiet && !options.verbose {
                let percentage = ((count + 1) as f64 / recheck_total as f64 * 100.0) as usize;
                print_progress_with_time(
                    &format!(
                        "[3/3] Rechecking {}/{} ({}%)...",
                        count + 1,
                        recheck_total,
                        percentage
                    ),
                    false,
                    step3_start,
                    count,
                );
            }
            run_checker_on_file(
                file,
                *lang,
                options.verbose,
                options.config_resolver.as_deref(),
            )
        })
        .collect();

    for issue in recheck_issues {
        result.add_issue(issue);
    }
}

/// Run CheckOnly mode with parallel processing and cache support.
fn run_check_only_mode(
    file_langs: &[(&PathBuf, Language)],
    options: &RunOptions,
    cache: &Option<Mutex<LintCache>>,
    project_root: &Path,
    result: &mut RunResult,
) {
    let total_files = file_langs.len();
    PROGRESS_COUNTER.store(0, Ordering::Relaxed);
    let check_start = Instant::now();

    let all_issues: Vec<_> = file_langs
        .par_iter()
        .flat_map(|(file, lang)| {
            let count = PROGRESS_COUNTER.fetch_add(1, Ordering::Relaxed);
            if !options.quiet && !options.verbose {
                let percentage = ((count + 1) as f64 / total_files as f64 * 100.0) as usize;
                print_progress_with_time(
                    &format!(
                        "Checking {}/{} ({}%)...",
                        count + 1,
                        total_files,
                        percentage
                    ),
                    false,
                    check_start,
                    count,
                );
            }
            if options.verbose {
                eprintln!("Processing: {} ({})", file.display(), lang.name());
            }
            check_file_with_cache(
                file,
                *lang,
                options.verbose,
                options.config_resolver.as_deref(),
                cache,
                project_root,
            )
        })
        .collect();

    for issue in all_issues {
        result.add_issue(issue);
    }
    print_progress("", options.quiet || options.verbose);
}

/// Run FormatOnly mode with parallel processing.
fn run_format_only_mode(
    file_langs: &[(&PathBuf, Language)],
    options: &RunOptions,
    result: &mut RunResult,
) {
    let total_files = file_langs.len();
    PROGRESS_COUNTER.store(0, Ordering::Relaxed);
    let format_start = Instant::now();

    let format_results: Vec<Option<FormatResult>> = file_langs
        .par_iter()
        .map(|(file, lang)| {
            let count = PROGRESS_COUNTER.fetch_add(1, Ordering::Relaxed);
            if !options.quiet && !options.verbose {
                let percentage = ((count + 1) as f64 / total_files as f64 * 100.0) as usize;
                print_progress_with_time(
                    &format!(
                        "Formatting {}/{} ({}%)...",
                        count + 1,
                        total_files,
                        percentage
                    ),
                    false,
                    format_start,
                    count,
                );
            }
            if options.verbose {
                eprintln!("Processing: {} ({})", file.display(), lang.name());
            }
            format_single_file(file, *lang, options.verbose)
        })
        .collect();

    for fr in format_results.into_iter().flatten() {
        result.add_format_result(fr);
    }
    print_progress("", options.quiet || options.verbose);
}

/// Run custom rules on all files and add issues to result.
fn run_custom_rules(
    custom_checker: &Option<CustomRulesChecker>,
    file_langs: &[(&PathBuf, Language)],
    options: &RunOptions,
    result: &mut RunResult,
) {
    let checker = match custom_checker {
        Some(c) if options.mode != RunMode::FormatOnly => c,
        _ => return,
    };
    if options.verbose {
        eprintln!("Running {} custom rules...", checker.rule_count());
    }
    for (file, lang) in file_langs {
        match checker.check(file, Some(lang.name())) {
            Ok(custom_issues) => {
                for mut issue in custom_issues {
                    issue.language = Some(*lang);
                    result.add_issue(issue);
                }
            }
            Err(e) => {
                if options.verbose {
                    eprintln!("Custom rule error for {}: {}", file.display(), e);
                }
            }
        }
    }
}

/// Save cache and print cache statistics.
fn finalize_cache(
    cache: Option<Mutex<LintCache>>,
    project_root: &Path,
    quiet: bool,
    verbose: bool,
) {
    let cache_mutex = match cache {
        Some(c) => c,
        None => return,
    };
    let cache_guard = cache_mutex.lock().unwrap();
    let stats = cache_guard.stats();

    if !quiet {
        if stats.total() == 0 {
            eprintln!("Running [lint] check...");
        } else {
            eprintln!(
                "Running [lint] check ({} cached, {} changed)",
                stats.cache_hits, stats.cache_misses,
            );
        }
    }

    if let Err(e) = cache_guard.save(project_root) {
        if verbose {
            eprintln!("Warning: Failed to save cache: {}", e);
        }
    }
}

/// Main entry point for running linthis.
pub fn run(options: &RunOptions) -> Result<RunResult> {
    use utils::types::RunModeKind;

    let start = Instant::now();
    let mut result = RunResult::new();

    result.run_mode = match options.mode {
        RunMode::Both => RunModeKind::Both,
        RunMode::CheckOnly => RunModeKind::CheckOnly,
        RunMode::FormatOnly => RunModeKind::FormatOnly,
    };

    // Scan and discover files
    let files = scan_files(options);

    let file_langs: Vec<_> = files
        .iter()
        .filter_map(|f| Language::from_path(f).map(|l| (f, l)))
        .collect();
    result.total_files = file_langs.len();

    // Pre-flight: auto-install missing tools
    if !file_langs.is_empty() {
        run_pre_flight_install(&file_langs, options, files.len());
    }

    // Load config and cache
    let project_root = utils::get_project_root();
    let config = Config::load_merged(&project_root);
    let cache = load_cache(
        options.no_cache,
        &options.mode,
        options.verbose,
        Some(config.retention.cache_days),
    );
    let rule_filter = RuleFilter::from_config(&config.rules);
    let custom_checker = load_custom_checker(&config, options.verbose);

    // Dispatch to mode-specific handler
    match options.mode {
        RunMode::Both => run_both_mode(&file_langs, options, &cache, &project_root, &mut result),
        RunMode::CheckOnly => {
            run_check_only_mode(&file_langs, options, &cache, &project_root, &mut result)
        }
        RunMode::FormatOnly => run_format_only_mode(&file_langs, options, &mut result),
    }

    // Run custom rules and apply filters
    run_custom_rules(&custom_checker, &file_langs, options, &mut result);
    apply_rule_filter(&rule_filter, &mut result, options.verbose);

    // Finalize
    result.count_files_with_issues();
    result.calculate_exit_code();
    result.duration_ms = start.elapsed().as_millis() as u64;
    result.unavailable_tools = collect_unavailable_tools();

    finalize_cache(cache, &project_root, options.quiet, options.verbose);

    Ok(result)
}

/// Scan file system for files to process, printing status messages.
fn scan_files(options: &RunOptions) -> Vec<std::path::PathBuf> {
    if !options.quiet && !options.plugins.is_empty() {
        eprintln!("📦 Plugins: {}", options.plugins.join(", "));
    }

    if !options.quiet {
        eprint!("\x1b[36m{}\x1b[0m Scanning files...", SPINNER_CHARS[0]);
        use std::io::Write;
        let _ = std::io::stderr().flush();
    }

    let walker_config = WalkerConfig {
        exclude_patterns: options.exclude_patterns.clone(),
        languages: options.languages.clone(),
        large_file_threshold: 1048576,
        ..Default::default()
    };

    let (files, path_warnings) = walk_paths(&options.paths, &walker_config);

    if !path_warnings.is_empty() && !options.quiet {
        eprint!("\r\x1b[K");
        for warning in &path_warnings {
            eprintln!("\x1b[33mWarning\x1b[0m: {}", warning);
        }
        print_found_files_status(files.len());
    } else if !options.quiet {
        print_found_files_status(files.len());
    }

    if options.verbose {
        eprintln!();
        eprintln!("Found {} files to process", files.len());
    }

    files
}

/// Run pre-flight tool installation checks.
fn run_pre_flight_install(
    file_langs: &[(&std::path::PathBuf, Language)],
    options: &RunOptions,
    file_count: usize,
) {
    let (installed, failed) = pre_flight_install(
        file_langs,
        &options.mode,
        &options.tool_install_mode,
        options.quiet,
    );
    if (!installed.is_empty() || !failed.is_empty()) && !options.quiet {
        print_found_files_status(file_count);
    }
    handle_install_failures(&failed, file_count, options.quiet);
}

/// Load custom rules checker from config.
fn load_custom_checker(config: &Config, verbose: bool) -> Option<CustomRulesChecker> {
    if !config.rules.has_custom_rules() {
        return None;
    }
    match CustomRulesChecker::new(&config.rules.custom) {
        Ok(checker) => {
            if verbose {
                eprintln!("Loaded {} custom rules", checker.rule_count());
            }
            Some(checker)
        }
        Err(e) => {
            eprintln!("\x1b[33mWarning\x1b[0m: Failed to load custom rules: {}", e);
            None
        }
    }
}

/// Apply rule filter to issues and log if verbose.
fn apply_rule_filter(rule_filter: &RuleFilter, result: &mut RunResult, verbose: bool) {
    let original_count = result.issues.len();
    result.issues = rule_filter.filter_issues(std::mem::take(&mut result.issues));
    let filtered_count = original_count - result.issues.len();
    if verbose && filtered_count > 0 {
        eprintln!(
            "Filtered out {} issues based on rules configuration",
            filtered_count
        );
    }
}

// Re-export commonly used types
pub use rules::{CustomRule, RulesConfig, SeverityOverride};
pub use utils::types::{FormatResult, LintIssue, Severity};
