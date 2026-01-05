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

/// Cpplint configuration for different languages
#[derive(Debug, Clone)]
pub struct CpplintConfig {
    /// Line length limit
    pub linelength: Option<u32>,
    /// Filter rules (e.g., "-build/c++11,-build/header_guard")
    pub filter: Option<String>,
}

impl Default for CpplintConfig {
    fn default() -> Self {
        Self {
            linelength: None,
            filter: None,
        }
    }
}

/// C/C++ checker using clang-tidy (preferred) or cpplint.
pub struct CppChecker {
    /// Custom .clang-tidy config path
    config_path: Option<PathBuf>,
    /// Custom compile_commands.json directory path
    compile_commands_dir: Option<PathBuf>,
    /// Cpplint config for C++ files
    cpplint_cpp_config: CpplintConfig,
    /// Cpplint config for Objective-C files
    cpplint_oc_config: CpplintConfig,
}

impl CppChecker {
    pub fn new() -> Self {
        // Try to load cpplint config from linthis config files
        let (cpp_config, oc_config) = Self::load_cpplint_configs();

        Self {
            config_path: None,
            compile_commands_dir: None,
            cpplint_cpp_config: cpp_config,
            cpplint_oc_config: oc_config,
        }
    }

    /// Load cpplint configs from linthis configuration
    fn load_cpplint_configs() -> (CpplintConfig, CpplintConfig) {
        use crate::config::Config;

        // Default configs with sensible defaults for each language
        let mut cpp_config = CpplintConfig {
            linelength: Some(120),
            filter: Some("-build/c++11,-build/c++14".to_string()),
        };

        // OC default: disable checks not applicable to Objective-C
        // -whitespace/parens: OC uses @property (attr) syntax which has space before (
        // -whitespace/operators: OC uses getter=xxx syntax which cpplint misinterprets
        let mut oc_config = CpplintConfig {
            linelength: Some(150),
            filter: Some("-build/c++11,-build/c++14,-build/header_guard,-build/include,-legal/copyright,-readability/casting,-runtime/references,-runtime/int,-whitespace/parens,-whitespace/braces,-whitespace/blank_line,-readability/braces,-whitespace/empty_if_body,-whitespace/operators".to_string()),
        };

        let project_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Load from .linthis/configs/{lang}/CPPLINT.cfg (plugin configs)
        // Merge filters instead of replacing to preserve essential OC defaults
        let cpp_cfg_path = project_dir.join(".linthis/configs/cpp/CPPLINT.cfg");
        let oc_cfg_path = project_dir.join(".linthis/configs/oc/CPPLINT.cfg");

        if let Some(cfg) = Self::parse_cpplint_cfg(&cpp_cfg_path) {
            if cfg.linelength.is_some() {
                cpp_config.linelength = cfg.linelength;
            }
            if let Some(ref f) = cfg.filter {
                cpp_config.filter = Some(Self::merge_filters(cpp_config.filter.as_deref(), f));
            }
        }
        if let Some(cfg) = Self::parse_cpplint_cfg(&oc_cfg_path) {
            if cfg.linelength.is_some() {
                oc_config.linelength = cfg.linelength;
            }
            if let Some(ref f) = cfg.filter {
                oc_config.filter = Some(Self::merge_filters(oc_config.filter.as_deref(), f));
            }
        }

        // Priority 2: Override with config.toml settings (if specified)
        let merged = Config::load_merged(&project_dir);

        if let Some(ref cpp) = merged.language_overrides.cpp {
            if cpp.linelength.is_some() {
                cpp_config.linelength = cpp.linelength;
            }
            if cpp.cpplint_filter.is_some() {
                cpp_config.filter = cpp.cpplint_filter.clone();
            }
        }

        if let Some(ref oc) = merged.language_overrides.oc {
            if oc.linelength.is_some() {
                oc_config.linelength = oc.linelength;
            }
            if oc.cpplint_filter.is_some() {
                oc_config.filter = oc.cpplint_filter.clone();
            }
        }

        (cpp_config, oc_config)
    }

