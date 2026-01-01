// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! C/C++ language formatter using clang-format, clang-tidy --fix, and cpplint fixer.

use crate::fixers::cpplint::{CpplintFixer, CpplintFixerConfig, HeaderGuardMode};
use crate::formatters::Formatter;
use crate::utils::types::FormatResult;
use crate::{Language, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

/// C/C++ formatter using clang-format, clang-tidy --fix, and cpplint fixer.
pub struct CppFormatter {
    /// Enable clang-tidy --fix for auto-fixing lint issues
    use_clang_tidy_fix: bool,
    /// Enable cpplint fixer for header guards, TODOs, etc.
    use_cpplint_fix: bool,
    /// Custom compile_commands.json directory path
    compile_commands_dir: Option<PathBuf>,
    /// Cpplint fixer instance (wrapped in Mutex for interior mutability)
    cpplint_fixer: Mutex<CpplintFixer>,
}

impl CppFormatter {
    pub fn new() -> Self {
        Self {
            use_clang_tidy_fix: true,  // Enable by default
            use_cpplint_fix: true,     // Enable by default
            compile_commands_dir: None,
            cpplint_fixer: Mutex::new(CpplintFixer::new()),
        }
    }

    /// Enable or disable clang-tidy --fix
    pub fn with_clang_tidy_fix(mut self, enable: bool) -> Self {
        self.use_clang_tidy_fix = enable;
        self
    }

    /// Enable or disable cpplint fixer
    pub fn with_cpplint_fix(mut self, enable: bool) -> Self {
        self.use_cpplint_fix = enable;
        self
    }

    /// Set custom compile_commands.json directory
    pub fn with_compile_commands_dir(mut self, path: PathBuf) -> Self {
        self.compile_commands_dir = Some(path);
        self
    }

    /// Configure cpplint fixer
    pub fn with_cpplint_config(self, config: CpplintFixerConfig) -> Self {
        *self.cpplint_fixer.lock().unwrap() = CpplintFixer::with_config(config);
        self
    }

    /// Set header guard mode
    pub fn with_header_guard_mode(self, mode: HeaderGuardMode) -> Self {
        {
            let mut fixer = self.cpplint_fixer.lock().unwrap();
            let config = CpplintFixerConfig {
                header_guard_mode: mode,
                ..Default::default()
            };
            *fixer = CpplintFixer::with_config(config);
        }
        self
    }

    /// Check if clang-tidy is available
    fn has_clang_tidy() -> bool {
        Command::new("clang-tidy")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
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
            if !current.pop() {
                break;
            }
        }
        None
    }

    /// Find compile_commands.json recursively
    fn find_compile_commands(start_path: &Path) -> Option<PathBuf> {
        let mut current = if start_path.is_file() {
            start_path.parent()?.to_path_buf()
        } else {
            start_path.to_path_buf()
        };

        loop {
            // Check current directory
            if current.join("compile_commands.json").exists() {
                return Some(current.clone());
            }

            // Check common build directories
            for build_dir in &["build", "Build", "out", "cmake-build-debug", "cmake-build-release"] {
                let compile_db = current.join(build_dir).join("compile_commands.json");
                if compile_db.exists() {
                    return Some(current.join(build_dir));
                }
            }

            // Recursively search build-like directories (up to 6 levels)
            if let Some(found) = Self::find_compile_commands_recursive(&current, 0, 6) {
                return Some(found);
            }

            if !current.pop() {
                break;
            }
        }
        None
    }

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

            let is_build_dir = name_lower.starts_with("cmake")
                || name_lower.starts_with("build")
                || name_lower.starts_with("out")
                || name_lower.ends_with("-build")
                || name_lower.ends_with("_build")
                || (depth > 0
                    && (name_lower.contains("android")
                        || name_lower.contains("ios")
                        || name_lower.contains("linux")
                        || name_lower.contains("windows")
                        || name_lower.contains("arm")
                        || name_lower.contains("x86")
                        || name_lower.contains("static")
                        || name_lower.contains("shared")
                        || name_lower.contains("debug")
                        || name_lower.contains("release")));

            if is_build_dir {
                if path.join("compile_commands.json").exists() {
                    return Some(path);
                }
                if let Some(found) = Self::find_compile_commands_recursive(&path, depth + 1, max_depth) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// Run clang-tidy --fix on a file
    fn run_clang_tidy_fix(&self, path: &Path) -> Result<bool> {
        if !Self::has_clang_tidy() {
            return Ok(false);
        }

        let mut cmd = Command::new("clang-tidy");
        cmd.arg(path);
        cmd.arg("--fix");
        cmd.arg("--fix-errors"); // Also fix errors, not just warnings

        // Add config file if found
        if let Some(config) = Self::find_clang_tidy_config(path) {
            cmd.arg(format!("--config-file={}", config.display()));
        }

        // Add compile_commands.json path
        if let Some(ref build_path) = self.compile_commands_dir {
            cmd.arg(format!("-p={}", build_path.display()));
        } else if let Some(build_path) = Self::find_compile_commands(path) {
            cmd.arg(format!("-p={}", build_path.display()));
        } else {
            cmd.arg("--");
        }

        let output = cmd.output().map_err(|e| {
            crate::LintisError::Formatter(format!("Failed to run clang-tidy --fix: {}", e))
        })?;

        // clang-tidy returns non-zero if there are unfixable issues, but fix still works
        Ok(output.status.success() || !output.stdout.is_empty())
    }
}

impl Default for CppFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for CppFormatter {
    fn name(&self) -> &str {
        match (self.use_clang_tidy_fix && Self::has_clang_tidy(), self.use_cpplint_fix) {
            (true, true) => "clang-format + clang-tidy + cpplint-fix",
            (true, false) => "clang-format + clang-tidy",
            (false, true) => "clang-format + cpplint-fix",
            (false, false) => "clang-format",
        }
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Cpp, Language::ObjectiveC]
    }

    fn format(&self, path: &Path) -> Result<FormatResult> {
        // Read original content for comparison
        let original = fs::read_to_string(path)
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to read file: {}", e)))?;

        // Step 1: Run cpplint fixer (fixes header guards, TODOs, copyright)
        if self.use_cpplint_fix {
            if let Ok(mut fixer) = self.cpplint_fixer.lock() {
                let _ = fixer.fix_file(path); // Ignore errors, continue with other fixes
            }
        }

        // Step 2: Run clang-tidy --fix (fixes code issues like C-style casts)
        if self.use_clang_tidy_fix {
            let _ = self.run_clang_tidy_fix(path); // Ignore errors, clang-format will still run
        }

        // Step 3: Run clang-format (-i modifies in place)
        let output = Command::new("clang-format")
            .args(["-i", "-style=Google"])
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::Formatter(format!("Failed to run clang-format: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(FormatResult::error(
                path.to_path_buf(),
                format!("clang-format failed: {}", stderr),
            ));
        }

        // Read new content and compare
        let new_content = fs::read_to_string(path).map_err(|e| {
            crate::LintisError::Formatter(format!("Failed to read formatted file: {}", e))
        })?;

        if original == new_content {
            Ok(FormatResult::unchanged(path.to_path_buf()))
        } else {
            Ok(FormatResult::changed(path.to_path_buf()))
        }
    }

    fn check(&self, path: &Path) -> Result<bool> {
        // Read current content
        let current = fs::read_to_string(path)
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to read file: {}", e)))?;

        // Run clang-format to get formatted output (without -i)
        let output = Command::new("clang-format")
            .args(["-style=Google"])
            .arg(path)
            .output()
            .map_err(|e| {
                crate::LintisError::Formatter(format!("Failed to run clang-format: {}", e))
            })?;

        let formatted = String::from_utf8_lossy(&output.stdout);

        // If they differ, file needs formatting
        Ok(current != formatted.as_ref())
    }

    fn is_available(&self) -> bool {
        Command::new("clang-format")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
