// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Utility modules for linthis.

pub mod language;
pub mod output;
pub mod types;
pub mod unicode;
pub mod walker;

use std::fs;
use std::path::Path;
use std::process::Command;

/// Default exclusion patterns for common directories that shouldn't be linted.
pub const DEFAULT_EXCLUDES: &[&str] = &[
    // Version control
    ".git/**",
    ".hg/**",
    ".svn/**",
    // Dependencies
    "node_modules/**",
    "vendor/**",
    "venv/**",
    ".venv/**",
    "__pycache__/**",
    // Third-party libraries
    "third_party/**",
    "thirdparty/**",
    "third-party/**",
    "3rdparty/**",
    "3rd_party/**",
    "3rd-party/**",
    "3party/**",
    "external/**",
    "externals/**",
    "deps/**",
    // Build outputs
    "target/**",
    "build/**",
    "dist/**",
    "out/**",
    "_build/**",
    // IDE and editor
    ".idea/**",
    ".vscode/**",
    ".vs/**",
    "*.swp",
    "*~",
    // Generated files
    "*.generated.*",
    "*.min.js",
    "*.min.css",
    // Package managers (iOS)
    "Pods/**",
    "**/Pods/**",
    "Carthage/**",
    "**/Carthage/**",
];

/// Get list of staged files from git.
pub fn get_staged_files() -> crate::Result<Vec<std::path::PathBuf>> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--name-only", "--diff-filter=d"])
        .output()
        .map_err(crate::LintisError::Io)?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    // Get git root directory
    let git_root = get_project_root();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let files = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            // Convert relative path to absolute path based on git root
            let relative_path = std::path::PathBuf::from(line.trim());
            git_root.join(relative_path)
        })
        .collect();

    Ok(files)
}

/// Check if a path matches any of the ignore patterns.
pub fn should_ignore(path: &Path, patterns: &[regex::Regex]) -> bool {
    let path_str = path.to_string_lossy();
    patterns.iter().any(|pattern| pattern.is_match(&path_str))
}

/// Read a specific line from a file (1-indexed).
pub fn read_file_line(path: &Path, line_number: usize) -> Option<String> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    if line_number == 0 {
        return None;
    }

    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    reader
        .lines()
        .nth(line_number - 1)
        .and_then(|line| line.ok())
}

/// Get the project root directory (git root or current directory).
pub fn get_project_root() -> std::path::PathBuf {
    Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

/// Check if we're in a git repository.
pub fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Parse .gitignore file and return glob patterns.
/// Converts gitignore patterns to glob patterns for use with our walker.
pub fn parse_gitignore(gitignore_path: &Path) -> Vec<String> {
    let mut patterns = Vec::new();

    let content = match fs::read_to_string(gitignore_path) {
        Ok(c) => c,
        Err(_) => return patterns,
    };

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Skip negation patterns (not supported in simple glob)
        if line.starts_with('!') {
            continue;
        }

        // Convert gitignore pattern to glob pattern
        let pattern = convert_gitignore_to_glob(line);
        if !pattern.is_empty() {
            patterns.push(pattern);
        }
    }

    patterns
}

/// Convert a gitignore pattern to a glob pattern.
fn convert_gitignore_to_glob(pattern: &str) -> String {
    let mut pattern = pattern.to_string();

    // Remove leading slash (gitignore root anchor)
    let rooted = pattern.starts_with('/');
    if rooted {
        pattern = pattern[1..].to_string();
    }

    // Handle trailing slash (directory indicator)
    let is_dir = pattern.ends_with('/');
    if is_dir {
        pattern = pattern[..pattern.len() - 1].to_string();
    }

    // If pattern doesn't contain / (except trailing), it matches anywhere
    // Convert to **/pattern
    if !rooted && !pattern.contains('/') {
        pattern = format!("**/{}", pattern);
    }

    // Add /** suffix for directories to match all contents
    if is_dir || !pattern.contains('.') {
        // Likely a directory, add /** to match contents
        if !pattern.ends_with("/**") && !pattern.ends_with("/*") {
            pattern.push_str("/**");
        }
    }

    pattern
}

/// Get all gitignore patterns from the project.
/// Reads .gitignore from project root and any nested .gitignore files.
pub fn get_gitignore_patterns(project_root: &Path) -> Vec<String> {
    let mut patterns = Vec::new();

    // Read root .gitignore
    let root_gitignore = project_root.join(".gitignore");
    if root_gitignore.exists() {
        patterns.extend(parse_gitignore(&root_gitignore));
    }

    // Also check for global gitignore
    if let Some(home) = std::env::var("HOME").ok().map(std::path::PathBuf::from) {
        let global_gitignore = home.join(".gitignore_global");
        if global_gitignore.exists() {
            patterns.extend(parse_gitignore(&global_gitignore));
        }
    }

    patterns
}
