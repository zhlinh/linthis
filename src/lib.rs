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
pub mod fixers;
pub mod formatters;
pub mod interactive;
pub mod lsp;
pub mod plugin;
pub mod presets;
pub mod reports;
pub mod rules;
pub mod security;
pub mod license;
pub mod complexity;
pub mod ai;
pub mod self_update;
pub mod templates;
pub mod tui;
pub mod utils;
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

use checkers::{
    Checker, CppChecker, DartChecker, GoChecker, JavaChecker, KotlinChecker, LuaChecker,
    PythonChecker, RustChecker, SwiftChecker, TypeScriptChecker,
};
use formatters::{
    CppFormatter, DartFormatter, Formatter, GoFormatter, JavaFormatter, KotlinFormatter,
    LuaFormatter, PythonFormatter, RustFormatter, SwiftFormatter, TypeScriptFormatter,
};
use cache::LintCache;
use config::Config;
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
        }
    }

    /// Check if error allows graceful degradation.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            LintisError::ToolNotAvailable { .. } | LintisError::Cache { .. }
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
        match ext.to_lowercase().as_str() {
            // Note: .h files need special handling via from_path() for smart detection
            "c" | "cc" | "cpp" | "cxx" | "hpp" | "hxx" => Some(Language::Cpp),
            "h" => None, // Handle .h files specially in from_path()
            "m" | "mm" => Some(Language::ObjectiveC),
            "java" => Some(Language::Java),
            "py" | "pyw" => Some(Language::Python),
            "rs" => Some(Language::Rust),
            "go" => Some(Language::Go),
            "js" | "jsx" | "mjs" | "cjs" => Some(Language::JavaScript),
            "ts" | "tsx" | "mts" | "cts" => Some(Language::TypeScript),
            "dart" => Some(Language::Dart),
            "swift" => Some(Language::Swift),
            "kt" | "kts" => Some(Language::Kotlin),
            "lua" => Some(Language::Lua),
            "sh" | "bash" | "zsh" | "ksh" => Some(Language::Shell),
            "rb" | "rake" | "gemspec" => Some(Language::Ruby),
            "php" | "phtml" => Some(Language::Php),
            "scala" | "sc" => Some(Language::Scala),
            "cs" | "csx" => Some(Language::CSharp),
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
        // 1. Check for corresponding .m/.mm file (same name) -> Objective-C
        // 2. Check for corresponding .cpp/.cc/.cxx file (same name) -> C++
        if let Some(parent) = path.parent() {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                // Check for Objective-C implementation files
                for ext in &["m", "mm"] {
                    let impl_path = parent.join(format!("{}.{}", stem, ext));
                    if impl_path.exists() {
                        return Language::ObjectiveC;
                    }
                }
                // Check for C++ implementation files
                for ext in &["cpp", "cc", "cxx", "c"] {
                    let impl_path = parent.join(format!("{}.{}", stem, ext));
                    if impl_path.exists() {
                        return Language::Cpp;
                    }
                }
            }
        }

        // 3. Check file content for language-specific patterns
        if let Ok(content) = std::fs::read_to_string(path) {
            // Objective-C patterns (comprehensive list matching formatter)
            let objc_patterns = [
                "#import",
                "@import", // OC module import: @import UIKit;
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
                "+ (",  // OC class method
                "- (",  // OC instance method
                " @\"", // OC string literal: @"string"
                " @[",  // OC array literal: @[@"a", @"b"]
            ];
            for pattern in objc_patterns {
                if content.contains(pattern) {
                    return Language::ObjectiveC;
                }
            }

            // Check for Foundation types: NS followed by uppercase letter
            // (e.g., NSString, NSArray, NSDictionary, NSURL, NSError)
            if Self::contains_ns_type(&content) {
                return Language::ObjectiveC;
            }

            // C++ patterns
            let cpp_patterns = ["namespace ", "template<", "template <"];
            for pattern in cpp_patterns {
                if content.contains(pattern) {
                    return Language::Cpp;
                }
            }
        }

        // 4. Check directory for other files to infer language
        if let Some(parent) = path.parent() {
            if let Ok(entries) = std::fs::read_dir(parent) {
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
                    return Language::ObjectiveC;
                }
            }
        }

        // 5. Default to C++
        Language::Cpp
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
            .field("config_resolver", &self.config_resolver.as_ref().map(|r| format!("{} configs", r.len())))
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
    get_formatter(lang).map(|f| f.is_available()).unwrap_or(false)
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

