// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! CLI helper functions.
//!
//! This module contains utility functions used by the CLI.

use colored::Colorize;
use std::path::PathBuf;
use std::process::ExitCode;

use super::commands::Cli;

/// Run benchmark comparing ruff vs flake8+black for Python
pub fn run_benchmark(cli: &Cli) -> ExitCode {
    use linthis::benchmark::{format_benchmark_table, run_python_benchmark};
    use linthis::utils::walker::{walk_paths, WalkerConfig};
    use linthis::Language;

    println!(
        "{}",
        "Running Python linting/formatting benchmark...".cyan()
    );
    println!("Comparing ruff vs flake8+black\n");

    // Get paths to scan (default to current directory if empty)
    let paths = if cli.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        cli.paths.clone()
    };

    // Configure walker for Python files only
    let walker_config = WalkerConfig {
        exclude_patterns: cli.exclude.clone().unwrap_or_default(),
        languages: vec![Language::Python],
        ..Default::default()
    };

    // Collect Python files
    let (files, _) = walk_paths(&paths, &walker_config);

    if files.is_empty() {
        println!("{}", "No Python files found to benchmark.".yellow());
        return ExitCode::SUCCESS;
    }

    println!("Found {} Python files", files.len());

    // Convert to Path references
    let file_refs: Vec<&std::path::Path> = files.iter().map(|p| p.as_path()).collect();

    // Run benchmark
    let comparison = run_python_benchmark(&file_refs);

    // Output results
    println!("{}", format_benchmark_table(&comparison));

    ExitCode::SUCCESS
}

/// Strip ANSI escape codes from a string for plain text output
pub fn strip_ansi_codes(s: &str) -> String {
    let ansi_regex = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    ansi_regex.replace_all(s, "").to_string()
}

/// Find the most recent result file in <project_root>/.linthis/result/
pub fn find_latest_result_file() -> Option<PathBuf> {
    use std::fs;

    // Use project root to find .linthis directory
    let project_root = linthis::utils::get_project_root();
    let result_dir = project_root.join(".linthis").join("result");
    if !result_dir.exists() {
        return None;
    }

    let entries = fs::read_dir(&result_dir).ok()?;
    let mut result_files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with("result-")
                && (name.ends_with(".json") || name.ends_with(".txt"))
        })
        .collect();

    if result_files.is_empty() {
        return None;
    }

    // Sort by modification time, newest first
    result_files.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|m| m.modified()).ok();
        let b_time = b.metadata().and_then(|m| m.modified()).ok();
        b_time.cmp(&a_time)
    });

    Some(result_files[0].path())
}

/// Resolve AI provider with priority: command line > env var > default
///
/// The environment variable is `LINTHIS_AI_PROVIDER`.
/// Default value is "claude".
pub fn resolve_ai_provider(cli_value: Option<&str>) -> String {
    // Priority 1: command line argument
    if let Some(value) = cli_value {
        return value.to_string();
    }

    // Priority 2: environment variable
    if let Ok(value) = std::env::var("LINTHIS_AI_PROVIDER") {
        if !value.is_empty() {
            return value;
        }
    }

    // Priority 3: default
    "claude".to_string()
}

/// Print hint about how to enter interactive fix mode
pub fn print_fix_hint() {
    eprintln!();
    eprintln!(
        "  {} To review and fix issues interactively:",
        "Tip:".cyan().bold()
    );
    eprintln!("       {}      - load last result and fix", "linthis fix".cyan());
    eprintln!("       {} - AI-powered fix suggestions", "linthis fix --ai".cyan());
}
