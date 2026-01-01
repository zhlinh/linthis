// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! C/C++ language checker using clang-tidy or cpplint.

use crate::checkers::Checker;
use crate::utils::types::{LintIssue, Severity};
use crate::{Language, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// C/C++ checker using clang-tidy (preferred) or cpplint.
pub struct CppChecker {
    /// Custom .clang-tidy config path
    config_path: Option<PathBuf>,
    /// Custom compile_commands.json directory path
    compile_commands_dir: Option<PathBuf>,
}

impl CppChecker {
    pub fn new() -> Self {
        Self {
            config_path: None,
            compile_commands_dir: None,
        }
    }

    /// Set custom .clang-tidy config path
    pub fn with_config(mut self, path: PathBuf) -> Self {
        self.config_path = Some(path);
        self
    }

    /// Set custom compile_commands.json directory path
    /// This is the directory containing compile_commands.json, not the file itself
    pub fn with_compile_commands_dir(mut self, path: PathBuf) -> Self {
        self.compile_commands_dir = Some(path);
        self
    }

    /// Find .clang-tidy config file by walking up from file path
    fn find_clang_tidy_config(start_path: &Path) -> Option<PathBuf> {
        let mut current = if start_path.is_file() {
            start_path.parent()?.to_path_buf()
        } else {
            start_path.to_path_buf()
        };

        loop {
            let config_path = current.join(".clang-tidy");
            if config_path.exists() {
                return Some(config_path);
            }

            // Also check for _clang-tidy (alternative name)
            let alt_config = current.join("_clang-tidy");
            if alt_config.exists() {
                return Some(alt_config);
            }

            if !current.pop() {
                break;
            }
        }
        None
    }

    /// Find compile_commands.json for better analysis
    /// Searches in common build directories recursively (up to max_depth levels)
    fn find_compile_commands(start_path: &Path) -> Option<PathBuf> {
        let mut current = if start_path.is_file() {
            start_path.parent()?.to_path_buf()
        } else {
            start_path.to_path_buf()
        };

        loop {
            // 1. Check in current directory directly
            let direct = current.join("compile_commands.json");
            if direct.exists() {
                return Some(current.clone());
            }

            // 2. Check common fixed build directory names (1 level)
            for build_dir in &[
                "build",
                "Build",
                "out",
                "output",
                "cmake-build-debug",
                "cmake-build-release",
                "cmake-build-relwithdebinfo",
                "cmake-build-minsizerel",
                ".build",
                "_build",
            ] {
                let compile_db = current.join(build_dir).join("compile_commands.json");
                if compile_db.exists() {
                    return Some(current.join(build_dir));
                }
            }

            // 3. Recursively search in directories matching build patterns (up to 6 levels deep)
            if let Some(found) = Self::find_compile_commands_recursive(&current, 0, 6) {
                return Some(found);
            }

            if !current.pop() {
                break;
            }
        }
        None
    }

    /// Recursively search for compile_commands.json in build-like directories
    fn find_compile_commands_recursive(dir: &Path, depth: usize, max_depth: usize) -> Option<PathBuf> {
        if depth >= max_depth {
            return None;
        }

        let entries = std::fs::read_dir(dir).ok()?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let name = path.file_name().and_then(|n| n.to_str())?;
            let name_lower = name.to_lowercase();

            // Only recurse into build-related directories
            let is_build_dir = name_lower.starts_with("cmake")
                || name_lower.starts_with("build")
                || name_lower.starts_with("out")
                || name_lower.ends_with("-build")
                || name_lower.ends_with("_build")
                // Also allow platform/arch subdirectories inside build dirs
                || (depth > 0
                    && (name_lower.contains("android")
                        || name_lower.contains("ios")
                        || name_lower.contains("linux")
                        || name_lower.contains("windows")
                        || name_lower.contains("macos")
                        || name_lower.contains("darwin")
                        || name_lower.contains("arm")
                        || name_lower.contains("x86")
                        || name_lower.contains("x64")
                        || name_lower.contains("static")
                        || name_lower.contains("shared")
                        || name_lower.contains("debug")
                        || name_lower.contains("release")));

            if is_build_dir {
                // Check if compile_commands.json exists here
                let compile_db = path.join("compile_commands.json");
                if compile_db.exists() {
                    return Some(path);
                }

                // Recurse deeper
                if let Some(found) = Self::find_compile_commands_recursive(&path, depth + 1, max_depth) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// Check if clang-tidy is available
    fn has_clang_tidy() -> bool {
        Command::new("clang-tidy")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Check if cpplint is available
    fn has_cpplint() -> bool {
        Command::new("cpplint")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Run clang-tidy on a file (check only, no fix)
    fn run_clang_tidy(&self, path: &Path) -> Result<Vec<LintIssue>> {
        let mut cmd = Command::new("clang-tidy");
        cmd.arg(path);

        // Add config file if specified or found
        if let Some(ref config) = self.config_path {
            cmd.arg(format!("--config-file={}", config.display()));
        } else if let Some(config) = Self::find_clang_tidy_config(path) {
            cmd.arg(format!("--config-file={}", config.display()));
        }

        // Add compile_commands.json path: user-specified > auto-detected
        if let Some(ref build_path) = self.compile_commands_dir {
            cmd.arg(format!("-p={}", build_path.display()));
        } else if let Some(build_path) = Self::find_compile_commands(path) {
            cmd.arg(format!("-p={}", build_path.display()));
        } else {
            // Use -- to separate clang-tidy args from compiler args
            cmd.arg("--");
        }

        let output = cmd
            .output()
            .map_err(|e| crate::LintisError::Checker(format!("Failed to run clang-tidy: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let issues = Self::parse_clang_tidy_output(&stdout, path);

        Ok(issues)
    }

    /// Run cpplint on a file
    fn run_cpplint(path: &Path) -> Result<Vec<LintIssue>> {
        let output = Command::new("cpplint")
            .arg(path)
            .output()
            .map_err(|e| crate::LintisError::Checker(format!("Failed to run cpplint: {}", e)))?;

        // cpplint outputs to stderr
        let stderr = String::from_utf8_lossy(&output.stderr);
        let issues = Self::parse_cpplint_output(&stderr, path);

        Ok(issues)
    }

    /// Parse clang-tidy output
    /// Format: file:line:col: severity: message [check-name]
    fn parse_clang_tidy_output(output: &str, file_path: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        for line in output.lines() {
            if let Some(issue) = Self::parse_clang_tidy_line(line, file_path) {
                issues.push(issue);
            }
        }

        issues
    }

    fn parse_clang_tidy_line(line: &str, default_path: &Path) -> Option<LintIssue> {
        // clang-tidy format: file:line:col: warning/error: message [check-name]
        // Example: test.cpp:10:5: warning: use nullptr [modernize-use-nullptr]
        if !line.contains(": warning:") && !line.contains(": error:") {
            return None;
        }

        let parts: Vec<&str> = line.splitn(5, ':').collect();
        if parts.len() < 5 {
            return None;
        }

        let file_path_parsed = std::path::PathBuf::from(parts[0]);
        let line_num = parts[1].trim().parse::<usize>().ok()?;
        let col = parts[2].trim().parse::<usize>().ok();

        let severity_str = parts[3].trim();
        let message_part = parts[4].trim();

        let severity = if severity_str.contains("error") {
            Severity::Error
        } else {
            Severity::Warning
        };

        // Extract message and check name
        let (message, code) = if let Some(bracket_start) = message_part.rfind('[') {
            let msg = message_part[..bracket_start].trim();
            let check = message_part[bracket_start..]
                .trim_matches(|c| c == '[' || c == ']')
                .to_string();
            (msg.to_string(), Some(check))
        } else {
            (message_part.to_string(), None)
        };

        let mut issue = LintIssue::new(
            if file_path_parsed.exists() {
                file_path_parsed
            } else {
                default_path.to_path_buf()
            },
            line_num,
            message,
            severity,
        )
        .with_source("clang-tidy".to_string());

        if let Some(c) = col {
            issue = issue.with_column(c);
        }
        if let Some(c) = code {
            issue = issue.with_code(c);
        }

        Some(issue)
    }

    /// Parse cpplint output and extract issues.
    /// Format: file:line: message [category] [confidence]
    fn parse_cpplint_output(output: &str, file_path: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        for line in output.lines() {
            if let Some(issue) = Self::parse_cpplint_line(line, file_path) {
                issues.push(issue);
            }
        }

        issues
    }

    fn parse_cpplint_line(line: &str, default_path: &Path) -> Option<LintIssue> {
        // cpplint format: file:line: message [category] [confidence]
        // Example: test.cpp:10: Missing space after comma [whitespace/comma] [3]
        if !line.contains(':')
            || line.starts_with("Done processing")
            || line.starts_with("Total errors")
        {
            return None;
        }

        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() < 3 {
            return None;
        }

        let file_path_parsed = std::path::PathBuf::from(parts[0]);
        let line_num = parts[1].trim().parse::<usize>().ok()?;
        let rest = parts[2].trim();

        // Parse message and extract category
        let (message, code) = if let Some(bracket_start) = rest.find('[') {
            let msg = rest[..bracket_start].trim();
            let category = rest[bracket_start..].trim_matches(|c| c == '[' || c == ']');
            // Extract just the first bracketed category
            let cat = category.split("] [").next().unwrap_or(category);
            (msg.to_string(), Some(cat.to_string()))
        } else {
            (rest.to_string(), None)
        };

        let severity = if message.to_lowercase().contains("error") {
            Severity::Error
        } else {
            Severity::Warning
        };

        let mut issue = LintIssue::new(
            if file_path_parsed.exists() {
                file_path_parsed
            } else {
                default_path.to_path_buf()
            },
            line_num,
            message,
            severity,
        )
        .with_source("cpplint".to_string());

        if let Some(c) = code {
            issue = issue.with_code(c);
        }

        Some(issue)
    }
}

impl Default for CppChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for CppChecker {
    fn name(&self) -> &str {
        if Self::has_clang_tidy() {
            "clang-tidy"
        } else {
            "cpplint"
        }
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Cpp, Language::ObjectiveC]
    }

    fn check(&self, path: &Path) -> Result<Vec<LintIssue>> {
        // Prefer clang-tidy if available, fall back to cpplint
        if Self::has_clang_tidy() {
            self.run_clang_tidy(path)
        } else if Self::has_cpplint() {
            Self::run_cpplint(path)
        } else {
            // Neither tool available
            Ok(Vec::new())
        }
    }

    fn is_available(&self) -> bool {
        Self::has_clang_tidy() || Self::has_cpplint()
    }
}
