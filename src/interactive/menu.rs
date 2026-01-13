// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Interactive menu for reviewing lint issues.
//!
//! Provides:
//! - Main menu with summary and options
//! - Issue-by-issue review with edit/ignore/skip actions
//! - Cross-platform terminal input handling

use crate::utils::types::{LintIssue, RunResult, Severity};
use colored::Colorize;
use std::io::{self, BufRead, Write};

use super::editor::open_in_editor;
use super::nolint::{add_nolint_comment, describe_nolint_action, NolintResult};
use super::quickfix::{default_quickfix_path, write_quickfix_file};

/// Action taken for a single issue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractiveAction {
    /// Open file in editor at issue location
    Edit,
    /// Add NOLINT comment to suppress issue
    Ignore,
    /// Skip this issue (do nothing)
    Skip,
    /// Quit interactive mode
    Quit,
}

/// Result of the interactive session
#[derive(Debug, Default)]
pub struct InteractiveResult {
    /// Number of issues opened in editor
    pub edited: usize,
    /// Number of issues ignored (NOLINT added)
    pub ignored: usize,
    /// Number of issues skipped
    pub skipped: usize,
    /// Whether user quit early
    pub quit_early: bool,
}

/// Run the interactive review mode
///
/// # Arguments
/// * `result` - The lint result to review
///
/// # Returns
/// * `InteractiveResult` with statistics about actions taken
pub fn run_interactive(result: &RunResult) -> InteractiveResult {
    let issues = &result.issues;

    if issues.is_empty() {
        println!("{}", "No issues to review.".green());
        return InteractiveResult::default();
    }

    // Show main menu
    loop {
        match show_main_menu(result) {
            MainMenuChoice::ReviewOneByOne => {
                return run_issue_review(issues);
            }
            MainMenuChoice::OpenInQuickfix => {
                if let Err(e) = open_quickfix(issues) {
                    eprintln!("{}: {}", "Error".red(), e);
                } else {
                    println!("{} Quickfix file created", "✓".green());
                }
            }
            MainMenuChoice::Exit => {
                return InteractiveResult::default();
            }
        }
    }
}

/// Main menu choices
#[derive(Debug, Clone, Copy)]
enum MainMenuChoice {
    ReviewOneByOne,
    OpenInQuickfix,
    Exit,
}

/// Show the main menu with issue summary
fn show_main_menu(result: &RunResult) -> MainMenuChoice {
    let issues = &result.issues;
    let error_count = issues.iter().filter(|i| i.severity == Severity::Error).count();
    let warning_count = issues.iter().filter(|i| i.severity == Severity::Warning).count();
    let info_count = issues.iter().filter(|i| i.severity == Severity::Info).count();

    println!();
    println!("{}", "═".repeat(60).dimmed());
    println!(
        "  Found {} issue{} ({} error{}, {} warning{})",
        issues.len().to_string().bold(),
        if issues.len() == 1 { "" } else { "s" },
        error_count.to_string().red(),
        if error_count == 1 { "" } else { "s" },
        warning_count.to_string().yellow(),
        if warning_count == 1 { "" } else { "s" },
    );
    if info_count > 0 {
        println!("         {} info", info_count.to_string().blue());
    }
    println!("{}", "═".repeat(60).dimmed());
    println!();
    println!("  [{}] Review issues one by one (interactive)", "1".cyan());
    println!("  [{}] Open all in editor (vim quickfix)", "2".cyan());
    println!("  [{}] Exit", "3".cyan());
    println!();
    print!("  > ");
    io::stdout().flush().ok();

    let choice = read_line().trim().to_lowercase();

    match choice.as_str() {
        "1" => MainMenuChoice::ReviewOneByOne,
        "2" => MainMenuChoice::OpenInQuickfix,
        "3" | "q" | "quit" | "exit" => MainMenuChoice::Exit,
        _ => {
            println!("{}", "Invalid choice, please try again.".yellow());
            show_main_menu(result)
        }
    }
}

/// Run the issue-by-issue review loop
fn run_issue_review(issues: &[LintIssue]) -> InteractiveResult {
    let mut result = InteractiveResult::default();
    let total = issues.len();

    for (idx, issue) in issues.iter().enumerate() {
        let action = show_issue_menu(issue, idx + 1, total);

        match action {
            InteractiveAction::Edit => {
                result.edited += 1;
                if let Err(e) = open_in_editor(&issue.file_path, issue.line, issue.column) {
                    eprintln!("{}: {}", "Failed to open editor".red(), e);
                }
            }
            InteractiveAction::Ignore => {
                match add_nolint_comment(issue) {
                    NolintResult::Success => {
                        result.ignored += 1;
                        println!("{} Added NOLINT comment", "✓".green());
                    }
                    NolintResult::AlreadyIgnored => {
                        println!("{}", "Already has NOLINT comment".yellow());
                        result.skipped += 1;
                    }
                    NolintResult::Error(e) => {
                        eprintln!("{}: {}", "Failed to add NOLINT".red(), e);
                        result.skipped += 1;
                    }
                }
            }
            InteractiveAction::Skip => {
                result.skipped += 1;
            }
            InteractiveAction::Quit => {
                result.quit_early = true;
                result.skipped += total - idx;
                break;
            }
        }
    }

    // Show summary
    println!();
    println!("{}", "═".repeat(60).dimmed());
    println!("  {}", "Interactive Review Summary".bold());
    println!("{}", "─".repeat(60).dimmed());
    println!("  Edited:  {}", result.edited.to_string().cyan());
    println!("  Ignored: {}", result.ignored.to_string().yellow());
    println!("  Skipped: {}", result.skipped.to_string().dimmed());
    if result.quit_early {
        println!("  {}", "(Quit early)".dimmed());
    }
    println!("{}", "═".repeat(60).dimmed());
    println!();

    result
}

