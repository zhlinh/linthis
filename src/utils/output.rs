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
//!
//! The `fix_tip_lines()` function provides a single source of truth for
//! the "Tip: To review and fix issues" hints used in both terminal output
//! and hook output boxes.

use crate::utils::types::{LintIssue, RunResult, Severity};
use colored::Colorize;
use crossterm::terminal;
use std::process::{Command, Stdio};

/// Tip lines for "To review and fix issues" hints.
/// Each entry is (command, description). Used by both terminal and hook output.
pub fn fix_tip_lines() -> Vec<(&'static str, &'static str)> {
    vec![
        ("linthis report show", "view full details"),
        ("linthis report show -f html --open", "view in browser"),
        (
            "linthis backup diff",
            "show diff from last format/fix/hook-fix",
        ),
        ("linthis undo", "undo last format/fix/hook-fix"),
        ("linthis fix", "load last result and fix"),
        (
            "linthis fix --ai",
            "AI-powered fix suggestions (--provider and --model supported)",
        ),
        (
            "linthis fix --auto",
            "(dangerously) auto-fix via AI agent (--provider and --model supported)",
        ),
    ]
}

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

/// Format a duration in milliseconds as a human-readable string.
fn format_duration_str(duration_ms: u64) -> String {
    if duration_ms >= 1000 {
        format!("{:.2}s", duration_ms as f64 / 1000.0)
    } else {
        format!("{}ms", duration_ms)
    }
}

/// Get the "all passed" message based on run mode.
fn all_passed_message(run_mode: &crate::utils::types::RunModeKind) -> &'static str {
    use crate::utils::types::RunModeKind;
    match run_mode {
        RunModeKind::FormatOnly => "All formats passed",
        RunModeKind::CheckOnly => "All checks passed",
        RunModeKind::Both => "All checks and formats passed",
    }
}

/// Format file stats string like " (0 errors, 0 warnings in N file(s))".
fn format_file_stats(total_files: usize) -> String {
    format!(
        " (0 errors, 0 warnings in {} file{})",
        total_files,
        if total_files == 1 { "" } else { "s" },
    )
}

/// Format the no-issues summary case.
fn format_summary_no_issues(result: &RunResult) -> String {
    let msg = all_passed_message(&result.run_mode);
    let file_stats = format_file_stats(result.total_files);
    let duration_str = format_duration_str(result.duration_ms);

    format!(
        "{} {}{}\nDone in {}",
        "✓".green(),
        msg.green().bold(),
        file_stats,
        duration_str.cyan()
    )
}

/// Append formatting stats to the summary if files were formatted.
fn append_formatting_stats(summary: &mut String, result: &RunResult) {
    if result.files_formatted > 0 {
        summary.push_str(&format!(
            "{} Formatted {} file{}",
            "✓".green(),
            result.files_formatted,
            if result.files_formatted == 1 { "" } else { "s" }
        ));
    }

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
}

/// Append remaining issues or all-passed message to the summary.
fn append_issues_or_passed(
    summary: &mut String,
    result: &RunResult,
    issue_count: usize,
    error_count: usize,
    warning_count: usize,
) {
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
        if !summary.is_empty() {
            summary.push('\n');
        }
        let msg = all_passed_message(&result.run_mode);
        let file_stats = format_file_stats(result.total_files);
        summary.push_str(&format!(
            "{} {}{} (0 errors, 0 warnings)",
            "✓".green(),
            msg.green().bold(),
            file_stats
        ));
    }
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
        return format_summary_no_issues(result);
    }

    let mut summary = String::new();
    append_formatting_stats(&mut summary, result);
    append_issues_or_passed(
        &mut summary,
        result,
        issue_count,
        error_count,
        warning_count,
    );

    if !summary.is_empty() {
        summary.push('\n');
    }
    let duration_str = format_duration_str(result.duration_ms);
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

/// Unified JSON output structure for all checks.
///
/// Each check (lint, security, complexity) has the same shape:
/// `{ total_files, total_issues, issues: [{ file, line, severity, message, source, ... }] }`
#[derive(serde::Serialize)]
struct UnifiedResult {
    checks_run: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lint: Option<CheckResultView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    security: Option<CheckResultView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    complexity: Option<CheckResultView>,
    duration_ms: u64,
    exit_code: i32,
    run_mode: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    unavailable_tools: Vec<UnavailableToolView>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    target_paths: Vec<String>,
}

