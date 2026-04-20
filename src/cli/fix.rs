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
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::cli::backup::{collect_files_from_issues, create_backup, handle_list_backups};
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
    /// Extra arguments passed to the AI provider CLI
    pub provider_args: Option<String>,
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
pub fn handle_fix_command(mut options: FixCommandOptions) -> ExitCode {
    // Handle --list-backups
    if options.list_backups {
        return handle_list_backups("linthis fix");
    }

    // Handle --undo (redirects to unified undo with fix filter)
    if options.undo {
        let filter = if options.source == "last" {
            "fix".to_string()
        } else {
            options.source.clone()
        };
        return super::backup::handle_undo_filtered(&filter);
    }

    // Load config for AI settings
    let project_root = linthis::utils::get_project_root();
    let config = Config::load_merged(&project_root);

    // Interactive provider selection when --ai without --provider
    if options.ai
        && options.provider.is_none()
        && std::env::var("LINTHIS_AI_PROVIDER").is_err()
        && config.ai.provider.is_none()
        && std::io::IsTerminal::is_terminal(&std::io::stdin())
    {
        if let Some(provider) = super::helpers::select_ai_provider_interactive() {
            options.provider = Some(provider);
        } else {
            println!("Fix cancelled");
            return ExitCode::SUCCESS;
        }
    }

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
            if options.format_only {
                "format"
            } else {
                "check"
            }
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
        tool_install_mode: linthis::ToolInstallMode::Prompt,
        hook_event: None,
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

            // For AI mode with accept_all, use iterative fix loop
            if options.ai && options.accept_all {
                return run_ai_fix_loop(options, config, result);
            }

            // Enter fix mode
            let (modified_files, fixed_count) = if options.ai {
                let provider =
                    resolve_ai_provider(options.provider.as_deref(), config.ai.provider.as_deref());
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
            eprintln!("{}: {}", "Error".red(), e);
            ExitCode::from(2)
        }
    }
}

/// Maximum number of AI fix iterations to prevent infinite loops
const MAX_AI_FIX_ITERATIONS: usize = 100;

/// Handle fix by loading from result file
/// Resolve the result file path from the source option.
fn resolve_result_path(source: &str) -> Result<PathBuf, ExitCode> {
    let path = if source == "last" {
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
                return Err(ExitCode::from(1));
            }
        }
    } else {
        PathBuf::from(source)
    };

    if !path.exists() {
        eprintln!(
            "{}: Result file not found: {}",
            "Error".red(),
            path.display()
        );
        return Err(ExitCode::from(1));
    }
    Ok(path)
}

/// Count issues by type (lint, security, complexity) and print summary.
fn print_issue_type_summary(issues: &[LintIssue]) {
    let lint_count = issues
        .iter()
        .filter(|i| {
            !i.source.as_deref().unwrap_or("").starts_with("security/")
                && i.source.as_deref() != Some("linthis-complexity")
        })
        .count();
    let sec_count = issues
        .iter()
        .filter(|i| i.source.as_deref().unwrap_or("").starts_with("security/"))
        .count();
    let cx_count = issues
        .iter()
        .filter(|i| i.source.as_deref() == Some("linthis-complexity"))
        .count();

    let mut parts = Vec::new();
    if lint_count > 0 {
        parts.push(format!("{} lint", lint_count));
    }
    if sec_count > 0 {
        parts.push(format!("{} security", sec_count));
    }
    if cx_count > 0 {
        parts.push(format!("{} complexity", cx_count));
    }
    println!(
        "  Found {} issue{} from previous run ({})\n",
        issues.len(),
        if issues.len() == 1 { "" } else { "s" },
        parts.join(", "),
    );
}

/// Run fix logic (AI or interactive) and recheck modified files.
fn run_fix_and_recheck(
    options: &FixCommandOptions,
    config: &Config,
    result: &linthis::utils::types::RunResult,
) {
    // Create backup before making changes
    let files_to_backup = collect_files_from_issues(&result.issues);
    let _backup_id = create_backup(&files_to_backup, "linthis fix", options.quiet);
    if !options.quiet && !files_to_backup.is_empty() {
        println!();
    }

    let (modified_files, fixed_count) = if options.ai {
        let provider =
            resolve_ai_provider(options.provider.as_deref(), config.ai.provider.as_deref());
        let ai_config = AiFixConfig::with_provider(&provider)
            .with_model(options.model.clone())
            .with_accept_all(options.accept_all)
            .with_verbose(options.verbose)
            .with_parallel(options.jobs);

        let ai_result = run_ai_fix_all(result, &ai_config);
        (ai_result.modified_files, ai_result.applied)
    } else {
        let interactive_result = run_interactive(result);
        let count = interactive_result.edited + interactive_result.ignored;
        (interactive_result.modified_files, count)
    };

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
}

