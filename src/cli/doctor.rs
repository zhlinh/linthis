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
            println!(
                "{}",
                serde_json::to_string_pretty(&result).unwrap_or_default()
            );
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

/// Static tool info entry: (name, command, version_arg)
struct ToolInfoEntry {
    name: &'static str,
    cmd: &'static str,
    version_arg: &'static str,
}

/// Get checker tool info for a language
fn get_checker_info(lang: Language) -> ToolInfoEntry {
    match lang {
        Language::Rust => ToolInfoEntry { name: "clippy", cmd: "cargo", version_arg: "clippy --version" },
        Language::Python => ToolInfoEntry { name: "ruff", cmd: "ruff", version_arg: "--version" },
        Language::Go => ToolInfoEntry { name: "golangci-lint", cmd: "golangci-lint", version_arg: "--version" },
        Language::TypeScript | Language::JavaScript => ToolInfoEntry { name: "eslint", cmd: "eslint", version_arg: "--version" },
        Language::Java => ToolInfoEntry { name: "checkstyle", cmd: "checkstyle", version_arg: "--version" },
        Language::Cpp | Language::ObjectiveC => ToolInfoEntry { name: "cpplint", cmd: "cpplint", version_arg: "--version" },
        Language::Dart => ToolInfoEntry { name: "dart-analyze", cmd: "dart", version_arg: "--version" },
        Language::Swift => ToolInfoEntry { name: "swiftlint", cmd: "swiftlint", version_arg: "version" },
        Language::Kotlin => ToolInfoEntry { name: "ktlint", cmd: "ktlint", version_arg: "--version" },
        Language::Lua => ToolInfoEntry { name: "luacheck", cmd: "luacheck", version_arg: "--version" },
        Language::Shell => ToolInfoEntry { name: "shellcheck", cmd: "shellcheck", version_arg: "--version" },
        Language::Ruby => ToolInfoEntry { name: "rubocop", cmd: "rubocop", version_arg: "--version" },
        Language::Php => ToolInfoEntry { name: "phpcs", cmd: "phpcs", version_arg: "--version" },
        Language::Scala => ToolInfoEntry { name: "scalafix", cmd: "scalafix", version_arg: "--version" },
        Language::CSharp => ToolInfoEntry { name: "dotnet-format", cmd: "dotnet", version_arg: "format --version" },
    }
}

/// Get formatter tool info for a language
fn get_formatter_info(lang: Language) -> ToolInfoEntry {
    match lang {
        Language::Rust => ToolInfoEntry { name: "rustfmt", cmd: "rustfmt", version_arg: "--version" },
        Language::Python => ToolInfoEntry { name: "ruff", cmd: "ruff", version_arg: "--version" },
        Language::Go => ToolInfoEntry { name: "gofmt", cmd: "gofmt", version_arg: "-h" },
        Language::TypeScript | Language::JavaScript => ToolInfoEntry { name: "prettier", cmd: "prettier", version_arg: "--version" },
        Language::Java => ToolInfoEntry { name: "google-java-format", cmd: "google-java-format", version_arg: "--version" },
        Language::Cpp | Language::ObjectiveC => ToolInfoEntry { name: "clang-format", cmd: "clang-format", version_arg: "--version" },
        Language::Dart => ToolInfoEntry { name: "dart-format", cmd: "dart", version_arg: "--version" },
        Language::Swift => ToolInfoEntry { name: "swift-format", cmd: "swift-format", version_arg: "--version" },
        Language::Kotlin => ToolInfoEntry { name: "ktlint", cmd: "ktlint", version_arg: "--version" },
        Language::Lua => ToolInfoEntry { name: "stylua", cmd: "stylua", version_arg: "--version" },
        Language::Shell => ToolInfoEntry { name: "shfmt", cmd: "shfmt", version_arg: "--version" },
        Language::Ruby => ToolInfoEntry { name: "rubocop", cmd: "rubocop", version_arg: "--version" },
        Language::Php => ToolInfoEntry { name: "php-cs-fixer", cmd: "php-cs-fixer", version_arg: "--version" },
        Language::Scala => ToolInfoEntry { name: "scalafmt", cmd: "scalafmt", version_arg: "--version" },
        Language::CSharp => ToolInfoEntry { name: "dotnet-format", cmd: "dotnet", version_arg: "format --version" },
    }
}