/// Unified check result — same structure for lint, security, complexity.
#[derive(serde::Serialize)]
struct CheckResultView {
    total_files: usize,
    total_issues: usize,
    issues: Vec<IssueView>,
    /// Extra metadata specific to this check type
    #[serde(skip_serializing_if = "Option::is_none")]
    extra: Option<serde_json::Value>,
}

/// Unified issue — same fields for all check types.
#[derive(serde::Serialize)]
struct IssueView {
    file: String,
    line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_line: Option<usize>,
    severity: String,
    message: String,
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestion: Option<String>,
    /// Function name (for complexity issues)
    #[serde(skip_serializing_if = "Option::is_none")]
    function: Option<String>,
}

#[derive(serde::Serialize)]
struct UnavailableToolView {
    tool: String,
    language: String,
    install_hint: String,
}

/// Build lint check result view from RunResult.
fn build_lint_view(result: &RunResult) -> CheckResultView {
    let issues: Vec<IssueView> = result
        .issues
        .iter()
        .map(|i| IssueView {
            file: i.file_path.to_string_lossy().to_string(),
            line: i.line,
            column: i.column,
            end_line: None,
            severity: i.severity.to_string(),
            message: i.message.clone(),
            source: i.source.clone().unwrap_or_default(),
            code: i.code.clone(),
            suggestion: i.suggestion.clone(),
            function: None,
        })
        .collect();

    let extra = serde_json::json!({
        "files_formatted": result.files_formatted,
        "issues_before_format": result.issues_before_format,
        "issues_fixed": result.issues_fixed,
    });

    CheckResultView {
        total_files: result.total_files,
        total_issues: issues.len(),
        issues,
        extra: Some(extra),
    }
}

/// Build security check result view from SastResult.
fn build_security_view(sast: &crate::security::sast::SastResult) -> CheckResultView {
    let issues: Vec<IssueView> = sast
        .findings
        .iter()
        .map(|f| {
            // Map security severity to unified error/warning/info
            let unified_severity = match f.severity {
                crate::security::Severity::Critical | crate::security::Severity::High => "error",
                crate::security::Severity::Medium => "warning",
                _ => "info",
            };
            IssueView {
                file: f.file_path.to_string_lossy().to_string(),
                line: f.line,
                column: f.column,
                end_line: f.end_line,
                severity: unified_severity.to_string(),
                message: f.message.clone(),
                source: f.source.clone(),
                code: Some(f.rule_id.clone()),
                suggestion: f.fix_suggestion.clone(),
                function: None,
            }
        })
        .collect();

    let total_files = {
        let mut files: std::collections::HashSet<String> = std::collections::HashSet::new();
        for f in &sast.findings {
            files.insert(f.file_path.to_string_lossy().to_string());
        }
        files.len()
    };

    let extra = serde_json::json!({
        "scanner_status": sast.scanner_status,
        "by_severity": sast.by_severity,
    });

    CheckResultView {
        total_files,
        total_issues: issues.len(),
        issues,
        extra: Some(extra),
    }
}

/// Build complexity check result view from AnalysisResult.
///
/// Converts per-function metrics into issues (one issue per function exceeding threshold).
fn build_complexity_view(analysis: &crate::complexity::AnalysisResult) -> CheckResultView {
    let threshold = if analysis.thresholds.cyclomatic.good > 0 {
        analysis.thresholds.cyclomatic.good
    } else {
        10
    };
    let warning_threshold = analysis.thresholds.cyclomatic.warning;
    let high_threshold = analysis.thresholds.cyclomatic.high;

    let mut issues: Vec<IssueView> = Vec::new();

    for file in &analysis.files {
        for func in &file.functions {
            if func.metrics.cyclomatic > threshold {
                let severity = if func.metrics.cyclomatic > high_threshold {
                    "error"
                } else if func.metrics.cyclomatic > warning_threshold {
                    "warning"
                } else {
                    "info"
                };

                let exceeded_threshold = match severity {
                    "error" => high_threshold,
                    "warning" => warning_threshold,
                    _ => threshold,
                };
                issues.push(IssueView {
                    file: file.path.to_string_lossy().to_string(),
                    line: func.start_line as usize,
                    column: None,
                    end_line: Some(func.end_line as usize),
                    severity: severity.to_string(),
                    message: format!(
                        "function `{}` cyclomatic complexity {} exceeds threshold {}",
                        func.name, func.metrics.cyclomatic, exceeded_threshold,
                    ),
                    source: "linthis-complexity".to_string(),
                    code: None,
                    suggestion: Some("Consider refactoring into smaller functions".to_string()),
                    function: Some(func.name.clone()),
                });
            }
        }
    }

    let extra = serde_json::json!({
        "summary": {
            "total_functions": analysis.summary.total_functions,
            "avg_cyclomatic": analysis.summary.avg_cyclomatic,
            "max_cyclomatic": analysis.summary.max_cyclomatic,
        },
    });

    CheckResultView {
        total_files: analysis.files.len(),
        total_issues: issues.len(),
        issues,
        extra: Some(extra),
    }
}

