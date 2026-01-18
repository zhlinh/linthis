// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Doctor command for checking tool availability and configuration health.
//!
//! This module provides functionality to:
//! - Check if linters and formatters are installed
//! - Display tool versions
//! - Provide installation hints for missing tools
//! - Validate configuration files

use colored::Colorize;
use serde::Serialize;
use std::process::{Command, ExitCode};

use linthis::{get_checker, Language};

/// Helper module for getting home directory
mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var("HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
    }
}

/// Result of checking a single tool
#[derive(Debug, Clone, Serialize)]
pub struct ToolStatus {
    pub name: String,
    pub language: String,
    pub tool_type: String,
    pub available: bool,
    pub version: Option<String>,
    pub install_hint: Option<String>,
}

/// Result of checking configuration
#[derive(Debug, Clone, Serialize)]
pub struct ConfigStatus {
    pub path: String,
    pub exists: bool,
    pub valid: bool,
    pub error: Option<String>,
}

/// Overall doctor result
#[derive(Debug, Clone, Serialize)]
pub struct DoctorResult {
    pub tools: Vec<ToolStatus>,
    pub configs: Vec<ConfigStatus>,
    pub all_passed: bool,
}

/// All supported languages
const ALL_LANGUAGES: &[Language] = &[
    Language::Rust,
    Language::Python,
    Language::Go,
    Language::TypeScript,
    Language::JavaScript,
    Language::Java,
    Language::Cpp,
    Language::ObjectiveC,
    Language::Dart,
    Language::Swift,
    Language::Kotlin,
    Language::Lua,
];

/// Run the doctor command
pub fn handle_doctor_command(all: bool, output_format: &str) -> ExitCode {
    let languages = if all {
        ALL_LANGUAGES.to_vec()
    } else {
        detect_project_languages()
    };

    let mut tools = Vec::new();

    // Check checkers and formatters for each language
    for lang in &languages {
        // Check linter
        let checker_status = check_tool(*lang, true);
        tools.push(checker_status);

        // Check formatter
        let formatter_status = check_tool(*lang, false);
        tools.push(formatter_status);
    }

    // Check configuration files
    let configs = check_configs();

    // Determine if all passed
    let all_tools_ok = tools.iter().all(|t| t.available);
    let all_configs_ok = configs.iter().all(|c| c.valid || !c.exists);
    let all_passed = all_tools_ok && all_configs_ok;

    let result = DoctorResult {
        tools,
        configs,
        all_passed,
    };

    // Output based on format
    match output_format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
        }
        _ => {
            print_human_output(&result, &languages);
        }
    }

    if all_passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// Check a single tool (checker or formatter) availability
fn check_tool(lang: Language, is_checker: bool) -> ToolStatus {
    let (name, cmd_name, version_arg) = get_tool_info(lang, is_checker);
    let tool_type = if is_checker { "checker" } else { "formatter" };

    // Check if the tool is available via our checker/formatter system
    let available = if is_checker {
        get_checker(lang).map(|c| c.is_available()).unwrap_or(false)
    } else {
        linthis::get_formatter_availability(lang)
    };

    // Get version if available
    let version = if available {
        get_tool_version(&cmd_name, version_arg)
    } else {
        None
    };

    // Get install hint if not available
    let install_hint = if !available {
        Some(get_install_hint(lang, is_checker))
    } else {
        None
    };

    ToolStatus {
        name,
        language: lang.name().to_string(),
        tool_type: tool_type.to_string(),
        available,
        version,
        install_hint,
    }
}