/// Show menu for a single issue
fn show_issue_menu(issue: &LintIssue, current: usize, total: usize) -> InteractiveAction {
    println!();
    println!("{}", "─".repeat(60).dimmed());

    // Issue header with severity badge
    let severity_badge = match issue.severity {
        Severity::Error => format!("[E{}]", current).red().bold(),
        Severity::Warning => format!("[W{}]", current).yellow().bold(),
        Severity::Info => format!("[I{}]", current).blue(),
    };

    // Language and source tags
    let lang_tag = issue
        .language
        .map(|l| format!("[{}]", format!("{:?}", l).to_lowercase()))
        .unwrap_or_default()
        .dimmed();

    let source_tag = issue
        .source
        .as_ref()
        .map(|s| format!("[{}]", s))
        .unwrap_or_default()
        .dimmed();

    // File location
    let location = if let Some(col) = issue.column {
        format!("{}:{}:{}", issue.file_path.display(), issue.line, col)
    } else {
        format!("{}:{}", issue.file_path.display(), issue.line)
    };

    // Progress indicator
    let progress = format!("({}/{})", current, total).dimmed();

    println!(
        "  {} {}{} {} {}",
        severity_badge,
        lang_tag,
        source_tag,
        location.white().bold(),
        progress
    );

    // Code context
    print_code_context(issue);

    // Message and code
    if let Some(ref code) = issue.code {
        println!("  {} ({})", issue.message, code.cyan());
    } else {
        println!("  {}", issue.message);
    }

    // Suggestion if available
    if let Some(ref suggestion) = issue.suggestion {
        println!("  {} {}", "-->".green(), suggestion);
    }

    println!();

    // Action menu
    let nolint_desc = describe_nolint_action(issue);
    println!("    [{}] Edit - open $EDITOR at this line", "e".cyan());
    println!("    [{}] Ignore - {}", "i".cyan(), nolint_desc.dimmed());
    println!("    [{}] Skip", "s".cyan());
    println!("    [{}] Quit", "q".cyan());
    println!();
    print!("  > ");
    io::stdout().flush().ok();

    let choice = read_line().trim().to_lowercase();

    match choice.as_str() {
        "e" | "edit" => InteractiveAction::Edit,
        "i" | "ignore" => InteractiveAction::Ignore,
        "s" | "skip" | "" => InteractiveAction::Skip, // Enter defaults to skip
        "q" | "quit" => InteractiveAction::Quit,
        _ => {
            println!("{}", "Invalid choice. Use: e/i/s/q".yellow());
            show_issue_menu(issue, current, total)
        }
    }
}

/// Print code context for an issue
fn print_code_context(issue: &LintIssue) {
    // Context before
    for (line_num, content) in &issue.context_before {
        println!(
            "      {} {}",
            format!("{:>5} |", line_num).dimmed(),
            content.dimmed()
        );
    }

    // Issue line (highlighted)
    if let Some(ref code_line) = issue.code_line {
        println!(
            "    {} {} {}",
            ">".red().bold(),
            format!("{:>5} |", issue.line).dimmed(),
            code_line
        );

        // Column indicator
        if let Some(col) = issue.column {
            let padding = " ".repeat(col.saturating_sub(1));
            println!(
                "      {} {}{}",
                "      |".dimmed(),
                padding,
                "^".red().bold()
            );
        }
    }

    // Context after
    for (line_num, content) in &issue.context_after {
        println!(
            "      {} {}",
            format!("{:>5} |", line_num).dimmed(),
            content.dimmed()
        );
    }
}

/// Open all issues in vim quickfix
fn open_quickfix(issues: &[LintIssue]) -> Result<(), String> {
    let path = default_quickfix_path();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    write_quickfix_file(issues, &path)?;

    println!(
        "{} Quickfix file written to: {}",
        "✓".green(),
        path.display()
    );
    println!();
    println!("To open in vim:");
    println!("  {} {}", "vim -q".cyan(), path.display());
    println!();
    println!("Or load in vim with:");
    println!("  {} {}", ":cfile".cyan(), path.display());

    Ok(())
}

/// Read a line from stdin (cross-platform)
fn read_line() -> String {
    let stdin = io::stdin();
    let mut line = String::new();

    // Lock stdin for reading
    let mut handle = stdin.lock();
    handle.read_line(&mut line).ok();

    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Language;
    use std::path::PathBuf;

    fn make_test_issue(severity: Severity) -> LintIssue {
        LintIssue::new(
            PathBuf::from("test.cpp"),
            42,
            "Test message".to_string(),
            severity,
        )
        .with_column(10)
        .with_code("TEST001".to_string())
        .with_source("test-linter".to_string())
        .with_language(Language::Cpp)
        .with_code_line("    int x = 42;".to_string())
    }

    #[test]
    fn test_interactive_result_default() {
        let result = InteractiveResult::default();
        assert_eq!(result.edited, 0);
        assert_eq!(result.ignored, 0);
        assert_eq!(result.skipped, 0);
        assert!(!result.quit_early);
    }

    #[test]
    fn test_interactive_action_variants() {
        assert_ne!(InteractiveAction::Edit, InteractiveAction::Skip);
        assert_ne!(InteractiveAction::Ignore, InteractiveAction::Quit);
    }
}
