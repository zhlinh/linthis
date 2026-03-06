// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! AI-powered fix integration for interactive mode.
//!
//! Provides AI-assisted code fixing capabilities integrated with the
//! interactive review workflow.

use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use colored::Colorize;
use rayon::prelude::*;

use crate::ai::{
    AiProvider, AiProviderConfig, AiProviderKind, AiSuggester, FixSuggestion, SuggestionOptions,
    SuggestionResult, get_custom_provider,
};
use crate::utils::types::{LintIssue, RunResult, Severity};

use super::menu::{print_code_context, print_diff};
use super::nolint::{add_nolint_comment, describe_nolint_action, NolintResult};

/// Configuration for AI fix operations
#[derive(Debug, Clone)]
pub struct AiFixConfig {
    /// AI provider kind (claude, openai, local, mock)
    pub provider: AiProviderKind,
    /// Custom model name (optional)
    pub model: Option<String>,
    /// Maximum suggestions per issue
    pub max_suggestions: usize,
    /// Auto-apply first suggestion without confirmation
    pub accept_all: bool,
    /// Show verbose output
    pub verbose: bool,
    /// Number of parallel jobs (0 = sequential, >0 = parallel)
    pub parallel_jobs: usize,
}

impl Default for AiFixConfig {
    fn default() -> Self {
        Self {
            provider: AiProviderKind::Claude,
            model: None,
            max_suggestions: 3,
            accept_all: false,
            verbose: false,
            parallel_jobs: 4,
        }
    }
}

impl AiFixConfig {
    /// Create config from environment with specified provider
    pub fn with_provider(provider: &str) -> Self {
        let provider_kind: AiProviderKind = provider.parse().unwrap_or_default();
        Self {
            provider: provider_kind,
            ..Default::default()
        }
    }

    /// Set the model
    pub fn with_model(mut self, model: Option<String>) -> Self {
        self.model = model;
        self
    }

    /// Set auto-apply mode
    pub fn with_accept_all(mut self, accept_all: bool) -> Self {
        self.accept_all = accept_all;
        self
    }

    /// Set verbose mode
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Set parallel jobs (0 = sequential, >0 = parallel with N threads)
    pub fn with_parallel(mut self, jobs: usize) -> Self {
        self.parallel_jobs = jobs;
        self
    }
}

/// Result of an AI fix operation
#[derive(Debug, Default)]
pub struct AiFixResult {
    /// Number of issues with suggestions generated
    pub suggested: usize,
    /// Number of suggestions applied
    pub applied: usize,
    /// Number of issues skipped
    pub skipped: usize,
    /// Number of errors encountered
    pub errors: usize,
    /// Whether user quit early
    pub quit_early: bool,
    /// Set of files that were modified
    pub modified_files: HashSet<PathBuf>,
}

/// Create an AI suggester from config
pub fn create_suggester(config: &AiFixConfig) -> Result<AiSuggester, String> {
    let mut provider_config = match &config.provider {
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
    };

    // Override model if specified
    if let Some(ref model) = config.model {
        provider_config.model = model.clone();
    }

    // Set API key from environment
    provider_config.api_key = match &config.provider {
        AiProviderKind::Claude => std::env::var("ANTHROPIC_AUTH_TOKEN")
            .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
            .ok(),
        AiProviderKind::CodeBuddy => std::env::var("CODEBUDDY_API_KEY").ok(),
        AiProviderKind::OpenAi | AiProviderKind::CodexCli => {
            std::env::var("OPENAI_API_KEY").ok()
        }
        AiProviderKind::Gemini => std::env::var("GEMINI_API_KEY")
            .or_else(|_| std::env::var("GOOGLE_API_KEY"))
            .ok(),
        _ => None,
    };

    // Set endpoint from environment
    match &config.provider {
        AiProviderKind::Claude => {
            if let Ok(base_url) = std::env::var("ANTHROPIC_BASE_URL") {
                provider_config.endpoint = Some(base_url);
            }
        }
        AiProviderKind::CodeBuddy => {
            if let Ok(base_url) = std::env::var("CODEBUDDY_BASE_URL") {
                provider_config.endpoint = Some(base_url);
            }
        }
        _ => {}
    }

    let provider = AiProvider::new(provider_config);
    let suggester = AiSuggester::with_provider(provider);

    if !suggester.is_available() {
        let hint = match &config.provider {
            AiProviderKind::Claude => "Set ANTHROPIC_AUTH_TOKEN or ANTHROPIC_API_KEY environment variable",
            AiProviderKind::ClaudeCli => "Install Claude CLI (claude command must be available)",
            AiProviderKind::CodeBuddy => "Set CODEBUDDY_API_KEY environment variable",
            AiProviderKind::CodeBuddyCli => "Install CodeBuddy CLI (codebuddy command must be available)",
            AiProviderKind::OpenAi => "Set OPENAI_API_KEY environment variable",
            AiProviderKind::CodexCli => "Install Codex CLI (npm install -g @openai/codex)",
            AiProviderKind::Gemini => "Set GEMINI_API_KEY or GOOGLE_API_KEY environment variable",
            AiProviderKind::GeminiCli => "Install Gemini CLI (npm install -g @google/gemini-cli)",
            AiProviderKind::Local => "Set LINTHIS_AI_ENDPOINT environment variable",
            AiProviderKind::Custom(name) => {
                return Err(format!(
                    "Custom AI provider '{}' is not available. Check your config and ensure required tools/keys are set.",
                    name
                ));
            }
            AiProviderKind::Mock => "Mock provider should always be available",
        };
        return Err(format!(
            "AI provider {} is not available. {}",
            suggester.provider_name(),
            hint
        ));
    }

    Ok(suggester)
}

/// Check if provider is a CLI provider that supports direct file editing
fn is_cli_provider(kind: &AiProviderKind) -> bool {
    match kind {
        AiProviderKind::ClaudeCli
        | AiProviderKind::CodeBuddyCli
        | AiProviderKind::CodexCli
        | AiProviderKind::GeminiCli => true,
        AiProviderKind::Custom(_) => {
            get_custom_provider().map(|cp| cp.is_cli).unwrap_or(false)
        }
        _ => false,
    }
}

/// Group issues by file path
fn group_issues_by_file(issues: &[LintIssue]) -> std::collections::HashMap<PathBuf, Vec<&LintIssue>> {
    let mut groups: std::collections::HashMap<PathBuf, Vec<&LintIssue>> = std::collections::HashMap::new();
    for issue in issues {
        groups.entry(issue.file_path.clone()).or_default().push(issue);
    }
    groups
}

