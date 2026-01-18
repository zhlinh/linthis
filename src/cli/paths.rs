// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Path collection and exclusion pattern handling.
//!
//! This module provides functionality for collecting paths to lint/format,
//! building exclusion patterns, and filtering files.

use colored::Colorize;
use std::path::PathBuf;

/// Options for path collection
pub struct PathCollectionOptions {
    /// Whether to check staged files
    pub staged: bool,
    /// Check files changed since this ref
    pub since: Option<String>,
    /// Whether to check uncommitted files
    pub uncommitted: bool,
    /// Whether to skip default exclude patterns
    pub no_default_excludes: bool,
    /// Whether to skip .gitignore patterns
    pub no_gitignore: bool,
    /// Additional exclude patterns from CLI
    pub exclude: Vec<String>,
    /// Paths specified on command line
    pub paths: Vec<PathBuf>,
    /// Whether to show verbose output
    pub verbose: bool,
}

/// Result of path collection
pub enum PathCollectionResult {
    /// Successfully collected paths with exclusion patterns
    Success(Vec<PathBuf>, Vec<String>),
    /// No files to check (with message to display)
    Empty(String),
    /// Error occurred (with error message and exit code)
    Error(String, i32),
}

/// Build exclusion patterns from all sources.
///
/// Combines patterns from:
/// - Default excludes (unless no_default_excludes is true)
/// - .gitignore patterns (unless no_gitignore is true)
/// - CLI-specified excludes
/// - Project config file excludes
pub fn build_exclusion_patterns(options: &PathCollectionOptions) -> Vec<String> {
    let mut exclude_patterns: Vec<String> = if options.no_default_excludes {
        Vec::new()
    } else {
        linthis::utils::DEFAULT_EXCLUDES
            .iter()
            .map(|s| s.to_string())
            .collect()
    };

    // Add .gitignore patterns if in a git repo and not disabled
    if !options.no_gitignore && linthis::utils::is_git_repo() {
        let project_root = linthis::utils::get_project_root();
        let gitignore_patterns = linthis::utils::get_gitignore_patterns(&project_root);
        if options.verbose && !gitignore_patterns.is_empty() {
            eprintln!(
                "Loaded {} patterns from .gitignore",
                gitignore_patterns.len()
            );
        }
        exclude_patterns.extend(gitignore_patterns);
    }

    // Add CLI-specified excludes
    exclude_patterns.extend(options.exclude.clone());

    // Add excludes from project config file
    let project_root = linthis::utils::get_project_root();
    if let Some(project_config) = linthis::config::Config::load_project_config(&project_root) {
        if !project_config.excludes.is_empty() {
            if options.verbose {
                eprintln!(
                    "Loaded {} exclude patterns from config",
                    project_config.excludes.len()
                );
            }
            exclude_patterns.extend(project_config.excludes);
        }
    }

    exclude_patterns
}

/// Filter files with exclusion patterns.
///
/// Applies glob patterns to filter out files that match any exclusion pattern.
pub fn filter_files_with_exclusions(
    files: Vec<PathBuf>,
    exclude_patterns: &[String],
    project_root: &PathBuf,
    verbose: bool,
) -> Vec<PathBuf> {
    use linthis::utils::walker::build_glob_set;
    let glob_set = build_glob_set(exclude_patterns);
    files
        .into_iter()
        .filter(|path| {
            if let Some(ref gs) = glob_set {
                if let Ok(relative) = path.strip_prefix(project_root) {
                    if gs.is_match(relative) {
                        if verbose {
                            eprintln!("Excluding: {}", relative.display());
                        }
                        return false;
                    }
                    let components: Vec<_> = relative.components().collect();
                    for i in 0..components.len() {
                        let subpath: PathBuf = components[i..].iter().collect();
                        if gs.is_match(&subpath) {
                            if verbose {
                                eprintln!(
                                    "Excluding: {} (matches from subpath {})",
                                    relative.display(),
                                    subpath.display()
                                );
                            }
                            return false;
                        }
                    }
                }
            }
            true
        })
        .collect()
}

/// Collect paths based on the specified options.
///
/// Handles staged, since, uncommitted, and default path modes,
/// applying exclusion filters to all results.
pub fn collect_paths(options: &PathCollectionOptions) -> PathCollectionResult {
    let exclude_patterns = build_exclusion_patterns(options);
    let project_root = linthis::utils::get_project_root();

    if options.staged {
        match linthis::utils::get_staged_files() {
            Ok(files) => {
                if files.is_empty() {
                    return PathCollectionResult::Empty("No staged files to check".yellow().to_string());
                }
                let filtered = filter_files_with_exclusions(
                    files,
                    &exclude_patterns,
                    &project_root,
                    options.verbose,
                );
                if filtered.is_empty() {
                    return PathCollectionResult::Empty(
                        "No staged files to check after exclusions".yellow().to_string(),
                    );
                }
                if options.verbose {
                    eprintln!("Checking {} staged file(s) after exclusions", filtered.len());
                }
                PathCollectionResult::Success(filtered, exclude_patterns)
            }
            Err(e) => PathCollectionResult::Error(
                format!("{}: {}", "Error getting staged files".red(), e),
                2,
            ),
        }
    } else if let Some(ref base_ref) = options.since {
        match linthis::utils::get_changed_files(Some(base_ref.as_str())) {
            Ok(files) => {
                if files.is_empty() {
                    return PathCollectionResult::Empty(
                        format!("No files changed since '{}'", base_ref).yellow().to_string(),
                    );
                }
                let filtered = filter_files_with_exclusions(
                    files,
                    &exclude_patterns,
                    &project_root,
                    options.verbose,
                );
                if filtered.is_empty() {
                    return PathCollectionResult::Empty(
                        format!("No files to check after exclusions (since '{}')", base_ref)
                            .yellow()
                            .to_string(),
                    );
                }
                if options.verbose {
                    eprintln!(
                        "Checking {} file(s) changed since '{}' after exclusions",
                        filtered.len(),
                        base_ref
                    );
                }
                PathCollectionResult::Success(filtered, exclude_patterns)
            }
            Err(e) => PathCollectionResult::Error(
                format!("{}: {}", "Error getting changed files".red(), e),
                2,
            ),
        }
    } else if options.uncommitted {
        match linthis::utils::get_uncommitted_files() {
            Ok(files) => {
                if files.is_empty() {
                    return PathCollectionResult::Empty(
                        "No uncommitted files to check".yellow().to_string(),
                    );
                }
                let filtered = filter_files_with_exclusions(
                    files,
                    &exclude_patterns,
                    &project_root,
                    options.verbose,
                );
                if filtered.is_empty() {
                    return PathCollectionResult::Empty(
                        "No uncommitted files to check after exclusions".yellow().to_string(),
                    );
                }
                if options.verbose {
                    eprintln!(
                        "Checking {} uncommitted file(s) after exclusions",
                        filtered.len()
                    );
                }
                PathCollectionResult::Success(filtered, exclude_patterns)
            }
            Err(e) => PathCollectionResult::Error(
                format!("{}: {}", "Error getting uncommitted files".red(), e),
                2,
            ),
        }
    } else if options.paths.is_empty() {
        // Default to current directory if no paths specified
        PathCollectionResult::Success(vec![PathBuf::from(".")], exclude_patterns)
    } else {
        PathCollectionResult::Success(options.paths.clone(), exclude_patterns)
    }
}
