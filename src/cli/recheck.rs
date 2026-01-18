// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Recheck functionality for modified files after interactive fixes.
//!
//! This module provides shared logic for rechecking files after
//! they've been modified in interactive fix mode.

use colored::Colorize;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::PathBuf;

use linthis::utils::language::language_from_path;
use linthis::utils::types::{LintIssue, Severity};
use linthis::Language;

/// Result of rechecking modified files
pub struct RecheckResult {
    /// Issues found during recheck
    pub issues: Vec<LintIssue>,
    /// Number of files that were rechecked
    pub files_checked: usize,
}

/// Recheck modified files after interactive fixes.
///
/// # Arguments
/// * `modified_files` - Set of files that were modified
/// * `original_issues` - Original issues to get language info from
/// * `quiet` - Whether to suppress progress output
/// * `verbose` - Whether to show verbose output
///
/// # Returns
/// A `RecheckResult` containing the issues found and count of files checked
pub fn recheck_modified_files(
    modified_files: &HashSet<PathBuf>,
    original_issues: &[LintIssue],
    quiet: bool,
    verbose: bool,
) -> RecheckResult {
    // Build a map of file -> language from original issues
    let mut file_languages: HashMap<PathBuf, Language> = HashMap::new();
    for issue in original_issues {
        if let Some(lang) = issue.language {
            file_languages.insert(issue.file_path.clone(), lang);
        }
    }

    // Recheck each modified file
    let modified_count = modified_files.len();
    let mut recheck_issues = Vec::new();

    for (i, file) in modified_files.iter().enumerate() {
        if !quiet {
            eprint!("\r⏳ Rechecking {}/{}...", i + 1, modified_count);
            std::io::stderr().flush().ok();
        }

        // Get language from original issues, or detect it
        let lang = file_languages
            .get(file)
            .copied()
            .or_else(|| language_from_path(file));

        if let Some(lang) = lang {
            if let Some(checker) = linthis::get_checker(lang) {
                if checker.is_available() {
                    match checker.check(file) {
                        Ok(file_issues) => {
                            for mut issue in file_issues {
                                issue.language = Some(lang);
                                recheck_issues.push(issue);
                            }
                        }
                        Err(e) => {
                            if verbose {
                                eprintln!("\n  Check error for {}: {}", file.display(), e);
                            }
                        }
                    }
                }
            }
        }
    }

    // Clear progress line
    if !quiet {
        eprint!("\r");
        std::io::stderr().flush().ok();
    }

    RecheckResult {
        issues: recheck_issues,
        files_checked: modified_count,
    }
}

/// Print the header for recheck output
pub fn print_recheck_header() {
    println!();
    println!("{}", "═".repeat(60).dimmed());
    println!("  {}", "Rechecking modified files...".bold());
    println!("{}", "─".repeat(60).dimmed());
}

/// Print the footer for recheck output
pub fn print_recheck_footer() {
    println!("{}", "═".repeat(60).dimmed());
    println!();
}

/// Print recheck results summary.
///
/// # Arguments
/// * `recheck_result` - The result from `recheck_modified_files`
/// * `fixed_count` - Number of issues that were fixed (edited + ignored)
pub fn print_recheck_summary(recheck_result: &RecheckResult, fixed_count: usize) {
    let remaining_count = recheck_result.issues.len();
    let modified_count = recheck_result.files_checked;

    if remaining_count == 0 {
        println!(
            "  {} All issues in modified files have been resolved!",
            "✓".green().bold()
        );
        println!(
            "  {} file(s) modified, {} issue(s) fixed",
            modified_count, fixed_count
        );
    } else {
        println!(
            "  {} {} remaining issue(s) in modified files",
            "⚠".yellow(),
            remaining_count
        );
        println!(
            "  {} file(s) modified, {} issue(s) fixed",
            modified_count, fixed_count
        );
        println!();

        // Show remaining issues
        let errors = recheck_result
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count();
        let warnings = recheck_result
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count();

        for issue in &recheck_result.issues {
            let severity_badge = match issue.severity {
                Severity::Error => "ERROR".red().bold(),
                Severity::Warning => "WARNING".yellow(),
                Severity::Info => "INFO".blue(),
            };

            let location = if let Some(col) = issue.column {
                format!("{}:{}:{}", issue.file_path.display(), issue.line, col)
            } else {
                format!("{}:{}", issue.file_path.display(), issue.line)
            };

            println!("  {} {} {}", severity_badge, location, issue.message);
        }

        println!();
        println!("  Summary: {} error(s), {} warning(s)", errors, warnings);
    }
}
