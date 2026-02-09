// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Fix subcommand handler for reviewing and fixing lint issues.
//!
//! This module handles the `fix` subcommand, supporting:
//! - Loading results from previous runs
//! - Running check/format first then fixing
//! - Interactive review mode
//! - AI-powered fix suggestions (batch and single-file modes)

use colored::Colorize;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::cli::helpers::{find_latest_result_file, resolve_ai_provider};
use crate::cli::recheck::{
    print_recheck_footer, print_recheck_header, print_recheck_summary, recheck_modified_files,
};
use linthis::ai::{AiProvider, AiProviderConfig, AiProviderKind, AiSuggester, SuggestionOptions};
use linthis::config::Config;
use linthis::interactive::{run_ai_fix_all, run_interactive, AiFixConfig};
use linthis::utils::types::LintIssue;

/// Options for the fix subcommand
pub struct FixCommandOptions {
    /// Source of lint results: "last" or a file path
    pub source: String,
    /// Run lint check first
    pub check: bool,
    /// Run format only first
    pub format_only: bool,
    /// Enable AI mode
    pub ai: bool,
    /// AI provider name
    pub provider: Option<String>,
    /// AI model name
    pub model: Option<String>,
    /// Max suggestions per issue
    pub max_suggestions: usize,
    /// Auto-apply suggestions
    pub accept_all: bool,
    /// Number of parallel jobs (0 = sequential)
    pub jobs: usize,
    /// Target specific file (for single-file AI mode)
    pub file: Option<PathBuf>,
    /// Target specific line
    pub line: Option<u32>,
    /// Issue message for context
    pub message: Option<String>,
    /// Rule ID for context
    pub rule: Option<String>,
    /// Output format (human, json, diff)
    pub output: String,
    /// Include code context in output
    pub with_context: bool,
    /// Verbose output
    pub verbose: bool,
    /// Quiet mode
    pub quiet: bool,
    /// Undo last fix (restore from backup)
    pub undo: bool,
    /// List available backups
    pub list_backups: bool,
}

/// Handle the fix subcommand
pub fn handle_fix_command(options: FixCommandOptions) -> ExitCode {
    // Handle --list-backups
    if options.list_backups {
        return handle_list_backups();
    }

    // Handle --undo
    if options.undo {
        return handle_undo_fix(&options.source);
    }

    // Load config for AI settings
    let project_root = linthis::utils::get_project_root();
    let config = Config::load_merged(&project_root);

    // If --check or --format-only is specified, run lint first
    if options.check || options.format_only {
        return handle_fix_with_lint(&options, &config);
    }

    // If single file mode (--ai with -i/--include and --line)
    if options.ai && options.file.is_some() && options.line.is_some() {
        return handle_single_file_ai_fix(&options, &config);
    }

    // Load from result file and fix
    handle_fix_from_result(&options, &config)
}

