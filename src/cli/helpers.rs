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
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use super::commands::Cli;
use linthis::ai::provider::{
    detect_available_providers, try_fallback_provider, AiProviderKind, ALL_AI_PROVIDERS,
};
use linthis::LintIssue;

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

/// Resolve AI provider with priority: command line > env var > config > default.
///
/// After resolving, checks if the provider is available. If not, tries to find
/// a compatible fallback (e.g., `claude` API → `claude-cli`, or vice versa)
/// and prints a warning message.
///
/// Priority order:
/// 1. CLI argument (--provider)
/// 2. Environment variable (LINTHIS_AI_PROVIDER)
/// 3. Config file (project or global)
/// 4. Default ("claude")
pub fn resolve_ai_provider(cli_value: Option<&str>, config_value: Option<&str>) -> String {
    // Priority 1: command line argument
    let resolved = if let Some(value) = cli_value {
        value.to_string()
    } else if let Ok(value) = std::env::var("LINTHIS_AI_PROVIDER") {
        // Priority 2: environment variable
        if !value.is_empty() {
            value
        } else {
            resolve_default_provider(config_value)
        }
    } else {
        resolve_default_provider(config_value)
    };

    // Try to parse the resolved provider and check availability with fallback
    if let Ok(kind) = resolved.parse::<AiProviderKind>() {
        if let Some((fallback_kind, message)) = try_fallback_provider(&kind) {
            eprintln!("{} {}", "⚠ Warning:".yellow().bold(), message);
            return fallback_kind.cli_name().to_string();
        }
    }

    resolved
}

fn resolve_default_provider(config_value: Option<&str>) -> String {
    // Priority 3: config file value
    if let Some(value) = config_value {
        if !value.is_empty() {
            return value.to_string();
        }
    }
    // Priority 4: default
    "claude".to_string()
}

/// Print hint about how to enter interactive fix mode
pub fn print_fix_hint(issues: &[LintIssue]) {
    // Check if there are many clang-tidy issues
    let clang_tidy_count = issues
        .iter()
        .filter(|i| i.source.as_deref() == Some("clang-tidy"))
        .count();

    if clang_tidy_count >= 10 {
        eprintln!();
        eprintln!(
            "  {} Found {} clang-tidy issues. To skip clang-tidy checks:",
            "Tip:".yellow().bold(),
            clang_tidy_count,
        );
        eprintln!(
            "       {}",
            "LINTHIS_SKIP_CLANG_TIDY=1 linthis".yellow()
        );
    }

    eprintln!();
    eprintln!(
        "  {} To review and fix issues interactively:",
        "Tip:".cyan().bold()
    );
    eprintln!("       {}      - load last result and fix", "linthis fix".cyan());
    eprintln!(
        "       {} - AI-powered fix suggestions ({} to choose agent)",
        "linthis fix --ai".cyan(),
        "--provider <name>".dimmed()
    );
    eprintln!(
        "       {} - {} auto-fix via AI agent",
        "linthis fix --auto".cyan(),
        "(dangerously)".red().bold()
    );
}

/// Interactive AI provider selection menu.
///
/// Shows available providers (detected first), prompts user to choose one.
/// Returns the selected provider name string, or None if cancelled.
pub fn select_ai_provider_interactive() -> Option<String> {
    let providers = detect_available_providers();

    // Reorder: available first, then unavailable
    let mut ordered: Vec<_> = providers.iter().filter(|(_, avail)| *avail).collect();
    let unavailable: Vec<_> = providers.iter().filter(|(_, avail)| !*avail).collect();
    ordered.extend(unavailable);

    eprintln!("{}", "Select AI provider:".cyan().bold());
    eprintln!();
    for (i, (kind, available)) in ordered.iter().enumerate() {
        let (_, name, desc) = ALL_AI_PROVIDERS
            .iter()
            .find(|(k, _, _)| k == kind)
            .unwrap();
        if *available {
            eprintln!(
                "  {} {}. {} - {}{}",
                "\u{2713}".green(),
                i + 1,
                name,
                desc,
                " (available)".cyan()
            );
        } else {
            eprintln!("    {}. {} - {}", i + 1, name, desc);
        }
    }
    let cancel_num = ordered.len() + 1;
    eprintln!("    {}. Cancel", cancel_num);
    eprintln!();
    eprint!("Choose [1]: ");
    io::stderr().flush().ok();

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).ok();
    let choice = choice.trim();

    // Default to 1 (first provider) if empty
    let num: usize = if choice.is_empty() {
        1
    } else if let Ok(n) = choice.parse() {
        n
    } else {
        return None;
    };

    if num == cancel_num || num == 0 {
        return None;
    }

    if num >= 1 && num <= ordered.len() {
        let (kind, _) = ordered[num - 1];
        let (_, name, _) = ALL_AI_PROVIDERS
            .iter()
            .find(|(k, _, _)| k == kind)
            .unwrap();
        Some(name.to_string())
    } else {
        None
    }
}
