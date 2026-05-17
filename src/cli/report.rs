// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Report generation CLI handlers.

use super::commands::ReportCommands;
use colored::Colorize;
use linthis::reports::{
    analyze_trends, get_last_result, load_result_from_file, ConsistencyAnalysis, HtmlReportOptions,
    ReportStatistics,
};
use linthis::utils::get_project_root;
use linthis::utils::output::{format_issue_human, format_summary_human};
use linthis::utils::types::{RunResult, Severity};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Handle report subcommands
pub fn handle_report_command(action: ReportCommands) -> ExitCode {
    match action {
        ReportCommands::Show {
            source,
            format,
            severity,
            output,
            open,
            limit,
            compact,
            with_trends,
            trend_count,
        } => match format.as_str() {
            "html" => handle_html_report(&source, output, open, with_trends, trend_count),
            _ => handle_show_report(&source, limit, compact, &severity, &format),
        },
        ReportCommands::Stats { source, format } => handle_stats_report(&source, &format),
        ReportCommands::Count { source } => handle_count_report(&source),
        ReportCommands::Trends { count, format } => handle_trends_report(count, &format),
        ReportCommands::Consistency { source, format } => {
            handle_consistency_report(&source, &format)
        }
    }
}

/// Load RunResult from source specification
fn load_result_from_source(source: &str) -> Option<RunResult> {
    let project_root = get_project_root();

    match source {
        "last" => get_last_result(&project_root).map(|(_, result)| result),
        "current" => {
            // "current" means run linting first - but for now, fall back to last
            eprintln!(
                "{}: 'current' source not yet supported, using 'last' result",
                "Note".yellow()
            );
            get_last_result(&project_root).map(|(_, result)| result)
        }
        path => {
            // Treat as file path
            let path = Path::new(path);
            if path.exists() {
                load_result_from_file(path)
            } else {
                // Try relative to global check/result/
                let result_path = linthis::utils::get_result_dir().join(path);
                if result_path.exists() {
                    load_result_from_file(&result_path)
                } else {
                    None
                }
            }
        }
    }
}

/// Parse severity filter string into a set of allowed severities.
fn parse_severity_filter(severity: &str) -> Vec<Severity> {
    if severity == "all" {
        return vec![Severity::Error, Severity::Warning, Severity::Info];
    }
    severity
        .split(',')
        .filter_map(|s| match s.trim() {
            "error" => Some(Severity::Error),
            "warning" => Some(Severity::Warning),
            "info" => Some(Severity::Info),
            _ => None,
        })
        .collect()
}

/// Filter issues by severity and apply a display limit.
fn filter_and_limit_issues<'a>(
    issues: &'a [linthis::utils::types::LintIssue],
    severity: &str,
    limit: usize,
) -> (Vec<&'a linthis::utils::types::LintIssue>, usize) {
    let allowed = parse_severity_filter(severity);
    let filtered: Vec<_> = issues
        .iter()
        .filter(|i| allowed.contains(&i.severity))
        .collect();

    let total = filtered.len();
    let display: Vec<_> = if limit > 0 {
        filtered.into_iter().take(limit).collect()
    } else {
        filtered
    };
    (display, total)
}

/// Output a JSON representation of the result with filtered issues.
fn print_json_report(
    result: &RunResult,
    filtered_issues: Vec<&linthis::utils::types::LintIssue>,
    limit: usize,
) {
    let mut filtered_result = result.clone();
    filtered_result.issues = filtered_issues.into_iter().cloned().collect();
    if limit > 0 {
        filtered_result.issues.truncate(limit);
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&filtered_result).unwrap_or_default()
    );
}