/// Handle fix with running lint first
fn handle_fix_with_lint(options: &FixCommandOptions, config: &Config) -> ExitCode {
    use linthis::{run, RunMode, RunOptions};

    let mode = if options.format_only {
        RunMode::FormatOnly
    } else {
        RunMode::CheckOnly
    };

    if !options.quiet {
        println!(
            "{} Running {} first...",
            "→".cyan(),
            if options.format_only { "format" } else { "check" }
        );
    }

    // Run lint/format
    let run_options = RunOptions {
        paths: vec![PathBuf::from(".")],
        mode,
        languages: vec![],
        exclude_patterns: vec![],
        verbose: options.verbose,
        quiet: options.quiet,
        plugins: vec![],
        no_cache: false,
        config_resolver: None,
    };

    match run(&run_options) {
        Ok(result) => {
            if result.issues.is_empty() {
                if !options.quiet {
                    println!("{}", "No issues found.".green());
                }
                return ExitCode::SUCCESS;
            }

            if !options.quiet {
                println!(
                    "  Found {} issue{}\n",
                    result.issues.len(),
                    if result.issues.len() == 1 { "" } else { "s" }
                );
            }

            // Create backup before making changes
            let files_to_backup = collect_files_from_issues(&result.issues);
            let _backup_id = create_backup(&files_to_backup, "linthis fix -c", options.quiet);
            if !options.quiet && !files_to_backup.is_empty() {
                println!();
            }

            // Enter fix mode
            let (modified_files, fixed_count) = if options.ai {
                let provider = resolve_ai_provider(
                    options.provider.as_deref(),
                    config.ai.provider.as_deref(),
                );
                let ai_config = AiFixConfig::with_provider(&provider)
                    .with_model(options.model.clone())
                    .with_accept_all(options.accept_all)
                    .with_verbose(options.verbose)
                    .with_parallel(options.jobs);

                let ai_result = run_ai_fix_all(&result, &ai_config);
                (ai_result.modified_files, ai_result.applied)
            } else {
                let interactive_result = run_interactive(&result);
                let count = interactive_result.edited + interactive_result.ignored;
                (interactive_result.modified_files, count)
            };

            // Recheck modified files
            if !modified_files.is_empty() {
                print_recheck_header();
                let recheck_result =
                    recheck_modified_files(&modified_files, &result.issues, options.quiet, options.verbose);
                print_recheck_summary(&recheck_result, fixed_count);
                print_recheck_footer();
            }

            ExitCode::from(result.exit_code as u8)
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            ExitCode::from(2)
        }
    }
}

/// Maximum number of AI fix iterations to prevent infinite loops
const MAX_AI_FIX_ITERATIONS: usize = 100;

/// Handle fix by loading from result file
fn handle_fix_from_result(options: &FixCommandOptions, config: &Config) -> ExitCode {
    let path = if options.source == "last" {
        match find_latest_result_file() {
            Some(p) => p,
            None => {
                let project_root = linthis::utils::get_project_root();
                let result_dir = project_root.join(".linthis").join("result");
                eprintln!(
                    "{}: No result files found in {}",
                    "Error".red(),
                    result_dir.display()
                );
                eprintln!(
                    "  Run {} first to generate a result file.",
                    "linthis -c".cyan()
                );
                return ExitCode::from(1);
            }
        }
    } else {
        PathBuf::from(&options.source)
    };

    if !path.exists() {
        eprintln!(
            "{}: Result file not found: {}",
            "Error".red(),
            path.display()
        );
        return ExitCode::from(1);
    }

    if !options.quiet {
        println!("{} Loading results from: {}", "→".cyan(), path.display());
    }

    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<linthis::utils::types::RunResult>(&content) {
            Ok(result) => {
                if result.issues.is_empty() {
                    if !options.quiet {
                        println!("{}", "No issues in the saved result.".green());
                    }
                    return ExitCode::SUCCESS;
                }

                if !options.quiet {
                    println!(
                        "  Found {} issue{} from previous run\n",
                        result.issues.len(),
                        if result.issues.len() == 1 { "" } else { "s" }
                    );
                }

                // For AI mode with accept_all, use iterative fix loop
                if options.ai && options.accept_all {
                    return run_ai_fix_loop(options, config, result);
                }

                // Create backup before making changes
                let files_to_backup = collect_files_from_issues(&result.issues);
                let _backup_id = create_backup(&files_to_backup, "linthis fix", options.quiet);
                if !options.quiet && !files_to_backup.is_empty() {
                    println!();
                }

                // Check if AI mode is enabled (non-accept-all mode)
                let (modified_files, fixed_count) = if options.ai {
                    let provider = resolve_ai_provider(
                        options.provider.as_deref(),
                        config.ai.provider.as_deref(),
                    );
                    let ai_config = AiFixConfig::with_provider(&provider)
                        .with_model(options.model.clone())
                        .with_accept_all(options.accept_all)
                        .with_verbose(options.verbose)
                        .with_parallel(options.jobs);

                    let ai_result = run_ai_fix_all(&result, &ai_config);
                    (ai_result.modified_files, ai_result.applied)
                } else {
                    let interactive_result = run_interactive(&result);
                    let count = interactive_result.edited + interactive_result.ignored;
                    (interactive_result.modified_files, count)
                };

                // Recheck modified files if any changes were made
                if !modified_files.is_empty() {
                    print_recheck_header();
                    let recheck_result = recheck_modified_files(
                        &modified_files,
                        &result.issues,
                        options.quiet,
                        options.verbose,
                    );
                    print_recheck_summary(&recheck_result, fixed_count);
                    print_recheck_footer();
                }

                ExitCode::from(result.exit_code as u8)
            }
            Err(e) => {
                eprintln!(
                    "{}: Failed to parse result file as JSON: {}",
                    "Error".red(),
                    e
                );
                eprintln!("  Result files are saved in JSON format by default.");
                eprintln!("  Make sure the file is a valid JSON result file.");
                ExitCode::from(2)
            }
        },
        Err(e) => {
            eprintln!("{}: Failed to read result file: {}", "Error".red(), e);
            ExitCode::from(2)
        }
    }
}

