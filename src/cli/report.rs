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
    analyze_trends, get_last_result, load_result_from_file, ConsistencyAnalysis,
    HtmlReportOptions, ReportStatistics,
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
            limit,
            compact,
            errors_only,
            warnings_only,
            format,
        } => handle_show_report(&source, limit, compact, errors_only, warnings_only, &format),
        ReportCommands::Html {
            source,
            output,
            with_trends,
            trend_count,
        } => handle_html_report(&source, output, with_trends, trend_count),
        ReportCommands::Stats { source, format } => handle_stats_report(&source, &format),
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
                // Try relative to .linthis/result/
                let result_path = project_root.join(".linthis").join("result").join(path);
                if result_path.exists() {
                    load_result_from_file(&result_path)
                } else {
                    None
                }
            }
        }
    }
}

/// Show lint results in human-readable format
fn handle_show_report(
    source: &str,
    limit: usize,
    compact: bool,
    errors_only: bool,
    warnings_only: bool,
    format: &str,
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

    // Filter issues based on flags
    let filtered_issues: Vec<_> = result
        .issues
        .iter()
        .filter(|i| {
            if errors_only {
                i.severity == Severity::Error
            } else if warnings_only {
                i.severity == Severity::Warning
            } else {
                true
            }
        })
        .collect();

    // Apply limit
    let display_issues: Vec<_> = if limit > 0 {
        filtered_issues.iter().take(limit).collect()
    } else {
        filtered_issues.iter().collect()
    };

    let total_filtered = filtered_issues.len();
    let displayed_count = display_issues.len();

    // Handle JSON format
    if format == "json" {
        // Create a modified result with filtered issues for JSON output
        let mut filtered_result = result.clone();
        filtered_result.issues = filtered_issues.into_iter().cloned().collect();
        if limit > 0 {
            filtered_result.issues.truncate(limit);
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&filtered_result).unwrap_or_default()
        );
        return ExitCode::SUCCESS;
    }

    // Human-readable output
    if display_issues.is_empty() {
        let filter_desc = if errors_only {
            "errors"
        } else if warnings_only {
            "warnings"
        } else {
            "issues"
        };
        println!("{} No {} found in result.", "✓".green(), filter_desc);
        return ExitCode::SUCCESS;
    }

    // Separate errors and warnings for numbered output
    let errors: Vec<_> = display_issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .collect();
    let warnings: Vec<_> = display_issues
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .collect();

    // Output errors
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

        if compact {
            // Compact format: [E1] file:line:col message
            let location = if let Some(col) = issue.column {
                format!("{}:{}:{}", issue.file_path.display(), issue.line, col)
            } else {
                format!("{}:{}", issue.file_path.display(), issue.line)
            };
            println!(
                "{}{}{} {}: {}",
                format!("[E{}]", idx + 1).red().bold(),
                lang_tag.red(),
                tool_tag.red(),
                location.bold(),
                issue.message
            );
        } else {
            // Full format with code context
            println!(
                "{}{}{} {}",
                format!("[E{}]", idx + 1).red().bold(),
                lang_tag.red(),
                tool_tag.red(),
                format_issue_human(issue)
            );
        }
    }

    // Output warnings
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

        if compact {
            // Compact format: [W1] file:line:col message
            let location = if let Some(col) = issue.column {
                format!("{}:{}:{}", issue.file_path.display(), issue.line, col)
            } else {
                format!("{}:{}", issue.file_path.display(), issue.line)
            };
            println!(
                "{}{}{} {}: {}",
                format!("[W{}]", idx + 1).yellow().bold(),
                lang_tag.yellow(),
                tool_tag.yellow(),
                location.bold(),
                issue.message
            );
        } else {
            // Full format with code context
            println!(
                "{}{}{} {}",
                format!("[W{}]", idx + 1).yellow().bold(),
                lang_tag.yellow(),
                tool_tag.yellow(),
                format_issue_human(issue)
            );
        }
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
                    eprintln!("{}: Failed to create reports directory: {}", "Error".red(), e);
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

            // Show summary
            println!();
            println!("  {} total files", result.total_files);
            println!("  {} files with issues", result.files_with_issues);
            println!("  {} total issues", result.issues.len());

            if with_trends && options.trends.is_some() {
                println!("  {} trend data points included", trend_count);
            }

            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}: Failed to write report: {}", "Error".red(), e);
            ExitCode::from(1)
        }
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

/// Analyze code quality trends
fn handle_trends_report(count: usize, format: &str) -> ExitCode {
    let project_root = get_project_root();
    let trends = analyze_trends(&project_root, count);

    if trends.data_points.is_empty() {
        eprintln!(
            "{}: No historical results found in .linthis/result/",
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