/// Get tool command info (name, command, version_arg)
fn get_tool_info(lang: Language, is_checker: bool) -> (String, String, &'static str) {
    match (lang, is_checker) {
        // Checkers
        (Language::Rust, true) => ("clippy".to_string(), "cargo".to_string(), "clippy --version"),
        (Language::Python, true) => ("ruff".to_string(), "ruff".to_string(), "--version"),
        (Language::Go, true) => ("golangci-lint".to_string(), "golangci-lint".to_string(), "--version"),
        (Language::TypeScript, true) | (Language::JavaScript, true) => {
            ("eslint".to_string(), "eslint".to_string(), "--version")
        }
        (Language::Java, true) => ("checkstyle".to_string(), "checkstyle".to_string(), "--version"),
        (Language::Cpp, true) | (Language::ObjectiveC, true) => {
            ("cpplint".to_string(), "cpplint".to_string(), "--version")
        }
        (Language::Dart, true) => ("dart-analyze".to_string(), "dart".to_string(), "--version"),
        (Language::Swift, true) => ("swiftlint".to_string(), "swiftlint".to_string(), "version"),
        (Language::Kotlin, true) => ("ktlint".to_string(), "ktlint".to_string(), "--version"),
        (Language::Lua, true) => ("luacheck".to_string(), "luacheck".to_string(), "--version"),
        (Language::Shell, true) => ("shellcheck".to_string(), "shellcheck".to_string(), "--version"),
        (Language::Ruby, true) => ("rubocop".to_string(), "rubocop".to_string(), "--version"),
        (Language::Php, true) => ("phpcs".to_string(), "phpcs".to_string(), "--version"),
        (Language::Scala, true) => ("scalafix".to_string(), "scalafix".to_string(), "--version"),
        (Language::CSharp, true) => ("dotnet-format".to_string(), "dotnet".to_string(), "format --version"),

        // Formatters
        (Language::Rust, false) => ("rustfmt".to_string(), "rustfmt".to_string(), "--version"),
        (Language::Python, false) => ("ruff".to_string(), "ruff".to_string(), "--version"),
        (Language::Go, false) => ("gofmt".to_string(), "gofmt".to_string(), "-h"), // gofmt doesn't have --version
        (Language::TypeScript, false) | (Language::JavaScript, false) => {
            ("prettier".to_string(), "prettier".to_string(), "--version")
        }
        (Language::Java, false) => ("google-java-format".to_string(), "google-java-format".to_string(), "--version"),
        (Language::Cpp, false) | (Language::ObjectiveC, false) => {
            ("clang-format".to_string(), "clang-format".to_string(), "--version")
        }
        (Language::Dart, false) => ("dart-format".to_string(), "dart".to_string(), "--version"),
        (Language::Swift, false) => ("swift-format".to_string(), "swift-format".to_string(), "--version"),
        (Language::Kotlin, false) => ("ktlint".to_string(), "ktlint".to_string(), "--version"),
        (Language::Lua, false) => ("stylua".to_string(), "stylua".to_string(), "--version"),
        (Language::Shell, false) => ("shfmt".to_string(), "shfmt".to_string(), "--version"),
        (Language::Ruby, false) => ("rubocop".to_string(), "rubocop".to_string(), "--version"),
        (Language::Php, false) => ("php-cs-fixer".to_string(), "php-cs-fixer".to_string(), "--version"),
        (Language::Scala, false) => ("scalafmt".to_string(), "scalafmt".to_string(), "--version"),
        (Language::CSharp, false) => ("dotnet-format".to_string(), "dotnet".to_string(), "format --version"),
    }
}

/// Get tool version by running command
fn get_tool_version(cmd: &str, version_arg: &str) -> Option<String> {
    // Special handling for cargo clippy
    if cmd == "cargo" && version_arg.contains("clippy") {
        let output = Command::new("cargo")
            .args(["clippy", "--version"])
            .output()
            .ok()?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            return Some(version.trim().to_string());
        }
        return None;
    }

    // Special handling for gofmt (no --version)
    if cmd == "gofmt" {
        // gofmt doesn't have a version flag, just check if it exists
        let output = Command::new("go").args(["version"]).output().ok()?;
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            // Extract just the version part
            return Some(version.trim().to_string());
        }
        return None;
    }

    let args: Vec<&str> = version_arg.split_whitespace().collect();
    let output = Command::new(cmd).args(&args).output().ok()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        let version = version.trim();
        // Take first line only
        let first_line = version.lines().next().unwrap_or(version);
        Some(first_line.to_string())
    } else {
        // Some tools output version to stderr
        let version = String::from_utf8_lossy(&output.stderr);
        let version = version.trim();
        if !version.is_empty() {
            let first_line = version.lines().next().unwrap_or(version);
            Some(first_line.to_string())
        } else {
            None
        }
    }
}

