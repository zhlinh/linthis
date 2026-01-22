// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! CLI handler for AI-assisted fix suggestions command.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use linthis::ai::{
    AiProvider, AiProviderConfig, AiProviderKind, AiSuggester, SuggestionOptions,
    SuggestionResult,
};
use linthis::utils::types::LintIssue;

/// Options for suggest command
pub struct SuggestCommandOptions {
    pub source: String,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
    pub message: Option<String>,
    pub rule: Option<String>,
    pub provider: String,
    pub model: Option<String>,
    pub max_suggestions: usize,
    pub interactive: bool,
    pub auto_apply: bool,
    pub format: String,
    pub with_context: bool,
    pub verbose: bool,
}

/// Handle the AI suggest command
pub fn handle_suggest_command(options: SuggestCommandOptions) -> ExitCode {
    // Create AI provider
    let provider_kind: AiProviderKind = options.provider.parse().unwrap_or_default();

    let mut config = match provider_kind {
        AiProviderKind::Claude => AiProviderConfig::claude(),
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
        AiProviderKind::Claude => std::env::var("ANTHROPIC_API_KEY").ok(),
        AiProviderKind::OpenAi => std::env::var("OPENAI_API_KEY").ok(),
        _ => None,
    };

    let provider = AiProvider::new(config);
    let suggester = AiSuggester::with_provider(provider);

    // Check if provider is available
    if !suggester.is_available() {
        eprintln!(
            "Error: AI provider {} is not available",
            suggester.provider_name()
        );
        match provider_kind {
            AiProviderKind::Claude => {
                eprintln!("Set ANTHROPIC_API_KEY environment variable");
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

    // Handle single file mode
    if let Some(ref file_path) = options.file {
        return handle_file_suggestion(
            &suggester,
            file_path,
            options.line,
            options.message.as_deref(),
            options.rule.as_deref(),
            &suggestion_options,
            &options,
        );
    }

    // Load issues from result file
    let issues = match load_issues(&options.source) {
        Ok(issues) => issues,
        Err(e) => {
            eprintln!("Error loading issues: {}", e);
            return ExitCode::FAILURE;
        }
    };

    if issues.is_empty() {
        println!("No issues found to suggest fixes for.");
        return ExitCode::SUCCESS;
    }

    if options.verbose {
        println!("Found {} issues to process", issues.len());
    }

    // Load source code for each file
    let mut source_codes: HashMap<String, String> = HashMap::new();
    for issue in &issues {
        let file_key = issue.file_path.to_string_lossy().to_string();
        if !source_codes.contains_key(&file_key) {
            if let Ok(content) = fs::read_to_string(&issue.file_path) {
                source_codes.insert(file_key, content);
            }
        }
    }

    // Generate suggestions
    if options.interactive {
        handle_interactive_mode(&suggester, &issues, &source_codes, &suggestion_options, &options)
    } else {
        handle_batch_mode(&suggester, &issues, &source_codes, &suggestion_options, &options)
    }
}

/// Handle single file suggestion
fn handle_file_suggestion(
    suggester: &AiSuggester,
    file_path: &PathBuf,
    line: Option<u32>,
    message: Option<&str>,
    rule: Option<&str>,
    suggestion_options: &SuggestionOptions,
    options: &SuggestCommandOptions,
) -> ExitCode {
    let line_number = match line {
        Some(l) => l,
        None => {
            eprintln!("Error: --line is required when using --file");
            return ExitCode::FAILURE;
        }
    };

    let message = message.unwrap_or("Issue at this line");
    let rule_id = rule.unwrap_or("UNKNOWN");

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
        suggestion_options,
    );

    format_single_result(&result, &options.format, options.with_context);

    if result.is_success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Handle interactive mode
fn handle_interactive_mode(
    suggester: &AiSuggester,
    issues: &[LintIssue],
    source_codes: &HashMap<String, String>,
    suggestion_options: &SuggestionOptions,
    options: &SuggestCommandOptions,
) -> ExitCode {
    use std::io::{self, BufRead, Write};

    println!("AI Fix Suggestions - Interactive Mode");
    println!("======================================");
    println!("Commands: [a]pply, [s]kip, [q]uit, [n]ext\n");

    let stdin = io::stdin();
    let mut applied = 0;
    let mut skipped = 0;

    for (idx, issue) in issues.iter().enumerate() {
        let file_key = issue.file_path.to_string_lossy().to_string();
        let source = match source_codes.get(&file_key) {
            Some(s) => s,
            None => {
                println!("Skipping {}: source not found", issue.file_path.display());
                continue;
            }
        };

        println!("Issue {}/{}: {}", idx + 1, issues.len(), issue.message);
        println!("  File: {}:{}", issue.file_path.display(), issue.line);
        println!("  Rule: {}", issue.code.as_deref().unwrap_or("UNKNOWN"));
        println!();

        let result = suggester.suggest_fix(issue, source, suggestion_options);

        if result.suggestions.is_empty() {
            println!("  No suggestions generated.\n");
            skipped += 1;
            continue;
        }

        for (sidx, suggestion) in result.suggestions.iter().enumerate() {
            println!("  Suggestion {}:", sidx + 1);
            println!("  ```");
            for line in suggestion.code.lines() {
                println!("  {}", line);
            }
            println!("  ```");
            if let Some(ref exp) = suggestion.explanation {
                println!("  Explanation: {}", exp);
            }
            println!();
        }

        print!("Action [a/s/q/n]: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        stdin.lock().read_line(&mut input).unwrap();
        let input = input.trim().to_lowercase();

        match input.as_str() {
            "a" | "apply" => {
                if options.auto_apply {
                    // Apply the first suggestion
                    if let Some(suggestion) = result.suggestions.first() {
                        if apply_suggestion(issue, suggestion) {
                            println!("  Applied!\n");
                            applied += 1;
                        } else {
                            println!("  Failed to apply.\n");
                        }
                    }
                } else {
                    println!("  Auto-apply not enabled. Use --auto-apply flag.\n");
                }
            }
            "q" | "quit" => {
                println!("\nQuitting...");
                break;
            }
            "s" | "skip" | "n" | "next" | "" => {
                skipped += 1;
            }
            _ => {
                println!("  Unknown command, skipping.\n");
                skipped += 1;
            }
        }
    }

    println!("\nSummary: {} applied, {} skipped", applied, skipped);
    ExitCode::SUCCESS
}

/// Handle batch mode
fn handle_batch_mode(
    suggester: &AiSuggester,
    issues: &[LintIssue],
    source_codes: &HashMap<String, String>,
    suggestion_options: &SuggestionOptions,
    options: &SuggestCommandOptions,
) -> ExitCode {
    let report = suggester.suggest_fixes(issues, source_codes, suggestion_options);

    match options.format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&report).unwrap_or_default();
            println!("{}", json);
        }
        "diff" => {
            for result in &report.results {
                if !result.suggestions.is_empty() {
                    println!("--- a/{}", result.file_path);
                    println!("+++ b/{}", result.file_path);
                    println!("@@ -{},{} +{},{} @@", result.line_number, 1, result.line_number, 1);
                    for suggestion in &result.suggestions {
                        println!("-{}", result.context.as_ref().map(|c| c.issue_lines.as_str()).unwrap_or(""));
                        println!("+{}", suggestion.code.lines().next().unwrap_or(""));
                    }
                    println!();
                }
            }
        }
        _ => {
            // Human-readable format
            println!("AI Fix Suggestions Report");
            println!("=========================\n");
            println!(
                "Provider: {} ({})",
                report.provider, report.model
            );
            println!(
                "Processed: {} issues in {}ms",
                report.total_issues, report.total_time_ms
            );
            println!(
                "Success rate: {:.1}%\n",
                report.success_rate() * 100.0
            );

            for result in &report.results {
                format_single_result(result, "human", options.with_context);
            }

            println!("\nSummary:");
            println!("  Successful: {}", report.successful);
            println!("  Failed: {}", report.failed);
        }
    }

    if report.successful > 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Format a single suggestion result
fn format_single_result(result: &SuggestionResult, format: &str, with_context: bool) {
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
            println!("{}:{}", result.file_path, result.line_number);
            println!("  Issue: {}", result.message);

            if let Some(ref err) = result.error {
                println!("  Error: {}", err);
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
                println!("  No suggestions generated.");
            } else {
                for (idx, suggestion) in result.suggestions.iter().enumerate() {
                    println!("  Suggestion {}:", idx + 1);
                    println!("  ```{}", suggestion.language);
                    for line in suggestion.code.lines() {
                        println!("  {}", line);
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

/// Load issues from a result file
fn load_issues(source: &str) -> Result<Vec<LintIssue>, String> {
    let path = if source == "last" {
        find_last_result_file()?
    } else {
        PathBuf::from(source)
    };

    if !path.exists() {
        return Err(format!("Result file not found: {}", path.display()));
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    // Try to parse as JSON RunResult
    if let Ok(result) = serde_json::from_str::<linthis::utils::types::RunResult>(&content) {
        return Ok(result.issues);
    }

    // Try to parse as JSON array of issues
    if let Ok(issues) = serde_json::from_str::<Vec<LintIssue>>(&content) {
        return Ok(issues);
    }

    Err("Failed to parse result file".to_string())
}

/// Find the most recent result file
fn find_last_result_file() -> Result<PathBuf, String> {
    let result_dir = PathBuf::from(".linthis/result");
    if !result_dir.exists() {
        return Err("No result directory found. Run 'linthis' first.".to_string());
    }

    let mut result_files: Vec<_> = fs::read_dir(&result_dir)
        .map_err(|e| format!("Failed to read result directory: {}", e))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().map(|ext| ext == "json").unwrap_or(false)
        })
        .collect();

    if result_files.is_empty() {
        return Err("No result files found. Run 'linthis' first.".to_string());
    }

    // Sort by modification time, most recent first
    result_files.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|m| m.modified()).ok();
        let b_time = b.metadata().and_then(|m| m.modified()).ok();
        b_time.cmp(&a_time)
    });

    Ok(result_files[0].path())
}

/// Apply a suggestion (write to file)
fn apply_suggestion(
    issue: &LintIssue,
    suggestion: &linthis::ai::FixSuggestion,
) -> bool {
    let content = match fs::read_to_string(&issue.file_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let lines: Vec<&str> = content.lines().collect();
    let line_idx = issue.line - 1;

    if line_idx >= lines.len() {
        return false;
    }

    // Replace the line with the suggestion
    let mut new_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();

    // Simple single-line replacement
    let suggestion_lines: Vec<&str> = suggestion.code.lines().collect();
    if suggestion_lines.is_empty() {
        return false;
    }

    // Replace the issue line with suggestion
    new_lines[line_idx] = suggestion_lines[0].to_string();

    // If suggestion has more lines, insert them
    for (i, line) in suggestion_lines.iter().skip(1).enumerate() {
        new_lines.insert(line_idx + 1 + i, line.to_string());
    }

    let new_content = new_lines.join("\n");
    fs::write(&issue.file_path, new_content).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_command_options() {
        let options = SuggestCommandOptions {
            source: "last".to_string(),
            file: None,
            line: None,
            message: None,
            rule: None,
            provider: "mock".to_string(),
            model: None,
            max_suggestions: 3,
            interactive: false,
            auto_apply: false,
            format: "human".to_string(),
            with_context: false,
            verbose: false,
        };

        assert_eq!(options.source, "last");
        assert_eq!(options.provider, "mock");
    }
}
