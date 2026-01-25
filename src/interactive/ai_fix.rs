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
    SuggestionResult,
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
    pub auto_apply: bool,
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
            auto_apply: false,
            verbose: false,
            parallel_jobs: 8,
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
    pub fn with_auto_apply(mut self, auto_apply: bool) -> Self {
        self.auto_apply = auto_apply;
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
    let mut provider_config = match config.provider {
        AiProviderKind::Claude => AiProviderConfig::claude(),
        AiProviderKind::ClaudeCli => AiProviderConfig::claude_cli(),
        AiProviderKind::OpenAi => AiProviderConfig::openai(),
        AiProviderKind::Local => AiProviderConfig::local(),
        AiProviderKind::Mock => AiProviderConfig::mock(),
    };

    // Override model if specified
    if let Some(ref model) = config.model {
        provider_config.model = model.clone();
    }

    // Set API key from environment (for Claude, try AUTH_TOKEN first)
    provider_config.api_key = match config.provider {
        AiProviderKind::Claude => std::env::var("ANTHROPIC_AUTH_TOKEN")
            .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
            .ok(),
        AiProviderKind::OpenAi => std::env::var("OPENAI_API_KEY").ok(),
        _ => None,
    };

    // Set endpoint from environment for Claude
    if config.provider == AiProviderKind::Claude {
        if let Ok(base_url) = std::env::var("ANTHROPIC_BASE_URL") {
            provider_config.endpoint = Some(base_url);
        }
    }

    let provider = AiProvider::new(provider_config);
    let suggester = AiSuggester::with_provider(provider);

    if !suggester.is_available() {
        let hint = match config.provider {
            AiProviderKind::Claude => "Set ANTHROPIC_AUTH_TOKEN or ANTHROPIC_API_KEY environment variable",
            AiProviderKind::ClaudeCli => "Install Claude CLI (claude command must be available)",
            AiProviderKind::OpenAi => "Set OPENAI_API_KEY environment variable",
            AiProviderKind::Local => "Set LINTHIS_AI_ENDPOINT environment variable",
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
    if config.auto_apply {
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
/// This uses a two-phase approach:
/// 1. Phase 1: Batch collect all AI suggestions (user waits once)
/// 2. Phase 2: Interactive review of all suggestions (no waiting)
pub fn run_ai_fix_all(result: &RunResult, config: &AiFixConfig) -> AiFixResult {
    let issues = &result.issues;

    if issues.is_empty() {
        println!("{}", "No issues to fix.".green());
        return AiFixResult::default();
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
    if config.auto_apply {
        println!("  Mode: {} (will apply automatically)", "Auto-apply".yellow());
    } else {
        println!("  Mode: Batch collect, then review");
    }
    println!("{}", "═".repeat(60).dimmed());
    println!();

    // Confirm before starting
    if !config.auto_apply {
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
    if config.auto_apply {
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
        assert!(!config.auto_apply);
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