/// Run AI fix in a loop until no issues remain or max iterations reached
fn run_ai_fix_loop(
    options: &FixCommandOptions,
    config: &Config,
    initial_result: linthis::utils::types::RunResult,
) -> ExitCode {
    use linthis::{run, RunMode, RunOptions};

    // Create backup before making any changes
    let files_to_backup = collect_files_from_issues(&initial_result.issues);
    let _backup_id = create_backup(&files_to_backup, "AI fix with --accept-all", options.quiet);

    if !options.quiet {
        println!();
    }

    let mut current_result = initial_result;
    let mut iteration = 0;
    let mut total_fixed = 0;

    loop {
        iteration += 1;

        if !options.quiet {
            println!(
                "\n{} AI Fix Iteration {} / {}",
                "→".cyan().bold(),
                iteration,
                MAX_AI_FIX_ITERATIONS
            );
            println!(
                "  {} issue{} to fix",
                current_result.issues.len(),
                if current_result.issues.len() == 1 { "" } else { "s" }
            );
        }

        // Run AI fix
        let provider = resolve_ai_provider(
            options.provider.as_deref(),
            config.ai.provider.as_deref(),
        );
        let ai_config = AiFixConfig::with_provider(&provider)
            .with_model(options.model.clone())
            .with_accept_all(true)
            .with_verbose(options.verbose)
            .with_parallel(options.jobs);

        let ai_result = run_ai_fix_all(&current_result, &ai_config);
        total_fixed += ai_result.applied;

        if ai_result.modified_files.is_empty() {
            if !options.quiet {
                println!(
                    "  {} No files modified in this iteration",
                    "⚠".yellow()
                );
            }
            break;
        }

        if !options.quiet {
            println!(
                "  {} Applied {} fix{}",
                "✓".green(),
                ai_result.applied,
                if ai_result.applied == 1 { "" } else { "es" }
            );
        }

        // Check if we've reached max iterations
        if iteration >= MAX_AI_FIX_ITERATIONS {
            if !options.quiet {
                println!(
                    "\n{} Reached maximum iterations ({})",
                    "⚠".yellow(),
                    MAX_AI_FIX_ITERATIONS
                );
            }
            break;
        }

        // Re-run lint check ONLY on modified files to see if there are remaining issues
        let modified_paths: Vec<PathBuf> = ai_result.modified_files.iter().cloned().collect();
        if modified_paths.is_empty() {
            break;
        }

        if !options.quiet {
            println!(
                "\n{} Re-checking {} modified file{}...",
                "→".cyan(),
                modified_paths.len(),
                if modified_paths.len() == 1 { "" } else { "s" }
            );
        }

        let run_options = RunOptions {
            paths: modified_paths,
            mode: RunMode::CheckOnly,
            languages: vec![],
            exclude_patterns: vec![],
            verbose: options.verbose,
            quiet: true, // Suppress normal output during recheck
            plugins: vec![],
            no_cache: true, // Don't use cache for recheck
            config_resolver: None,
        };

        match run(&run_options) {
            Ok(result) => {
                if result.issues.is_empty() {
                    if !options.quiet {
                        println!(
                            "\n{} All issues fixed after {} iteration{}!",
                            "✓".green().bold(),
                            iteration,
                            if iteration == 1 { "" } else { "s" }
                        );
                        println!(
                            "  Total fixes applied: {}",
                            total_fixed.to_string().cyan()
                        );
                    }
                    return ExitCode::SUCCESS;
                }

                if !options.quiet {
                    println!(
                        "  {} remaining issue{}",
                        result.issues.len(),
                        if result.issues.len() == 1 { "" } else { "s" }
                    );
                }

                // Continue with remaining issues
                current_result = result;
            }
            Err(e) => {
                eprintln!("{}: Re-check failed: {}", "Error".red(), e);
                break;
            }
        }
    }

    // Final summary
    if !options.quiet {
        println!("\n{}", "─".repeat(50));
        println!(
            "{} AI Fix completed after {} iteration{}",
            "→".cyan(),
            iteration,
            if iteration == 1 { "" } else { "s" }
        );
        println!("  Total fixes applied: {}", total_fixed.to_string().cyan());

        if !current_result.issues.is_empty() {
            println!(
                "  {} remaining issue{}",
                current_result.issues.len().to_string().yellow(),
                if current_result.issues.len() == 1 { "" } else { "s" }
            );
            println!(
                "\n  Run {} to see remaining issues",
                "linthis report show".cyan()
            );
        }

        println!(
            "\n  To undo: {}",
            "linthis fix --undo".cyan()
        );
    }

    if current_result.issues.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// Handle single file AI fix mode
fn handle_single_file_ai_fix(options: &FixCommandOptions, config: &Config) -> ExitCode {
    let file_path = options.file.as_ref().unwrap();
    let line_number = options.line.unwrap();

    // Create AI provider
    let provider_str = resolve_ai_provider(
        options.provider.as_deref(),
        config.ai.provider.as_deref(),
    );
    let provider_kind: AiProviderKind = provider_str.parse().unwrap_or_default();

    let mut config = match provider_kind {
        AiProviderKind::Claude => AiProviderConfig::claude(),
        AiProviderKind::ClaudeCli => AiProviderConfig::claude_cli(),
        AiProviderKind::CodeBuddy => AiProviderConfig::codebuddy(),
        AiProviderKind::CodeBuddyCli => AiProviderConfig::codebuddy_cli(),
        AiProviderKind::OpenAi => AiProviderConfig::openai(),
        AiProviderKind::Local => AiProviderConfig::local(),
        AiProviderKind::Mock => AiProviderConfig::mock(),
    };

    // Override model if specified
    if let Some(ref model) = options.model {
        config.model = model.clone();
    }

    // Set API key from environment
    config.api_key = match provider_kind {
        AiProviderKind::Claude => std::env::var("ANTHROPIC_AUTH_TOKEN")
            .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
            .ok(),
        AiProviderKind::CodeBuddy => std::env::var("CODEBUDDY_API_KEY").ok(),
        AiProviderKind::OpenAi => std::env::var("OPENAI_API_KEY").ok(),
        _ => None,
    };

    // Set endpoint from environment for Claude or CodeBuddy
    match provider_kind {
        AiProviderKind::Claude => {
            if let Ok(base_url) = std::env::var("ANTHROPIC_BASE_URL") {
                config.endpoint = Some(base_url);
            }
        }
        AiProviderKind::CodeBuddy => {
            if let Ok(base_url) = std::env::var("CODEBUDDY_BASE_URL") {
                config.endpoint = Some(base_url);
            }
        }
        _ => {}
    }

    let provider = AiProvider::new(config);
    let suggester = AiSuggester::with_provider(provider);

    // Check if provider is available
    if !suggester.is_available() {
        eprintln!(
            "{}: AI provider {} is not available",
            "Error".red(),
            suggester.provider_name()
        );
        match provider_kind {
            AiProviderKind::Claude => {
                eprintln!("Set ANTHROPIC_AUTH_TOKEN or ANTHROPIC_API_KEY environment variable");
            }
            AiProviderKind::ClaudeCli => {
                eprintln!("Install Claude CLI (claude command must be available)");
            }
            AiProviderKind::OpenAi => {
                eprintln!("Set OPENAI_API_KEY environment variable");
            }
            AiProviderKind::Local => {
                eprintln!("Set LINTHIS_AI_ENDPOINT environment variable");
            }
            _ => {}
        }
        return ExitCode::FAILURE;
    }

    if options.verbose {
        println!(
            "Using AI provider: {} ({})",
            suggester.provider_name(),
            suggester.model_name()
        );
    }

    // Create suggestion options
    let suggestion_options = SuggestionOptions {
        max_suggestions: options.max_suggestions,
        include_explanation: true,
        include_confidence: true,
        ..Default::default()
    };

    let message = options.message.as_deref().unwrap_or("Issue at this line");
    let rule_id = options.rule.as_deref().unwrap_or("UNKNOWN");

    if options.verbose {
        println!(
            "Generating suggestions for {}:{}",
            file_path.display(),
            line_number
        );
    }

    let result = suggester.suggest_fix_for_file(
        file_path,
        line_number as usize,
        message,
        rule_id,
        &suggestion_options,
    );

    // Format output
    format_single_result(&result, &options.output, options.with_context);

    if result.is_success() {
        // Handle auto-apply
        if options.accept_all && !result.suggestions.is_empty() {
            if let Some(suggestion) = result.suggestions.first() {
                // Create a temporary issue for apply_suggestion
                let issue = LintIssue {
                    file_path: file_path.clone(),
                    line: line_number as usize,
                    column: None,
                    severity: linthis::utils::types::Severity::Error,
                    message: message.to_string(),
                    code: Some(rule_id.to_string()),
                    source: Some("ai-fix".to_string()),
                    language: None,
                    suggestion: None,
                    code_line: None,
                    context_before: vec![],
                    context_after: vec![],
                };

                if apply_suggestion(&issue, suggestion) {
                    println!("{} Applied suggestion!", "✓".green());
                } else {
                    eprintln!("{} Failed to apply suggestion.", "✗".red());
                    return ExitCode::FAILURE;
                }
            }
        }
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Format a single suggestion result
fn format_single_result(
    result: &linthis::ai::SuggestionResult,
    format: &str,
    with_context: bool,
) {
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(result).unwrap_or_default();
            println!("{}", json);
        }
        "diff" => {
            if !result.suggestions.is_empty() {
                println!("--- a/{}", result.file_path);
                println!("+++ b/{}", result.file_path);
                for suggestion in &result.suggestions {
                    println!(
                        "@@ -{},{} +{},{} @@",
                        result.line_number, 1, result.line_number, 1
                    );
                    println!(
                        "-{}",
                        result
                            .context
                            .as_ref()
                            .map(|c| c.issue_lines.as_str())
                            .unwrap_or("")
                    );
                    println!("+{}", suggestion.code.lines().next().unwrap_or(""));
                }
            }
        }
        _ => {
            // Human-readable format
            println!("{}:{}", result.file_path, result.line_number);
            println!("  Issue: {}", result.message);

            if let Some(ref err) = result.error {
                println!("  {}: {}", "Error".red(), err);
                return;
            }

            if with_context {
                if let Some(ref ctx) = result.context {
                    println!("  Context:");
                    println!("  ```{}", ctx.language);
                    for line in ctx.full_snippet.lines().take(10) {
                        println!("  {}", line);
                    }
                    println!("  ```");
                }
            }

            if result.suggestions.is_empty() {
                println!("  {}", "No suggestions generated.".yellow());
            } else {
                for (idx, suggestion) in result.suggestions.iter().enumerate() {
                    println!("  {} {}:", format!("[{}]", idx + 1).cyan(), "Suggestion".bold());
                    println!("  ```{}", suggestion.language);
                    for line in suggestion.code.lines() {
                        println!("    {}", line.green());
                    }
                    println!("  ```");

                    if let Some(ref exp) = suggestion.explanation {
                        println!("  Explanation: {}", exp);
                    }
                    if let Some(conf) = suggestion.confidence {
                        println!("  Confidence: {:.0}%", conf * 100.0);
                    }
                }
            }
            println!();
        }
    }
}

/// Apply a suggestion to a file
fn apply_suggestion(issue: &LintIssue, suggestion: &linthis::ai::FixSuggestion) -> bool {
    let content = match fs::read_to_string(&issue.file_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let lines: Vec<&str> = content.lines().collect();
    let line_idx = issue.line.saturating_sub(1);

    if line_idx >= lines.len() {
        return false;
    }

    // Build new content
    let mut new_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();

    // Get suggestion lines
    let suggestion_lines: Vec<&str> = suggestion.code.lines().collect();
    if suggestion_lines.is_empty() {
        return false;
    }

    // Determine if this is a single-line or multi-line replacement
    let replacement_end = suggestion.end_line.max(issue.line);
    let lines_to_replace = replacement_end - issue.line + 1;

    // Remove old lines and insert new ones
    let remove_count = lines_to_replace.min(new_lines.len() - line_idx);
    for _ in 0..remove_count {
        if line_idx < new_lines.len() {
            new_lines.remove(line_idx);
        }
    }

    // Insert suggestion lines
    for (i, line) in suggestion_lines.iter().enumerate() {
        new_lines.insert(line_idx + i, line.to_string());
    }

    // Write back
    let new_content = new_lines.join("\n");

    // Preserve trailing newline if original had one
    let final_content = if content.ends_with('\n') && !new_content.ends_with('\n') {
        format!("{}\n", new_content)
    } else {
        new_content
    };

    fs::write(&issue.file_path, final_content).is_ok()
}

// ============================================================================
// Backup and Restore Functions
// ============================================================================

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Maximum number of backups to keep
const MAX_BACKUPS: usize = 5;

/// Backup manifest containing metadata about backed up files
#[derive(Debug, Serialize, Deserialize)]
struct BackupManifest {
    /// Timestamp when backup was created
    timestamp: String,
    /// List of files that were backed up (relative paths)
    files: Vec<String>,
    /// Description of the backup
    description: String,
}

/// Get the backup directory path
fn get_backup_dir() -> PathBuf {
    let project_root = linthis::utils::get_project_root();
    project_root.join(".linthis").join("backup")
}

/// Create a backup of files that will be modified
pub fn create_backup(files: &[PathBuf], description: &str, quiet: bool) -> Option<String> {
    if files.is_empty() {
        return None;
    }

    let backup_dir = get_backup_dir();
    let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_path = backup_dir.join(&timestamp);

    // Create backup directory
    if let Err(e) = fs::create_dir_all(&backup_path) {
        eprintln!("{}: Failed to create backup directory: {}", "Warning".yellow(), e);
        return None;
    }

    let project_root = linthis::utils::get_project_root();
    let mut backed_up_files = Vec::new();

    // Copy each file to backup
    for file in files {
        // Get relative path from project root
        let rel_path = match file.strip_prefix(&project_root) {
            Ok(p) => p.to_path_buf(),
            Err(_) => file.clone(),
        };

        let backup_file_path = backup_path.join(&rel_path);

        // Create parent directories
        if let Some(parent) = backup_file_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("{}: Failed to create directory {}: {}", "Warning".yellow(), parent.display(), e);
                continue;
            }
        }

        // Copy file
        if file.exists() {
            if let Err(e) = fs::copy(file, &backup_file_path) {
                eprintln!("{}: Failed to backup {}: {}", "Warning".yellow(), file.display(), e);
                continue;
            }
            backed_up_files.push(rel_path.to_string_lossy().to_string());
        }
    }

    if backed_up_files.is_empty() {
        // No files backed up, remove empty directory
        let _ = fs::remove_dir_all(&backup_path);
        return None;
    }

    // Write manifest
    let manifest = BackupManifest {
        timestamp: timestamp.clone(),
        files: backed_up_files.clone(),
        description: description.to_string(),
    };

    let manifest_path = backup_path.join("manifest.json");
    if let Err(e) = fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap_or_default()) {
        eprintln!("{}: Failed to write backup manifest: {}", "Warning".yellow(), e);
    }

    if !quiet {
        println!(
            "{} Backup created: {}",
            "✓".green(),
            backup_path.display()
        );
        println!(
            "  {} file{} backed up",
            backed_up_files.len(),
            if backed_up_files.len() == 1 { "" } else { "s" }
        );
    }

    // Clean up old backups
    cleanup_old_backups();

    Some(timestamp)
}

