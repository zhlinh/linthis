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

/// Get files changed since a git ref (branch, tag, or commit).
///
/// Example: `get_changed_files(Some("main"))` returns files changed since main branch.
pub fn get_changed_files(base: Option<&str>) -> crate::Result<Vec<std::path::PathBuf>> {
    let base_ref = base.unwrap_or("HEAD");

    // Get files that differ from the base ref
    let output = Command::new("git")
        .args(["diff", "--name-only", "--diff-filter=d", base_ref])
        .output()
        .map_err(crate::LintisError::Io)?;

    if !output.status.success() {
        // Try with merge-base for branches
        let merge_base = Command::new("git")
            .args(["merge-base", "HEAD", base_ref])
            .output()
            .map_err(crate::LintisError::Io)?;

        if merge_base.status.success() {
            let base_commit = String::from_utf8_lossy(&merge_base.stdout)
                .trim()
                .to_string();
            let output = Command::new("git")
                .args(["diff", "--name-only", "--diff-filter=d", &base_commit])
                .output()
                .map_err(crate::LintisError::Io)?;

            if output.status.success() {
                return parse_git_file_list(&output.stdout);
            }
        }
        return Ok(Vec::new());
    }

    parse_git_file_list(&output.stdout)
}

/// Get uncommitted files (both staged and unstaged changes).
pub fn get_uncommitted_files() -> crate::Result<Vec<std::path::PathBuf>> {
    use std::collections::HashSet;

    let mut files: HashSet<std::path::PathBuf> = HashSet::new();

    // Get staged files
    let staged = get_staged_files()?;
    files.extend(staged);

    // Get unstaged modified files
    let output = Command::new("git")
        .args(["diff", "--name-only", "--diff-filter=d"])
        .output()
        .map_err(crate::LintisError::Io)?;

    if output.status.success() {
        if let Ok(parsed) = parse_git_file_list(&output.stdout) {
            files.extend(parsed);
        }
    }

    // Get untracked files
    let output = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .output()
        .map_err(crate::LintisError::Io)?;

    if output.status.success() {
        if let Ok(parsed) = parse_git_file_list(&output.stdout) {
            files.extend(parsed);
        }
    }

    Ok(files.into_iter().collect())
}

/// Parse git command output into file paths.
fn parse_git_file_list(output: &[u8]) -> crate::Result<Vec<std::path::PathBuf>> {
    let git_root = get_project_root();
    let stdout = String::from_utf8_lossy(output);

    let files = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let relative_path = std::path::PathBuf::from(line.trim());
            git_root.join(relative_path)
        })
        .filter(|p| p.exists()) // Filter out deleted files
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

/// Result of reading a file line with context.
pub struct LineWithContext {
    /// The target line content
    pub line: String,
    /// Context lines before (line_number, content)
    pub before: Vec<(usize, String)>,
    /// Context lines after (line_number, content)
    pub after: Vec<(usize, String)>,
}

/// Read a specific line from a file with surrounding context (1-indexed).
/// Returns the target line and up to `context_lines` lines before and after.
pub fn read_file_line_with_context(
    path: &Path,
    line_number: usize,
    context_lines: usize,
) -> Option<LineWithContext> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    if line_number == 0 {
        return None;
    }

    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    // Calculate the range of lines to read
    let start_line = line_number.saturating_sub(context_lines);
    let end_line = line_number + context_lines;

    let mut before = Vec::new();
    let mut target_line = None;
    let mut after = Vec::new();

    for (idx, line_result) in reader.lines().enumerate() {
        let current_line = idx + 1; // 1-indexed

        if current_line < start_line {
            continue;
        }
        if current_line > end_line {
            break;
        }

        if let Ok(content) = line_result {
            if current_line < line_number {
                before.push((current_line, content));
            } else if current_line == line_number {
                target_line = Some(content);
            } else {
                after.push((current_line, content));
            }
        }
    }

    target_line.map(|line| LineWithContext {
        line,
        before,
        after,
    })
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