/// Print a single numbered issue line in either compact or full format.
fn print_numbered_issue(
    issue: &linthis::utils::types::LintIssue,
    idx: usize,
    prefix: &str,
    compact: bool,
    is_error: bool,
) {
    let lang_tag = issue
        .language
        .map(|l| format!("[{}]", l.name()))
        .unwrap_or_default();
    let tool_tag = issue
        .source
        .as_ref()
        .map(|s| format!("[{}]", s))
        .unwrap_or_default();

    if compact {
        let location = if let Some(col) = issue.column {
            format!("{}:{}:{}", issue.file_path.display(), issue.line, col)
        } else {
            format!("{}:{}", issue.file_path.display(), issue.line)
        };
        if is_error {
            println!(
                "{}{}{} {}: {}",
                format!("[{}{}]", prefix, idx + 1).red().bold(),
                lang_tag.red(),
                tool_tag.red(),
                location.bold(),
                issue.message
            );
        } else {
            println!(
                "{}{}{} {}: {}",
                format!("[{}{}]", prefix, idx + 1).yellow().bold(),
                lang_tag.yellow(),
                tool_tag.yellow(),
                location.bold(),
                issue.message
            );
        }
    } else if is_error {
        println!(
            "{}{}{} {}",
            format!("[{}{}]", prefix, idx + 1).red().bold(),
            lang_tag.red(),
            tool_tag.red(),
            format_issue_human(issue)
        );
    } else {
        println!(
            "{}{}{} {}",
            format!("[{}{}]", prefix, idx + 1).yellow().bold(),
            lang_tag.yellow(),
            tool_tag.yellow(),
            format_issue_human(issue)
        );
    }
}

/// Show lint results in human-readable or JSON format
fn handle_show_report(
    source: &str,
    limit: usize,
    compact: bool,
    severity: &str,
    format: &str,
) -> ExitCode {
    let mut result = match load_result_from_source(source) {
        Some(r) => r,
        None => {
            eprintln!(
                "{}: No results found. Run 'linthis -c' first.",
                "Error".red()
            );
            return ExitCode::from(1);
        }
    };

    // Merge security/complexity findings into unified issues list
    result.merge_all_check_issues();

    // Apply rule filter from .linthisignore so disabled rules are hidden here too
    {
        let project_root = linthis::utils::get_project_root();
        let linthisignore = linthis::utils::linthisignore::LinthisIgnore::load(&project_root);
        let config = linthis::config::Config::load_merged(&project_root);
        let mut rules = config.rules;
        rules.disable.extend(linthisignore.rule_codes);
        let filter = linthis::rules::RuleFilter::from_config(&rules);
        result.issues = filter.filter_issues(std::mem::take(&mut result.issues));
    }

    let (display_issues, total_filtered) = filter_and_limit_issues(&result.issues, severity, limit);
    let displayed_count = display_issues.len();

    // Handle JSON format
    if format == "json" {
        let (json_issues, _) = filter_and_limit_issues(&result.issues, severity, 0);
        print_json_report(&result, json_issues, limit);
        return ExitCode::SUCCESS;
    }

    // Human-readable output
    if display_issues.is_empty() {
        println!(
            "{} No issues found matching severity filter '{}'.",
            "✓".green(),
            severity
        );
        return ExitCode::SUCCESS;
    }

    // Separate errors, warnings, info for numbered output
    for (idx, issue) in display_issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .enumerate()
    {
        print_numbered_issue(issue, idx, "E", compact, true);
    }
    for (idx, issue) in display_issues
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .enumerate()
    {
        print_numbered_issue(issue, idx, "W", compact, false);
    }
    for (idx, issue) in display_issues
        .iter()
        .filter(|i| i.severity == Severity::Info)
        .enumerate()
    {
        print_numbered_issue(issue, idx, "I", compact, false);
    }

    // Show summary
    println!();
    if limit > 0 && total_filtered > displayed_count {
        println!(
            "{} Showing {} of {} issues (use -n 0 to show all)",
            "→".cyan(),
            displayed_count,
            total_filtered
        );
        println!();
    }
    println!("{}", format_summary_human(&result));

    ExitCode::SUCCESS
}