/// Clean up old backups, keeping only the most recent MAX_BACKUPS
fn cleanup_old_backups() {
    let backup_dir = get_backup_dir();
    if !backup_dir.exists() {
        return;
    }

    let mut backups: Vec<_> = match fs::read_dir(&backup_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.path())
            .collect(),
        Err(_) => return,
    };

    // Sort by name (which is timestamp, so newest last)
    backups.sort();

    // Remove oldest backups if we have too many
    while backups.len() > MAX_BACKUPS {
        if let Some(oldest) = backups.first() {
            let _ = fs::remove_dir_all(oldest);
            backups.remove(0);
        }
    }
}

/// List available backups
fn handle_list_backups() -> ExitCode {
    let backup_dir = get_backup_dir();

    if !backup_dir.exists() {
        println!("{} No backups found.", "→".cyan());
        println!("  Backups are created automatically when running fix commands.");
        return ExitCode::SUCCESS;
    }

    let mut backups: Vec<_> = match fs::read_dir(&backup_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.path())
            .collect(),
        Err(e) => {
            eprintln!("{}: Failed to read backup directory: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    if backups.is_empty() {
        println!("{} No backups found.", "→".cyan());
        return ExitCode::SUCCESS;
    }

    // Sort by name (newest last)
    backups.sort();
    backups.reverse(); // Show newest first

    println!("{} Available backups:", "→".cyan());
    println!();

    for (idx, backup_path) in backups.iter().enumerate() {
        let backup_name = backup_path.file_name().unwrap_or_default().to_string_lossy();
        let manifest_path = backup_path.join("manifest.json");

        let (file_count, description) = if manifest_path.exists() {
            match fs::read_to_string(&manifest_path) {
                Ok(content) => {
                    match serde_json::from_str::<BackupManifest>(&content) {
                        Ok(m) => (m.files.len(), m.description),
                        Err(_) => (0, String::new()),
                    }
                }
                Err(_) => (0, String::new()),
            }
        } else {
            (0, String::new())
        };

        let marker = if idx == 0 { "(latest)" } else { "" };
        println!(
            "  {} {} {} - {} file{}",
            format!("[{}]", idx + 1).cyan(),
            backup_name,
            marker.green(),
            file_count,
            if file_count == 1 { "" } else { "s" }
        );
        if !description.is_empty() {
            println!("      {}", description.dimmed());
        }
    }

    println!();
    println!("To restore: {} or {}",
        "linthis fix --undo".cyan(),
        "linthis fix --undo <backup-name>".cyan()
    );

    ExitCode::SUCCESS
}

/// Restore files from a backup
fn handle_undo_fix(source: &str) -> ExitCode {
    let backup_dir = get_backup_dir();

    if !backup_dir.exists() {
        eprintln!("{}: No backups found.", "Error".red());
        eprintln!("  Run a fix command first to create a backup.");
        return ExitCode::from(1);
    }

    // Find the backup to restore
    let backup_path = if source == "last" {
        // Find the most recent backup
        let mut backups: Vec<_> = match fs::read_dir(&backup_dir) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.path())
                .collect(),
            Err(e) => {
                eprintln!("{}: Failed to read backup directory: {}", "Error".red(), e);
                return ExitCode::from(1);
            }
        };

        if backups.is_empty() {
            eprintln!("{}: No backups found.", "Error".red());
            return ExitCode::from(1);
        }

        backups.sort();
        backups.pop().unwrap() // Get the most recent
    } else {
        // Use specified backup name
        let path = backup_dir.join(source);
        if !path.exists() {
            eprintln!("{}: Backup not found: {}", "Error".red(), source);
            eprintln!("  Run {} to see available backups.", "linthis fix --list-backups".cyan());
            return ExitCode::from(1);
        }
        path
    };

    let backup_name = backup_path.file_name().unwrap_or_default().to_string_lossy();
    println!("{} Restoring from backup: {}", "→".cyan(), backup_name);

    // Read manifest
    let manifest_path = backup_path.join("manifest.json");
    let manifest: BackupManifest = if manifest_path.exists() {
        match fs::read_to_string(&manifest_path) {
            Ok(content) => {
                match serde_json::from_str(&content) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: Failed to parse manifest: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("{}: Failed to read manifest: {}", "Error".red(), e);
                return ExitCode::from(1);
            }
        }
    } else {
        eprintln!("{}: Backup manifest not found.", "Error".red());
        return ExitCode::from(1);
    };

    let project_root = linthis::utils::get_project_root();
    let mut restored_count = 0;
    let mut failed_count = 0;

    // Restore each file
    for rel_path in &manifest.files {
        let backup_file = backup_path.join(rel_path);
        let target_file = project_root.join(rel_path);

        if !backup_file.exists() {
            eprintln!("  {} Missing backup file: {}", "⚠".yellow(), rel_path);
            failed_count += 1;
            continue;
        }

        // Create parent directories if needed
        if let Some(parent) = target_file.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("  {} Failed to create directory for {}: {}", "✗".red(), rel_path, e);
                failed_count += 1;
                continue;
            }
        }

        // Copy file back
        match fs::copy(&backup_file, &target_file) {
            Ok(_) => {
                println!("  {} Restored: {}", "✓".green(), rel_path);
                restored_count += 1;
            }
            Err(e) => {
                eprintln!("  {} Failed to restore {}: {}", "✗".red(), rel_path, e);
                failed_count += 1;
            }
        }
    }

    println!();
    if failed_count == 0 {
        println!(
            "{} Restored {} file{} from backup {}",
            "✓".green().bold(),
            restored_count,
            if restored_count == 1 { "" } else { "s" },
            backup_name
        );
        ExitCode::SUCCESS
    } else {
        println!(
            "{} Restored {} file{}, {} failed",
            "⚠".yellow(),
            restored_count,
            if restored_count == 1 { "" } else { "s" },
            failed_count
        );
        ExitCode::from(1)
    }
}

/// Collect unique files from lint issues
pub fn collect_files_from_issues(issues: &[LintIssue]) -> Vec<PathBuf> {
    let mut files: HashSet<PathBuf> = HashSet::new();
    for issue in issues {
        files.insert(issue.file_path.clone());
    }
    files.into_iter().collect()
}