/// Get installation instructions for a language's linter (platform-specific)
fn get_checker_install_hint(lang: Language) -> String {
    match lang {
        Language::Rust => "Install: rustup component add clippy".to_string(),
        Language::Python => "Install: pip install ruff".to_string(),
        Language::Go => {
            if cfg!(target_os = "macos") {
                "Install: brew install golangci-lint\n         Or: go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest".to_string()
            } else if cfg!(target_os = "windows") {
                "Install: go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest\n         Or: choco install golangci-lint".to_string()
            } else {
                "Install: go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest\n         Or: sudo apt install golangci-lint (Ubuntu/Debian)".to_string()
            }
        }
        Language::TypeScript | Language::JavaScript => "Install: npm install -g eslint".to_string(),
        Language::Java => {
            if cfg!(target_os = "macos") {
                "Install: brew install checkstyle".to_string()
            } else if cfg!(target_os = "windows") {
                "Install: choco install checkstyle\n         Or download from: https://checkstyle.org/".to_string()
            } else {
                "Install: sudo apt install checkstyle (Ubuntu/Debian)\n         Or download from: https://checkstyle.org/".to_string()
            }
        }
        Language::Cpp | Language::ObjectiveC => {
            if cfg!(target_os = "macos") {
                "Install: brew install llvm (for clang-tidy)\n         Or: pip install cpplint"
                    .to_string()
            } else if cfg!(target_os = "windows") {
                "Install: choco install llvm (for clang-tidy)\n         Or: pip install cpplint"
                    .to_string()
            } else {
                "Install: sudo apt install clang-tidy (Ubuntu/Debian)\n         Or: pip install cpplint".to_string()
            }
        }
        Language::Dart => "Install: Dart SDK (includes dart analyze)\n         https://dart.dev/get-dart".to_string(),
        Language::Swift => {
            if cfg!(target_os = "macos") {
                "Install: brew install swiftlint".to_string()
            } else {
                "Install: https://github.com/realm/SwiftLint".to_string()
            }
        }
        Language::Kotlin => {
            if cfg!(target_os = "macos") {
                "Install: brew install ktlint".to_string()
            } else {
                "Install: https://github.com/pinterest/ktlint".to_string()
            }
        }
        Language::Lua => "Install: luarocks install luacheck".to_string(),
        Language::Shell => {
            if cfg!(target_os = "macos") {
                "Install: brew install shellcheck".to_string()
            } else if cfg!(target_os = "windows") {
                "Install: choco install shellcheck\n         Or: scoop install shellcheck".to_string()
            } else {
                "Install: sudo apt install shellcheck (Ubuntu/Debian)".to_string()
            }
        }
        Language::Ruby => "Install: gem install rubocop".to_string(),
        Language::Php => "Install: composer global require squizlabs/php_codesniffer".to_string(),
        Language::Scala => {
            if cfg!(target_os = "macos") {
                "Install: brew install scalafix\n         Or: cs install scalafix".to_string()
            } else {
                "Install: cs install scalafix\n         https://scalacenter.github.io/scalafix/".to_string()
            }
        }
        Language::CSharp => "Install: dotnet tool install -g dotnet-format".to_string(),
    }
}