fn handle_fix_from_result(options: &FixCommandOptions, config: &Config) -> ExitCode {
    let path = match resolve_result_path(&options.source) {
        Ok(p) => p,
        Err(code) => return code,
    };

    if !options.quiet {
        println!("{} Loading results from: {}", "→".cyan(), path.display());
    }

    match linthis::reports::load_result_from_file(&path) {
        Some(mut result) => {
            result.merge_all_check_issues();

            if result.issues.is_empty() {
                if !options.quiet {
                    println!("{}", "No issues in the saved result.".green());
                }
                return ExitCode::SUCCESS;
            }

            if !options.quiet {
                print_issue_type_summary(&result.issues);
            }

            if options.ai && options.accept_all {
                return run_ai_fix_loop(options, config, result);
            }

            run_fix_and_recheck(options, config, &result);
            ExitCode::from(result.exit_code as u8)
        }
        None => {
            eprintln!(
                "{}: Failed to load result file: {}",
                "Error".red(),
                path.display()
            );
            eprintln!("  Make sure the file is a valid JSON result file.");
            eprintln!(
                "  Run {} first to generate a result file.",
                "linthis -c".cyan()
            );
            ExitCode::from(2)
        }
    }
}

/// Run AI fix in a loop until no issues remain or max iterations reached
/// Build an `AiFixConfig` from the command options and resolved provider.
fn build_ai_fix_config(
    options: &FixCommandOptions,
    config: &Config,
    accept_all: bool,
) -> AiFixConfig {
    let provider = resolve_ai_provider(options.provider.as_deref(), config.ai.provider.as_deref());
    AiFixConfig::with_provider(&provider)
        .with_model(options.model.clone())
        .with_accept_all(accept_all)
        .with_verbose(options.verbose)
        .with_parallel(options.jobs)
}

/// Re-run lint checks on modified files and return the new result.
///
/// Returns `Some(result)` on success or `None` on recheck failure.
fn recheck_modified_paths(
    modified_files: &std::collections::HashSet<PathBuf>,
    verbose: bool,
    quiet: bool,
) -> Option<linthis::utils::types::RunResult> {
    use linthis::{run, RunMode, RunOptions};

    let modified_paths: Vec<PathBuf> = modified_files.iter().cloned().collect();
    if modified_paths.is_empty() {
        return None;
    }

    if !quiet {
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
        verbose,
        quiet: true,
        plugins: vec![],
        no_cache: true,
        config_resolver: None,
        tool_install_mode: linthis::ToolInstallMode::Disabled,
        hook_event: None,
    };

    match run(&run_options) {
        Ok(result) => Some(result),
        Err(e) => {
            eprintln!("{}: Re-check failed: {}", "Error".red(), e);
            None
        }
    }
}

/// Print the final summary for an AI fix loop.
fn print_ai_fix_summary(
    current_result: &linthis::utils::types::RunResult,
    iteration: usize,
    total_fixed: usize,
    elapsed: std::time::Duration,
) {
    let elapsed_str = format_duration(elapsed);
    println!("\n{}", "─".repeat(50));
    println!(
        "{} AI Fix completed after {} iteration{}",
        "→".cyan(),
        iteration,
        if iteration == 1 { "" } else { "s" }
    );
    println!("  Total fixes applied: {}", total_fixed.to_string().cyan());
    println!("  Total time: {}", elapsed_str.cyan());

    if !current_result.issues.is_empty() {
        println!(
            "  {} remaining issue{}",
            current_result.issues.len().to_string().yellow(),
            if current_result.issues.len() == 1 {
                ""
            } else {
                "s"
            }
        );
        println!(
            "\n  Run {} to see remaining issues",
            "linthis report show".cyan()
        );
    }

    println!("\n  To undo: {}", "linthis fix --undo".cyan());
}

/// Print the iteration header showing the current iteration number and
/// remaining issue count.
fn print_iteration_header(iteration: usize, issue_count: usize) {
    println!(
        "\n{} AI Fix Iteration {} / {}",
        "→".cyan().bold(),
        iteration,
        MAX_AI_FIX_ITERATIONS
    );
    println!(
        "  {} issue{} to fix",
        issue_count,
        if issue_count == 1 { "" } else { "s" }
    );
}

/// Print the "all issues fixed" success banner.
fn print_all_fixed_banner(iteration: usize, total_fixed: usize, elapsed: std::time::Duration) {
    let elapsed_str = format_duration(elapsed);
    println!(
        "\n{} All issues fixed after {} iteration{}!",
        "✓".green().bold(),
        iteration,
        if iteration == 1 { "" } else { "s" }
    );
    println!("  Total fixes applied: {}", total_fixed.to_string().cyan());
    println!("  Total time: {}", elapsed_str.cyan());
}

