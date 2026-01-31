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
    pub auto_apply: bool,
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
}

/// Handle the fix subcommand
pub fn handle_fix_command(options: FixCommandOptions) -> ExitCode {
    // If --check or --format-only is specified, run lint first
    if options.check || options.format_only {
        return handle_fix_with_lint(&options);
    }

    // If single file mode (--ai with -i/--include and --line)
    if options.ai && options.file.is_some() && options.line.is_some() {
        return handle_single_file_ai_fix(&options);
    }

    // Load from result file and fix
    handle_fix_from_result(&options)
}

/// Handle fix with running lint first
fn handle_fix_with_lint(options: &FixCommandOptions) -> ExitCode {
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

            // Enter fix mode
            let (modified_files, fixed_count) = if options.ai {
                let provider = resolve_ai_provider(options.provider.as_deref());
                let ai_config = AiFixConfig::with_provider(&provider)
                    .with_model(options.model.clone())
                    .with_auto_apply(options.auto_apply)
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

/// Handle fix by loading from result file
fn handle_fix_from_result(options: &FixCommandOptions) -> ExitCode {
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

                // Check if AI mode is enabled
                let (modified_files, fixed_count) = if options.ai {
                    let provider = resolve_ai_provider(options.provider.as_deref());
                    let ai_config = AiFixConfig::with_provider(&provider)
                        .with_model(options.model.clone())
                        .with_auto_apply(options.auto_apply)
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

/// Handle single file AI fix mode
fn handle_single_file_ai_fix(options: &FixCommandOptions) -> ExitCode {
    let file_path = options.file.as_ref().unwrap();
    let line_number = options.line.unwrap();

    // Create AI provider
    let provider_str = resolve_ai_provider(options.provider.as_deref());
    let provider_kind: AiProviderKind = provider_str.parse().unwrap_or_default();

    let mut config = match provider_kind {
        AiProviderKind::Claude => AiProviderConfig::claude(),
        AiProviderKind::ClaudeCli => AiProviderConfig::claude_cli(),
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
        AiProviderKind::OpenAi => std::env::var("OPENAI_API_KEY").ok(),
        _ => None,
    };

    // Set endpoint from environment for Claude
    if provider_kind == AiProviderKind::Claude {
        if let Ok(base_url) = std::env::var("ANTHROPIC_BASE_URL") {
            config.endpoint = Some(base_url);
        }
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
        if options.auto_apply && !result.suggestions.is_empty() {
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