/// Get installation instructions for a language's formatter (platform-specific)
fn get_formatter_install_hint(lang: Language) -> String {
    match lang {
        Language::Rust => "Install: rustup component add rustfmt".to_string(),
        Language::Python => "Install: pip install ruff".to_string(),
        Language::Go => "Install: Go formatter (gofmt) is included with Go".to_string(),
        Language::TypeScript | Language::JavaScript => {
            "Install: npm install -g prettier".to_string()
        }
        Language::Java => {
            if cfg!(target_os = "macos") {
                "Install: brew install google-java-format".to_string()
            } else if cfg!(target_os = "windows") {
                "Install: Download from https://github.com/google/google-java-format/releases"
                    .to_string()
            } else {
                "Install: Download from https://github.com/google/google-java-format/releases\n         Or use your package manager".to_string()
            }
        }
        Language::Cpp | Language::ObjectiveC => {
            if cfg!(target_os = "macos") {
                "Install: brew install clang-format\n         Or: brew install llvm".to_string()
            } else if cfg!(target_os = "windows") {
                "Install: choco install llvm (includes clang-format)".to_string()
            } else {
                "Install: sudo apt install clang-format (Ubuntu/Debian)".to_string()
            }
        }
        Language::Dart => "Install: Dart SDK (includes dart format)\n         https://dart.dev/get-dart".to_string(),
        Language::Swift => {
            if cfg!(target_os = "macos") {
                "Install: brew install swift-format".to_string()
            } else {
                "Install: https://github.com/apple/swift-format".to_string()
            }
        }
        Language::Kotlin => {
            if cfg!(target_os = "macos") {
                "Install: brew install ktlint".to_string()
            } else {
                "Install: https://github.com/pinterest/ktlint".to_string()
            }
        }
        Language::Lua => "Install: cargo install stylua".to_string(),
        Language::Shell => {
            if cfg!(target_os = "macos") {
                "Install: brew install shfmt".to_string()
            } else if cfg!(target_os = "windows") {
                "Install: choco install shfmt\n         Or: scoop install shfmt".to_string()
            } else {
                "Install: sudo apt install shfmt (Ubuntu/Debian)\n         Or: go install mvdan.cc/sh/v3/cmd/shfmt@latest".to_string()
            }
        }
        Language::Ruby => "Install: gem install rubocop".to_string(),
        Language::Php => "Install: composer global require friendsofphp/php-cs-fixer".to_string(),
        Language::Scala => {
            if cfg!(target_os = "macos") {
                "Install: brew install scalafmt\n         Or: cs install scalafmt".to_string()
            } else {
                "Install: cs install scalafmt\n         https://scalameta.org/scalafmt/".to_string()
            }
        }
        Language::CSharp => "Install: dotnet tool install -g dotnet-format".to_string(),
    }
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

        // Record the unavailable tool
        let tool_name = get_tool_name(lang, is_checker);
        record_unavailable_tool(utils::types::UnavailableTool::new(
            &tool_name,
            lang.name(),
            tool_type,
            &hint,
        ));

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

/// Get the name of the tool for a language
fn get_tool_name(lang: Language, is_checker: bool) -> String {
    match (lang, is_checker) {
        (Language::Rust, true) => "clippy".to_string(),
        (Language::Rust, false) => "rustfmt".to_string(),
        (Language::Python, true) | (Language::Python, false) => "ruff".to_string(),
        (Language::Go, true) => "golangci-lint".to_string(),
        (Language::Go, false) => "gofmt".to_string(),
        (Language::TypeScript, true) | (Language::JavaScript, true) => "eslint".to_string(),
        (Language::TypeScript, false) | (Language::JavaScript, false) => "prettier".to_string(),
        (Language::Java, true) => "checkstyle".to_string(),
        (Language::Java, false) => "google-java-format".to_string(),
        (Language::Cpp, true) | (Language::ObjectiveC, true) => "cpplint".to_string(),
        (Language::Cpp, false) | (Language::ObjectiveC, false) => "clang-format".to_string(),
        (Language::Dart, true) => "dart-analyze".to_string(),
        (Language::Dart, false) => "dart-format".to_string(),
        (Language::Swift, true) => "swiftlint".to_string(),
        (Language::Swift, false) => "swift-format".to_string(),
        (Language::Kotlin, true) | (Language::Kotlin, false) => "ktlint".to_string(),
        (Language::Lua, true) => "luacheck".to_string(),
        (Language::Lua, false) => "stylua".to_string(),
        (Language::Shell, true) => "shellcheck".to_string(),
        (Language::Shell, false) => "shfmt".to_string(),
        (Language::Ruby, true) | (Language::Ruby, false) => "rubocop".to_string(),
        (Language::Php, true) => "phpcs".to_string(),
        (Language::Php, false) => "php-cs-fixer".to_string(),
        (Language::Scala, true) => "scalafix".to_string(),
        (Language::Scala, false) => "scalafmt".to_string(),
        (Language::CSharp, true) | (Language::CSharp, false) => "dotnet-format".to_string(),
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
        let exists = list.iter().any(|t| t.tool == tool.tool && t.language == tool.language);
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
            let config_path = config_resolver.and_then(|r| {
                r.get_plugin_config(lang.name(), checker.name())
            });

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

/// Print progress message (respects quiet mode)
fn print_progress(msg: &str, quiet: bool) {
    if !quiet {
        eprint!("\r\x1b[K{}", msg); // Clear line and print
        use std::io::Write;
        let _ = std::io::stderr().flush();
    }
}

/// Main entry point for running linthis.
pub fn run(options: &RunOptions) -> Result<RunResult> {
    use utils::types::RunModeKind;

    let start = Instant::now();
    let mut result = RunResult::new();

    // Set run mode for appropriate output messages
    result.run_mode = match options.mode {
        RunMode::Both => RunModeKind::Both,
        RunMode::CheckOnly => RunModeKind::CheckOnly,
        RunMode::FormatOnly => RunModeKind::FormatOnly,
    };

    // Print plugins in use
    if !options.quiet && !options.plugins.is_empty() {
        eprintln!("📦 Plugins: {}", options.plugins.join(", "));
    }

    // Print starting message
    if !options.quiet {
        eprint!("⏳ Scanning files...");
        use std::io::Write;
        let _ = std::io::stderr().flush();
    }

    // Configure walker with large file detection (default: 1MB threshold)
    let walker_config = WalkerConfig {
        exclude_patterns: options.exclude_patterns.clone(),
        languages: options.languages.clone(),
        large_file_threshold: 1048576, // 1MB default
        ..Default::default()
    };

    // Collect files to process
    let (files, path_warnings) = walk_paths(&options.paths, &walker_config);

    // Print warnings about paths (clear line first, then print warnings)
    if !path_warnings.is_empty() && !options.quiet {
        eprint!("\r\x1b[K"); // Clear "Scanning files..." line
        for warning in &path_warnings {
            eprintln!("\x1b[33mWarning\x1b[0m: {}", warning);
        }
        eprint!("⏳ Found {} files, checking...", files.len());
        use std::io::Write;
        let _ = std::io::stderr().flush();
    } else if !options.quiet {
        eprint!("\r\x1b[K⏳ Found {} files, checking...", files.len());
        use std::io::Write;
        let _ = std::io::stderr().flush();
    }

    if options.verbose {
        eprintln!();
        eprintln!("Found {} files to process", files.len());
    }

    // Build file-to-language map
    let file_langs: Vec<_> = files
        .iter()
        .filter_map(|f| Language::from_path(f).map(|l| (f, l)))
        .collect();

    // Set total_files to actual processable files count
    result.total_files = file_langs.len();

    // Load cache if enabled (only for check modes)
    let project_root = utils::get_project_root();
    let cache = if !options.no_cache && options.mode != RunMode::FormatOnly {
        match LintCache::load(&project_root) {
            Ok(mut c) => {
                c.prune(None); // Clean old entries
                c.reset_stats();
                Some(Mutex::new(c))
            }
            Err(e) => {
                if options.verbose {
                    eprintln!("Cache load failed: {}, starting fresh", e);
                }
                Some(Mutex::new(LintCache::new()))
            }
        }
    } else {
        None
    };

    // Load config for custom rules and rule filtering
    let config = Config::load_merged(&project_root);

    // Create rule filter from config
    let rule_filter = RuleFilter::from_config(&config.rules);

    // Create custom rules checker if custom rules are defined
    let custom_checker = if config.rules.has_custom_rules() {
        match CustomRulesChecker::new(&config.rules.custom) {
            Ok(checker) => {
                if options.verbose {
                    eprintln!("Loaded {} custom rules", checker.rule_count());
                }
                Some(checker)
            }
            Err(e) => {
                eprintln!("\x1b[33mWarning\x1b[0m: Failed to load custom rules: {}", e);
                None
            }
        }
    } else {
        None
    };

    // For RunMode::Both: lint → format → lint (only files with issues)
    if options.mode == RunMode::Both {
        // Step 1: First lint pass (before formatting) - parallel processing
        if options.verbose {
            eprintln!("Step 1: Checking for issues...");
        }
        let total_files = file_langs.len();
        PROGRESS_COUNTER.store(0, Ordering::Relaxed);

        let check_results: Vec<(PathBuf, Vec<_>)> = file_langs
            .par_iter()
            .map(|(file, lang)| {
                let count = PROGRESS_COUNTER.fetch_add(1, Ordering::Relaxed);
                if !options.quiet && !options.verbose {
                    let percentage = ((count + 1) as f64 / total_files as f64 * 100.0) as usize;
                    print_progress(
                        &format!("⏳ [1/3] Checking {}/{} ({}%)...", count + 1, total_files, percentage),
                        false,
                    );
                }

                // Check cache first
                if let Some(ref cache_mutex) = cache {
                    let mut cache_guard = cache_mutex.lock().unwrap();
                    if let Some(cached_issues) = cache_guard.check_file(
                        lang.name(),
                        file,
                        &project_root,
                    ) {
                        return ((*file).clone(), cached_issues);
                    }
                }

                // Cache miss - run actual check
                let file_issues = run_checker_on_file(
                    file,
                    *lang,
                    options.verbose,
                    options.config_resolver.as_deref(),
                );

                // Update cache with results
                if let Some(ref cache_mutex) = cache {
                    let mut cache_guard = cache_mutex.lock().unwrap();
                    let _ = cache_guard.update_file(
                        lang.name(),
                        file,
                        &project_root,
                        &file_issues,
                    );
                }

                ((*file).clone(), file_issues)
            })
            .collect();

        // Collect results
        let mut issues_before = Vec::new();
        let mut files_with_issues: HashSet<PathBuf> = HashSet::new();
        for (file, file_issues) in check_results {
            if !file_issues.is_empty() {
                files_with_issues.insert(file);
            }
            issues_before.extend(file_issues);
        }
        result.issues_before_format = issues_before.len();

        // Step 2: Format files (only files with issues to save time) - parallel processing
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

        let format_results: Vec<(PathBuf, Option<FormatResult>)> = files_to_format
            .par_iter()
            .map(|(file, lang)| {
                let count = PROGRESS_COUNTER.fetch_add(1, Ordering::Relaxed);
                if !options.quiet && !options.verbose {
                    let percentage = ((count + 1) as f64 / format_total as f64 * 100.0) as usize;
                    print_progress(
                        &format!("⏳ [2/3] Formatting {}/{} ({}%)...", count + 1, format_total, percentage),
                        false,
                    );
                }

                let mut format_result = None;
                if let Some(formatter) = get_formatter(*lang) {
                    if formatter.is_available() {
                        match formatter.format(file) {
                            Ok(result) => {
                                format_result = Some(result);
                            }
                            Err(e) => {
                                if options.verbose {
                                    eprintln!("Format error for {}: {}", file.display(), e);
                                }
                            }
                        }
                    } else {
                        warn_missing_tool("formatter", *lang, false);
                    }
                }
                ((*file).clone(), format_result)
            })
            .collect();

        // Collect formatted files and add results
        let mut formatted_files: HashSet<PathBuf> = HashSet::new();
        for (file, format_result) in format_results {
            if let Some(fr) = format_result {
                if fr.changed {
                    formatted_files.insert(file);
                }
                result.add_format_result(fr);
            }
        }

        // Step 3: Second lint pass (only re-check files that were formatted) - parallel processing
        if options.verbose {
            eprintln!(
                "Step 3: Rechecking {} formatted files...",
                formatted_files.len()
            );
        }

        // Helper to normalize paths for comparison
        // Convert to absolute path for reliable comparison
        fn normalize_path(p: &Path) -> PathBuf {
            // Try to canonicalize (absolute path), fall back to simple normalization
            if let Ok(canonical) = p.canonicalize() {
                canonical
            } else {
                // If canonicalize fails (e.g., file doesn't exist), try to make absolute
                if p.is_absolute() {
                    p.to_path_buf()
                } else {
                    // Make relative path absolute
                    if let Ok(current_dir) = std::env::current_dir() {
                        let joined = current_dir.join(p);
                        // Try to canonicalize the joined path
                        joined.canonicalize().unwrap_or(joined)
                    } else {
                        // Fall back to removing "./" prefix
                        let s = p.to_string_lossy();
                        let s = s.strip_prefix("./").unwrap_or(&s);
                        PathBuf::from(s)
                    }
                }
            }
        }

        let recheck_total = formatted_files.len();
        PROGRESS_COUNTER.store(0, Ordering::Relaxed);

        // Re-check formatted files in parallel
        let recheck_issues: Vec<_> = file_langs
            .par_iter()
            .flat_map(|(file, lang)| {
                if formatted_files.contains(*file) {
                    let count = PROGRESS_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if !options.quiet && !options.verbose {
                        let percentage = ((count + 1) as f64 / recheck_total as f64 * 100.0) as usize;
                        print_progress(
                            &format!("⏳ [3/3] Rechecking {}/{} ({}%)...", count + 1, recheck_total, percentage),
                            false,
                        );
                    }
                    run_checker_on_file(
                        file,
                        *lang,
                        options.verbose,
                        options.config_resolver.as_deref(),
                    )
                } else {
                    vec![]
                }
            })
            .collect();

        // Add rechecked issues
        for issue in recheck_issues {
            result.add_issue(issue);
        }

        // Keep original issues for files that weren't formatted
        for (file, _) in &file_langs {
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

        // Clear progress line
        print_progress("", options.quiet || options.verbose);

        // Calculate fixed issues (only if some files were actually formatted)
        if !formatted_files.is_empty() && result.issues_before_format > result.issues.len() {
            result.issues_fixed = result.issues_before_format - result.issues.len();
        }
    } else {
        // FormatOnly or CheckOnly mode
        let total_files = file_langs.len();
        let mode_name = if options.mode == RunMode::FormatOnly {
            "Formatting"
        } else {
            "Checking"
        };

        // CheckOnly mode: use parallel processing for better performance with cache support
        if options.mode == RunMode::CheckOnly {
            PROGRESS_COUNTER.store(0, Ordering::Relaxed);

            let all_issues: Vec<_> = file_langs
                .par_iter()
                .flat_map(|(file, lang)| {
                    let count = PROGRESS_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if !options.quiet && !options.verbose {
                        let percentage = ((count + 1) as f64 / total_files as f64 * 100.0) as usize;
                        print_progress(
                            &format!("⏳ {} {}/{} ({}%)...", mode_name, count + 1, total_files, percentage),
                            false,
                        );
                    }
                    if options.verbose {
                        eprintln!("Processing: {} ({})", file.display(), lang.name());
                    }

                    // Check cache first
                    if let Some(ref cache_mutex) = cache {
                        let mut cache_guard = cache_mutex.lock().unwrap();
                        if let Some(cached_issues) = cache_guard.check_file(
                            lang.name(),
                            file,
                            &project_root,
                        ) {
                            return cached_issues;
                        }
                    }

                    // Cache miss - run actual check
                    let issues = run_checker_on_file(
                        file,
                        *lang,
                        options.verbose,
                        options.config_resolver.as_deref(),
                    );

                    // Update cache with results
                    if let Some(ref cache_mutex) = cache {
                        let mut cache_guard = cache_mutex.lock().unwrap();
                        let _ = cache_guard.update_file(
                            lang.name(),
                            file,
                            &project_root,
                            &issues,
                        );
                    }

                    issues
                })
                .collect();

            for issue in all_issues {
                result.add_issue(issue);
            }
        } else {
            // FormatOnly mode: parallel processing
            PROGRESS_COUNTER.store(0, Ordering::Relaxed);

            let format_results: Vec<Option<FormatResult>> = file_langs
                .par_iter()
                .map(|(file, lang)| {
                    let count = PROGRESS_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if !options.quiet && !options.verbose {
                        let percentage = ((count + 1) as f64 / total_files as f64 * 100.0) as usize;
                        print_progress(
                            &format!("⏳ {} {}/{} ({}%)...", mode_name, count + 1, total_files, percentage),
                            false,
                        );
                    }
                    if options.verbose {
                        eprintln!("Processing: {} ({})", file.display(), lang.name());
                    }

                    let mut format_result = None;
                    if let Some(formatter) = get_formatter(*lang) {
                        if formatter.is_available() {
                            match formatter.format(file) {
                                Ok(result) => {
                                    format_result = Some(result);
                                }
                                Err(e) => {
                                    if options.verbose {
                                        eprintln!("Format error for {}: {}", file.display(), e);
                                    }
                                }
                            }
                        } else {
                            warn_missing_tool("formatter", *lang, false);
                        }
                    }
                    format_result
                })
                .collect();

            // Add format results
            for fr in format_results.into_iter().flatten() {
                result.add_format_result(fr);
            }
        }
        // Clear progress line
        print_progress("", options.quiet || options.verbose);
    }

    // Run custom rules checker on all files (if defined)
    if let Some(ref checker) = custom_checker {
        if options.mode != RunMode::FormatOnly {
            if options.verbose {
                eprintln!("Running {} custom rules...", checker.rule_count());
            }
            for (file, lang) in &file_langs {
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
    }

    // Apply rule filter to remove disabled rules and adjust severity
    let original_count = result.issues.len();
    result.issues = rule_filter.filter_issues(result.issues);
    let filtered_count = original_count - result.issues.len();
    if options.verbose && filtered_count > 0 {
        eprintln!("Filtered out {} issues based on rules configuration", filtered_count);
    }

    // Calculate final stats
    result.count_files_with_issues();
    result.calculate_exit_code();
    result.duration_ms = start.elapsed().as_millis() as u64;

    // Collect unavailable tools for reporting
    result.unavailable_tools = collect_unavailable_tools();

    // Save cache and show stats
    if let Some(cache_mutex) = cache {
        let cache_guard = cache_mutex.lock().unwrap();
        let stats = cache_guard.stats();

        if !options.quiet && stats.total() > 0 {
            eprintln!(
                "Cache: {} hits, {} misses ({:.1}% hit rate)",
                stats.cache_hits,
                stats.cache_misses,
                stats.hit_rate()
            );
        }

        if let Err(e) = cache_guard.save(&project_root) {
            if options.verbose {
                eprintln!("Warning: Failed to save cache: {}", e);
            }
        }
    }

    Ok(result)
}

// Re-export commonly used types
pub use rules::{CustomRule, RulesConfig, SeverityOverride};
pub use utils::types::{FormatResult, LintIssue, Severity};
