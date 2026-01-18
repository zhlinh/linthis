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
use linthis::utils::types::RunResult;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Handle report subcommands
pub fn handle_report_command(action: ReportCommands) -> ExitCode {
    match action {
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
