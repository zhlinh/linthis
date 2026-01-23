// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Output formatting utilities for linthis results.

use crate::utils::types::{LintIssue, RunResult, Severity};
use colored::Colorize;

/// Output format enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Human,
    Json,
    GithubActions,
    Hook,
}

impl OutputFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "human" => Some(OutputFormat::Human),
            "json" => Some(OutputFormat::Json),
            "github-actions" | "github" | "ga" => Some(OutputFormat::GithubActions),
            "hook" => Some(OutputFormat::Hook),
            _ => None,
        }
    }
}

/// Format a single lint issue for human-readable output.
pub fn format_issue_human(issue: &LintIssue) -> String {
    let severity_str = match issue.severity {
        Severity::Error => "error".red().bold(),
        Severity::Warning => "warning".yellow().bold(),
        Severity::Info => "info".blue().bold(),
    };

    let location = if let Some(col) = issue.column {
        format!("{}:{}:{}", issue.file_path.display(), issue.line, col)
    } else {
        format!("{}:{}", issue.file_path.display(), issue.line)
    };

    let code_str = issue
        .code
        .as_ref()
        .map(|c| format!(" ({})", c))
        .unwrap_or_default();

    let mut output = format!(
        "{}: {}: {}{}",
        location.bold(),
        severity_str,
        issue.message,
        code_str
    );

    // Show context and source code lines if available
    if let Some(code_line) = &issue.code_line {
        // Calculate line number width based on max line number (context_after last line or issue line)
        let max_line = if !issue.context_after.is_empty() {
            issue.context_after.last().map(|(n, _)| *n).unwrap_or(issue.line)
        } else {
            issue.line
        };
        let line_width = max_line.to_string().len().max(5);

        // Show context before (dimmed)
        for (line_num, content) in &issue.context_before {
            let num_str = format!("{:>width$}", line_num, width = line_width);
            output.push_str(&format!("\n  {} | {}", num_str.dimmed(), content.dimmed()));
        }

        // Show the issue line (highlighted with >)
        let line_num = format!("{:>width$}", issue.line, width = line_width);
        output.push_str(&format!("\n{} {} | {}", ">".red().bold(), line_num.cyan().bold(), code_line));

        // Show column indicator if available
        if let Some(col) = issue.column {
            let spaces = " ".repeat(line_width + 5 + col.saturating_sub(1));
            output.push_str(&format!("\n{}^", spaces.red()));
        }

        // Show context after (dimmed)
        for (line_num, content) in &issue.context_after {
            let num_str = format!("{:>width$}", line_num, width = line_width);
            output.push_str(&format!("\n  {} | {}", num_str.dimmed(), content.dimmed()));
        }
    }

    if let Some(suggestion) = &issue.suggestion {
        output.push_str(&format!("\n  --> {}", suggestion.cyan()));
    }

    output
}

/// Format a single lint issue for GitHub Actions output.
pub fn format_issue_github_actions(issue: &LintIssue) -> String {
    let severity = match issue.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "notice",
    };

    let col_str = issue
        .column
        .map(|c| format!(",col={}", c))
        .unwrap_or_default();

    let code_str = issue
        .code
        .as_ref()
        .map(|c| format!(" ({})", c))
        .unwrap_or_default();

    format!(
        "::{} file={},line={}{}::{}{}",
        severity,
        issue.file_path.display(),
        issue.line,
        col_str,
        issue.message,
        code_str
    )
}