/// Format the entire run result as unified JSON.
pub fn format_result_json(result: &RunResult) -> String {
    let lint_view =
        if result.checks_run.contains(&"lint".to_string()) || result.checks_run.is_empty() {
            Some(build_lint_view(result))
        } else {
            None
        };

    let security_view = result.security.as_ref().map(build_security_view);
    let complexity_view = result.complexity.as_ref().map(build_complexity_view);

    let unavailable: Vec<UnavailableToolView> = result
        .unavailable_tools
        .iter()
        .map(|t| UnavailableToolView {
            tool: t.tool.clone(),
            language: t.language.clone(),
            install_hint: t.install_hint.clone(),
        })
        .collect();

    let unified = UnifiedResult {
        checks_run: if result.checks_run.is_empty() {
            vec!["lint".to_string()]
        } else {
            result.checks_run.clone()
        },
        lint: lint_view,
        security: security_view,
        complexity: complexity_view,
        duration_ms: result.duration_ms,
        exit_code: result.exit_code,
        run_mode: format!("{:?}", result.run_mode).to_lowercase(),
        unavailable_tools: unavailable,
        target_paths: result.target_paths.clone(),
    };
    serde_json::to_string_pretty(&unified).unwrap_or_else(|_| "{}".to_string())
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

/// Get the fix_commit_mode suffix for hook footer display.
/// Returns empty string for default mode (squash), otherwise ", --fix-commit-mode <mode>".
fn fix_commit_mode_suffix(hook_type: Option<&str>) -> String {
    let project_root = crate::utils::get_project_root();
    let config = crate::config::Config::load_merged(&project_root);
    let mode = match hook_type {
        Some("pre-push") => config.hook.pre_push.fix_commit_mode.clone(),
        _ => config.hook.pre_commit.fix_commit_mode.clone(),
    };
    if mode != "squash" {
        format!("--fix-commit-mode {}", mode)
    } else {
        String::new()
    }
}

/// Build a parenthesized suffix like " (--type git, --fix-commit-mode dirty)".
fn build_hook_suffix(hook_path: &std::path::Path, mode_suffix: &str) -> String {
    let mut parts = Vec::new();
    if let Some(t) = extract_hook_script_type(hook_path) {
        parts.push(format!("--type {}", t));
    }
    if !mode_suffix.is_empty() {
        parts.push(mode_suffix.to_string());
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join(", "))
    }
}

fn format_hook_paths_footer(hook_type: Option<&str>) -> String {
    let hook_filename = match hook_type {
        Some("pre-push") => "pre-push",
        Some("commit-msg") => "commit-msg",
        _ => "pre-commit",
    };

    let mode_suffix = fix_commit_mode_suffix(hook_type);

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
        let suffix = build_hook_suffix(&p, &mode_suffix);
        lines.push(format!("  Global: {}{}", p.display(), suffix).dimmed().to_string());
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
        let suffix = build_hook_suffix(&p, &mode_suffix);
        lines.push(format!("  Local:  {}{}", p.display(), suffix).dimmed().to_string());
    }

    if lines.is_empty() {
        return String::new();
    }
    format!("\n{}", lines.join("\n"))
}

/// Box drawing context for hook output formatting.
struct HookBoxContext {
    content_width: usize,
    top_border: String,
    mid_border: String,
    bot_border: String,
}

impl HookBoxContext {
    fn new(config_width: Option<u32>) -> Self {
        let box_width = match config_width {
            Some(w) if w > 0 => (w as usize).clamp(50, 120),
            _ => get_terminal_width().clamp(50, 120),
        };
        let content_width = box_width - 4;
        Self {
            content_width,
            top_border: format!("╭{}╮", "─".repeat(box_width - 2)),
            mid_border: format!("├{}┤", "─".repeat(box_width - 2)),
            bot_border: format!("╰{}╯", "─".repeat(box_width - 2)),
        }
    }