    /// Merge two filter strings, removing duplicates
    fn merge_filters(base: Option<&str>, additional: &str) -> String {
        use std::collections::HashSet;

        let mut filters: HashSet<&str> = HashSet::new();

        // Add base filters
        if let Some(base) = base {
            for f in base.split(',') {
                let f = f.trim();
                if !f.is_empty() {
                    filters.insert(f);
                }
            }
        }

        // Add additional filters
        for f in additional.split(',') {
            let f = f.trim();
            if !f.is_empty() {
                filters.insert(f);
            }
        }

        filters.into_iter().collect::<Vec<_>>().join(",")
    }

    /// Parse CPPLINT.cfg file and extract linelength and filter settings
    fn parse_cpplint_cfg(path: &Path) -> Option<CpplintConfig> {
        let content = std::fs::read_to_string(path).ok()?;

        let mut linelength = None;
        let mut filters = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(value) = line.strip_prefix("linelength=") {
                linelength = value.trim().parse().ok();
            } else if let Some(value) = line.strip_prefix("filter=") {
                filters.push(value.trim().to_string());
            }
        }

        // Combine all filter lines into one comma-separated string
        let filter = if filters.is_empty() {
            None
        } else {
            Some(filters.join(","))
        };

        Some(CpplintConfig { linelength, filter })
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

    /// Set cpplint config for C++ files
    pub fn with_cpplint_cpp_config(mut self, config: CpplintConfig) -> Self {
        self.cpplint_cpp_config = config;
        self
    }

    /// Set cpplint config for Objective-C files
    pub fn with_cpplint_oc_config(mut self, config: CpplintConfig) -> Self {
        self.cpplint_oc_config = config;
        self
    }