/// Format the run result summary for human-readable output.
pub fn format_summary_human(result: &RunResult) -> String {
    use crate::utils::types::RunModeKind;

    let issue_count = result.issues.len();
    let error_count = result
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count();
    let warning_count = result
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .count();

    if issue_count == 0 && result.files_formatted == 0 && result.issues_fixed == 0 {
        let msg = match result.run_mode {
            RunModeKind::FormatOnly => "All formats passed",
            RunModeKind::CheckOnly => "All checks passed",
            RunModeKind::Both => "All checks and formats passed",
        };

        // Add file statistics
        let file_stats = if result.total_files > 0 {
            format!(
                " ({} file{} checked, {} formatted)",
                result.total_files,
                if result.total_files == 1 { "" } else { "s" },
                result.files_formatted
            )
        } else {
            String::new()
        };

        // Add duration
        let duration_str = if result.duration_ms >= 1000 {
            format!("{:.2}s", result.duration_ms as f64 / 1000.0)
        } else {
            format!("{}ms", result.duration_ms)
        };

        return format!(
            "{} {}{} (0 errors, 0 warnings)\nDone in {}",
            "✓".green(),
            msg.green().bold(),
            file_stats,
            duration_str.cyan()
        );
    }

    let mut summary = String::new();

    // Show formatting stats first
    if result.files_formatted > 0 {
        summary.push_str(&format!(
            "{} Formatted {} file{}",
            "✓".green(),
            result.files_formatted,
            if result.files_formatted == 1 { "" } else { "s" }
        ));
    }

    // Show fixed issues (from formatting)
    if result.issues_fixed > 0 {
        if !summary.is_empty() {
            summary.push('\n');
        }
        summary.push_str(&format!(
            "{} Fixed {} issue{} by formatting",
            "✓".green(),
            result.issues_fixed,
            if result.issues_fixed == 1 { "" } else { "s" }
        ));
    }

    // Show remaining issues
    if issue_count > 0 {
        if !summary.is_empty() {
            summary.push('\n');
        }
        summary.push_str(&format!(
            "{} {} remaining issue{} ({} error{}, {} warning{}) in {} of {} file{}",
            "✗".red(),
            issue_count,
            if issue_count == 1 { "" } else { "s" },
            error_count,
            if error_count == 1 { "" } else { "s" },
            warning_count,
            if warning_count == 1 { "" } else { "s" },
            result.files_with_issues,
            result.total_files,
            if result.total_files == 1 { "" } else { "s" }
        ));
    } else if result.files_formatted > 0 || result.issues_fixed > 0 {
        // All issues were fixed
        if !summary.is_empty() {
            summary.push('\n');
        }
        let msg = match result.run_mode {
            RunModeKind::FormatOnly => "All formats passed",
            RunModeKind::CheckOnly => "All checks passed",
            RunModeKind::Both => "All checks and formats passed",
        };

        // Add file statistics
        let file_stats = if result.total_files > 0 {
            format!(
                " ({} file{} checked, {} formatted)",
                result.total_files,
                if result.total_files == 1 { "" } else { "s" },
                result.files_formatted
            )
        } else {
            String::new()
        };

        summary.push_str(&format!(
            "{} {}{} (0 errors, 0 warnings)",
            "✓".green(),
            msg.green().bold(),
            file_stats
        ));
    }

    // Show duration
    if !summary.is_empty() {
        summary.push('\n');
    }
    let duration_str = if result.duration_ms >= 1000 {
        format!("{:.2}s", result.duration_ms as f64 / 1000.0)
    } else {
        format!("{}ms", result.duration_ms)
    };
    summary.push_str(&format!("Done in {}", duration_str.cyan()));

    summary
}