/// Get tool command info (name, command, version_arg)
fn get_tool_info(lang: Language, is_checker: bool) -> (String, String, &'static str) {
    let entry = if is_checker {
        get_checker_info(lang)
    } else {
        get_formatter_info(lang)
    };
    (entry.name.to_string(), entry.cmd.to_string(), entry.version_arg)
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

/// Pick an install command based on the current OS.
/// Arguments: (macos_hint, windows_hint, linux_hint).
/// Pass `None` for platforms that share the same fallback.
fn platform_hint(macos: &str, windows: Option<&str>, linux: &str) -> String {
    if cfg!(target_os = "macos") {
        macos.to_string()
    } else if cfg!(target_os = "windows") {
        windows.unwrap_or(linux).to_string()
    } else {
        linux.to_string()
    }
}

/// Generate a Python tool install command, preferring uv > pipx > pip (without "Install: " prefix).
fn python_tool_install_cmd(tool: &str) -> String {
    if linthis::is_command_available("uv") {
        format!("uv tool install {}", tool)
    } else if linthis::is_command_available("pipx") {
        format!("pipx install {}", tool)
    } else {
        format!("pip install {}", tool)
    }
}

/// Get installation hint for a checker tool
fn get_checker_install_hint(lang: Language) -> String {
    match lang {
        Language::Rust => "rustup component add clippy".to_string(),
        Language::Python => python_tool_install_cmd("ruff"),
        Language::Go => platform_hint("brew install golangci-lint", None, "go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest"),
        Language::TypeScript | Language::JavaScript => "npm install -g eslint".to_string(),
        Language::Java => platform_hint("brew install checkstyle", None, "https://checkstyle.sourceforge.io/"),
        Language::Cpp | Language::ObjectiveC => python_tool_install_cmd("cpplint"),
        Language::Dart => "https://dart.dev/get-dart".to_string(),
        Language::Swift => platform_hint("brew install swiftlint", None, "https://github.com/realm/SwiftLint"),
        Language::Kotlin => platform_hint("brew install ktlint", None, "https://github.com/pinterest/ktlint"),
        // macOS's Homebrew ships a ready-to-run luacheck; the luarocks
        // path fails on Lua 5.5 default + missing argparse rockspec.
        Language::Lua => platform_hint(
            "brew install luacheck",
            None,
            "luarocks install luacheck",
        ),
        Language::Shell => platform_hint("brew install shellcheck", Some("choco install shellcheck"), "apt install shellcheck"),
        Language::Ruby => "gem install rubocop".to_string(),
        Language::Php => "composer global require squizlabs/php_codesniffer".to_string(),
        Language::Scala => "cs install scalafix".to_string(),
        Language::CSharp => "dotnet tool install -g dotnet-format".to_string(),
    }
}

/// Get installation hint for a formatter tool
fn get_formatter_install_hint(lang: Language) -> String {
    match lang {
        Language::Rust => "rustup component add rustfmt".to_string(),
        Language::Python => python_tool_install_cmd("ruff"),
        Language::Go => "Included with Go installation".to_string(),
        Language::TypeScript | Language::JavaScript => "npm install -g prettier".to_string(),
        Language::Java => platform_hint("brew install google-java-format", None, "https://github.com/google/google-java-format/releases"),
        Language::Cpp | Language::ObjectiveC => platform_hint("brew install clang-format", Some("https://releases.llvm.org/download.html"), "apt install clang-format"),
        Language::Dart => "https://dart.dev/get-dart".to_string(),
        Language::Swift => platform_hint("brew install swift-format", None, "https://github.com/apple/swift-format"),
        Language::Kotlin => platform_hint("brew install ktlint", None, "https://github.com/pinterest/ktlint"),
        Language::Lua => "cargo install stylua".to_string(),
        Language::Shell => platform_hint("brew install shfmt", Some("choco install shfmt"), "go install mvdan.cc/sh/v3/cmd/shfmt@latest"),
        Language::Ruby => "gem install rubocop".to_string(),
        Language::Php => "composer global require friendsofphp/php-cs-fixer".to_string(),
        Language::Scala => "cs install scalafmt".to_string(),
        Language::CSharp => "dotnet tool install -g dotnet-format".to_string(),
    }
}

/// Get installation hint for a tool
fn get_install_hint(lang: Language, is_checker: bool) -> String {
    if is_checker {
        get_checker_install_hint(lang)
    } else {
        get_formatter_install_hint(lang)
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

/// Print a single tool's status line
fn print_tool_status(tool: &ToolStatus) {
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
            status_icon, tool.name, type_badge,
            version.dimmed()
        );
    } else {
        println!(
            "    {} {} ({}) - {}",
            status_icon, tool.name, type_badge,
            "not found".red()
        );
        if let Some(ref hint) = tool.install_hint {
            println!("      {}: {}", "Install".yellow(), hint);
        }
    }
}

/// Print tool availability grouped by language
fn print_tools_section(result: &DoctorResult, languages: &[Language]) {
    println!("{}", "Tool Availability:".bold());
    println!();

    for lang in languages {
        let lang_name = lang.name();
        println!("  {} {}:", "●".dimmed(), lang_name.bold());

        for tool in result.tools.iter().filter(|t| t.language == lang_name) {
            print_tool_status(tool);
        }
        println!();
    }
}

/// Print configuration file status section
fn print_configs_section(configs: &[ConfigStatus]) {
    println!("{}", "Configuration:".bold());
    println!();

    for config in configs {
        if !config.exists {
            println!(
                "    {} {} - {}",
                "○".dimmed(), config.path,
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
}

/// Print the summary footer
fn print_doctor_summary(result: &DoctorResult) {
    if result.all_passed {
        println!("{}", "All checks passed!".green().bold());
    } else {
        let missing_tools = result.tools.iter().filter(|t| !t.available).count();
        let invalid_configs = result.configs.iter().filter(|c| c.exists && !c.valid).count();

        if missing_tools > 0 {
            println!("{} {} tool(s) not available", "⚠".yellow(), missing_tools);
        }
        if invalid_configs > 0 {
            println!("{} {} config file(s) invalid", "⚠".yellow(), invalid_configs);
        }
    }
}

/// Print human-readable output
fn print_human_output(result: &DoctorResult, languages: &[Language]) {
    println!();
    println!("{}", "Linthis Doctor".bold().cyan());
    println!("{}", "═".repeat(50));
    println!();

    print_tools_section(result, languages);
    print_configs_section(&result.configs);

    println!();
    println!("{}", "═".repeat(50));

    print_doctor_summary(result);

    println!();
}