/// Outcome of a single AI fix iteration, used to decide whether the loop
/// should continue.
enum IterationOutcome {
    /// All issues have been resolved.
    AllFixed,
    /// Some issues remain; continue with the updated result.
    Continue(Box<linthis::utils::types::RunResult>),
    /// No further progress can be made; stop the loop.
    Stop,
}

/// Execute one iteration of the AI fix loop.  Returns an
/// `IterationOutcome` that tells the caller what to do next.
fn run_single_ai_iteration(
    current_result: &linthis::utils::types::RunResult,
    options: &FixCommandOptions,
    config: &Config,
    iteration: usize,
) -> (usize, IterationOutcome) {
    let ai_config = build_ai_fix_config(options, config, true);
    let ai_result = run_ai_fix_all(current_result, &ai_config);
    let applied = ai_result.applied;

    if ai_result.modified_files.is_empty() {
        if !options.quiet {
            println!("  {} No files modified in this iteration", "⚠".yellow());
        }
        return (applied, IterationOutcome::Stop);
    }

    if !options.quiet {
        println!(
            "  {} Applied {} fix{}",
            "✓".green(),
            applied,
            if applied == 1 { "" } else { "es" }
        );
    }

    if iteration >= MAX_AI_FIX_ITERATIONS {
        if !options.quiet {
            println!(
                "\n{} Reached maximum iterations ({})",
                "⚠".yellow(),
                MAX_AI_FIX_ITERATIONS
            );
        }
        return (applied, IterationOutcome::Stop);
    }

    match recheck_modified_paths(&ai_result.modified_files, options.verbose, options.quiet) {
        Some(result) if result.issues.is_empty() => (applied, IterationOutcome::AllFixed),
        Some(result) => {
            if !options.quiet {
                println!(
                    "  {} remaining issue{}",
                    result.issues.len(),
                    if result.issues.len() == 1 { "" } else { "s" }
                );
            }
            (applied, IterationOutcome::Continue(Box::new(result)))
        }
        None => (applied, IterationOutcome::Stop),
    }
}

fn run_ai_fix_loop(
    options: &FixCommandOptions,
    config: &Config,
    initial_result: linthis::utils::types::RunResult,
) -> ExitCode {
    let files_to_backup = collect_files_from_issues(&initial_result.issues);
    let _backup_id = create_backup(&files_to_backup, "AI fix with --yes", options.quiet);

    if !options.quiet {
        println!();
    }

    let mut current_result = initial_result;
    let mut iteration = 0;
    let mut total_fixed = 0;
    let start_time = std::time::Instant::now();

    loop {
        iteration += 1;

        if !options.quiet {
            print_iteration_header(iteration, current_result.issues.len());
        }

        let (applied, outcome) =
            run_single_ai_iteration(&current_result, options, config, iteration);
        total_fixed += applied;

        match outcome {
            IterationOutcome::AllFixed => {
                if !options.quiet {
                    print_all_fixed_banner(iteration, total_fixed, start_time.elapsed());
                }
                return ExitCode::SUCCESS;
            }
            IterationOutcome::Continue(result) => {
                current_result = *result;
            }
            IterationOutcome::Stop => break,
        }
    }

    if !options.quiet {
        print_ai_fix_summary(
            &current_result,
            iteration,
            total_fixed,
            start_time.elapsed(),
        );
    }

    if current_result.issues.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// Format a duration into a human-readable string (e.g. "3m 25s")
fn format_duration(duration: std::time::Duration) -> String {
    let total_secs = duration.as_secs();
    if total_secs >= 60 {
        format!("{}m {}s", total_secs / 60, total_secs % 60)
    } else {
        format!("{}s", total_secs)
    }
}

/// Handle single file AI fix mode
/// Build an `AiProviderConfig` from the provider kind.
fn build_provider_config(provider_kind: &AiProviderKind) -> AiProviderConfig {
    match provider_kind {
        AiProviderKind::Claude => AiProviderConfig::claude(),
        AiProviderKind::ClaudeCli => AiProviderConfig::claude_cli(),
        AiProviderKind::CodeBuddy => AiProviderConfig::codebuddy(),
        AiProviderKind::CodeBuddyCli => AiProviderConfig::codebuddy_cli(),
        AiProviderKind::OpenAi => AiProviderConfig::openai(),
        AiProviderKind::CodexCli => AiProviderConfig::codex_cli(),
        AiProviderKind::Gemini => AiProviderConfig::gemini(),
        AiProviderKind::GeminiCli => AiProviderConfig::gemini_cli(),
        AiProviderKind::Local => AiProviderConfig::local(),
        AiProviderKind::Custom(name) => AiProviderConfig {
            kind: AiProviderKind::Custom(name.clone()),
            ..AiProviderConfig::default()
        },
        AiProviderKind::Mock => AiProviderConfig::mock(),
    }
}

/// Resolve the API key for a given provider kind from environment variables.
fn resolve_api_key(provider_kind: &AiProviderKind) -> Option<String> {
    match provider_kind {
        AiProviderKind::Claude => std::env::var("ANTHROPIC_AUTH_TOKEN")
            .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
            .ok(),
        AiProviderKind::CodeBuddy => std::env::var("CODEBUDDY_API_KEY").ok(),
        AiProviderKind::OpenAi | AiProviderKind::CodexCli => std::env::var("OPENAI_API_KEY").ok(),
        AiProviderKind::Gemini => std::env::var("GEMINI_API_KEY")
            .or_else(|_| std::env::var("GOOGLE_API_KEY"))
            .ok(),
        _ => None,
    }
}

/// Resolve the endpoint override for a given provider kind from environment variables.
fn resolve_endpoint(provider_kind: &AiProviderKind) -> Option<String> {
    match provider_kind {
        AiProviderKind::Claude => std::env::var("ANTHROPIC_BASE_URL").ok(),
        AiProviderKind::CodeBuddy => std::env::var("CODEBUDDY_BASE_URL").ok(),
        _ => None,
    }
}

/// Create an `AiSuggester` from the command options and app config.
fn create_suggester(
    options: &FixCommandOptions,
    app_config: &Config,
) -> (AiSuggester, AiProviderKind) {
    let provider_str = resolve_ai_provider(
        options.provider.as_deref(),
        app_config.ai.provider.as_deref(),
    );
    let provider_kind: AiProviderKind = provider_str.parse().unwrap_or_default();

    let mut prov_config = build_provider_config(&provider_kind);
    if let Some(ref model) = options.model {
        prov_config.model = model.clone();
    }
    if let Some(ref pa) = options.provider_args {
        prov_config.extra_args = pa.split_whitespace().map(|s| s.to_string()).collect();
    }
    prov_config.api_key = resolve_api_key(&provider_kind);
    prov_config.endpoint = resolve_endpoint(&provider_kind);

    let provider = AiProvider::new(prov_config);
    (AiSuggester::with_provider(provider), provider_kind)
}

/// Print a help message when the AI provider is unavailable.
fn print_provider_unavailable_hint(provider_kind: &AiProviderKind) {
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
        AiProviderKind::CodexCli => {
            eprintln!("Install Codex CLI (npm install -g @openai/codex)");
        }
        AiProviderKind::Gemini => {
            eprintln!("Set GEMINI_API_KEY or GOOGLE_API_KEY environment variable");
        }
        AiProviderKind::GeminiCli => {
            eprintln!("Install Gemini CLI (npm install -g @google/gemini-cli)");
        }
        AiProviderKind::Local => {
            eprintln!("Set LINTHIS_AI_ENDPOINT environment variable");
        }
        _ => {}
    }
}

