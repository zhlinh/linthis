// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Linthis - A fast, cross-platform multi-language linter and formatter.

pub mod benchmark;
pub mod checkers;
pub mod config;
pub mod formatters;
pub mod plugin;
pub mod presets;
pub mod utils;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;
use thiserror::Error;

/// Track which tool warnings have been shown (to avoid duplicate warnings)
static WARNED_TOOLS: Mutex<Option<HashSet<String>>> = Mutex::new(None);

use checkers::{
    Checker, CppChecker, GoChecker, JavaChecker, PythonChecker, RustChecker, TypeScriptChecker,
};
use formatters::{
    CppFormatter, Formatter, GoFormatter, JavaFormatter, PythonFormatter, RustFormatter,
    TypeScriptFormatter,
};
use utils::types::RunResult;
use utils::walker::{walk_paths, WalkerConfig};

#[derive(Error, Debug)]
pub enum LintisError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Checker error: {0}")]
    Checker(String),

    #[error("Formatter error: {0}")]
    Formatter(String),

    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),
}

pub type Result<T> = std::result::Result<T, LintisError>;

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Cpp,
    ObjectiveC,
    Java,
    Python,
    Rust,
    Go,
    JavaScript,
    TypeScript,
}

impl Language {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "c" | "cc" | "cpp" | "cxx" | "h" | "hpp" | "hxx" => Some(Language::Cpp),
            "m" | "mm" => Some(Language::ObjectiveC),
            "java" => Some(Language::Java),
            "py" | "pyw" => Some(Language::Python),
            "rs" => Some(Language::Rust),
            "go" => Some(Language::Go),
            "js" | "jsx" | "mjs" | "cjs" => Some(Language::JavaScript),
            "ts" | "tsx" | "mts" | "cts" => Some(Language::TypeScript),
            _ => None,
        }
    }

    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::from_extension)
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
        }
    }
}

/// Get the checker for a given language.
fn get_checker(lang: Language) -> Option<Box<dyn Checker>> {
    match lang {
        Language::Rust => Some(Box::new(RustChecker::new())),
        Language::Python => Some(Box::new(PythonChecker::new())),
        Language::TypeScript | Language::JavaScript => Some(Box::new(TypeScriptChecker::new())),
        Language::Go => Some(Box::new(GoChecker::new())),
        Language::Java => Some(Box::new(JavaChecker::new())),
        Language::Cpp | Language::ObjectiveC => Some(Box::new(CppChecker::new())),
    }
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
    }
}