/// Format the entire run result for human-readable output.
pub fn format_result_human(result: &RunResult) -> String {
    let mut output = String::new();

    // Separate errors and warnings for numbered output
    let errors: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .collect();
    let warnings: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .collect();

    // Output errors with [E1][lang][tool], [E2][lang][tool], etc.
    for (idx, issue) in errors.iter().enumerate() {
        let lang_tag = issue
            .language
            .map(|l| format!("[{}]", l.name()))
            .unwrap_or_default();
        let tool_tag = issue
            .source
            .as_ref()
            .map(|s| format!("[{}]", s))
            .unwrap_or_default();
        output.push_str(&format!(
            "{}{}{} {}",
            format!("[E{}]", idx + 1).red().bold(),
            lang_tag.red(),
            tool_tag.red(),
            format_issue_human(issue)
        ));
        output.push('\n');
    }

    // Output warnings with [W1][lang][tool], [W2][lang][tool], etc.
    for (idx, issue) in warnings.iter().enumerate() {
        let lang_tag = issue
            .language
            .map(|l| format!("[{}]", l.name()))
            .unwrap_or_default();
        let tool_tag = issue
            .source
            .as_ref()
            .map(|s| format!("[{}]", s))
            .unwrap_or_default();
        output.push_str(&format!(
            "{}{}{} {}",
            format!("[W{}]", idx + 1).yellow().bold(),
            lang_tag.yellow(),
            tool_tag.yellow(),
            format_issue_human(issue)
        ));
        output.push('\n');
    }

    if !result.issues.is_empty() {
        output.push('\n');
    }

    output.push_str(&format_summary_human(result));

    // Show unavailable tools warning
    if !result.unavailable_tools.is_empty() {
        output.push_str("\n\n");
        output.push_str(&format!(
            "{} {} tool(s) were not available:",
            "⚠".yellow(),
            result.unavailable_tools.len()
        ));
        for tool in &result.unavailable_tools {
            output.push_str(&format!(
                "\n  {} {} ({}) - {}",
                "•".dimmed(),
                tool.tool,
                tool.language,
                tool.install_hint
            ));
        }
        output.push_str(&format!(
            "\n\n{}",
            "Run 'linthis doctor' for detailed tool status.".dimmed()
        ));
    }

    output
}

/// Format the entire run result as JSON.
pub fn format_result_json(result: &RunResult) -> String {
    serde_json::to_string_pretty(result).unwrap_or_else(|_| "{}".to_string())
}