    fn pad_line(&self, content: &str, emoji_count: usize) -> String {
        let visual_len = content.chars().count() + emoji_count;
        let padding = self.content_width.saturating_sub(visual_len);
        format!("│ {}{} │", content, " ".repeat(padding))
    }
}

/// Get the hook display name based on hook type.
fn hook_display_name(hook_type: Option<&str>) -> &'static str {
    match hook_type {
        Some("pre-push") => "📤 [Pre-push]",
        Some("commit-msg") => "📝 [Commit-msg]",
        _ => "🔍 [Pre-commit]",
    }
}

/// Get the skip command based on hook type.
fn hook_skip_command(hook_type: Option<&str>) -> &'static str {
    match hook_type {
        Some("pre-push") => "git push --no-verify",
        _ => "git commit --no-verify",
    }
}

/// Format the success box for hook output.
fn format_hook_success_box(
    result: &RunResult,
    hook_type: Option<&str>,
    ctx: &HookBoxContext,
) -> String {
    let hook_name = hook_display_name(hook_type);
    let mut output = String::new();
    output.push_str(&format!("{}\n", ctx.top_border.green()));
    let header = format!("{} Linthis {} Passed", "✓", hook_name);
    output.push_str(&format!("{}\n", ctx.pad_line(&header, 1).green()));
    output.push_str(&format!("{}\n", ctx.mid_border.green()));
    let checks_msg = if hook_type == Some("pre-push") {
        "All reviews finish"
    } else {
        "All checks passed!"
    };
    output.push_str(&format!("{}\n", ctx.pad_line(checks_msg, 0).green()));
    output.push_str(&format!("{}\n", ctx.pad_line("", 0)));
    output.push_str(&format!(
        "{}\n",
        ctx.pad_line(&format!("Files checked:   {:>3}", result.total_files), 0)
    ));
    output.push_str(&format!(
        "{}\n",
        ctx.pad_line(
            &format!("Files formatted: {:>3}", result.files_formatted),
            0
        )
    ));

    // Show formatted file list when files were actually formatted
    if result.files_formatted > 0 {
        for fr in &result.format_results {
            if !fr.changed {
                continue;
            }
            let filename = fr
                .file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            let lines_changed = fr
                .diff
                .as_ref()
                .map(|d| {
                    d.lines()
                        .filter(|l| l.starts_with('+') || l.starts_with('-'))
                        .count()
                })
                .unwrap_or(0);
            let detail = if lines_changed > 0 {
                format!("  {}  ({} lines)", filename, lines_changed)
            } else {
                format!("  {}", filename)
            };
            output.push_str(&format!("{}\n", ctx.pad_line(&detail, 0)));
        }
        output.push_str(&format!("{}\n", ctx.pad_line("", 0)));
        output.push_str(&format!(
            "{}\n",
            ctx.pad_line("Pre-change state saved in stash.", 0)
        ));
        output.push_str(&format!(
            "{}\n",
            ctx.pad_line("  git stash show -p  \u{2014} review format changes", 0)
        ));
        output.push_str(&format!(
            "{}\n",
            ctx.pad_line("  git stash drop     \u{2014} discard snapshot", 0)
        ));
    }

    output.push_str(&format!("{}", ctx.bot_border.green()));
    output.push_str(&format_hook_paths_footer(hook_type));
    output
}

/// Format a truncated issue line for box display.
fn format_hook_issue_line(issue: &LintIssue, content_width: usize) -> String {
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
    let location_max = (content_width / 3).clamp(10, 35);
    let location_display = if location.len() > location_max {
        format!("{}...", &location[..location_max.saturating_sub(3)])
    } else {
        location
    };
    let msg_prefix_len = 4;
    let max_msg_len = content_width.saturating_sub(msg_prefix_len + location_display.len());
    let msg = if issue.message.len() > max_msg_len {
        format!("{}...", &issue.message[..max_msg_len.saturating_sub(3)])
    } else {
        issue.message.clone()
    };
    format!(" {} {} {}", severity_char, location_display, msg)
}

