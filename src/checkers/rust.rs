// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Rust language checker using clippy.

use crate::checkers::Checker;
use crate::utils::types::{LintIssue, Severity};
use crate::{Language, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

/// Cache for clippy results per project root
static CLIPPY_CACHE: Mutex<Option<HashMap<PathBuf, Vec<LintIssue>>>> = Mutex::new(None);

/// Rust checker using cargo clippy.
pub struct RustChecker;

impl RustChecker {
    pub fn new() -> Self {
        Self
    }

    /// Find the Cargo.toml for a given file path
    fn find_cargo_root(path: &Path) -> Option<PathBuf> {
        let mut current = if path.is_file() {
            path.parent()?.to_path_buf()
        } else {
            path.to_path_buf()
        };

        loop {
            let cargo_toml = current.join("Cargo.toml");
            if cargo_toml.exists() {
                return Some(current);
            }
            if !current.pop() {
                break;
            }
        }
        None
    }

    /// Run cargo clippy on a project and cache the results
    fn run_cargo_clippy(project_root: &Path) -> Result<Vec<LintIssue>> {
        let output = Command::new("cargo")
            .args(["clippy", "--message-format=short", "--", "-D", "warnings"])
            .current_dir(project_root)
            .output()
            .map_err(|e| {
                crate::LintisError::Checker(format!("Failed to run cargo clippy: {}", e))
            })?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let issues = Self::parse_clippy_output(&stderr, project_root);

        Ok(issues)
    }

    /// Parse clippy output and extract issues.
    fn parse_clippy_output(output: &str, project_root: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Clippy short format: file:line:col: severity: message
        for line in output.lines() {
            if let Some(issue) = Self::parse_clippy_line(line, project_root) {
                issues.push(issue);
            }
        }

        issues
    }

    fn parse_clippy_line(line: &str, project_root: &Path) -> Option<LintIssue> {
        // Short format: path:line:col: severity: message
        // Example: src/main.rs:10:5: warning: unused variable `x`
        if !line.contains(": warning:") && !line.contains(": error:") {
            return None;
        }

        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() < 4 {
            return None;
        }

        let relative_path = PathBuf::from(parts[0]);
        let file_path = project_root.join(relative_path);
        let line_num = parts[1].trim().parse::<usize>().ok()?;
        let col = parts[2].trim().parse::<usize>().ok();

        let rest = parts[3];
        let (severity, message) = if rest.contains("warning:") {
            let msg = rest.trim_start_matches(" warning:").trim();
            (Severity::Warning, msg.to_string())
        } else if rest.contains("error:") {
            let msg = rest.trim_start_matches(" error:").trim();
            (Severity::Error, msg.to_string())
        } else {
            return None;
        };

        let mut issue = LintIssue::new(file_path, line_num, message, severity)
            .with_source("clippy".to_string());

        if let Some(c) = col {
            issue = issue.with_column(c);
        }

        Some(issue)
    }

    /// Get cached issues for a project, running clippy if not cached
    fn get_cached_issues(project_root: &Path) -> Result<Vec<LintIssue>> {
        let mut cache = CLIPPY_CACHE.lock().unwrap();
        if cache.is_none() {
            *cache = Some(HashMap::new());
        }

        let cache_map = cache.as_mut().unwrap();
        if let Some(issues) = cache_map.get(project_root) {
            return Ok(issues.clone());
        }

        // Run clippy and cache results
        let issues = Self::run_cargo_clippy(project_root)?;
        cache_map.insert(project_root.to_path_buf(), issues.clone());
        Ok(issues)
    }
}

impl Default for RustChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for RustChecker {
    fn name(&self) -> &str {
        "clippy"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Rust]
    }

    fn check(&self, path: &Path) -> Result<Vec<LintIssue>> {
        // Find the Cargo project root
        let project_root = match Self::find_cargo_root(path) {
            Some(root) => root,
            None => {
                // Not a Cargo project, skip
                return Ok(Vec::new());
            }
        };

        // Get all issues for this project (cached)
        let all_issues = Self::get_cached_issues(&project_root)?;

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
        Command::new("cargo")
            .args(["clippy", "--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// Clear the clippy cache (useful for testing or forcing re-run)
pub fn clear_clippy_cache() {
    let mut cache = CLIPPY_CACHE.lock().unwrap();
    *cache = None;
}