    /// Detect if a file is Objective-C (uses smart detection from Language)
    fn is_objective_c(path: &Path) -> bool {
        // Use the same smart detection logic as Language::from_path
        crate::Language::from_path(path)
            .map(|lang| lang == crate::Language::ObjectiveC)
            .unwrap_or(false)
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
    fn find_compile_commands_recursive(
        dir: &Path,
        depth: usize,
        max_depth: usize,
    ) -> Option<PathBuf> {
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
                if let Some(found) =
                    Self::find_compile_commands_recursive(&path, depth + 1, max_depth)
                {
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

    /// Run cpplint on a file with language-specific config
    fn run_cpplint(&self, path: &Path) -> Result<Vec<LintIssue>> {
        let mut cmd = Command::new("cpplint");

        // Select config based on file type (Objective-C vs C++)
        let is_oc = Self::is_objective_c(path);
        let config = if is_oc {
            &self.cpplint_oc_config
        } else {
            &self.cpplint_cpp_config
        };

        // Add extensions for Objective-C files (cpplint doesn't recognize .m/.mm by default)
        if is_oc {
            cmd.arg("--extensions=m,mm,h");
        }

        // Apply linelength if configured
        if let Some(linelength) = config.linelength {
            cmd.arg(format!("--linelength={}", linelength));
        }

        // Apply filter if configured
        if let Some(ref filter) = config.filter {
            cmd.arg(format!("--filter={}", filter));
        }

        cmd.arg(path);

        let output = cmd
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

        // Filter out all clang-diagnostic-* errors: these are compiler diagnostics
        // (missing headers, type errors, etc.) not actual code style/quality issues
        if let Some(ref c) = code {
            if c.starts_with("clang-diagnostic-") {
                return None;
            }
        }

        let file_path = if file_path_parsed.exists() {
            file_path_parsed
        } else {
            default_path.to_path_buf()
        };

        let mut issue = LintIssue::new(file_path.clone(), line_num, message, severity)
            .with_source("clang-tidy".to_string());

        if let Some(c) = col {
            issue = issue.with_column(c);
        }
        if let Some(c) = code {
            issue = issue.with_code(c);
        }

        // Read the source code line
        if let Some(code_line) = crate::utils::read_file_line(&file_path, line_num) {
            issue = issue.with_code_line(code_line);
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

        let file_path = if file_path_parsed.exists() {
            file_path_parsed
        } else {
            default_path.to_path_buf()
        };

        let mut issue =
            LintIssue::new(file_path.clone(), line_num, message, severity)
                .with_source("cpplint".to_string());

        if let Some(c) = code {
            issue = issue.with_code(c);
        }

        // Read the source code line
        if let Some(code_line) = crate::utils::read_file_line(&file_path, line_num) {
            issue = issue.with_code_line(code_line);
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
            self.run_cpplint(path)
        } else {
            // Neither tool available
            Ok(Vec::new())
        }
    }

    fn is_available(&self) -> bool {
        Self::has_clang_tidy() || Self::has_cpplint()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // ==================== merge_filters tests ====================

    #[test]
    fn test_merge_filters_both_present() {
        let result =
            CppChecker::merge_filters(Some("-build/c++11,-build/c++14"), "-whitespace/tab");
        // Result should contain all three filters
        assert!(result.contains("-build/c++11"));
        assert!(result.contains("-build/c++14"));
        assert!(result.contains("-whitespace/tab"));
    }

    #[test]
    fn test_merge_filters_base_none() {
        let result = CppChecker::merge_filters(None, "-build/c++11,-whitespace/tab");
        assert!(result.contains("-build/c++11"));
        assert!(result.contains("-whitespace/tab"));
    }

    #[test]
    fn test_merge_filters_removes_duplicates() {
        let result =
            CppChecker::merge_filters(Some("-build/c++11"), "-build/c++11,-whitespace/tab");
        // Should not have duplicate -build/c++11
        let count = result.matches("-build/c++11").count();
        assert_eq!(count, 1);
        assert!(result.contains("-whitespace/tab"));
    }

    #[test]
    fn test_merge_filters_trims_whitespace() {
        let result =
            CppChecker::merge_filters(Some(" -build/c++11 , -build/c++14 "), " -whitespace/tab ");
        assert!(result.contains("-build/c++11"));
        assert!(result.contains("-build/c++14"));
        assert!(result.contains("-whitespace/tab"));
    }

    #[test]
    fn test_merge_filters_empty_strings() {
        let result = CppChecker::merge_filters(Some(""), "");
        assert!(result.is_empty());
    }

    // ==================== parse_cpplint_cfg tests ====================

    fn create_temp_cfg(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_parse_cpplint_cfg_linelength() {
        let file = create_temp_cfg("linelength=120\n");
        let config = CppChecker::parse_cpplint_cfg(file.path()).unwrap();
        assert_eq!(config.linelength, Some(120));
        assert!(config.filter.is_none());
    }

    #[test]
    fn test_parse_cpplint_cfg_filter() {
        let file = create_temp_cfg("filter=-build/c++11,-whitespace/tab\n");
        let config = CppChecker::parse_cpplint_cfg(file.path()).unwrap();
        assert!(config.linelength.is_none());
        assert_eq!(
            config.filter,
            Some("-build/c++11,-whitespace/tab".to_string())
        );
    }

    #[test]
    fn test_parse_cpplint_cfg_both() {
        let file = create_temp_cfg("linelength=100\nfilter=-build/header_guard\n");
        let config = CppChecker::parse_cpplint_cfg(file.path()).unwrap();
        assert_eq!(config.linelength, Some(100));
        assert_eq!(config.filter, Some("-build/header_guard".to_string()));
    }

    #[test]
    fn test_parse_cpplint_cfg_multiple_filters() {
        let file = create_temp_cfg("filter=-build/c++11\nfilter=-whitespace/tab\n");
        let config = CppChecker::parse_cpplint_cfg(file.path()).unwrap();
        // Multiple filter lines should be joined
        let filter = config.filter.unwrap();
        assert!(filter.contains("-build/c++11"));
        assert!(filter.contains("-whitespace/tab"));
    }

    #[test]
    fn test_parse_cpplint_cfg_with_comments() {
        let file = create_temp_cfg("# This is a comment\nlinelength=80\n# Another comment\n");
        let config = CppChecker::parse_cpplint_cfg(file.path()).unwrap();
        assert_eq!(config.linelength, Some(80));
    }

    #[test]
    fn test_parse_cpplint_cfg_empty_lines() {
        let file = create_temp_cfg("\n\nlinelength=150\n\n");
        let config = CppChecker::parse_cpplint_cfg(file.path()).unwrap();
        assert_eq!(config.linelength, Some(150));
    }

    #[test]
    fn test_parse_cpplint_cfg_nonexistent_file() {
        let result = CppChecker::parse_cpplint_cfg(Path::new("/nonexistent/path/CPPLINT.cfg"));
        assert!(result.is_none());
    }

    // ==================== parse_clang_tidy_line tests ====================

    #[test]
    fn test_parse_clang_tidy_warning() {
        let line = "test.cpp:10:5: warning: use nullptr [modernize-use-nullptr]";
        let default_path = Path::new("default.cpp");
        let issue = CppChecker::parse_clang_tidy_line(line, default_path).unwrap();

        assert_eq!(issue.line, 10);
        assert_eq!(issue.column, Some(5));
        assert_eq!(issue.severity, Severity::Warning);
        assert!(issue.message.contains("use nullptr"));
        assert_eq!(issue.code, Some("modernize-use-nullptr".to_string()));
        assert_eq!(issue.source, Some("clang-tidy".to_string()));
    }

    #[test]
    fn test_parse_clang_tidy_error() {
        // Use a non-clang-diagnostic error (clang-diagnostic-* are filtered out)
        let line = "main.cpp:5:1: error: no matching function for call [misc-error]";
        let default_path = Path::new("default.cpp");
        let issue = CppChecker::parse_clang_tidy_line(line, default_path).unwrap();

        assert_eq!(issue.line, 5);
        assert_eq!(issue.column, Some(1));
        assert_eq!(issue.severity, Severity::Error);
        assert!(issue.message.contains("no matching function"));
        assert_eq!(issue.code, Some("misc-error".to_string()));
    }

    #[test]
    fn test_parse_clang_tidy_clang_diagnostic_filtered() {
        // clang-diagnostic-* errors are compiler diagnostics, should be filtered out
        let line = "main.cpp:5:1: error: unknown type name 'foo' [clang-diagnostic-error]";
        let default_path = Path::new("default.cpp");
        let result = CppChecker::parse_clang_tidy_line(line, default_path);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_clang_tidy_no_bracket() {
        let line = "test.cpp:20:3: warning: some warning without bracket";
        let default_path = Path::new("default.cpp");
        let issue = CppChecker::parse_clang_tidy_line(line, default_path).unwrap();

        assert_eq!(issue.line, 20);
        assert!(issue.code.is_none());
        assert!(issue.message.contains("some warning without bracket"));
    }

    #[test]
    fn test_parse_clang_tidy_irrelevant_line() {
        let line = "In file included from test.cpp:1:";
        let default_path = Path::new("default.cpp");
        let result = CppChecker::parse_clang_tidy_line(line, default_path);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_clang_tidy_note_line() {
        let line = "test.cpp:10:5: note: previous declaration is here";
        let default_path = Path::new("default.cpp");
        let result = CppChecker::parse_clang_tidy_line(line, default_path);
        assert!(result.is_none()); // notes are not warnings or errors
    }

    // ==================== parse_cpplint_line tests ====================

    #[test]
    fn test_parse_cpplint_standard_warning() {
        let line = "test.cpp:10: Missing space after comma [whitespace/comma] [3]";
        let default_path = Path::new("default.cpp");
        let issue = CppChecker::parse_cpplint_line(line, default_path).unwrap();

        assert_eq!(issue.line, 10);
        assert_eq!(issue.severity, Severity::Warning);
        assert!(issue.message.contains("Missing space after comma"));
        assert_eq!(issue.code, Some("whitespace/comma".to_string()));
        assert_eq!(issue.source, Some("cpplint".to_string()));
    }

    #[test]
    fn test_parse_cpplint_header_guard() {
        let line = "test.h:0: No #ifndef header guard found, suggested CPP variable is: TEST_H_ [build/header_guard] [5]";
        let default_path = Path::new("test.h");
        let issue = CppChecker::parse_cpplint_line(line, default_path).unwrap();

        assert_eq!(issue.line, 0);
        assert!(issue.message.contains("header guard"));
        assert_eq!(issue.code, Some("build/header_guard".to_string()));
    }

    #[test]
    fn test_parse_cpplint_endif_comment() {
        let line =
            r##"test.h:50: #endif line should be "#endif  // TEST_H_" [build/header_guard] [5]"##;
        let default_path = Path::new("test.h");
        let issue = CppChecker::parse_cpplint_line(line, default_path).unwrap();

        assert_eq!(issue.line, 50);
        assert!(issue.message.contains("#endif"));
        assert_eq!(issue.code, Some("build/header_guard".to_string()));
    }

    #[test]
    fn test_parse_cpplint_line_length() {
        let line =
            "main.cpp:25: Lines should be <= 120 characters long [whitespace/line_length] [2]";
        let default_path = Path::new("main.cpp");
        let issue = CppChecker::parse_cpplint_line(line, default_path).unwrap();

        assert_eq!(issue.line, 25);
        assert!(issue.message.contains("120 characters"));
        assert_eq!(issue.code, Some("whitespace/line_length".to_string()));
    }

    #[test]
    fn test_parse_cpplint_done_processing() {
        let line = "Done processing test.cpp";
        let default_path = Path::new("test.cpp");
        let result = CppChecker::parse_cpplint_line(line, default_path);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_cpplint_total_errors() {
        let line = "Total errors found: 5";
        let default_path = Path::new("test.cpp");
        let result = CppChecker::parse_cpplint_line(line, default_path);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_cpplint_comment_spacing() {
        let line =
            "test.cpp:15: Should have a space between // and comment [whitespace/comments] [4]";
        let default_path = Path::new("test.cpp");
        let issue = CppChecker::parse_cpplint_line(line, default_path).unwrap();

        assert_eq!(issue.line, 15);
        assert!(issue.message.contains("space between //"));
        assert_eq!(issue.code, Some("whitespace/comments".to_string()));
    }

    // ==================== CpplintConfig tests ====================

    #[test]
    fn test_cpplint_config_default() {
        let config = CpplintConfig::default();
        assert!(config.linelength.is_none());
        assert!(config.filter.is_none());
    }

    // ==================== CppChecker builder tests ====================

    #[test]
    fn test_cpp_checker_with_config() {
        let checker = CppChecker::new().with_config(PathBuf::from("/custom/.clang-tidy"));
        assert_eq!(
            checker.config_path,
            Some(PathBuf::from("/custom/.clang-tidy"))
        );
    }

    #[test]
    fn test_cpp_checker_with_compile_commands_dir() {
        let checker = CppChecker::new().with_compile_commands_dir(PathBuf::from("/build"));
        assert_eq!(checker.compile_commands_dir, Some(PathBuf::from("/build")));
    }

    #[test]
    fn test_cpp_checker_with_cpplint_cpp_config() {
        let config = CpplintConfig {
            linelength: Some(80),
            filter: Some("-build/c++11".to_string()),
        };
        let checker = CppChecker::new().with_cpplint_cpp_config(config.clone());
        assert_eq!(checker.cpplint_cpp_config.linelength, Some(80));
    }

    #[test]
    fn test_cpp_checker_with_cpplint_oc_config() {
        let config = CpplintConfig {
            linelength: Some(200),
            filter: Some("-whitespace/parens".to_string()),
        };
        let checker = CppChecker::new().with_cpplint_oc_config(config);
        assert_eq!(checker.cpplint_oc_config.linelength, Some(200));
    }
}