/// Run CLI-based file fix (direct file editing mode)
/// This lets the CLI agent directly edit files, then shows the diff
pub fn run_cli_file_fix(issues: &[LintIssue], config: &AiFixConfig) -> AiFixResult {
    let mut fix_result = AiFixResult::default();

    // Group issues by file
    let file_groups = group_issues_by_file(issues);
    let total_files = file_groups.len();

    println!();
    println!("{}", "─".repeat(60).dimmed());
    println!(
        "  {} Direct file editing mode ({} files{})",
        "CLI Fix:".cyan().bold(),
        total_files,
        if config.accept_all && config.parallel_jobs > 1 {
            format!(", {} parallel", config.parallel_jobs)
        } else {
            String::new()
        }
    );
    println!("{}", "─".repeat(60).dimmed());
    println!();

    // Create provider for CLI operations
    let provider_config = match &config.provider {
        AiProviderKind::ClaudeCli => AiProviderConfig::claude_cli(),
        AiProviderKind::CodeBuddyCli => AiProviderConfig::codebuddy_cli(),
        AiProviderKind::CodexCli => AiProviderConfig::codex_cli(),
        AiProviderKind::GeminiCli => AiProviderConfig::gemini_cli(),
        AiProviderKind::Custom(name) => AiProviderConfig {
            kind: AiProviderKind::Custom(name.clone()),
            ..AiProviderConfig::default()
        },
        _ => return fix_result,
    };

    let file_list: Vec<_> = file_groups.into_iter().collect();

    // Parallel mode: accept_all with parallel_jobs > 1
    if config.accept_all && config.parallel_jobs > 1 {
        return run_cli_file_fix_parallel(&file_list, &provider_config, config, total_files);
    }

    // Sequential mode: interactive or single-threaded
    let provider = AiProvider::new(provider_config);

    for (file_idx, (file_path, file_issues)) in file_list.iter().enumerate() {
        println!(
            "  [{}/{}] Processing: {}",
            file_idx + 1,
            total_files,
            file_path.display()
        );

        // Backup original content
        let original_content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("    {} Failed to read file: {}", "✗".red(), e);
                fix_result.errors += file_issues.len();
                continue;
            }
        };

        // Prepare issues for CLI
        let issues_data: Vec<(usize, String, String)> = file_issues
            .iter()
            .map(|i| (i.line, i.message.clone(), i.code.clone().unwrap_or_default()))
            .collect();

        println!("    {} issues to fix", issues_data.len());

        // Start spinner with elapsed time in a background thread
        let cli_name: String = match &config.provider {
            AiProviderKind::ClaudeCli => "Claude".into(),
            AiProviderKind::CodeBuddyCli => "CodeBuddy".into(),
            AiProviderKind::CodexCli => "Codex".into(),
            AiProviderKind::GeminiCli => "Gemini".into(),
            AiProviderKind::Custom(name) => name.clone(),
            _ => "CLI".into(),
        };
        let spinner_running = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let spinner_running_clone = Arc::clone(&spinner_running);

        let spinner_handle = std::thread::spawn(move || {
            let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let start_time = std::time::Instant::now();
            let mut idx = 0;
            let mut first_print = true;

            while spinner_running_clone.load(std::sync::atomic::Ordering::Relaxed) {
                let elapsed = start_time.elapsed();
                let secs = elapsed.as_secs();
                let time_str = if secs >= 60 {
                    format!("{}m {}s", secs / 60, secs % 60)
                } else {
                    format!("{}s", secs)
                };

                if first_print {
                    // First time: print spinner line and empty line below
                    println!(
                        "    {} Running {} CLI... ({})",
                        spinner_chars[idx].to_string().cyan(),
                        cli_name,
                        time_str.dimmed()
                    );
                    println!(); // Empty line for cursor
                    first_print = false;
                } else {
                    // Update: move up 2 lines, print, then move back down
                    print!(
                        "\x1B[2A\r    {} Running {} CLI... ({})\x1B[K\n\n",
                        spinner_chars[idx].to_string().cyan(),
                        cli_name,
                        time_str.dimmed()
                    );
                }
                io::stdout().flush().ok();

                idx = (idx + 1) % spinner_chars.len();
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        });

        // Let CLI fix the file
        let diff_result = provider.fix_file_with_cli(file_path, &issues_data);

        // Stop spinner and clear both lines (spinner line and empty line)
        spinner_running.store(false, std::sync::atomic::Ordering::Relaxed);
        let _ = spinner_handle.join();
        // Move up 2 lines and clear them
        print!("\x1B[2A\x1B[K\n\x1B[K\x1B[A");
        io::stdout().flush().ok();

        match diff_result {
            Ok(diff) => {
                if diff.is_empty() {
                    println!("    {} No changes made", "⚠".yellow());
                    fix_result.skipped += file_issues.len();
                    continue;
                }

                // Show diff
                println!();
                println!("    {}", "Changes:".bold());
                for line in diff.lines() {
                    if line.starts_with('+') && !line.starts_with("+++") {
                        println!("    {}", line.green());
                    } else if line.starts_with('-') && !line.starts_with("---") {
                        println!("    {}", line.red());
                    } else if line.starts_with("@@") {
                        println!("    {}", line.cyan());
                    } else {
                        println!("    {}", line.dimmed());
                    }
                }
                println!();

                if config.accept_all {
                    // Auto-accept
                    println!("    {} Changes applied", "✓".green());
                    fix_result.applied += file_issues.len();
                    fix_result.modified_files.insert(file_path.clone());
                } else {
                    // Ask for confirmation
                    print!("    Apply changes? [Y/n/r(estore)]: ");
                    io::stdout().flush().ok();
                    let input = read_line().trim().to_lowercase();

                    match input.as_str() {
                        "n" | "no" => {
                            // Restore original
                            let _ = fs::write(file_path, &original_content);
                            println!("    {} Changes discarded", "⚠".yellow());
                            fix_result.skipped += file_issues.len();
                        }
                        "r" | "restore" => {
                            let _ = fs::write(file_path, &original_content);
                            println!("    {} File restored", "↺".cyan());
                            fix_result.skipped += file_issues.len();
                        }
                        _ => {
                            println!("    {} Changes applied", "✓".green());
                            fix_result.applied += file_issues.len();
                            fix_result.modified_files.insert(file_path.clone());
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("    {} CLI error: {}", "✗".red(), e);
                // Restore original on error
                let _ = fs::write(file_path, &original_content);
                fix_result.errors += file_issues.len();
            }
        }

        println!();
    }

    // Print summary
    print_cli_fix_summary(&fix_result, total_files);

    fix_result
}

/// Result of a batch fix (multiple files in one CLI call)
#[derive(Debug)]
struct BatchResult {
    /// Per-file diffs (file_path -> diff)
    diffs: std::collections::HashMap<PathBuf, String>,
    /// Files in this batch with their issue counts
    files: Vec<(PathBuf, usize)>,
    /// Error if the entire batch failed
    error: Option<String>,
}

/// Maximum number of files per batch CLI call
const FILES_PER_BATCH: usize = 8;

/// Run CLI file fix in parallel batches.
/// Groups files into batches, each batch handled by one `claude -p` call.
/// Multiple batches run in parallel via rayon.
fn run_cli_file_fix_parallel(
    file_list: &[(PathBuf, Vec<&LintIssue>)],
    provider_config: &AiProviderConfig,
    config: &AiFixConfig,
    total_files: usize,
) -> AiFixResult {
    use std::sync::Mutex;

    let mut fix_result = AiFixResult::default();
    let cli_name = match &config.provider {
            AiProviderKind::ClaudeCli => "Claude",
            AiProviderKind::CodeBuddyCli => "CodeBuddy",
            AiProviderKind::CodexCli => "Codex",
            AiProviderKind::GeminiCli => "Gemini",
            AiProviderKind::Custom(name) => name.as_str(),
            _ => "CLI",
        };

    // Prepare file data
    let file_data: Vec<(PathBuf, Vec<(usize, String, String)>, usize)> = file_list
        .iter()
        .map(|(path, issues)| {
            let issues_data: Vec<(usize, String, String)> = issues
                .iter()
                .map(|i| (i.line, i.message.clone(), i.code.clone().unwrap_or_default()))
                .collect();
            let count = issues.len();
            (path.clone(), issues_data, count)
        })
        .collect();

    // Split into batches
    let batches: Vec<Vec<&(PathBuf, Vec<(usize, String, String)>, usize)>> = file_data
        .iter()
        .collect::<Vec<_>>()
        .chunks(FILES_PER_BATCH)
        .map(|chunk| chunk.to_vec())
        .collect();

    let total_batches = batches.len();
    let total_issues: usize = file_data.iter().map(|(_, _, count)| count).sum();
    let actual_parallel = config.parallel_jobs.min(total_batches);
    let actual_files_per_batch = if total_batches > 0 {
        (total_files + total_batches - 1) / total_batches // ceiling division
    } else {
        total_files
    };

    println!(
        "  {} {} issues in {} files, {} batch{} (up to {} files/batch, {} parallel)",
        "→".cyan(),
        total_issues,
        total_files,
        total_batches,
        if total_batches == 1 { "" } else { "es" },
        actual_files_per_batch,
        actual_parallel
    );
    println!();

    // Progress counter (tracks completed batches)
    let progress = Arc::new(AtomicUsize::new(0));

    // Start progress display thread
    let progress_clone = Arc::clone(&progress);
    let cli_name_owned = cli_name.to_string();
    let progress_handle = std::thread::spawn(move || {
        let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let start_time = std::time::Instant::now();
        let mut idx = 0;

        loop {
            let current = progress_clone.load(Ordering::Relaxed);
            let elapsed = start_time.elapsed();
            let secs = elapsed.as_secs();
            let time_str = if secs >= 60 {
                format!("{}m {}s", secs / 60, secs % 60)
            } else {
                format!("{}s", secs)
            };

            print!(
                "\r  {} [batch {}/{}] Running {} CLI... ({})\x1B[K",
                spinner_chars[idx].to_string().cyan(),
                current,
                total_batches,
                cli_name_owned,
                time_str.dimmed()
            );
            io::stdout().flush().ok();

            if current >= total_batches {
                break;
            }

            idx = (idx + 1) % spinner_chars.len();
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    });

    // Build thread pool
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.parallel_jobs)
        .build()
        .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().build().unwrap());

    // Find common working directory (project root)
    let working_dir = crate::utils::get_project_root();

    // Process batches in parallel
    let results_mutex = Arc::new(Mutex::new(Vec::new()));
    let provider_config_clone = provider_config.clone();

    pool.install(|| {
        batches.par_iter().for_each(|batch| {
            let provider = AiProvider::new(provider_config_clone.clone());

            // Build batch file list for the provider
            let batch_files: Vec<(&std::path::Path, &[(usize, String, String)])> = batch
                .iter()
                .map(|(path, issues, _count)| (path.as_path(), issues.as_slice()))
                .collect();

            let files_info: Vec<(PathBuf, usize)> = batch
                .iter()
                .map(|(path, _, count)| (path.clone(), *count))
                .collect();

            let result = match provider.fix_files_batch_with_cli(&batch_files, &working_dir) {
                Ok(diffs) => BatchResult {
                    diffs,
                    files: files_info,
                    error: None,
                },
                Err(e) => BatchResult {
                    diffs: std::collections::HashMap::new(),
                    files: files_info,
                    error: Some(e),
                },
            };

            progress.fetch_add(1, Ordering::Relaxed);
            results_mutex.lock().unwrap().push(result);
        });
    });

    // Wait for progress thread to finish
    let _ = progress_handle.join();
    println!(); // New line after progress
    println!();

    // Collect and display results
    let results = Arc::try_unwrap(results_mutex)
        .expect("All parallel tasks completed")
        .into_inner()
        .unwrap();

    let mut file_idx = 0;
    for batch_result in &results {
        if let Some(ref error) = batch_result.error {
            for (file_path, issue_count) in &batch_result.files {
                file_idx += 1;
                println!(
                    "  [{}/{}] {}",
                    file_idx, total_files, file_path.display()
                );
                eprintln!("    {} CLI error: {}", "✗".red(), error);
                fix_result.errors += issue_count;
            }
        } else {
            for (file_path, issue_count) in &batch_result.files {
                file_idx += 1;
                println!(
                    "  [{}/{}] {}",
                    file_idx, total_files, file_path.display()
                );

                if let Some(diff) = batch_result.diffs.get(file_path) {
                    for line in diff.lines() {
                        if line.starts_with('+') && !line.starts_with("+++") {
                            println!("    {}", line.green());
                        } else if line.starts_with('-') && !line.starts_with("---") {
                            println!("    {}", line.red());
                        } else if line.starts_with("@@") {
                            println!("    {}", line.cyan());
                        } else {
                            println!("    {}", line.dimmed());
                        }
                    }
                    println!("    {} Changes applied", "✓".green());
                    fix_result.applied += issue_count;
                    fix_result.modified_files.insert(file_path.clone());
                } else {
                    println!("    {} No changes made", "⚠".yellow());
                    fix_result.skipped += issue_count;
                }
                println!();
            }
        }
    }

    // Print summary
    print_cli_fix_summary(&fix_result, total_files);

    fix_result
}

/// Print CLI fix summary
fn print_cli_fix_summary(fix_result: &AiFixResult, total_files: usize) {
    println!("{}", "═".repeat(60).dimmed());
    println!("  {}", "CLI Fix Summary".bold());
    println!("{}", "─".repeat(60).dimmed());
    println!("  Files processed: {}", total_files.to_string().cyan());
    println!("  Issues applied:  {}", fix_result.applied.to_string().green());
    println!("  Issues skipped:  {}", fix_result.skipped.to_string().yellow());
    println!("  Errors:          {}", fix_result.errors.to_string().red());
    println!("{}", "═".repeat(60).dimmed());

    // Add warning for C++ signature changes
    if fix_result.applied > 0 {
        println!();
        println!("{}", "  ⚠ Important for C/C++ projects:".yellow().bold());
        println!("  {}", "If function signatures were changed, verify that:".dimmed());
        println!("  {}", "- All declarations and definitions are updated".dimmed());
        println!("  {}", "- All call sites use correct argument types".dimmed());
        println!("  {}", "- The code still compiles successfully".dimmed());
        println!();
        println!("  {}", "Recommended: Run your build command to verify:".cyan());
        println!("  {}", "  make        # or cmake --build build, etc.".dimmed());
    }
    println!();
}

/// Get AI suggestion for a single issue
pub fn get_suggestion_for_issue(
    suggester: &AiSuggester,
    issue: &LintIssue,
    config: &AiFixConfig,
) -> SuggestionResult {
    let source = match fs::read_to_string(&issue.file_path) {
        Ok(s) => s,
        Err(e) => {
            return SuggestionResult::failure(
                issue.code.as_deref().unwrap_or("UNKNOWN"),
                &issue.file_path.to_string_lossy(),
                issue.line,
                &issue.message,
                &format!("Failed to read file: {}", e),
            );
        }
    };

    let options = SuggestionOptions {
        max_suggestions: config.max_suggestions,
        include_explanation: true,
        include_confidence: true,
        skip_with_suggestion: false, // Always call AI, linter suggestions are text not code
        ..Default::default()
    };

    suggester.suggest_fix(issue, &source, &options)
}

/// Display AI suggestions for an issue and handle user interaction
///
/// Returns: (applied: bool, quit: bool)
pub fn show_ai_suggestions(
    issue: &LintIssue,
    result: &SuggestionResult,
    config: &AiFixConfig,
) -> (bool, bool) {
    println!();

    if let Some(ref error) = result.error {
        println!("  {} {}", "AI Error:".red(), error);
        return (false, false);
    }

    if result.suggestions.is_empty() {
        println!("  {}", "No AI suggestions available for this issue.".yellow());
        return (false, false);
    }

    println!(
        "  {} {} suggestion{}",
        "AI Generated".green().bold(),
        result.suggestions.len(),
        if result.suggestions.len() == 1 { "" } else { "s" }
    );
    println!();

    // Show each suggestion as diff
    for (idx, suggestion) in result.suggestions.iter().enumerate() {
        println!(
            "  {} {}",
            format!("[{}]", idx + 1).cyan().bold(),
            "Suggestion:".bold()
        );

        // Show suggestion as diff preview
        print_suggestion_preview(issue, suggestion);

        // Show explanation if available
        if let Some(ref explanation) = suggestion.explanation {
            println!("  {} {}", "Explanation:".dimmed(), explanation);
        }

        // Show confidence if available
        if let Some(confidence) = suggestion.confidence {
            let confidence_str = format!("{:.0}%", confidence * 100.0);
            let colored = if confidence >= 0.8 {
                confidence_str.green()
            } else if confidence >= 0.5 {
                confidence_str.yellow()
            } else {
                confidence_str.red()
            };
            println!("  {} {}", "Confidence:".dimmed(), colored);
        }

        println!();
    }

    // Auto-apply mode
    if config.accept_all {
        if let Some(suggestion) = result.suggestions.first() {
            println!("  {} Applying first suggestion...", "→".cyan());
            // Capture original content before applying
            let original_content = fs::read_to_string(&issue.file_path).ok();
            let original_lines: Vec<&str> = original_content
                .as_ref()
                .map(|c| c.lines().collect())
                .unwrap_or_default();
            let start_line = issue.line;
            let end_line = suggestion.end_line.max(issue.line);

            if apply_suggestion(issue, suggestion) {
                println!("  {} Applied successfully!", "✓".green());
                println!();
                print_suggestion_diff(&original_lines, suggestion, start_line, end_line);
                return (true, false);
            } else {
                println!("  {} Failed to apply.", "✗".red());
                return (false, false);
            }
        }
    }

    // Interactive mode - ask user what to do
    // Show numbered options for each suggestion
    for i in 1..=result.suggestions.len() {
        if i == 1 {
            println!(
                "  [{}] Apply suggestion #{} {}",
                i.to_string().cyan(),
                i,
                "(default, press Enter)".dimmed()
            );
        } else {
            println!("  [{}] Apply suggestion #{}", i.to_string().cyan(), i);
        }
    }
    println!("  [{}] Skip this issue", "s".cyan());
    println!("  [{}] Quit AI fix mode", "q".cyan());
    println!();
    print!("  > ");
    io::stdout().flush().ok();

    let input = read_line().trim().to_lowercase();

    // Empty input (Enter) applies suggestion #1 by default
    let input = if input.is_empty() { "1".to_string() } else { input };

    match input.as_str() {
        "s" | "skip" => (false, false),
        "q" | "quit" => (false, true),
        _ => {
            // Try to parse as number
            if let Ok(num) = input.parse::<usize>() {
                if num >= 1 && num <= result.suggestions.len() {
                    let suggestion = &result.suggestions[num - 1];
                    // Capture original content before applying
                    let original_content = fs::read_to_string(&issue.file_path).ok();
                    let original_lines: Vec<&str> = original_content
                        .as_ref()
                        .map(|c| c.lines().collect())
                        .unwrap_or_default();
                    let start_line = issue.line;
                    let end_line = suggestion.end_line.max(issue.line);

                    if apply_suggestion(issue, suggestion) {
                        println!("  {} Applied suggestion #{}!", "✓".green(), num);
                        println!();
                        print_suggestion_diff(&original_lines, suggestion, start_line, end_line);
                        return (true, false);
                    } else {
                        println!("  {} Failed to apply suggestion.", "✗".red());
                        return (false, false);
                    }
                }
            }
            println!("  {} Invalid choice, skipping.", "Invalid:".yellow());
            (false, false)
        }
    }
}

/// Validate that a suggestion is reasonable before applying
fn validate_suggestion(issue: &LintIssue, suggestion: &FixSuggestion, original_lines: &[&str]) -> bool {
    let suggestion_lines: Vec<&str> = suggestion.code.lines().collect();
    let lines_to_replace = suggestion.end_line.saturating_sub(issue.line) + 1;

    // Check 1: If we're replacing few lines but suggestion has way more, reject
    if lines_to_replace <= 3 && suggestion_lines.len() > lines_to_replace * 4 {
        eprintln!(
            "  {} Suggestion rejected: replacing {} lines with {} lines is too different",
            "⚠".yellow(),
            lines_to_replace,
            suggestion_lines.len()
        );
        return false;
    }

    // Check 2: If suggestion contains function/class definitions but original doesn't, reject
    let line_idx = issue.line.saturating_sub(1);
    if line_idx < original_lines.len() {
        let original_line = original_lines[line_idx].trim();
        let first_suggestion_line = suggestion_lines.first().map(|s| s.trim()).unwrap_or("");

        // Check for new function/class definitions that weren't in original
        let def_patterns = ["def ", "class ", "fn ", "func ", "function "];
        let orig_has_def = def_patterns.iter().any(|p| original_line.starts_with(p));
        let sugg_has_def = def_patterns.iter().any(|p| first_suggestion_line.starts_with(p));

        if sugg_has_def && !orig_has_def {
            eprintln!(
                "  {} Suggestion rejected: introduces function/class definition where original had none",
                "⚠".yellow()
            );
            return false;
        }
    }

    true
}

/// Apply a suggestion to the file
pub fn apply_suggestion(issue: &LintIssue, suggestion: &FixSuggestion) -> bool {
    let content = match fs::read_to_string(&issue.file_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let lines: Vec<&str> = content.lines().collect();
    let line_idx = issue.line.saturating_sub(1);

    if line_idx >= lines.len() {
        return false;
    }

    // Validate the suggestion before applying
    if !validate_suggestion(issue, suggestion, &lines) {
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

/// Print the diff after applying a suggestion
fn print_suggestion_diff(
    original_lines: &[&str],
    suggestion: &FixSuggestion,
    start_line: usize,
    end_line: usize,
) {
    println!("  {}", "Changes:".bold());

    let suggestion_lines: Vec<&str> = suggestion.code.lines().collect();

    // Show context before (one line before start_line)
    if start_line > 1 {
        if let Some(context_line) = original_lines.get(start_line - 2) {
            println!(
                "  {} {}",
                format!(" {:>4} |", start_line - 1).dimmed(),
                context_line.dimmed()
            );
        }
    }

    // Show removed lines (old content)
    for i in start_line..=end_line {
        if let Some(old_line) = original_lines.get(i - 1) {
            println!(
                "  {} {}",
                format!("-{:>4} |", i).red(),
                old_line.red()
            );
        }
    }

    // Show added lines (new content from suggestion)
    for (i, new_line) in suggestion_lines.iter().enumerate() {
        println!(
            "  {} {}",
            format!("+{:>4} |", start_line + i).green(),
            new_line.green()
        );
    }

    // Show context after (one line after the new content ends)
    let new_end_line = start_line + suggestion_lines.len();
    // Context line is from original file, at position end_line + 1
    if let Some(context_line) = original_lines.get(end_line) {
        println!(
            "  {} {}",
            format!(" {:>4} |", new_end_line).dimmed(),
            context_line.dimmed()
        );
    }

    println!();
}

/// Print suggestion preview as diff (before applying)
fn print_suggestion_preview(issue: &LintIssue, suggestion: &FixSuggestion) {
    let start_line = issue.line;
    let end_line = suggestion.end_line.max(issue.line);
    let suggestion_lines: Vec<&str> = suggestion.code.lines().collect();

    // Show context before (from issue.context_before)
    for (line_num, content) in &issue.context_before {
        println!(
            "    {} {}",
            format!(" {:>4} |", line_num).dimmed(),
            content.dimmed()
        );
    }

    // Show removed lines (old content) - use issue.code_line for the issue line
    // For multi-line, we need to read the file to get all lines
    if end_line > start_line {
        // Multi-line replacement: read original lines from file
        if let Ok(content) = fs::read_to_string(&issue.file_path) {
            let lines: Vec<&str> = content.lines().collect();
            for i in start_line..=end_line {
                if let Some(old_line) = lines.get(i - 1) {
                    println!(
                        "    {} {}",
                        format!("-{:>4} |", i).red(),
                        old_line.red()
                    );
                }
            }
        }
    } else {
        // Single line: use issue.code_line if available
        if let Some(ref code_line) = issue.code_line {
            println!(
                "    {} {}",
                format!("-{:>4} |", start_line).red(),
                code_line.red()
            );
        }
    }

    // Show added lines (new content from suggestion)
    for (i, new_line) in suggestion_lines.iter().enumerate() {
        println!(
            "    {} {}",
            format!("+{:>4} |", start_line + i).green(),
            new_line.green()
        );
    }

    // Show context after (from issue.context_after)
    for (line_num, content) in &issue.context_after {
        println!(
            "    {} {}",
            format!(" {:>4} |", line_num).dimmed(),
            content.dimmed()
        );
    }
}

/// Cached suggestion for an issue
struct CachedSuggestion {
    issue_idx: usize,
    result: SuggestionResult,
}

/// Collect suggestions sequentially (default)
fn collect_suggestions_sequential(
    issues: &[LintIssue],
    suggester: &AiSuggester,
    config: &AiFixConfig,
    total: usize,
) -> Vec<CachedSuggestion> {
    let mut cached_suggestions: Vec<CachedSuggestion> = Vec::new();

    for (idx, issue) in issues.iter().enumerate() {
        // Show progress
        print!(
            "\r  [{}/{}] Analyzing: {}:{}{}",
            idx + 1,
            total,
            issue.file_path.display(),
            issue.line,
            " ".repeat(20) // Clear any remaining chars
        );
        io::stdout().flush().ok();

        let suggestion_result = get_suggestion_for_issue(suggester, issue, config);

        cached_suggestions.push(CachedSuggestion {
            issue_idx: idx,
            result: suggestion_result,
        });
    }

    // Clear progress line
    print!("\r{}\r", " ".repeat(80));
    io::stdout().flush().ok();

    cached_suggestions
}

/// Collect suggestions in parallel using rayon
fn collect_suggestions_parallel(
    issues: &[LintIssue],
    suggester: &AiSuggester,
    config: &AiFixConfig,
    total: usize,
) -> Vec<CachedSuggestion> {
    // Set thread pool size
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.parallel_jobs)
        .build()
        .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().build().unwrap());

    // Progress counter
    let progress = Arc::new(AtomicUsize::new(0));

    // Use a separate thread to show progress
    let progress_clone = Arc::clone(&progress);
    let total_clone = total;
    let progress_handle = std::thread::spawn(move || {
        let mut last_printed = usize::MAX;
        loop {
            let current = progress_clone.load(Ordering::Relaxed);

            // Only print if progress changed
            if current != last_printed {
                print!("\r  [{}/{}] Analyzing in parallel...{}", current, total_clone, " ".repeat(30));
                io::stdout().flush().ok();
                last_printed = current;
            }

            if current >= total_clone {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    });

    // Collect suggestions in parallel
    let results: Vec<CachedSuggestion> = pool.install(|| {
        issues
            .par_iter()
            .enumerate()
            .map(|(idx, issue)| {
                let suggestion_result = get_suggestion_for_issue(suggester, issue, config);
                progress.fetch_add(1, Ordering::Relaxed);
                CachedSuggestion {
                    issue_idx: idx,
                    result: suggestion_result,
                }
            })
            .collect()
    });

    // Wait for progress thread to finish
    let _ = progress_handle.join();

    // Clear progress line
    print!("\r{}\r", " ".repeat(80));
    io::stdout().flush().ok();

    // Sort by issue index to maintain order
    let mut sorted_results = results;
    sorted_results.sort_by_key(|c| c.issue_idx);
    sorted_results
}

/// Run AI fix for all issues in a result
///
/// For CLI providers (claude-cli, codebuddy-cli):
/// - Uses direct file editing mode where CLI edits files directly
/// - Groups issues by file to avoid conflicts
///
/// For API providers:
/// - Uses two-phase approach: collect suggestions, then review
pub fn run_ai_fix_all(result: &RunResult, config: &AiFixConfig) -> AiFixResult {
    let issues = &result.issues;

    if issues.is_empty() {
        println!("{}", "No issues to fix.".green());
        return AiFixResult::default();
    }

    // For CLI providers, use direct file editing mode
    if is_cli_provider(&config.provider) {
        return run_cli_file_fix(issues, config);
    }

    // Create suggester
    let suggester = match create_suggester(config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return AiFixResult {
                errors: issues.len(),
                ..Default::default()
            };
        }
    };

    println!();
    println!("{}", "═".repeat(60).dimmed());
    println!("  {} - Batch Mode", "AI Fix".green().bold());
    println!("{}", "─".repeat(60).dimmed());
    println!(
        "  Provider: {} ({})",
        suggester.provider_name().cyan(),
        suggester.model_name()
    );
    println!("  Issues: {}", issues.len());
    if config.accept_all {
        println!("  Mode: {} (will apply automatically)", "Auto-apply".yellow());
    } else {
        println!("  Mode: Batch collect, then review");
    }
    println!("{}", "═".repeat(60).dimmed());
    println!();

    // Confirm before starting
    if !config.accept_all {
        print!("  Start AI analysis? [Y/n]: ");
        io::stdout().flush().ok();
        let input = read_line().trim().to_lowercase();
        if input == "n" || input == "no" {
            println!("  Cancelled.");
            return AiFixResult::default();
        }
    }

    let total = issues.len();

    // ═══════════════════════════════════════════════════════════
    // Phase 1: Batch collect all AI suggestions
    // ═══════════════════════════════════════════════════════════
    println!();
    println!("{}", "─".repeat(60).dimmed());
    if config.parallel_jobs > 1 {
        println!(
            "  {} Collecting AI suggestions ({} parallel)...",
            "Phase 1:".cyan().bold(),
            config.parallel_jobs
        );
    } else {
        println!("  {} Collecting AI suggestions...", "Phase 1:".cyan().bold());
    }
    println!("{}", "─".repeat(60).dimmed());

    let cached_suggestions = if config.parallel_jobs > 1 {
        // Parallel collection
        collect_suggestions_parallel(issues, &suggester, config, total)
    } else {
        // Sequential collection
        collect_suggestions_sequential(issues, &suggester, config, total)
    };

    let errors = cached_suggestions
        .iter()
        .filter(|c| c.result.error.is_some() || c.result.suggestions.is_empty())
        .count();

    let successful = cached_suggestions.len() - errors;

    println!(
        "  {} Collected {} suggestion{} ({} failed)",
        "✓".green(),
        successful.to_string().cyan(),
        if successful == 1 { "" } else { "s" },
        errors.to_string().red()
    );
    println!();

    // If auto-apply mode, apply all and return
    if config.accept_all {
        return apply_all_suggestions(issues, &cached_suggestions, config);
    }

    // ═══════════════════════════════════════════════════════════
    // Phase 2: Interactive review (no waiting)
    // ═══════════════════════════════════════════════════════════
    println!("{}", "─".repeat(60).dimmed());
    println!("  {} Review suggestions (no more waiting)", "Phase 2:".cyan().bold());
    println!("{}", "─".repeat(60).dimmed());
    println!();
    println!("  Navigation: [p]revious, [g]o to #N, [q]uit");
    println!();

    let mut fix_result = AiFixResult::default();
    fix_result.errors = errors;
    fix_result.suggested = successful;

    let mut idx = 0;
    let mut processed = vec![false; total];

    while idx < total {
        let issue = &issues[idx];
        let cached = &cached_suggestions[idx];

        // Show issue header (same format as non-AI mode)
        println!();
        println!("{}", "─".repeat(60).dimmed());

        // Severity badge
        let current = idx + 1;
        let severity_badge = match issue.severity {
            Severity::Error => format!("[E{}]", current).red().bold(),
            Severity::Warning => format!("[W{}]", current).yellow().bold(),
            Severity::Info => format!("[I{}]", current).blue(),
        };

        // Language and source tags
        let lang_tag = issue
            .language
            .map(|l| format!("[{}]", format!("{:?}", l).to_lowercase()))
            .unwrap_or_default()
            .dimmed();

        let source_tag = issue
            .source
            .as_ref()
            .map(|s| format!("[{}]", s))
            .unwrap_or_default()
            .dimmed();

        // File location
        let location = if let Some(col) = issue.column {
            format!("{}:{}:{}", issue.file_path.display(), issue.line, col)
        } else {
            format!("{}:{}", issue.file_path.display(), issue.line)
        };

        // Progress indicator
        let progress = format!("({}/{})", current, total).dimmed();

        println!(
            "  {} {}{} {} {}",
            severity_badge,
            lang_tag,
            source_tag,
            location.white().bold(),
            progress
        );

        // Code context
        print_code_context(issue);

        // Message and code
        if let Some(ref code) = issue.code {
            println!("  {} ({})", issue.message, code.cyan());
        } else {
            println!("  {}", issue.message);
        }

        // Verbose mode: show additional info
        if config.verbose {
            if let Some(ref suggestion) = issue.suggestion {
                println!("  {} {}", "-->".green(), suggestion);
            }
        }

        // Show cached suggestions
        let (applied, action) = show_cached_suggestions(issue, &cached.result, idx, total);

        if applied {
            fix_result.applied += 1;
            fix_result.modified_files.insert(issue.file_path.clone());
            processed[idx] = true;
        }

        match action {
            ReviewAction::Next => {
                if !applied && !processed[idx] {
                    fix_result.skipped += 1;
                }
                processed[idx] = true;
                idx += 1;
            }
            ReviewAction::Previous => {
                if idx > 0 {
                    idx -= 1;
                    println!("{}", "  (Going back to previous issue)".dimmed());
                } else {
                    println!("{}", "  Already at first issue".yellow());
                }
            }
            ReviewAction::GoTo(target) => {
                if target > 0 && target <= total {
                    idx = target - 1;
                } else {
                    println!(
                        "  {} Issue #{} out of range (1-{})",
                        "Invalid:".yellow(),
                        target,
                        total
                    );
                }
            }
            ReviewAction::Ignore => {
                processed[idx] = true;
                match add_nolint_comment(issue) {
                    NolintResult::Success(diffs) => {
                        fix_result.applied += 1;
                        println!("{} Added NOLINT comment", "✓".green());
                        println!();
                        print_diff(&diffs, &issue.file_path);
                        fix_result.modified_files.insert(issue.file_path.clone());
                    }
                    NolintResult::AlreadyIgnored => {
                        println!("{}", "Already has NOLINT comment".yellow());
                        fix_result.skipped += 1;
                    }
                    NolintResult::Error(e) => {
                        eprintln!("{}: {}", "Failed to add NOLINT".red(), e);
                        fix_result.skipped += 1;
                    }
                }
                idx += 1;
            }
            ReviewAction::AcceptAll => {
                println!();
                println!(
                    "  {} Applying all remaining suggestions...",
                    "→".cyan().bold()
                );
                println!();

                // Apply current issue first (if not already applied)
                if !applied && !processed[idx] {
                    let current_cached = &cached_suggestions[idx];
                    if let Some(suggestion) = current_cached.result.suggestions.first() {
                        let current_issue = &issues[idx];
                        let original_content = fs::read_to_string(&current_issue.file_path).ok();
                        let original_lines: Vec<&str> = original_content
                            .as_ref()
                            .map(|c| c.lines().collect())
                            .unwrap_or_default();
                        let start_line = current_issue.line;
                        let end_line = suggestion.end_line.max(current_issue.line);

                        if apply_suggestion(current_issue, suggestion) {
                            println!(
                                "  {} Applied issue #{} ({}:{})",
                                "✓".green(),
                                idx + 1,
                                current_issue.file_path.display(),
                                current_issue.line
                            );
                            print_suggestion_diff(&original_lines, suggestion, start_line, end_line);
                            fix_result.applied += 1;
                            fix_result.modified_files.insert(current_issue.file_path.clone());
                        } else {
                            println!(
                                "  {} Failed to apply issue #{}",
                                "✗".red(),
                                idx + 1
                            );
                            fix_result.skipped += 1;
                        }
                    }
                }
                processed[idx] = true;

                // Apply remaining issues
                for remaining_idx in (idx + 1)..total {
                    if processed[remaining_idx] {
                        continue;
                    }

                    let remaining_cached = &cached_suggestions[remaining_idx];
                    let remaining_issue = &issues[remaining_idx];

                    if remaining_cached.result.error.is_some()
                        || remaining_cached.result.suggestions.is_empty()
                    {
                        fix_result.skipped += 1;
                        processed[remaining_idx] = true;
                        continue;
                    }

                    if let Some(suggestion) = remaining_cached.result.suggestions.first() {
                        let original_content = fs::read_to_string(&remaining_issue.file_path).ok();
                        let original_lines: Vec<&str> = original_content
                            .as_ref()
                            .map(|c| c.lines().collect())
                            .unwrap_or_default();
                        let start_line = remaining_issue.line;
                        let end_line = suggestion.end_line.max(remaining_issue.line);

                        if apply_suggestion(remaining_issue, suggestion) {
                            println!(
                                "  {} Applied issue #{} ({}:{})",
                                "✓".green(),
                                remaining_idx + 1,
                                remaining_issue.file_path.display(),
                                remaining_issue.line
                            );
                            print_suggestion_diff(&original_lines, suggestion, start_line, end_line);
                            fix_result.applied += 1;
                            fix_result.modified_files.insert(remaining_issue.file_path.clone());
                        } else {
                            println!(
                                "  {} Failed to apply issue #{}",
                                "✗".red(),
                                remaining_idx + 1
                            );
                            fix_result.skipped += 1;
                        }
                    }
                    processed[remaining_idx] = true;
                }

                // All done, exit the loop
                break;
            }
            ReviewAction::Quit => {
                fix_result.quit_early = true;
                // Count unprocessed as skipped
                for (i, &was_processed) in processed.iter().enumerate() {
                    if !was_processed && i >= idx {
                        fix_result.skipped += 1;
                    }
                }
                break;
            }
        }
    }

    // Print summary
    println!();
    println!("{}", "═".repeat(60).dimmed());
    println!("  {}", "AI Fix Summary".bold());
    println!("{}", "─".repeat(60).dimmed());
    println!("  Suggestions collected: {}", fix_result.suggested.to_string().cyan());
    println!("  Applied:  {}", fix_result.applied.to_string().green());
    println!("  Skipped:  {}", fix_result.skipped.to_string().yellow());
    println!("  Errors:   {}", fix_result.errors.to_string().red());
    if fix_result.quit_early {
        println!("  {}", "(Quit early)".dimmed());
    }
    println!("{}", "═".repeat(60).dimmed());
    println!();

    fix_result
}

/// Action to take after reviewing a suggestion
enum ReviewAction {
    Next,
    Previous,
    GoTo(usize),
    Ignore,
    AcceptAll,
    Quit,
}

/// Show cached suggestions and handle user interaction
/// Returns: (applied: bool, action: ReviewAction)
fn show_cached_suggestions(
    issue: &LintIssue,
    result: &SuggestionResult,
    current: usize,
    total: usize,
) -> (bool, ReviewAction) {
    println!();

    if let Some(ref error) = result.error {
        println!("  {} {}", "AI Error:".red(), error);
        return prompt_navigation(issue, current, total, false);
    }

    if result.suggestions.is_empty() {
        println!("  {}", "No AI suggestions available for this issue.".yellow());
        return prompt_navigation(issue, current, total, false);
    }

    println!(
        "  {} {} suggestion{}",
        "AI Generated".green().bold(),
        result.suggestions.len(),
        if result.suggestions.len() == 1 { "" } else { "s" }
    );
    println!();

    // Show each suggestion as diff
    for (idx, suggestion) in result.suggestions.iter().enumerate() {
        println!(
            "  {} {}",
            format!("[{}]", idx + 1).cyan().bold(),
            "Suggestion:".bold()
        );

        // Show suggestion as diff preview
        print_suggestion_preview(issue, suggestion);

        // Show explanation if available
        if let Some(ref explanation) = suggestion.explanation {
            println!("  {} {}", "Explanation:".dimmed(), explanation);
        }

        // Show confidence if available
        if let Some(confidence) = suggestion.confidence {
            let confidence_str = format!("{:.0}%", confidence * 100.0);
            let colored = if confidence >= 0.8 {
                confidence_str.green()
            } else if confidence >= 0.5 {
                confidence_str.yellow()
            } else {
                confidence_str.red()
            };
            println!("  {} {}", "Confidence:".dimmed(), colored);
        }

        println!();
    }

    // Show options
    let nolint_desc = describe_nolint_action(issue);
    println!("  {}", format!("Issue {}/{}", current + 1, total).bold().cyan());
    println!();
    for i in 1..=result.suggestions.len() {
        if i == 1 {
            println!(
                "    [{}] Apply suggestion #{} {}",
                i.to_string().cyan(),
                i,
                "(default, press Enter)".dimmed()
            );
        } else {
            println!("    [{}] Apply suggestion #{}", i.to_string().cyan(), i);
        }
    }
    println!("    [{}] Ignore - {}", "i".cyan(), nolint_desc.dimmed());
    println!("    [{}] Skip", "s".cyan());
    if current > 0 {
        println!("    [{}] Previous - go back to issue #{}", "p".cyan(), current);
    }
    println!("    [{}] Go to #N - jump to specific issue", "g".cyan());
    println!(
        "    [{}] Accept all - apply all remaining suggestions",
        "a".cyan()
    );
    println!("    [{}] Quit", "q".cyan());
    println!();
    print!("  > ");
    io::stdout().flush().ok();

    let input = read_line().trim().to_lowercase();

    // Empty input (Enter) applies suggestion #1 by default
    let input = if input.is_empty() { "1" } else { &input };

    match input {
        "i" | "ignore" => (false, ReviewAction::Ignore),
        "a" | "accept" | "all" => (false, ReviewAction::AcceptAll),
        "s" | "skip" => (false, ReviewAction::Next),
        "p" | "prev" | "previous" => (false, ReviewAction::Previous),
        "q" | "quit" => (false, ReviewAction::Quit),
        input if input.starts_with("g") => {
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(num) = parts[1].parse::<usize>() {
                    return (false, ReviewAction::GoTo(num));
                }
            }
            // Prompt for number
            print!("  {} ", "Go to issue #:".cyan());
            io::stdout().flush().ok();
            let num_input = read_line().trim().to_string();
            if let Ok(num) = num_input.parse::<usize>() {
                (false, ReviewAction::GoTo(num))
            } else {
                println!("{}", "Invalid issue number".yellow());
                (false, ReviewAction::Next)
            }
        }
        _ => {
            // Try to parse as number for applying suggestion
            if let Ok(num) = input.parse::<usize>() {
                if num >= 1 && num <= result.suggestions.len() {
                    let suggestion = &result.suggestions[num - 1];
                    // Capture original content before applying
                    let original_content = fs::read_to_string(&issue.file_path).ok();
                    let original_lines: Vec<&str> = original_content
                        .as_ref()
                        .map(|c| c.lines().collect())
                        .unwrap_or_default();
                    let start_line = issue.line;
                    let end_line = suggestion.end_line.max(issue.line);

                    if apply_suggestion(issue, suggestion) {
                        println!("  {} Applied suggestion #{}!", "✓".green(), num);
                        println!();
                        // Show diff after applying
                        print_suggestion_diff(&original_lines, suggestion, start_line, end_line);
                        return (true, ReviewAction::Next);
                    } else {
                        println!("  {} Failed to apply suggestion.", "✗".red());
                        return (false, ReviewAction::Next);
                    }
                }
            }
            println!("  {} Invalid choice, skipping.", "Invalid:".yellow());
            (false, ReviewAction::Next)
        }
    }
}

/// Prompt for navigation only (when no suggestions available)
fn prompt_navigation(issue: &LintIssue, current: usize, total: usize, _applied: bool) -> (bool, ReviewAction) {
    let nolint_desc = describe_nolint_action(issue);
    println!();
    println!("  {}", format!("Issue {}/{}", current + 1, total).bold().cyan());
    println!();
    println!("    [{}] Ignore - {}", "i".cyan(), nolint_desc.dimmed());
    println!("    [{}] Skip", "s".cyan());
    if current > 0 {
        println!("    [{}] Previous - go back to issue #{}", "p".cyan(), current);
    }
    println!("    [{}] Go to #N - jump to specific issue", "g".cyan());
    println!(
        "    [{}] Accept all - apply all remaining suggestions",
        "a".cyan()
    );
    println!("    [{}] Quit", "q".cyan());
    println!();
    print!("  > ");
    io::stdout().flush().ok();

    let input = read_line().trim().to_lowercase();

    match input.as_str() {
        "i" | "ignore" => (false, ReviewAction::Ignore),
        "a" | "accept" | "all" => (false, ReviewAction::AcceptAll),
        "p" | "prev" | "previous" => (false, ReviewAction::Previous),
        "q" | "quit" => (false, ReviewAction::Quit),
        input if input.starts_with("g") => {
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(num) = parts[1].parse::<usize>() {
                    return (false, ReviewAction::GoTo(num));
                }
            }
            print!("  {} ", "Go to issue #:".cyan());
            io::stdout().flush().ok();
            let num_input = read_line().trim().to_string();
            if let Ok(num) = num_input.parse::<usize>() {
                (false, ReviewAction::GoTo(num))
            } else {
                (false, ReviewAction::Next)
            }
        }
        _ => (false, ReviewAction::Next),
    }
}

/// Apply all suggestions automatically (for auto-apply mode)
fn apply_all_suggestions(
    issues: &[LintIssue],
    cached: &[CachedSuggestion],
    _config: &AiFixConfig,
) -> AiFixResult {
    let mut fix_result = AiFixResult::default();

    for cached_suggestion in cached {
        let issue = &issues[cached_suggestion.issue_idx];
        let result = &cached_suggestion.result;

        if result.error.is_some() || result.suggestions.is_empty() {
            fix_result.errors += 1;
            continue;
        }

        fix_result.suggested += 1;

        if let Some(suggestion) = result.suggestions.first() {
            println!(
                "  {} Applying to {}:{}",
                "→".cyan(),
                issue.file_path.display(),
                issue.line
            );

            // Capture original content before applying
            let original_content = fs::read_to_string(&issue.file_path).ok();
            let original_lines: Vec<&str> = original_content
                .as_ref()
                .map(|c| c.lines().collect())
                .unwrap_or_default();
            let start_line = issue.line;
            let end_line = suggestion.end_line.max(issue.line);

            if apply_suggestion(issue, suggestion) {
                println!("  {} Applied!", "✓".green());
                println!();
                print_suggestion_diff(&original_lines, suggestion, start_line, end_line);
                fix_result.applied += 1;
                fix_result.modified_files.insert(issue.file_path.clone());
            } else {
                println!("  {} Failed to apply.", "✗".red());
                fix_result.skipped += 1;
            }
        }
    }

    // Print summary
    println!();
    println!("{}", "═".repeat(60).dimmed());
    println!("  {}", "AI Fix Summary".bold());
    println!("{}", "─".repeat(60).dimmed());
    println!("  Suggestions collected: {}", fix_result.suggested.to_string().cyan());
    println!("  Applied:  {}", fix_result.applied.to_string().green());
    println!("  Skipped:  {}", fix_result.skipped.to_string().yellow());
    println!("  Errors:   {}", fix_result.errors.to_string().red());
    println!("{}", "═".repeat(60).dimmed());
    println!();

    fix_result
}

/// Run AI fix for a single issue (used from issue menu)
pub fn run_ai_fix_single(
    issue: &LintIssue,
    config: &AiFixConfig,
) -> Result<(bool, HashSet<PathBuf>), String> {
    // Create suggester
    let suggester = create_suggester(config)?;

    if config.verbose {
        println!(
            "  {} {} ({})",
            "Using:".dimmed(),
            suggester.provider_name(),
            suggester.model_name()
        );
    }

    // Get suggestion
    print!("  {} ", "Getting AI suggestion...".dimmed());
    io::stdout().flush().ok();

    let result = get_suggestion_for_issue(&suggester, issue, config);

    // Clear the "Getting..." line
    print!("\r{}\r", " ".repeat(40));
    io::stdout().flush().ok();

    let (applied, _quit) = show_ai_suggestions(issue, &result, config);

    let mut modified = HashSet::new();
    if applied {
        modified.insert(issue.file_path.clone());
    }

    Ok((applied, modified))
}

/// Read a line from stdin
fn read_line() -> String {
    let stdin = io::stdin();
    let mut line = String::new();
    use std::io::BufRead;
    let mut handle = stdin.lock();
    handle.read_line(&mut line).ok();
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_fix_config_default() {
        let config = AiFixConfig::default();
        assert_eq!(config.provider, AiProviderKind::Claude);
        assert_eq!(config.max_suggestions, 3);
        assert!(!config.accept_all);
    }

    #[test]
    fn test_ai_fix_config_with_provider() {
        let config = AiFixConfig::with_provider("openai");
        assert_eq!(config.provider, AiProviderKind::OpenAi);

        let config = AiFixConfig::with_provider("local");
        assert_eq!(config.provider, AiProviderKind::Local);

        let config = AiFixConfig::with_provider("mock");
        assert_eq!(config.provider, AiProviderKind::Mock);
    }

    #[test]
    fn test_ai_fix_result_default() {
        let result = AiFixResult::default();
        assert_eq!(result.suggested, 0);
        assert_eq!(result.applied, 0);
        assert_eq!(result.skipped, 0);
        assert!(!result.quit_early);
    }
}