/// Warn about missing tool (once per tool)
fn warn_missing_tool(tool_type: &str, lang: Language, is_checker: bool) {
    let tool_key = format!("{}-{}", tool_type, lang.name());
    if should_warn_tool(&tool_key) {
        let hint = if is_checker {
            get_checker_install_hint(lang)
        } else {
            get_formatter_install_hint(lang)
        };
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
fn run_checker_on_file(file: &Path, lang: Language, verbose: bool) -> Vec<utils::types::LintIssue> {
    let mut issues = Vec::new();
    if let Some(checker) = get_checker(lang) {
        if checker.is_available() {
            match checker.check(file) {
                Ok(file_issues) => {
                    issues.extend(file_issues);
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
    let start = Instant::now();
    let mut result = RunResult::new();

    // Print starting message
    if !options.quiet {
        eprint!("⏳ Scanning files...");
        use std::io::Write;
        let _ = std::io::stderr().flush();
    }

    // Configure walker
    let walker_config = WalkerConfig {
        exclude_patterns: options.exclude_patterns.clone(),
        languages: options.languages.clone(),
        ..Default::default()
    };

    // Collect files to process
    let files = walk_paths(&options.paths, &walker_config);
    result.total_files = files.len();

    if !options.quiet {
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

    // For RunMode::Both: lint → format → lint (only files with issues)
    if options.mode == RunMode::Both {
        // Step 1: First lint pass (before formatting)
        if options.verbose {
            eprintln!("Step 1: Checking for issues...");
        }
        let mut issues_before = Vec::new();
        let mut files_with_issues: HashSet<PathBuf> = HashSet::new();
        let total_files = file_langs.len();
        for (idx, (file, lang)) in file_langs.iter().enumerate() {
            print_progress(
                &format!("⏳ [1/3] Checking ({}/{})...", idx + 1, total_files),
                options.quiet || options.verbose,
            );
            let file_issues = run_checker_on_file(file, *lang, options.verbose);
            if !file_issues.is_empty() {
                files_with_issues.insert((*file).clone());
            }
            issues_before.extend(file_issues);
        }
        result.issues_before_format = issues_before.len();

        // Step 2: Format files (only files with issues to save time)
        if options.verbose {
            eprintln!(
                "Step 2: Formatting {} files with issues...",
                files_with_issues.len()
            );
        }
        let mut formatted_files: HashSet<PathBuf> = HashSet::new();
        let files_to_format: Vec<_> = file_langs
            .iter()
            .filter(|(f, _)| files_with_issues.contains(*f))
            .collect();
        let format_total = files_to_format.len();
        for (idx, (file, lang)) in files_to_format.iter().enumerate() {
            print_progress(
                &format!("⏳ [2/3] Formatting ({}/{})...", idx + 1, format_total),
                options.quiet || options.verbose,
            );
            if let Some(formatter) = get_formatter(*lang) {
                if formatter.is_available() {
                    match formatter.format(file) {
                        Ok(format_result) => {
                            if format_result.changed {
                                formatted_files.insert((*file).clone());
                            }
                            result.add_format_result(format_result);
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
        }

        // Step 3: Second lint pass (only re-check files that were formatted)
        if options.verbose {
            eprintln!(
                "Step 3: Rechecking {} formatted files...",
                formatted_files.len()
            );
        }

        // Helper to normalize paths for comparison
        fn normalize_path(p: &Path) -> PathBuf {
            let s = p.to_string_lossy();
            let s = s.strip_prefix("./").unwrap_or(&s);
            PathBuf::from(s)
        }

        let recheck_total = formatted_files.len();
        let mut recheck_idx = 0;
        for (file, lang) in &file_langs {
            if formatted_files.contains(*file) {
                recheck_idx += 1;
                print_progress(
                    &format!("⏳ [3/3] Rechecking ({}/{})...", recheck_idx, recheck_total),
                    options.quiet || options.verbose,
                );
                // Re-check formatted files
                for issue in run_checker_on_file(file, *lang, options.verbose) {
                    result.add_issue(issue);
                }
            } else if files_with_issues.contains(*file) {
                // Keep original issues for files that weren't formatted
                let normalized_file = normalize_path(file);
                for issue in &issues_before {
                    let normalized_issue_path = normalize_path(&issue.file_path);
                    if normalized_issue_path == normalized_file {
                        result.add_issue(issue.clone());
                    }
                }
            }
            // Files without issues: no need to add anything
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
        for (idx, (file, lang)) in file_langs.iter().enumerate() {
            print_progress(
                &format!("⏳ {} ({}/{})...", mode_name, idx + 1, total_files),
                options.quiet || options.verbose,
            );
            if options.verbose {
                eprintln!("Processing: {} ({})", file.display(), lang.name());
            }

            // Run formatter if needed
            if options.mode == RunMode::FormatOnly {
                if let Some(formatter) = get_formatter(*lang) {
                    if formatter.is_available() {
                        match formatter.format(file) {
                            Ok(format_result) => {
                                result.add_format_result(format_result);
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
            }

            // Run checker if needed
            if options.mode == RunMode::CheckOnly {
                for issue in run_checker_on_file(file, *lang, options.verbose) {
                    result.add_issue(issue);
                }
            }
        }
        // Clear progress line
        print_progress("", options.quiet || options.verbose);
    }

    // Calculate final stats
    result.count_files_with_issues();
    result.calculate_exit_code();
    result.duration_ms = start.elapsed().as_millis() as u64;

    Ok(result)
}

// Re-export commonly used types
pub use utils::types::{FormatResult, LintIssue, Severity};