/// Get installation hint for a tool
fn get_install_hint(lang: Language, is_checker: bool) -> String {
    match (lang, is_checker) {
        // Checkers
        (Language::Rust, true) => "rustup component add clippy".to_string(),
        (Language::Python, true) => "pip install ruff".to_string(),
        (Language::Go, true) => {
            if cfg!(target_os = "macos") {
                "brew install golangci-lint".to_string()
            } else {
                "go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest".to_string()
            }
        }
        (Language::TypeScript, true) | (Language::JavaScript, true) => {
            "npm install -g eslint".to_string()
        }
        (Language::Java, true) => {
            if cfg!(target_os = "macos") {
                "brew install checkstyle".to_string()
            } else {
                "https://checkstyle.sourceforge.io/".to_string()
            }
        }
        (Language::Cpp, true) | (Language::ObjectiveC, true) => {
            "pip install cpplint".to_string()
        }
        (Language::Dart, true) => "https://dart.dev/get-dart".to_string(),
        (Language::Swift, true) => {
            if cfg!(target_os = "macos") {
                "brew install swiftlint".to_string()
            } else {
                "https://github.com/realm/SwiftLint".to_string()
            }
        }
        (Language::Kotlin, true) => {
            if cfg!(target_os = "macos") {
                "brew install ktlint".to_string()
            } else {
                "https://github.com/pinterest/ktlint".to_string()
            }
        }
        (Language::Lua, true) => "luarocks install luacheck".to_string(),
        (Language::Shell, true) => {
            if cfg!(target_os = "macos") {
                "brew install shellcheck".to_string()
            } else if cfg!(target_os = "windows") {
                "choco install shellcheck".to_string()
            } else {
                "apt install shellcheck".to_string()
            }
        }
        (Language::Ruby, true) => "gem install rubocop".to_string(),
        (Language::Php, true) => "composer global require squizlabs/php_codesniffer".to_string(),
        (Language::Scala, true) => "cs install scalafix".to_string(),
        (Language::CSharp, true) => "dotnet tool install -g dotnet-format".to_string(),

        // Formatters
        (Language::Rust, false) => "rustup component add rustfmt".to_string(),
        (Language::Python, false) => "pip install ruff".to_string(),
        (Language::Go, false) => "Included with Go installation".to_string(),
        (Language::TypeScript, false) | (Language::JavaScript, false) => {
            "npm install -g prettier".to_string()
        }
        (Language::Java, false) => {
            if cfg!(target_os = "macos") {
                "brew install google-java-format".to_string()
            } else {
                "https://github.com/google/google-java-format/releases".to_string()
            }
        }
        (Language::Cpp, false) | (Language::ObjectiveC, false) => {
            if cfg!(target_os = "macos") {
                "brew install clang-format".to_string()
            } else if cfg!(target_os = "linux") {
                "apt install clang-format".to_string()
            } else {
                "https://releases.llvm.org/download.html".to_string()
            }
        }
        (Language::Dart, false) => "https://dart.dev/get-dart".to_string(),
        (Language::Swift, false) => {
            if cfg!(target_os = "macos") {
                "brew install swift-format".to_string()
            } else {
                "https://github.com/apple/swift-format".to_string()
            }
        }
        (Language::Kotlin, false) => {
            if cfg!(target_os = "macos") {
                "brew install ktlint".to_string()
            } else {
                "https://github.com/pinterest/ktlint".to_string()
            }
        }
        (Language::Lua, false) => "cargo install stylua".to_string(),
        (Language::Shell, false) => {
            if cfg!(target_os = "macos") {
                "brew install shfmt".to_string()
            } else if cfg!(target_os = "windows") {
                "choco install shfmt".to_string()
            } else {
                "go install mvdan.cc/sh/v3/cmd/shfmt@latest".to_string()
            }
        }
        (Language::Ruby, false) => "gem install rubocop".to_string(),
        (Language::Php, false) => "composer global require friendsofphp/php-cs-fixer".to_string(),
        (Language::Scala, false) => "cs install scalafmt".to_string(),
        (Language::CSharp, false) => "dotnet tool install -g dotnet-format".to_string(),
    }
}

/// Detect languages used in the current project
fn detect_project_languages() -> Vec<Language> {
    use std::collections::HashSet;
    use walkdir::WalkDir;

    let mut detected: HashSet<Language> = HashSet::new();
    let current_dir = std::env::current_dir().unwrap_or_default();

    // Walk directory and detect languages from file extensions
    for entry in WalkDir::new(&current_dir)
        .max_depth(5) // Don't go too deep
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip hidden directories and common non-source directories
        if path.components().any(|c| {
            let s = c.as_os_str().to_string_lossy();
            s.starts_with('.') || s == "node_modules" || s == "target" || s == "vendor"
        }) {
            continue;
        }

        if let Some(ext) = path.extension() {
            if let Some(lang) = Language::from_extension(&ext.to_string_lossy()) {
                detected.insert(lang);
            }
        }
    }

    // Convert to sorted vec
    let mut langs: Vec<Language> = detected.into_iter().collect();
    langs.sort_by_key(|l| l.name());
    langs
}

