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
}

impl OutputFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "human" => Some(OutputFormat::Human),
            "json" => Some(OutputFormat::Json),
            "github-actions" | "github" | "ga" => Some(OutputFormat::GithubActions),
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
        return format!("{}", "All checks passed".green().bold());
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
            "{} {} remaining issue{} ({} error{}, {} warning{}) in {} file{}",
            "✗".red(),
            issue_count,
            if issue_count == 1 { "" } else { "s" },
            error_count,
            if error_count == 1 { "" } else { "s" },
            warning_count,
            if warning_count == 1 { "" } else { "s" },
            result.files_with_issues,
            if result.files_with_issues == 1 {
                ""
            } else {
                "s"
            }
        ));
    } else if result.files_formatted > 0 || result.issues_fixed > 0 {
        // All issues were fixed
        if !summary.is_empty() {
            summary.push('\n');
        }
        summary.push_str(&format!("{}", "All checks passed".green().bold()));
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

    // Output errors with [E1], [E2], etc.
    for (idx, issue) in errors.iter().enumerate() {
        output.push_str(&format!(
            "{} {}",
            format!("[E{}]", idx + 1).red().bold(),
            format_issue_human(issue)
        ));
        output.push('\n');
    }

    // Output warnings with [W1], [W2], etc.
    for (idx, issue) in warnings.iter().enumerate() {
        output.push_str(&format!(
            "{} {}",
            format!("[W{}]", idx + 1).yellow().bold(),
            format_issue_human(issue)
        ));
        output.push('\n');
    }

    if !result.issues.is_empty() {
        output.push('\n');
    }

    output.push_str(&format_summary_human(result));

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

/// Format result according to the specified output format.
pub fn format_result(result: &RunResult, format: OutputFormat) -> String {
    match format {
        OutputFormat::Human => format_result_human(result),
        OutputFormat::Json => format_result_json(result),
        OutputFormat::GithubActions => format_result_github_actions(result),
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
