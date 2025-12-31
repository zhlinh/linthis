// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Go language checker using golangci-lint or go vet.

use crate::checkers::Checker;
use crate::utils::types::{LintIssue, Severity};
use crate::{Language, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

/// Cache for Go lint results per module root
static GO_LINT_CACHE: Mutex<Option<HashMap<PathBuf, Vec<LintIssue>>>> = Mutex::new(None);

/// Go checker using golangci-lint (preferred) or go vet.
pub struct GoChecker;

impl GoChecker {
    pub fn new() -> Self {
        Self
    }

    /// Find the go.mod for a given file path (Go module root)
    fn find_module_root(path: &Path) -> Option<PathBuf> {
        let mut current = if path.is_file() {
            path.parent()?.to_path_buf()
        } else {
            path.to_path_buf()
        };

        loop {
            let go_mod = current.join("go.mod");
            if go_mod.exists() {
                return Some(current);
            }
            if !current.pop() {
                break;
            }
        }
        None
    }

    /// Check if golangci-lint is available
    fn has_golangci_lint() -> bool {
        Command::new("golangci-lint")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Run golangci-lint on a Go module
    fn run_golangci_lint(module_root: &Path) -> Result<Vec<LintIssue>> {
        let output = Command::new("golangci-lint")
            .args(["run", "--out-format=line-number", "./..."])
            .current_dir(module_root)
            .output()
            .map_err(|e| {
                crate::LintisError::Checker(format!("Failed to run golangci-lint: {}", e))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let issues = Self::parse_golangci_output(&stdout, module_root);

        Ok(issues)
    }

    /// Run go vet on a Go module (fallback)
    fn run_go_vet(module_root: &Path) -> Result<Vec<LintIssue>> {
        let output = Command::new("go")
            .args(["vet", "./..."])
            .current_dir(module_root)
            .output()
            .map_err(|e| crate::LintisError::Checker(format!("Failed to run go vet: {}", e)))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let issues = Self::parse_go_vet_output(&stderr, module_root);

        Ok(issues)
    }

    /// Parse golangci-lint output
    /// Format: file:line:col: message (from linter)
    fn parse_golangci_output(output: &str, module_root: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        for line in output.lines() {
            if let Some(issue) = Self::parse_lint_line(line, module_root, "golangci-lint") {
                issues.push(issue);
            }
        }

        issues
    }

    /// Parse go vet output
    /// Format: file:line:column: message
    fn parse_go_vet_output(output: &str, module_root: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        for line in output.lines() {
            // Skip lines that don't look like error output
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if let Some(issue) = Self::parse_lint_line(line, module_root, "go vet") {
                issues.push(issue);
            }
        }

        issues
    }

    /// Parse a single lint output line
    fn parse_lint_line(line: &str, module_root: &Path, source: &str) -> Option<LintIssue> {
        // Format: path:line:column: message
        // Example: cmd/main.go:10:5: unreachable code
        if !line.contains(':') {
            return None;
        }

        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() < 3 {
            return None;
        }

        let relative_path = PathBuf::from(parts[0]);
        let file_path = module_root.join(relative_path);

        // Skip if this doesn't look like a Go file path
        if !parts[0].ends_with(".go") {
            return None;
        }

        let line_num = parts[1].trim().parse::<usize>().ok()?;

        // Column and message handling
        let (col, message) = if parts.len() >= 4 {
            let col = parts[2].trim().parse::<usize>().ok();
            (col, parts[3].trim().to_string())
        } else {
            (None, parts[2].trim().to_string())
        };

        // Determine severity from message
        let severity = if message.to_lowercase().contains("error") {
            Severity::Error
        } else {
            Severity::Warning
        };

        let mut issue =
            LintIssue::new(file_path, line_num, message, severity).with_source(source.to_string());

        if let Some(c) = col {
            issue = issue.with_column(c);
        }

        Some(issue)
    }

    /// Get cached issues for a module, running linter if not cached
    fn get_cached_issues(module_root: &Path) -> Result<Vec<LintIssue>> {
        let mut cache = GO_LINT_CACHE.lock().unwrap();
        if cache.is_none() {
            *cache = Some(HashMap::new());
        }

        let cache_map = cache.as_mut().unwrap();
        if let Some(issues) = cache_map.get(module_root) {
            return Ok(issues.clone());
        }

        // Run linter and cache results
        // Prefer golangci-lint if available, fall back to go vet
        let issues = if Self::has_golangci_lint() {
            Self::run_golangci_lint(module_root)?
        } else {
            Self::run_go_vet(module_root)?
        };

        cache_map.insert(module_root.to_path_buf(), issues.clone());
        Ok(issues)
    }
}

impl Default for GoChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for GoChecker {
    fn name(&self) -> &str {
        if Self::has_golangci_lint() {
            "golangci-lint"
        } else {
            "go vet"
        }
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Go]
    }

    fn check(&self, path: &Path) -> Result<Vec<LintIssue>> {
        // Find the Go module root
        let module_root = match Self::find_module_root(path) {
            Some(root) => root,
            None => {
                // Not a Go module, skip
                return Ok(Vec::new());
            }
        };

        // Get all issues for this module (cached)
        let all_issues = Self::get_cached_issues(&module_root)?;

        // Normalize paths for comparison
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        // Filter issues for this specific file
        let file_issues: Vec<LintIssue> = all_issues
            .into_iter()
            .filter(|issue| {
                let issue_canonical = issue
                    .file_path
                    .canonicalize()
                    .unwrap_or_else(|_| issue.file_path.clone());
                issue_canonical == canonical_path
            })
            .collect();

        Ok(file_issues)
    }

    fn is_available(&self) -> bool {
        // Either golangci-lint or go must be available
        Self::has_golangci_lint()
            || Command::new("go")
                .arg("version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
    }
}

/// Clear the Go lint cache (useful for testing or forcing re-run)
pub fn clear_go_lint_cache() {
    let mut cache = GO_LINT_CACHE.lock().unwrap();
    *cache = None;
}
