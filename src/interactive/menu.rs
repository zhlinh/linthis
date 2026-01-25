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
use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use super::ai_fix::{run_ai_fix_all, run_ai_fix_single, AiFixConfig};
use super::editor::{open_in_editor, LineChange};
use super::nolint::{add_nolint_comment, describe_nolint_action, LineDiff, NolintResult};
use super::quickfix::{default_quickfix_path, write_quickfix_file};

/// Action taken for a single issue
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractiveAction {
    /// Open file in editor at issue location
    Edit,
    /// Add NOLINT comment to suppress issue
    Ignore,
    /// Skip this issue (do nothing)
    Skip,
    /// Go to previous issue
    Previous,
    /// Go to specific issue number
    GoTo(usize),
    /// Get AI-powered fix suggestion
    AiFix,
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
    /// Set of files that were modified (for rechecking)
    pub modified_files: HashSet<PathBuf>,
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
            MainMenuChoice::AiFixAll => {
                let ai_config = AiFixConfig::default();
                let ai_result = run_ai_fix_all(result, &ai_config);
                return InteractiveResult {
                    edited: ai_result.applied,
                    ignored: 0,
                    skipped: ai_result.skipped,
                    quit_early: ai_result.quit_early,
                    modified_files: ai_result.modified_files,
                };
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
    AiFixAll,
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
    println!("  [{}] AI Fix - get AI-powered suggestions", "3".cyan());
    println!("  [{}] Exit", "4".cyan());
    println!();
    println!("  {}", "Vim quickfix shortcuts:".dimmed());
    println!("    {} - next issue  {} - previous  {} - list all", ":cn".cyan(), ":cp".cyan(), ":copen".cyan());
    println!();
    print!("  > ");
    io::stdout().flush().ok();

    let choice = read_line().trim().to_lowercase();

    match choice.as_str() {
        "1" => MainMenuChoice::ReviewOneByOne,
        "2" => MainMenuChoice::OpenInQuickfix,
        "3" | "ai" => MainMenuChoice::AiFixAll,
        "4" | "q" | "quit" | "exit" => MainMenuChoice::Exit,
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
    let mut idx: usize = 0;

    // Track which issues have been processed
    let mut processed = vec![false; total];

    while idx < total {
        let issue = &issues[idx];
        let action = show_issue_menu(issue, idx + 1, total);

        match action {
            InteractiveAction::Edit => {
                result.edited += 1;
                processed[idx] = true;

                let editor_result = open_in_editor(&issue.file_path, issue.line, issue.column);

                if editor_result.success {
                    println!();
                    if editor_result.changes.is_empty() {
                        println!("  {}", "No changes made".dimmed());
                    } else {
                        print_editor_changes(&editor_result.changes, &issue.file_path);
                        // Track this file for rechecking
                        result.modified_files.insert(issue.file_path.clone());
                    }
                } else if let Some(ref error) = editor_result.error {
                    eprintln!("{}: {}", "Failed to open editor".red(), error);
                }
                idx += 1;
            }
            InteractiveAction::Ignore => {
                processed[idx] = true;
                match add_nolint_comment(issue) {
                    NolintResult::Success(diffs) => {
                        result.ignored += 1;
                        println!("{} Added NOLINT comment", "✓".green());
                        println!();
                        print_diff(&diffs, &issue.file_path);
                        // Track this file for rechecking
                        result.modified_files.insert(issue.file_path.clone());
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
                idx += 1;
            }
            InteractiveAction::AiFix => {
                processed[idx] = true;
                let ai_config = AiFixConfig::default();
                match run_ai_fix_single(issue, &ai_config) {
                    Ok((applied, modified_files)) => {
                        if applied {
                            result.edited += 1;
                            result.modified_files.extend(modified_files);
                        } else {
                            result.skipped += 1;
                        }
                    }
                    Err(e) => {
                        eprintln!("{}: {}", "AI fix error".red(), e);
                        result.skipped += 1;
                    }
                }
                idx += 1;
            }
            InteractiveAction::Skip => {
                result.skipped += 1;
                processed[idx] = true;
                idx += 1;
            }
            InteractiveAction::Previous => {
                if idx > 0 {
                    idx -= 1;
                    // Adjust counters if we're revisiting a processed issue
                    if processed[idx] {
                        println!("{}", "  (Revisiting previously processed issue)".dimmed());
                    }
                } else {
                    println!("{}", "  Already at first issue".yellow());
                }
            }
            InteractiveAction::GoTo(target) => {
                if target > 0 && target <= total {
                    idx = target - 1; // Convert to 0-based index
                    if processed[idx] {
                        println!("{}", "  (Jumping to previously processed issue)".dimmed());
                    }
                } else {
                    println!(
                        "  {} Issue #{} out of range (1-{})",
                        "Invalid:".yellow(),
                        target,
                        total
                    );
                }
            }
            InteractiveAction::Quit => {
                result.quit_early = true;
                // Count unprocessed issues as skipped
                for &was_processed in &processed[idx..] {
                    if !was_processed {
                        result.skipped += 1;
                    }
                }
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

    // Action menu with progress indicator
    println!("  {}", format!("Issue {}/{}", current, total).bold().cyan());
    println!();
    let nolint_desc = describe_nolint_action(issue);
    println!("    [{}] Edit - open $EDITOR at this line", "e".cyan());
    println!("    [{}] Ignore - {}", "i".cyan(), nolint_desc.dimmed());
    println!("    [{}] AI fix - get AI suggestion for this issue", "a".cyan());
    println!("    [{}] Skip", "s".cyan());
    if current > 1 {
        println!("    [{}] Previous - go back to issue #{}", "p".cyan(), current - 1);
    }
    println!("    [{}] Go to #N - jump to specific issue", "g".cyan());
    println!("    [{}] Quit", "q".cyan());
    println!();
    print!("  > ");
    io::stdout().flush().ok();

    let choice = read_line().trim().to_lowercase();

    match choice.as_str() {
        "e" | "edit" => InteractiveAction::Edit,
        "i" | "ignore" => InteractiveAction::Ignore,
        "a" | "ai" | "aifix" | "ai-fix" => InteractiveAction::AiFix,
        "s" | "skip" | "" => InteractiveAction::Skip, // Enter defaults to skip
        "p" | "prev" | "previous" => InteractiveAction::Previous,
        "q" | "quit" => InteractiveAction::Quit,
        input if input.starts_with("g") => {
            // Handle "g N" or just "g" (will prompt for number)
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(num) = parts[1].parse::<usize>() {
                    InteractiveAction::GoTo(num)
                } else {
                    println!("{}", "Invalid issue number".yellow());
                    show_issue_menu(issue, current, total)
                }
            } else {
                // Prompt for issue number
                print!("  {} ", "Go to issue #:".cyan());
                io::stdout().flush().ok();
                let num_input = read_line().trim().to_string();
                if let Ok(num) = num_input.parse::<usize>() {
                    InteractiveAction::GoTo(num)
                } else {
                    println!("{}", "Invalid issue number".yellow());
                    show_issue_menu(issue, current, total)
                }
            }
        }
        _ => {
            println!("{}", "Invalid choice. Use: e/i/s/p/g/q".yellow());
            show_issue_menu(issue, current, total)
        }
    }
}

/// Print code context for an issue
pub(crate) fn print_code_context(issue: &LintIssue) {
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
fn open_quickfix(issues: &[LintIssue]) -> super::InteractiveResult<()> {
    use super::InteractiveError;

    let path = default_quickfix_path();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            InteractiveError::FileOperation(format!(
                "Failed to create directory '{}': {}",
                parent.display(),
                e
            ))
        })?;
    }

    write_quickfix_file(issues, &path)?;

    println!(
        "{} Quickfix file written to: {}",
        "✓".green(),
        path.display()
    );
    println!();

    // Try to detect vim-compatible editor
    let editor = detect_quickfix_editor();

    if let Some(editor_cmd) = editor {
        println!("Opening in {}...", editor_cmd.cyan());
        println!("  Use {} to jump between issues", ":cn / :cp".cyan());
        println!();

        // Launch editor with quickfix
        match launch_quickfix_editor(&editor_cmd, &path) {
            Ok(()) => {
                println!();
                println!("{} Editor closed", "✓".green());
            }
            Err(e) => {
                eprintln!("{}: {}", "Failed to open editor".red(), e);
                println!();
                show_manual_quickfix_instructions(&path);
            }
        }
    } else {
        // No compatible editor found, show manual instructions
        show_manual_quickfix_instructions(&path);
    }

    Ok(())
}

/// Show manual instructions for opening quickfix
fn show_manual_quickfix_instructions(path: &std::path::Path) {
    println!("To open in vim:");
    println!("  {} {}", "vim -q".cyan(), path.display());
    println!();
    println!("Or load in vim with:");
    println!("  {} {}", ":cfile".cyan(), path.display());
    println!();
    println!("{}", "Quickfix shortcuts in vim:".bold());
    println!("  {} - Jump to next issue", ":cn".cyan());
    println!("  {} - Jump to previous issue", ":cp".cyan());
    println!("  {} - Open quickfix window", ":copen".cyan());
    println!("  {} - Close quickfix window", ":cclose".cyan());
    println!("  {} - Jump to issue #N", ":cc N".cyan());
}

/// Detect a quickfix-compatible editor
fn detect_quickfix_editor() -> Option<String> {
    // Check for vim-compatible editors in order of preference
    let vim_editors = [
        "nvim",      // Neovim (best vim compatibility)
        "vim",       // Vim
        "vi",        // Vi (basic but widely available)
    ];

    // Check $EDITOR first if it's a vim variant
    if let Ok(editor) = std::env::var("EDITOR") {
        let editor_lower = editor.to_lowercase();
        if editor_lower.contains("vim") || editor_lower.contains("nvim") || editor_lower.contains("vi") {
            return Some(editor);
        }
    }

    // Try to find a vim-compatible editor
    for editor in &vim_editors {
        if which_exists(editor) {
            return Some(editor.to_string());
        }
    }

    None
}

/// Check if a command exists in PATH
fn which_exists(cmd: &str) -> bool {
    use std::process::{Command, Stdio};

    #[cfg(windows)]
    let which_cmd = "where";
    #[cfg(not(windows))]
    let which_cmd = "which";

    Command::new(which_cmd)
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Launch editor with quickfix file
fn launch_quickfix_editor(editor: &str, path: &std::path::Path) -> super::InteractiveResult<()> {
    use super::InteractiveError;
    use std::process::Command;

    let mut cmd = Command::new(editor);
    cmd.arg("-q").arg(path);

    match cmd.status() {
        Ok(status) => {
            if status.success() {
                Ok(())
            } else {
                Err(InteractiveError::EditorLaunch {
                    editor: editor.to_string(),
                    message: format!("exited with status: {}", status.code().unwrap_or(-1)),
                })
            }
        }
        Err(e) => Err(InteractiveError::EditorLaunch {
            editor: editor.to_string(),
            message: e.to_string(),
        }),
    }
}

/// Print diff information in git-style format (for NOLINT comments)
pub(crate) fn print_diff(diffs: &[LineDiff], _file_path: &PathBuf) {
    println!("  {}", "Changes:".bold());

    for diff in diffs {
        // Show context before (from diff)
        if let Some(ref context_before) = diff.context_before {
            println!(
                "  {} {}",
                format!(" {:>4} |", diff.line_number - 1).dimmed(),
                context_before.dimmed()
            );
        }

        // Show removed line (if not empty, meaning it was a modification)
        if !diff.old_content.is_empty() {
            println!(
                "  {} {}",
                format!("-{:>4} |", diff.line_number).red(),
                diff.old_content.red()
            );
        }

        // Show added/modified line
        println!(
            "  {} {}",
            format!("+{:>4} |", diff.line_number).green(),
            diff.new_content.green()
        );

        // Show context after (from diff)
        if let Some(ref context_after) = diff.context_after {
            println!(
                "  {} {}",
                format!(" {:>4} |", diff.line_number + 1).dimmed(),
                context_after.dimmed()
            );
        }
    }
    println!();
}

/// Print changes made in the editor
fn print_editor_changes(changes: &[LineChange], file_path: &PathBuf) {
    use std::fs;

    println!("  {}", "Changes:".bold());

    // Read file to get context lines
    let file_content = fs::read_to_string(file_path).ok();
    let lines: Vec<String> = file_content
        .as_ref()
        .map(|content| content.lines().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    // Limit the number of changes shown to avoid overwhelming output
    const MAX_CHANGES_SHOWN: usize = 20;
    let changes_to_show = if changes.len() > MAX_CHANGES_SHOWN {
        &changes[..MAX_CHANGES_SHOWN]
    } else {
        changes
    };

    for change in changes_to_show {
        let line_idx = change.line_number.saturating_sub(1);

        // Show context before (one line before)
        if line_idx > 0 && line_idx <= lines.len() {
            if let Some(context_line) = lines.get(line_idx - 1) {
                println!(
                    "  {} {}",
                    format!(" {:>4} |", change.line_number - 1).dimmed(),
                    context_line.dimmed()
                );
            }
        }

        // Show removed line (if not empty, meaning it was a modification)
        if !change.old_content.is_empty() {
            println!(
                "  {} {}",
                format!("-{:>4} |", change.line_number).red(),
                change.old_content.red()
            );
        }

        // Show added/modified line (if not empty)
        if !change.new_content.is_empty() {
            println!(
                "  {} {}",
                format!("+{:>4} |", change.line_number).green(),
                change.new_content.green()
            );
        }

        // Show context after (one line after)
        if line_idx + 1 < lines.len() {
            if let Some(context_line) = lines.get(line_idx + 1) {
                println!(
                    "  {} {}",
                    format!(" {:>4} |", change.line_number + 1).dimmed(),
                    context_line.dimmed()
                );
            }
        }
    }

    if changes.len() > MAX_CHANGES_SHOWN {
        println!(
            "  {} ({} more changes not shown)",
            "...".dimmed(),
            changes.len() - MAX_CHANGES_SHOWN
        );
    }

    println!();
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
        assert_ne!(InteractiveAction::Previous, InteractiveAction::Skip);
        assert_ne!(InteractiveAction::AiFix, InteractiveAction::Edit);
    }

    #[test]
    fn test_interactive_action_goto() {
        let goto1 = InteractiveAction::GoTo(1);
        let goto2 = InteractiveAction::GoTo(2);
        assert_ne!(goto1, goto2);
        assert_eq!(goto1, InteractiveAction::GoTo(1));
    }

    #[test]
    fn test_interactive_action_clone() {
        let action = InteractiveAction::GoTo(42);
        let cloned = action.clone();
        assert_eq!(action, cloned);
    }
}
