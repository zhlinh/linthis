// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! File system walker with parallel processing and exclusion support.

use crate::Language;
use globset::{Glob, GlobSet, GlobSetBuilder};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// File walker configuration.
#[derive(Debug, Clone, Default)]
pub struct WalkerConfig {
    /// Glob patterns to exclude
    pub exclude_patterns: Vec<String>,
    /// Only include files with these languages (empty = all)
    pub languages: Vec<Language>,
    /// Maximum directory depth (0 = unlimited)
    pub max_depth: usize,
    /// Follow symbolic links
    pub follow_links: bool,
}

/// Build a GlobSet from patterns.
pub fn build_glob_set(patterns: &[String]) -> Option<GlobSet> {
    if patterns.is_empty() {
        return None;
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
    }
    builder.build().ok()
}

/// Check if a path should be excluded based on glob patterns.
fn is_excluded(path: &Path, glob_set: &Option<GlobSet>) -> bool {
    if let Some(gs) = glob_set {
        // Check the full path
        if gs.is_match(path) {
            return true;
        }

        // Check path without leading "./"
        let path_str = path.to_string_lossy();
        if let Some(stripped_str) = path_str.strip_prefix("./") {
            let stripped = Path::new(stripped_str);
            if gs.is_match(stripped) {
                return true;
            }
        }

        // Check just the file/dir name
        if let Some(name) = path.file_name() {
            if gs.is_match(name) {
                return true;
            }
        }

        // Check each path component (for patterns like "target/**")
        for component in path.components() {
            if let std::path::Component::Normal(name) = component {
                if gs.is_match(Path::new(name)) {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a file matches the language filter.
fn matches_language_filter(path: &Path, languages: &[Language]) -> bool {
    if languages.is_empty() {
        return true;
    }

    if let Some(lang) = Language::from_path(path) {
        languages.contains(&lang)
    } else {
        false
    }
}

/// Walk a directory and collect files matching the criteria.
pub fn walk_files(root: &Path, config: &WalkerConfig) -> Vec<PathBuf> {
    let glob_set = build_glob_set(&config.exclude_patterns);

    let mut walker = WalkDir::new(root).follow_links(config.follow_links);

    if config.max_depth > 0 {
        walker = walker.max_depth(config.max_depth);
    }

    walker
        .into_iter()
        .filter_entry(|e| {
            // Skip excluded directories early
            !is_excluded(e.path(), &glob_set)
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| !is_excluded(e.path(), &glob_set))
        .filter(|e| matches_language_filter(e.path(), &config.languages))
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// Walk files and process them in parallel.
pub fn walk_files_parallel<F, T>(root: &Path, config: &WalkerConfig, processor: F) -> Vec<T>
where
    F: Fn(&Path) -> T + Send + Sync,
    T: Send,
{
    let files = walk_files(root, config);

    files.par_iter().map(|path| processor(path)).collect()
}

/// Walk multiple paths (files or directories).
/// Returns (files, warnings) tuple.
pub fn walk_paths(paths: &[PathBuf], config: &WalkerConfig) -> (Vec<PathBuf>, Vec<String>) {
    let glob_set = build_glob_set(&config.exclude_patterns);

    let mut result = Vec::new();
    let mut warnings = Vec::new();

    for path in paths {
        if path.is_file() {
            if is_excluded(path, &glob_set) {
                warnings.push(format!(
                    "Path '{}' is excluded by exclude patterns",
                    path.display()
                ));
            } else if !matches_language_filter(path, &config.languages) {
                warnings.push(format!(
                    "Path '{}' does not match language filter",
                    path.display()
                ));
            } else {
                result.push(path.clone());
            }
        } else if path.is_dir() {
            result.extend(walk_files(path, config));
        } else if !path.exists() {
            warnings.push(format!("Path '{}' does not exist", path.display()));
        }
    }

    (result, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_excluded() {
        let patterns = vec!["*.log".to_string(), "node_modules/**".to_string()];
        let glob_set = build_glob_set(&patterns);

        assert!(is_excluded(Path::new("debug.log"), &glob_set));
        assert!(is_excluded(
            Path::new("node_modules/package/index.js"),
            &glob_set
        ));
        assert!(!is_excluded(Path::new("src/main.rs"), &glob_set));
    }

    #[test]
    fn test_matches_language_filter() {
        let languages = vec![Language::Rust, Language::Python];

        assert!(matches_language_filter(
            Path::new("src/main.rs"),
            &languages
        ));
        assert!(matches_language_filter(
            Path::new("scripts/build.py"),
            &languages
        ));
        assert!(!matches_language_filter(Path::new("index.js"), &languages));

        // Empty filter matches all
        assert!(matches_language_filter(Path::new("index.js"), &[]));
    }
}