/// Format the entire run result for GitHub Actions.
pub fn format_result_github_actions(result: &RunResult) -> String {
    result
        .issues
        .iter()
        .map(format_issue_github_actions)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format the entire run result for git hook output.
/// Compact format with summary at top, error list, and fix instructions.
pub fn format_result_hook(result: &RunResult, hook_type: Option<&str>) -> String {
    let hook_name = match hook_type {
        Some("pre-push") => "Pre-push",
        Some("commit-msg") => "Commit-msg",
        _ => "Pre-commit",
    };
    let skip_command = match hook_type {
        Some("pre-push") => "git push --no-verify",
        _ => "git commit --no-verify",
    };
    let error_count = result
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count();
    let warning_count = result
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .count();
    let total_issues = result.issues.len();

    // Helper to pad content to fixed width (38 chars for content, accounting for emoji width)
    // Box width: 42 total, 40 inside borders, we use "│ " prefix and " │" suffix = 38 for content
    let pad_line = |content: &str, emoji_count: usize| -> String {
        // Each emoji displays as ~2 chars but counts as 1 in len(), so we subtract emoji_count
        let visual_len = content.chars().count() + emoji_count;
        let padding = 38_usize.saturating_sub(visual_len);
        format!("│ {}{} │", content, " ".repeat(padding))
    };

    // If no issues, show success
    if total_issues == 0 {
        let mut output = String::new();
        output.push_str(&format!("{}\n", "╭────────────────────────────────────────╮".green()));
        let header = format!("{} Linthis {} Hook Passed", "✓", hook_name);
        output.push_str(&format!("{}\n", pad_line(&header, 0).green()));
        output.push_str(&format!("{}\n", "├────────────────────────────────────────┤".green()));
        output.push_str(&format!("{}\n", pad_line("All checks passed!", 0).green()));
        output.push_str(&format!("{}\n", pad_line("", 0)));
        output.push_str(&format!("{}\n", pad_line(&format!("Files checked:   {:>3}", result.total_files), 0)));
        output.push_str(&format!("{}\n", pad_line(&format!("Files formatted: {:>3}", result.files_formatted), 0)));
        output.push_str(&format!("{}", "╰────────────────────────────────────────╯".green()));
        return output;
    }

    let mut output = String::new();

    // Header
    output.push_str(&format!("{}\n", "╭────────────────────────────────────────╮".red()));
    let header = format!("X Linthis {} Hook Failed", hook_name);
    output.push_str(&format!("{}\n", pad_line(&header, 0).red()));
    output.push_str(&format!("{}\n", "├────────────────────────────────────────┤".red()));

    // Summary line
    let summary = format!(
        "{} error{}, {} warning{} in {} file{}",
        error_count,
        if error_count == 1 { "" } else { "s" },
        warning_count,
        if warning_count == 1 { "" } else { "s" },
        result.files_with_issues,
        if result.files_with_issues == 1 { "" } else { "s" }
    );
    output.push_str(&format!("{}\n", pad_line(&summary, 0)));
    output.push_str(&format!("{}\n", pad_line("", 0)));

    // List issues (compact format: file:line message)
    let max_issues = 8; // Limit to avoid too long output
    for issue in result.issues.iter().take(max_issues) {
        let filename = issue.file_path.file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let location = format!("{}:{}", filename, issue.line);
        let severity_char = match issue.severity {
            Severity::Error => "E",
            Severity::Warning => "W",
            Severity::Info => "I",
        };
        // Truncate location if too long
        let location_display = if location.len() > 15 {
            format!("{}...", &location[..12])
        } else {
            location
        };
        // Truncate message to fit
        let max_msg_len = 38 - 4 - location_display.len(); // "  X " prefix + location + " "
        let msg = if issue.message.len() > max_msg_len {
            format!("{}...", &issue.message[..max_msg_len.saturating_sub(3)])
        } else {
            issue.message.clone()
        };
        let line_content = format!(" {} {} {}", severity_char, location_display, msg);
        output.push_str(&format!("{}\n", pad_line(&line_content, 0)));
    }

    if total_issues > max_issues {
        let more_line = format!(" ... and {} more issue{}",
            total_issues - max_issues,
            if total_issues - max_issues == 1 { "" } else { "s" }
        );
        output.push_str(&format!("{}\n", pad_line(&more_line, 0)));
    }

    output.push_str(&format!("{}\n", "├────────────────────────────────────────┤".red()));

    // Fix instructions
    output.push_str(&format!("{}\n", pad_line("To fix automatically:", 0)));
    output.push_str(&format!("{}\n", pad_line(&format!("  {}", "linthis -c -f"), 0)));
    output.push_str(&format!("{}\n", pad_line("", 0)));
    output.push_str(&format!("{}\n", pad_line("To skip this check:", 0)));
    output.push_str(&format!("{}\n", pad_line(&format!("  {}", skip_command), 0)));
    output.push_str(&format!("{}", "╰────────────────────────────────────────╯".red()));

    output
}

/// Format result according to the specified output format.
pub fn format_result(result: &RunResult, format: OutputFormat) -> String {
    format_result_with_hook_type(result, format, None)
}

/// Format result with optional hook type for hook output.
pub fn format_result_with_hook_type(result: &RunResult, format: OutputFormat, hook_type: Option<&str>) -> String {
    match format {
        OutputFormat::Human => format_result_human(result),
        OutputFormat::Json => format_result_json(result),
        OutputFormat::GithubActions => format_result_github_actions(result),
        OutputFormat::Hook => format_result_hook(result, hook_type),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_format_issue_human() {
        let issue = LintIssue::new(
            PathBuf::from("src/main.rs"),
            42,
            "unused variable".to_string(),
            Severity::Warning,
        )
        .with_column(10)
        .with_code("W0001".to_string());

        let output = format_issue_human(&issue);
        assert!(output.contains("src/main.rs:42:10"));
        assert!(output.contains("unused variable"));
        assert!(output.contains("W0001"));
    }

    #[test]
    fn test_format_issue_github_actions() {
        let issue = LintIssue::new(
            PathBuf::from("src/main.rs"),
            42,
            "unused variable".to_string(),
            Severity::Error,
        )
        .with_column(10);

        let output = format_issue_github_actions(&issue);
        assert!(output.starts_with("::error"));
        assert!(output.contains("file=src/main.rs"));
        assert!(output.contains("line=42"));
        assert!(output.contains("col=10"));
    }
}