/// Append the tip and skip hint sections for hook failure box.
fn append_hook_tip_section(
    output: &mut String,
    result: &RunResult,
    hook_type: Option<&str>,
    ctx: &HookBoxContext,
) {
    output.push_str(&format!(
        "{}\n",
        ctx.pad_line("Tip: To review and fix issues:", 0)
    ));
    for (cmd, desc) in fix_tip_lines() {
        let line = format!("  {:<36} : {}", cmd, desc);
        output.push_str(&format!("{}\n", ctx.pad_line(&line, 0)));
    }
    output.push_str(&format!("{}\n", ctx.pad_line("", 0)));

    let clang_tidy_count = result
        .issues
        .iter()
        .filter(|i| i.source.as_deref() == Some("clang-tidy"))
        .count();
    if clang_tidy_count >= 10 {
        output.push_str(&format!(
            "{}\n",
            ctx.pad_line(
                &format!(
                    "Too many clang-tidy issues ({})? Skip with:",
                    clang_tidy_count
                ),
                0
            )
        ));
        output.push_str(&format!(
            "{}\n",
            ctx.pad_line("  LINTHIS_SKIP_CLANG_TIDY=1", 0)
        ));
        output.push_str(&format!("{}\n", ctx.pad_line("", 0)));
    }

    let skip_command = hook_skip_command(hook_type);
    output.push_str(&format!("{}\n", ctx.pad_line("To skip this check:", 0)));
    output.push_str(&format!(
        "{}\n",
        ctx.pad_line(&format!("  {}", skip_command), 0)
    ));
}

pub fn format_result_hook_with_width(
    result: &RunResult,
    hook_type: Option<&str>,
    config_width: Option<u32>,
) -> String {
    let ctx = HookBoxContext::new(config_width);

    // exit_code is already computed based on fail_on config:
    //   0 = pass, 1 = error, 2 = warning, 3 = info, 4 = format error
    if result.exit_code == 0 {
        return format_hook_success_box(result, hook_type, &ctx);
    }

    let shown_issues = filter_issues_by_exit_code(&result.issues, result.exit_code);

    let hook_name = hook_display_name(hook_type);
    let mut output = String::new();

    // Header
    output.push_str(&format!("{}\n", ctx.top_border.red()));
    let header = format!("X Linthis {} Blocked", hook_name);
    output.push_str(&format!("{}\n", ctx.pad_line(&header, 1).red()));
    output.push_str(&format!("{}\n", ctx.mid_border.red()));

    // Summary + issue list
    append_issue_summary(&mut output, &shown_issues, &ctx);
    append_issue_list(&mut output, &shown_issues, &ctx);

    output.push_str(&format!("{}\n", ctx.mid_border.red()));

    // Tip and skip sections
    append_hook_tip_section(&mut output, result, hook_type, &ctx);

    output.push_str(&format!("{}", ctx.bot_border.red()));
    output.push_str(&format_hook_paths_footer(hook_type));

    output
}

/// Filter issues based on exit_code severity level.
fn filter_issues_by_exit_code(issues: &[LintIssue], exit_code: i32) -> Vec<&LintIssue> {
    issues
        .iter()
        .filter(|i| match exit_code {
            2 => i.severity == Severity::Error || i.severity == Severity::Warning,
            3 => true,
            _ => i.severity == Severity::Error,
        })
        .collect()
}

/// Append summary line (e.g. "2 errors, 1 warning in 3 files") to output.
fn append_issue_summary(output: &mut String, issues: &[&LintIssue], ctx: &HookBoxContext) {
    let error_count = issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count();
    let warning_count = issues
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .count();
    let info_count = issues
        .iter()
        .filter(|i| i.severity == Severity::Info)
        .count();
    let files_with_issues = {
        let mut files = std::collections::HashSet::new();
        for i in issues {
            files.insert(&i.file_path);
        }
        files.len()
    };

    let mut parts = Vec::new();
    if error_count > 0 {
        parts.push(format!(
            "{} error{}",
            error_count,
            if error_count == 1 { "" } else { "s" }
        ));
    }
    if warning_count > 0 {
        parts.push(format!(
            "{} warning{}",
            warning_count,
            if warning_count == 1 { "" } else { "s" }
        ));
    }
    if info_count > 0 {
        parts.push(format!("{} info", info_count));
    }
    let summary = format!(
        "{} in {} file{}",
        parts.join(", "),
        files_with_issues,
        if files_with_issues == 1 { "" } else { "s" },
    );
    output.push_str(&format!("{}\n", ctx.pad_line(&summary, 0)));
    output.push_str(&format!("{}\n", ctx.pad_line("", 0)));
}