/// Generate HTML report
fn handle_html_report(
    source: &str,
    output: Option<PathBuf>,
    open: bool,
    with_trends: bool,
    trend_count: usize,
) -> ExitCode {
    let result = match load_result_from_source(source) {
        Some(r) => r,
        None => {
            eprintln!(
                "{}: No lint results found. Run 'linthis -c' first.",
                "Error".red()
            );
            return ExitCode::from(1);
        }
    };

    // Build options
    let mut options = HtmlReportOptions::default();
    if with_trends {
        options.include_trends = true;
        let project_root = get_project_root();
        let trends = analyze_trends(&project_root, trend_count);
        if !trends.data_points.is_empty() {
            options.trends = Some(trends);
        }
    }

    // Generate HTML
    let html = linthis::reports::generate_html_report(&result, &options);

    // Determine output path
    let output_path = match output {
        Some(p) => p,
        None => {
            let project_root = get_project_root();
            let reports_dir = project_root.join(".linthis").join("reports");
            if !reports_dir.exists() {
                if let Err(e) = fs::create_dir_all(&reports_dir) {
                    eprintln!(
                        "{}: Failed to create reports directory: {}",
                        "Error".red(),
                        e
                    );
                    return ExitCode::from(1);
                }
            }
            let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
            reports_dir.join(format!("report-{}.html", timestamp))
        }
    };

    // Write HTML file
    match fs::write(&output_path, html) {
        Ok(()) => {
            println!("{} HTML report generated", "✓".green());
            println!("  Location: {}", output_path.display());

            println!();
            println!("  {} total files", result.total_files);
            println!("  {} files with issues", result.files_with_issues);
            println!("  {} total issues", result.issues.len());

            if with_trends && options.trends.is_some() {
                println!("  {} trend data points included", trend_count);
            }

            // Open in browser if requested
            if open {
                open_in_browser(&output_path);
            }

            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}: Failed to write report: {}", "Error".red(), e);
            ExitCode::from(1)
        }
    }
}

/// Open a file in the system default browser.
fn open_in_browser(path: &Path) {
    let cmd = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "cmd"
    } else {
        "xdg-open"
    };

    let result = if cfg!(target_os = "windows") {
        std::process::Command::new(cmd)
            .args(["/c", "start", "", &path.display().to_string()])
            .spawn()
    } else {
        std::process::Command::new(cmd).arg(path).spawn()
    };

    match result {
        Ok(_) => println!("  Opened in browser"),
        Err(e) => eprintln!("  {}: Failed to open browser: {}", "Warning".yellow(), e),
    }
}

/// Show statistics from lint results
fn handle_stats_report(source: &str, format: &str) -> ExitCode {
    let result = match load_result_from_source(source) {
        Some(r) => r,
        None => {
            eprintln!(
                "{}: No lint results found. Run 'linthis -c' first.",
                "Error".red()
            );
            return ExitCode::from(1);
        }
    };

    let stats = ReportStatistics::from_run_result(&result);

    match format {
        "json" => {
            println!("{}", stats.format_json());
        }
        _ => {
            println!("{}", stats.format_human());
        }
    }

    ExitCode::SUCCESS
}

/// Print issue counts on a single whitespace-separated line:
/// `<errors> <warnings> <info> <files_with_issues>`
/// Shell-friendly format for `read` in hook scripts (no jq needed).
/// Returns exit 0 with "0 0 0 0" if no results found (fail-safe default).
fn handle_count_report(source: &str) -> ExitCode {
    let result = match load_result_from_source(source) {
        Some(r) => r,
        None => {
            println!("0 0 0 0");
            return ExitCode::SUCCESS;
        }
    };

    let stats = ReportStatistics::from_run_result(&result);
    println!(
        "{} {} {} {}",
        stats.severity_counts.errors,
        stats.severity_counts.warnings,
        stats.severity_counts.info,
        stats.summary.files_with_issues,
    );

    ExitCode::SUCCESS
}

/// Analyze code quality trends
fn handle_trends_report(count: usize, format: &str) -> ExitCode {
    let project_root = get_project_root();
    let trends = analyze_trends(&project_root, count);

    if trends.data_points.is_empty() {
        eprintln!(
            "{}: No historical results found in check/result/",
            "Warning".yellow()
        );
        eprintln!("  Run 'linthis -c' multiple times to generate trend data.");
        return ExitCode::SUCCESS;
    }

    match format {
        "json" => {
            println!("{}", trends.format_json());
        }
        _ => {
            println!("{}", trends.format_human());
        }
    }

    ExitCode::SUCCESS
}

/// Analyze code consistency
fn handle_consistency_report(source: &str, format: &str) -> ExitCode {
    let result = match load_result_from_source(source) {
        Some(r) => r,
        None => {
            eprintln!(
                "{}: No lint results found. Run 'linthis -c' first.",
                "Error".red()
            );
            return ExitCode::from(1);
        }
    };

    let analysis = ConsistencyAnalysis::from_run_result(&result);

    match format {
        "json" => {
            println!("{}", analysis.format_json());
        }
        _ => {
            println!("{}", analysis.format_human());
        }
    }

    ExitCode::SUCCESS
}
