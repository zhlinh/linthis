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
use crossterm::terminal;
use std::process::{Command, Stdio};

/// Get the terminal width, with fallback to 80 columns.
pub fn get_terminal_width() -> usize {
    terminal::size().map(|(w, _)| w as usize).unwrap_or(80)
}

/// Detect the first available AI CLI provider (claude-cli or codebuddy-cli).
/// Returns the provider name if found, or None if neither is available.
pub fn detect_available_cli_provider() -> Option<&'static str> {
    // Check for claude CLI first
    if Command::new("claude")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Some("claude-cli");
    }

    // Check for codebuddy CLI
    if Command::new("codebuddy")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Some("codebuddy-cli");
    }

    None
}

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
            issue
                .context_after
                .last()
                .map(|(n, _)| *n)
                .unwrap_or(issue.line)
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
        output.push_str(&format!(
            "\n{} {} | {}",
            ">".red().bold(),
            line_num.cyan().bold(),
            code_line
        ));

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
            "{} {} tool(s) not available:",
            "⚠".yellow(),
            result.unavailable_tools.len()
        ));
        for tool in &result.unavailable_tools {
            let status = if tool.auto_install_failed {
                "(auto-install failed)".red().to_string()
            } else {
                "(not installed)".yellow().to_string()
            };
            output.push_str(&format!(
                "\n  {} {} ({}) {}",
                "•".dimmed(),
                tool.tool,
                tool.language,
                status
            ));
            output.push_str(&format!("\n    {}", tool.install_hint));
            if tool.auto_install_failed {
                output.push_str(&format!(
                    "\n    {}",
                    "Ensure pip/uv/brew/choco is in PATH, then retry or install manually.".dimmed()
                ));
            }
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
///
/// # Arguments
/// * `result` - The run result to format
/// * `hook_type` - Optional hook type ("pre-push", "commit-msg", or default "pre-commit")
/// * `config_width` - Optional configured width (0 or None = auto-detect terminal width)
pub fn format_result_hook(result: &RunResult, hook_type: Option<&str>) -> String {
    format_result_hook_with_width(result, hook_type, None)
}

/// Build a footer showing the global and local git hook file paths.
pub fn format_hook_paths_footer_pub(hook_type: Option<&str>) -> String {
    format_hook_paths_footer(hook_type)
}

/// Extract `--type <value>` from a thin-wrapper hook script, if present.
fn extract_hook_script_type(path: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    // thin wrapper format: exec linthis hook run --event <e> --type <t> [...]
    content
        .split("--type ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .map(|s| s.to_string())
}

fn format_hook_paths_footer(hook_type: Option<&str>) -> String {
    let hook_filename = match hook_type {
        Some("pre-push") => "pre-push",
        Some("commit-msg") => "commit-msg",
        _ => "pre-commit",
    };

    let mut lines = Vec::new();

    // Global: check core.hooksPath
    if let Some(p) = Command::new("git")
        .args(["config", "--global", "core.hooksPath"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if s.is_empty() {
                    return None;
                }
                let p = std::path::PathBuf::from(s).join(hook_filename);
                if p.exists() {
                    Some(p)
                } else {
                    None
                }
            } else {
                None
            }
        })
    {
        let type_suffix = extract_hook_script_type(&p)
            .map(|t| format!(" (--type {})", t))
            .unwrap_or_default();
        lines.push(
            format!("  Global: {}{}", p.display(), type_suffix)
                .dimmed()
                .to_string(),
        );
    }

    // Local: check .git/hooks/{event}
    if let Some(p) = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                let git_dir = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if git_dir.is_empty() {
                    return None;
                }
                let p = std::path::PathBuf::from(git_dir)
                    .join("hooks")
                    .join(hook_filename);
                if p.exists() {
                    Some(p)
                } else {
                    None
                }
            } else {
                None
            }
        })
    {
        let type_suffix = extract_hook_script_type(&p)
            .map(|t| format!(" (--type {})", t))
            .unwrap_or_default();
        lines.push(
            format!("  Local:  {}{}", p.display(), type_suffix)
                .dimmed()
                .to_string(),
        );
    }

    if lines.is_empty() {
        return String::new();
    }
    format!("\n{}", lines.join("\n"))
}