/// Append truncated issue list (max 8 items) to output.
fn append_issue_list(output: &mut String, issues: &[&LintIssue], ctx: &HookBoxContext) {
    let max_issues = 8;
    for issue in issues.iter().take(max_issues) {
        let line_content = format_hook_issue_line(issue, ctx.content_width);
        output.push_str(&format!("{}\n", ctx.pad_line(&line_content, 0)));
    }

    if issues.len() > max_issues {
        let remaining = issues.len() - max_issues;
        let more_line = format!(
            " ... and {} more issue{}",
            remaining,
            if remaining == 1 { "" } else { "s" }
        );
        output.push_str(&format!("{}\n", ctx.pad_line(&more_line, 0)));
    }
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
/// Format a single review issue line with truncation for box display.
fn format_review_issue_line(issue: &crate::review::ReviewIssue, content_width: usize) -> String {
    use crate::review::Severity;

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
    let location_max = (content_width / 3).clamp(10, 35);
    let location_display = if location.len() > location_max {
        format!("{}...", &location[..location_max.saturating_sub(3)])
    } else {
        location
    };
    let msg_prefix_len = 4;
    let max_msg_len = content_width.saturating_sub(msg_prefix_len + location_display.len());
    let msg = if issue.message.len() > max_msg_len {
        format!("{}...", &issue.message[..max_msg_len.saturating_sub(3)])
    } else {
        issue.message.clone()
    };
    format!(" {} {} {}", severity_char, location_display, msg)
}

/// Append the review issues section to the output.
fn append_review_issues(
    output: &mut String,
    issues: &[crate::review::ReviewIssue],
    content_width: usize,
    pad_line: &dyn Fn(&str, usize) -> String,
) {
    if issues.is_empty() {
        return;
    }
    output.push_str(&format!("{}\n", pad_line("", 0)));
    let max_issues = 6;
    for issue in issues.iter().take(max_issues) {
        let line_content = format_review_issue_line(issue, content_width);
        output.push_str(&format!("{}\n", pad_line(&line_content, 0)));
    }
    if issues.len() > max_issues {
        let more = format!(
            " ... and {} more issue{}",
            issues.len() - max_issues,
            if issues.len() - max_issues == 1 {
                ""
            } else {
                "s"
            }
        );
        output.push_str(&format!("{}\n", pad_line(&more, 0)));
    }
}

pub fn format_review_box(result: &crate::review::ReviewResult) -> String {
    use crate::review::Assessment;

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

    // Header with color based on success/failure
    let color_border = |s: &str, success: bool| -> String {
        if success {
            s.green().to_string()
        } else {
            s.red().to_string()
        }
    };
    output.push_str(&format!("{}\n", color_border(&top_border, is_success)));
    output.push_str(&format!(
        "{}\n",
        color_border(&pad_line(&header, 1), is_success)
    ));
    output.push_str(&format!("{}\n", color_border(&mid_border, is_success)));

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

    // Issues section
    append_review_issues(&mut output, &result.issues, content_width, &pad_line);

    // Bottom border
    output.push_str(&color_border(&bot_border, is_success));

    output
}

/// Format a commit-msg hook result box (passed or blocked).
///
/// Returns the box as a `String` without trailing newline.
/// The footer (hook file paths) is handled by the caller.
pub fn format_cmsg_result(passed: bool, first_line: &str) -> String {
    if passed {
        let mut out = String::new();
        out.push_str(&format!(
            "{}\n",
            "╭────────────────────────────────────────╮".green()
        ));
        out.push_str(&format!(
            "{}\n",
            "│ ✓ Linthis 📝 [Commit-msg] Passed       │".green()
        ));
        out.push_str(&format!(
            "{}\n",
            "├────────────────────────────────────────┤".green()
        ));
        out.push_str(&format!(
            "{}\n",
            "│ Commit message is valid                │".green()
        ));
        out.push_str(&format!(
            "{}",
            "╰────────────────────────────────────────╯".green()
        ));
        out
    } else {
        let mut out = String::new();
        out.push_str(&format!(
            "{}\n",
            "╭────────────────────────────────────────╮".red()
        ));
        out.push_str(&format!(
            "{}\n",
            "│ X Linthis 📝 [Commit-msg] Blocked      │".red()
        ));
        out.push_str(&format!(
            "{}\n",
            "├────────────────────────────────────────┤".red()
        ));
        out.push_str(&format!(
            "{}\n",
            "│ Validation Failed!                     │".red()
        ));
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
        out.push_str(&format!(
            "{}\n",
            "├────────────────────────────────────────┤".red()
        ));
        out.push_str("│ To skip this check:                    │\n");
        out.push_str("│   git commit --no-verify               │\n");
        out.push_str(&format!(
            "{}",
            "╰────────────────────────────────────────╯".red()
        ));
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