/// Try to auto-apply the first suggestion from a successful result.
fn try_auto_apply(
    result: &linthis::ai::SuggestionResult,
    file_path: &Path,
    line_number: u32,
    message: &str,
    rule_id: &str,
    accept_all: bool,
) -> Option<ExitCode> {
    if !accept_all || result.suggestions.is_empty() {
        return None;
    }

    let suggestion = result.suggestions.first()?;
    let issue = LintIssue {
        file_path: file_path.to_path_buf(),
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
        None
    } else {
        eprintln!("{} Failed to apply suggestion.", "✗".red());
        Some(ExitCode::FAILURE)
    }
}

fn handle_single_file_ai_fix(options: &FixCommandOptions, config: &Config) -> ExitCode {
    let file_path = options.file.as_ref().unwrap();
    let line_number = options.line.unwrap();

    let (suggester, provider_kind) = create_suggester(options, config);

    if !suggester.is_available() {
        eprintln!(
            "{}: AI provider {} is not available",
            "Error".red(),
            suggester.provider_name()
        );
        print_provider_unavailable_hint(&provider_kind);
        return ExitCode::FAILURE;
    }

    if options.verbose {
        println!(
            "Using AI provider: {} ({})",
            suggester.provider_name(),
            suggester.model_name()
        );
    }

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

    format_single_result(&result, &options.output, options.with_context);

    if result.is_success() {
        if let Some(code) = try_auto_apply(
            &result,
            file_path,
            line_number,
            message,
            rule_id,
            options.accept_all,
        ) {
            return code;
        }
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Format a single suggestion result
fn format_single_result(result: &linthis::ai::SuggestionResult, format: &str, with_context: bool) {
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
                    println!(
                        "  {} {}:",
                        format!("[{}]", idx + 1).cyan(),
                        "Suggestion".bold()
                    );
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
// Backup and Restore Functions (delegated to crate::cli::backup)
// ============================================================================

// Backup functions (create_backup, handle_list_backups, handle_undo, collect_files_from_issues)
// are now in crate::cli::backup and re-exported via mod.rs