/// Format the entire run result for git hook output with configurable width.
///
/// # Arguments
/// * `result` - The run result to format
/// * `hook_type` - Optional hook type ("pre-push", "commit-msg", or default "pre-commit")
/// * `config_width` - Optional configured width (0 or None = auto-detect terminal width)
pub fn format_result_hook_with_width(
    result: &RunResult,
    hook_type: Option<&str>,
    config_width: Option<u32>,
) -> String {
    let hook_name = match hook_type {
        Some("pre-push") => "📤 [Pre-push]",
        Some("commit-msg") => "📝 [Commit-msg]",
        _ => "🔍 [Pre-commit]",
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

    // Calculate box width: use config width if provided and > 0, otherwise auto-detect
    // Clamp to min 50, max 120 for readability
    let box_width = match config_width {
        Some(w) if w > 0 => (w as usize).clamp(50, 120),
        _ => get_terminal_width().clamp(50, 120),
    };
    // Content width = box width - 4 (for "│ " prefix and " │" suffix)
    let content_width = box_width - 4;

    // Create border strings dynamically
    let top_border = format!("╭{}╮", "─".repeat(box_width - 2));
    let mid_border = format!("├{}┤", "─".repeat(box_width - 2));
    let bot_border = format!("╰{}╯", "─".repeat(box_width - 2));

    // Helper to pad content to dynamic width
    let pad_line = |content: &str, emoji_count: usize| -> String {
        // Each emoji displays as ~2 chars but counts as 1 in len(), so we subtract emoji_count
        let visual_len = content.chars().count() + emoji_count;
        let padding = content_width.saturating_sub(visual_len);
        format!("│ {}{} │", content, " ".repeat(padding))
    };

    // If no issues, show success
    if total_issues == 0 {
        let mut output = String::new();
        output.push_str(&format!("{}\n", top_border.green()));
        let header = format!("{} Linthis {} Passed", "✓", hook_name);
        output.push_str(&format!("{}\n", pad_line(&header, 1).green()));
        output.push_str(&format!("{}\n", mid_border.green()));
        let checks_msg = if hook_type == Some("pre-push") {
            "All reviews finish"
        } else {
            "All checks passed!"
        };
        output.push_str(&format!("{}\n", pad_line(checks_msg, 0).green()));
        output.push_str(&format!("{}\n", pad_line("", 0)));
        output.push_str(&format!(
            "{}\n",
            pad_line(&format!("Files checked:   {:>3}", result.total_files), 0)
        ));
        output.push_str(&format!(
            "{}\n",
            pad_line(
                &format!("Files formatted: {:>3}", result.files_formatted),
                0
            )
        ));
        output.push_str(&format!("{}", bot_border.green()));
        output.push_str(&format_hook_paths_footer(hook_type));
        return output;
    }

    let mut output = String::new();

    // Header
    output.push_str(&format!("{}\n", top_border.red()));
    let header = format!("X Linthis {} Blocked", hook_name);
    output.push_str(&format!("{}\n", pad_line(&header, 1).red()));
    output.push_str(&format!("{}\n", mid_border.red()));

    // Summary line
    let summary = format!(
        "{} error{}, {} warning{} in {} file{}",
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
    );
    output.push_str(&format!("{}\n", pad_line(&summary, 0)));
    output.push_str(&format!("{}\n", pad_line("", 0)));

    // Dynamic truncation lengths based on content width
    // location: 1/3 of content width, clamped to 10-35 chars
    let location_max = (content_width / 3).clamp(10, 35);
    // message: remaining space after " X " (3 chars) + location + " " (1 char)
    let msg_prefix_len = 4; // " X " + trailing space after location

    // List issues (compact format: file:line message)
    let max_issues = 8; // Limit to avoid too long output
    for issue in result.issues.iter().take(max_issues) {
        let filename = issue
            .file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let location = format!("{}:{}", filename, issue.line);
        let severity_char = match issue.severity {
            Severity::Error => "E",
            Severity::Warning => "W",
            Severity::Info => "I",
        };
        // Truncate location if too long
        let location_display = if location.len() > location_max {
            format!("{}...", &location[..location_max.saturating_sub(3)])
        } else {
            location
        };
        // Truncate message to fit
        let max_msg_len = content_width.saturating_sub(msg_prefix_len + location_display.len());
        let msg = if issue.message.len() > max_msg_len {
            format!("{}...", &issue.message[..max_msg_len.saturating_sub(3)])
        } else {
            issue.message.clone()
        };
        let line_content = format!(" {} {} {}", severity_char, location_display, msg);
        output.push_str(&format!("{}\n", pad_line(&line_content, 0)));
    }

    if total_issues > max_issues {
        let more_line = format!(
            " ... and {} more issue{}",
            total_issues - max_issues,
            if total_issues - max_issues == 1 {
                ""
            } else {
                "s"
            }
        );
        output.push_str(&format!("{}\n", pad_line(&more_line, 0)));
    }

    output.push_str(&format!("{}\n", mid_border.red()));

    // Tip section
    output.push_str(&format!(
        "{}\n",
        pad_line("Tip: To review and fix issues:", 0)
    ));
    output.push_str(&format!(
        "{}\n",
        pad_line("  linthis report show  - view full details", 0)
    ));
    output.push_str(&format!(
        "{}\n",
        pad_line("  linthis fix          - interactive fix", 0)
    ));
    output.push_str(&format!("{}\n", pad_line("", 0)));

    // clang-tidy skip hint if too many clang-tidy issues
    let clang_tidy_count = result
        .issues
        .iter()
        .filter(|i| i.source.as_deref() == Some("clang-tidy"))
        .count();
    if clang_tidy_count >= 10 {
        output.push_str(&format!(
            "{}\n",
            pad_line(
                &format!(
                    "Too many clang-tidy issues ({})? Skip with:",
                    clang_tidy_count
                ),
                0
            )
        ));
        output.push_str(&format!("{}\n", pad_line("  LINTHIS_SKIP_CLANG_TIDY=1", 0)));
        output.push_str(&format!("{}\n", pad_line("", 0)));
    }

    // Skip check hint
    output.push_str(&format!("{}\n", pad_line("To skip this check:", 0)));
    output.push_str(&format!(
        "{}\n",
        pad_line(&format!("  {}", skip_command), 0)
    ));
    output.push_str(&format!("{}", bot_border.red()));
    output.push_str(&format_hook_paths_footer(hook_type));

    output
}

/// Format result according to the specified output format.
pub fn format_result(result: &RunResult, format: OutputFormat) -> String {
    format_result_with_hook_type(result, format, None)
}

/// Format result with optional hook type for hook output.
pub fn format_result_with_hook_type(
    result: &RunResult,
    format: OutputFormat,
    hook_type: Option<&str>,
) -> String {
    match format {
        OutputFormat::Human => format_result_human(result),
        OutputFormat::Json => format_result_json(result),
        OutputFormat::GithubActions => format_result_github_actions(result),
        OutputFormat::Hook => format_result_hook(result, hook_type),
    }
}

/// Format a review result summary in a bordered box for terminal output.
///
/// Produces a box similar to the hook result box, with assessment header,
/// issue counts, and top issues listed.
pub fn format_review_box(result: &crate::review::ReviewResult) -> String {
    use crate::review::{Assessment, Severity};

    let box_width = get_terminal_width().clamp(50, 120);
    let content_width = box_width - 4;

    let top_border = format!("╭{}╮", "─".repeat(box_width - 2));
    let mid_border = format!("├{}┤", "─".repeat(box_width - 2));
    let bot_border = format!("╰{}╯", "─".repeat(box_width - 2));

    let pad_line = |content: &str, emoji_count: usize| -> String {
        let visual_len = content.chars().count() + emoji_count;
        let padding = content_width.saturating_sub(visual_len);
        format!("│ {}{} │", content, " ".repeat(padding))
    };

    let summary = &result.summary;
    let (header_icon, header_text) = match summary.assessment {
        Assessment::Ready => ("✓", "Code Review — Ready"),
        Assessment::NeedsWork => ("!", "Code Review — Needs Work"),
        Assessment::CriticalIssues => ("X", "Code Review — Critical Issues"),
    };
    let header = format!("{} {}", header_icon, header_text);

    let is_success = summary.assessment == Assessment::Ready;

    let mut output = String::new();

    // Header
    if is_success {
        output.push_str(&format!("{}\n", top_border.green()));
        output.push_str(&format!("{}\n", pad_line(&header, 1).green()));
        output.push_str(&format!("{}\n", mid_border.green()));
    } else {
        output.push_str(&format!("{}\n", top_border.red()));
        output.push_str(&format!("{}\n", pad_line(&header, 1).red()));
        output.push_str(&format!("{}\n", mid_border.red()));
    }

    // Summary counts
    let counts = format!(
        "{} issue{}: {} critical, {} important, {} minor",
        summary.total_issues,
        if summary.total_issues == 1 { "" } else { "s" },
        summary.critical_count,
        summary.important_count,
        summary.minor_count
    );
    output.push_str(&format!("{}\n", pad_line(&counts, 0)));
    output.push_str(&format!(
        "{}\n",
        pad_line(&format!("Files reviewed: {}", summary.files_reviewed), 0)
    ));
    output.push_str(&format!(
        "{}\n",
        pad_line(
            &format!("Diff: {}..{}", result.base_ref, result.head_ref),
            0
        )
    ));

    // Top issues (if any)
    if !result.issues.is_empty() {
        output.push_str(&format!("{}\n", pad_line("", 0)));
        let max_issues = 6;
        for issue in result.issues.iter().take(max_issues) {
            let severity_char = match issue.severity {
                Severity::Critical => "C",
                Severity::Important => "I",
                Severity::Minor => "M",
            };
            let location = if let Some(line) = issue.line {
                format!("{}:{}", issue.file.display(), line)
            } else {
                issue.file.display().to_string()
            };
            // Truncate location if needed
            let location_max = (content_width / 3).clamp(10, 35);
            let location_display = if location.len() > location_max {
                format!("{}...", &location[..location_max.saturating_sub(3)])
            } else {
                location
            };
            let msg_prefix_len = 4; // " C " + trailing space
            let max_msg_len = content_width.saturating_sub(msg_prefix_len + location_display.len());
            let msg = if issue.message.len() > max_msg_len {
                format!("{}...", &issue.message[..max_msg_len.saturating_sub(3)])
            } else {
                issue.message.clone()
            };
            let line_content = format!(" {} {} {}", severity_char, location_display, msg);
            output.push_str(&format!("{}\n", pad_line(&line_content, 0)));
        }
        if result.issues.len() > max_issues {
            let more = format!(
                " ... and {} more issue{}",
                result.issues.len() - max_issues,
                if result.issues.len() - max_issues == 1 {
                    ""
                } else {
                    "s"
                }
            );
            output.push_str(&format!("{}\n", pad_line(&more, 0)));
        }
    }

    // Bottom border
    if is_success {
        output.push_str(&format!("{}", bot_border.green()));
    } else {
        output.push_str(&format!("{}", bot_border.red()));
    }

    output
}

/// Format a commit-msg hook result box (passed or blocked).
///
/// Returns the box as a `String` without trailing newline.
/// The footer (hook file paths) is handled by the caller.
pub fn format_cmsg_result(passed: bool, first_line: &str) -> String {
    if passed {
        let mut out = String::new();
        out.push_str(&format!("{}\n", "╭────────────────────────────────────────╮".green()));
        out.push_str(&format!("{}\n", "│ ✓ Linthis 📝 [Commit-msg] Passed       │".green()));
        out.push_str(&format!("{}\n", "├────────────────────────────────────────┤".green()));
        out.push_str(&format!("{}\n", "│ Commit message is valid                │".green()));
        out.push_str(&format!("{}", "╰────────────────────────────────────────╯".green()));
        out
    } else {
        let mut out = String::new();
        out.push_str(&format!("{}\n", "╭────────────────────────────────────────╮".red()));
        out.push_str(&format!("{}\n", "│ X Linthis 📝 [Commit-msg] Blocked      │".red()));
        out.push_str(&format!("{}\n", "├────────────────────────────────────────┤".red()));
        out.push_str(&format!("{}\n", "│ Validation Failed!                     │".red()));
        out.push_str("│                                        │\n");
        out.push_str("│ Your message:                          │\n");
        // Format: "│   {msg}{padding} │\n"
        // Inner width = 40: prefix "   " (3) + msg + padding + " " (1) = 40 → padding = 36 - len
        let truncated = if first_line.chars().count() > 36 {
            format!("{}...", &first_line.chars().take(33).collect::<String>())
        } else {
            first_line.to_string()
        };
        let padding = 36usize.saturating_sub(truncated.chars().count());
        out.push_str(&format!("│   {}{} │\n", truncated, " ".repeat(padding)));
        out.push_str("│                                        │\n");
        out.push_str("│ Expected format (Conventional Commits):│\n");
        out.push_str("│   type(scope)?: description            │\n");
        out.push_str("│                                        │\n");
        out.push_str("│ Valid types:                           │\n");
        out.push_str("│   feat, fix, docs, style, refactor,   │\n");
        out.push_str("│   perf, test, build, ci, chore, revert │\n");
        out.push_str("│                                        │\n");
        out.push_str("│ Examples:                              │\n");
        out.push_str("│   feat: add user authentication        │\n");
        out.push_str("│   fix(api): handle null response       │\n");
        out.push_str("│   docs: update README                  │\n");
        out.push_str(&format!("{}\n", "├────────────────────────────────────────┤".red()));
        out.push_str("│ To skip this check:                    │\n");
        out.push_str("│   git commit --no-verify               │\n");
        out.push_str(&format!("{}", "╰────────────────────────────────────────╯".red()));
        out
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