/// Check configuration files
fn check_configs() -> Vec<ConfigStatus> {
    let mut configs = Vec::new();

    // Check project config
    let project_config = std::env::current_dir()
        .unwrap_or_default()
        .join(".linthis")
        .join("config.toml");

    let project_status = check_config_file(&project_config);
    configs.push(project_status);

    // Check global config
    if let Some(home) = dirs::home_dir() {
        let global_config = home.join(".linthis").join("config.toml");
        let global_status = check_config_file(&global_config);
        configs.push(global_status);
    }

    configs
}

/// Check a single configuration file
fn check_config_file(path: &std::path::Path) -> ConfigStatus {
    let path_str = path.display().to_string();

    if !path.exists() {
        return ConfigStatus {
            path: path_str,
            exists: false,
            valid: true, // Non-existent is not an error
            error: None,
        };
    }

    match std::fs::read_to_string(path) {
        Ok(content) => {
            // Try to parse as TOML
            match toml::from_str::<toml::Value>(&content) {
                Ok(_) => ConfigStatus {
                    path: path_str,
                    exists: true,
                    valid: true,
                    error: None,
                },
                Err(e) => ConfigStatus {
                    path: path_str,
                    exists: true,
                    valid: false,
                    error: Some(e.to_string()),
                },
            }
        }
        Err(e) => ConfigStatus {
            path: path_str,
            exists: true,
            valid: false,
            error: Some(format!("Failed to read: {}", e)),
        },
    }
}

/// Print human-readable output
fn print_human_output(result: &DoctorResult, languages: &[Language]) {
    println!();
    println!("{}", "Linthis Doctor".bold().cyan());
    println!("{}", "═".repeat(50));
    println!();

    // Group tools by language
    println!("{}", "Tool Availability:".bold());
    println!();

    for lang in languages {
        let lang_name = lang.name();
        let checkers: Vec<_> = result
            .tools
            .iter()
            .filter(|t| t.language == lang_name && t.tool_type == "checker")
            .collect();
        let formatters: Vec<_> = result
            .tools
            .iter()
            .filter(|t| t.language == lang_name && t.tool_type == "formatter")
            .collect();

        println!("  {} {}:", "●".dimmed(), lang_name.bold());

        for tool in checkers.iter().chain(formatters.iter()) {
            let status_icon = if tool.available {
                "✓".green()
            } else {
                "✗".red()
            };

            let type_badge = if tool.tool_type == "checker" {
                "lint".dimmed()
            } else {
                "fmt".dimmed()
            };

            if tool.available {
                let version = tool.version.as_deref().unwrap_or("unknown");
                println!(
                    "    {} {} ({}) - {}",
                    status_icon,
                    tool.name,
                    type_badge,
                    version.dimmed()
                );
            } else {
                println!(
                    "    {} {} ({}) - {}",
                    status_icon,
                    tool.name,
                    type_badge,
                    "not found".red()
                );
                if let Some(ref hint) = tool.install_hint {
                    println!("      {}: {}", "Install".yellow(), hint);
                }
            }
        }
        println!();
    }

    // Configuration status
    println!("{}", "Configuration:".bold());
    println!();

    for config in &result.configs {
        if !config.exists {
            println!(
                "    {} {} - {}",
                "○".dimmed(),
                config.path,
                "not found (optional)".dimmed()
            );
        } else if config.valid {
            println!("    {} {} - {}", "✓".green(), config.path, "valid".green());
        } else {
            println!("    {} {} - {}", "✗".red(), config.path, "invalid".red());
            if let Some(ref error) = config.error {
                println!("      {}: {}", "Error".red(), error);
            }
        }
    }

    println!();
    println!("{}", "═".repeat(50));

    if result.all_passed {
        println!("{}", "All checks passed!".green().bold());
    } else {
        let missing_tools = result.tools.iter().filter(|t| !t.available).count();
        let invalid_configs = result
            .configs
            .iter()
            .filter(|c| c.exists && !c.valid)
            .count();

        if missing_tools > 0 {
            println!(
                "{} {} tool(s) not available",
                "⚠".yellow(),
                missing_tools
            );
        }
        if invalid_configs > 0 {
            println!(
                "{} {} config file(s) invalid",
                "⚠".yellow(),
                invalid_configs
            );
        }
    }

    println!();
}
