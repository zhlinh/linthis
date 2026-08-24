// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! AI-assisted fix suggestion generation.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;

use super::context::{extract_context, CodeContext, ContextOptions};
use super::prompts::{IssueCategory, PromptBuilder, PromptVariables};
use super::provider::{AiProvider, AiProviderTrait};
use crate::utils::types::{LintIssue, Severity};

/// Options for suggestion generation
#[derive(Debug, Clone)]
pub struct SuggestionOptions {
    /// Context extraction options
    pub context_options: ContextOptions,
    /// Maximum suggestions per issue
    pub max_suggestions: usize,
    /// Include explanation with fix
    pub include_explanation: bool,
    /// Include confidence score
    pub include_confidence: bool,
    /// Batch size for parallel processing
    pub batch_size: usize,
    /// Skip if issue already has a suggestion
    pub skip_with_suggestion: bool,
}

impl Default for SuggestionOptions {
    fn default() -> Self {
        Self {
            context_options: ContextOptions::default(),
            max_suggestions: 3,
            include_explanation: true,
            include_confidence: true,
            batch_size: 5,
            skip_with_suggestion: true,
        }
    }
}

/// A single fix suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixSuggestion {
    /// The suggested fix code
    pub code: String,
    /// Explanation of the fix
    pub explanation: Option<String>,
    /// Confidence score (0.0 - 1.0)
    pub confidence: Option<f32>,
    /// Whether this is a complete replacement or partial
    pub is_complete: bool,
    /// Start line for the fix
    pub start_line: usize,
    /// End line for the fix
    pub end_line: usize,
    /// Language of the code
    pub language: String,
}

impl FixSuggestion {
    pub fn new(code: String, start_line: usize, end_line: usize, language: &str) -> Self {
        Self {
            code,
            explanation: None,
            confidence: None,
            is_complete: true,
            start_line,
            end_line,
            language: language.to_string(),
        }
    }

    pub fn with_explanation(mut self, explanation: &str) -> Self {
        self.explanation = Some(explanation.to_string());
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = Some(confidence.clamp(0.0, 1.0));
        self
    }
}

/// Result of suggestion generation for a single issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionResult {
    /// The original issue code
    pub issue_code: String,
    /// File path
    pub file_path: String,
    /// Line number
    pub line_number: usize,
    /// Issue message
    pub message: String,
    /// Generated suggestions
    pub suggestions: Vec<FixSuggestion>,
    /// Error if generation failed
    pub error: Option<String>,
    /// Generation time in milliseconds
    pub generation_time_ms: u64,
    /// Code context used
    pub context: Option<CodeContext>,
}

impl SuggestionResult {
    pub fn success(
        issue_code: &str,
        file_path: &str,
        line_number: usize,
        message: &str,
        suggestions: Vec<FixSuggestion>,
        generation_time_ms: u64,
    ) -> Self {
        Self {
            issue_code: issue_code.to_string(),
            file_path: file_path.to_string(),
            line_number,
            message: message.to_string(),
            suggestions,
            error: None,
            generation_time_ms,
            context: None,
        }
    }

    pub fn failure(
        issue_code: &str,
        file_path: &str,
        line_number: usize,
        message: &str,
        error: &str,
    ) -> Self {
        Self {
            issue_code: issue_code.to_string(),
            file_path: file_path.to_string(),
            line_number,
            message: message.to_string(),
            suggestions: Vec::new(),
            error: Some(error.to_string()),
            generation_time_ms: 0,
            context: None,
        }
    }

    pub fn with_context(mut self, context: CodeContext) -> Self {
        self.context = Some(context);
        self
    }

    pub fn is_success(&self) -> bool {
        self.error.is_none() && !self.suggestions.is_empty()
    }
}

/// Full suggestions report for multiple issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionsReport {
    /// Individual suggestion results
    pub results: Vec<SuggestionResult>,
    /// Total issues processed
    pub total_issues: usize,
    /// Successfully generated suggestions
    pub successful: usize,
    /// Failed generations
    pub failed: usize,
    /// Total processing time in milliseconds
    pub total_time_ms: u64,
    /// AI provider used
    pub provider: String,
    /// Model used
    pub model: String,
}

impl SuggestionsReport {
    pub fn new(provider: &str, model: &str) -> Self {
        Self {
            results: Vec::new(),
            total_issues: 0,
            successful: 0,
            failed: 0,
            total_time_ms: 0,
            provider: provider.to_string(),
            model: model.to_string(),
        }
    }

    pub fn add_result(&mut self, result: SuggestionResult) {
        self.total_issues += 1;
        if result.is_success() {
            self.successful += 1;
        } else {
            self.failed += 1;
        }
        self.results.push(result);
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_issues == 0 {
            0.0
        } else {
            self.successful as f64 / self.total_issues as f64
        }
    }
}

/// Main AI suggester
pub struct AiSuggester {
    provider: AiProvider,
    prompt_builder: PromptBuilder,
}

impl AiSuggester {
    /// Create a new suggester with the given provider
    pub fn with_provider(provider: AiProvider) -> Self {
        Self {
            provider,
            prompt_builder: PromptBuilder::new(),
        }
    }

    /// Create suggester from environment variables
    pub fn from_env() -> Self {
        Self::with_provider(AiProvider::default())
    }

    /// Check if the suggester is available
    pub fn is_available(&self) -> bool {
        self.provider.is_available()
    }

    /// Get the provider name
    pub fn provider_name(&self) -> &str {
        self.provider.name()
    }

    /// Get the model name
    pub fn model_name(&self) -> &str {
        &self.provider.config().model
    }

    /// Generate fix suggestion for a single issue
    pub fn suggest_fix(
        &self,
        issue: &LintIssue,
        source_code: &str,
        options: &SuggestionOptions,
    ) -> SuggestionResult {
        let start = Instant::now();

        let file_path_str = issue.file_path.to_string_lossy().to_string();
        let issue_code = issue.code.as_deref().unwrap_or("UNKNOWN");

        // Note: skip_with_suggestion is now disabled by default for AI fix
        // because the linter suggestion is human-readable text, not actual code
        if options.skip_with_suggestion && issue.suggestion.is_some() {
            return SuggestionResult::success(
                issue_code,
                &file_path_str,
                issue.line,
                &issue.message,
                vec![],
                0,
            );
        }

        // Extract code context
        let context = match extract_context_from_source(
            &file_path_str,
            source_code,
            issue.line,
            &options.context_options,
        ) {
            Ok(ctx) => ctx,
            Err(e) => {
                return SuggestionResult::failure(
                    issue_code,
                    &file_path_str,
                    issue.line,
                    &issue.message,
                    &format!("Failed to extract context: {}", e),
                );
            }
        };

        // Determine issue category
        let category = categorize_issue(issue);

        // Build prompt variables
        let vars = PromptVariables::new()
            .with_language(&context.language)
            .with_file_path(&context.file_path)
            .with_line_number(context.line_number as u32)
            .with_issue_message(&issue.message)
            .with_rule_id(issue_code)
            .with_code_context(&context.full_snippet)
            .with_issue_line(&context.issue_lines);

        let vars = if let Some(ref imports) = context.imports {
            vars.with_imports(imports)
        } else {
            vars
        };

        let vars = if let Some(ref scope) = context.scope {
            vars.with_scope(scope)
        } else {
            vars
        };

        // Build and send prompt
        let (system_prompt, user_prompt) = self.prompt_builder.build_prompt(category, &vars);

        let response = match self.provider.complete(&user_prompt, Some(&system_prompt)) {
            Ok(r) => r,
            Err(e) => {
                return SuggestionResult::failure(
                    issue_code,
                    &file_path_str,
                    issue.line,
                    &issue.message,
                    &e,
                );
            }
        };

        // Parse response into suggestions
        let suggestions = parse_ai_response(&response, &context.language, issue.line);

        let elapsed = start.elapsed().as_millis() as u64;

        let mut result = SuggestionResult::success(
            issue_code,
            &file_path_str,
            issue.line,
            &issue.message,
            suggestions,
            elapsed,
        );

        result = result.with_context(context);

        result
    }

    /// Generate suggestions for multiple issues
    pub fn suggest_fixes(
        &self,
        issues: &[LintIssue],
        source_codes: &std::collections::HashMap<String, String>,
        options: &SuggestionOptions,
    ) -> SuggestionsReport {
        let start = Instant::now();
        let mut report = SuggestionsReport::new(self.provider_name(), self.model_name());

        for issue in issues {
            let file_path_str = issue.file_path.to_string_lossy().to_string();
            let issue_code = issue.code.as_deref().unwrap_or("UNKNOWN");

            let source = match source_codes.get(&file_path_str) {
                Some(s) => s,
                None => {
                    report.add_result(SuggestionResult::failure(
                        issue_code,
                        &file_path_str,
                        issue.line,
                        &issue.message,
                        "Source code not found",
                    ));
                    continue;
                }
            };

            let result = self.suggest_fix(issue, source, options);
            report.add_result(result);
        }

        report.total_time_ms = start.elapsed().as_millis() as u64;

        report
    }

    /// Suggest fix for a file at a specific line
    pub fn suggest_fix_for_file(
        &self,
        file_path: &Path,
        line_number: usize,
        message: &str,
        rule_id: &str,
        options: &SuggestionOptions,
    ) -> SuggestionResult {
        let start = Instant::now();
        let file_path_str = file_path.to_string_lossy().to_string();

        // Extract context from file
        let context = match extract_context(file_path, line_number as u32, &options.context_options)
        {
            Ok(ctx) => ctx,
            Err(e) => {
                return SuggestionResult::failure(
                    rule_id,
                    &file_path_str,
                    line_number,
                    message,
                    &e,
                );
            }
        };

        // Determine category from rule_id pattern
        let category = categorize_from_rule_id(rule_id);

        // Build prompt
        let vars = PromptVariables::new()
            .with_language(&context.language)
            .with_file_path(&context.file_path)
            .with_line_number(line_number as u32)
            .with_issue_message(message)
            .with_rule_id(rule_id)
            .with_code_context(&context.full_snippet)
            .with_issue_line(&context.issue_lines);

        let (system_prompt, user_prompt) = self.prompt_builder.build_prompt(category, &vars);

        let response = match self.provider.complete(&user_prompt, Some(&system_prompt)) {
            Ok(r) => r,
            Err(e) => {
                return SuggestionResult::failure(
                    rule_id,
                    &file_path_str,
                    line_number,
                    message,
                    &e,
                );
            }
        };

        let suggestions = parse_ai_response(&response, &context.language, line_number);
        let elapsed = start.elapsed().as_millis() as u64;

        SuggestionResult::success(
            rule_id,
            &file_path_str,
            line_number,
            message,
            suggestions,
            elapsed,
        )
        .with_context(context)
    }
}

/// Extract context from source code string
fn extract_context_from_source(
    file_path: &str,
    source: &str,
    line_number: usize,
    options: &ContextOptions,
) -> Result<CodeContext, String> {
    let lines: Vec<&str> = source.lines().collect();
    let total_lines = lines.len();

    if total_lines == 0 {
        return Err("File is empty".to_string());
    }

    // Handle line 0 as file-level issue (use beginning of file)
    let effective_line = if line_number == 0 { 1 } else { line_number };

    if effective_line > total_lines {
        return Err(format!(
            "Line number {} out of range (file has {} lines)",
            line_number, total_lines
        ));
    }

    let idx = effective_line - 1;
    let language = detect_language_from_path(file_path);

    let mut context = CodeContext::new(file_path, &language, effective_line as u32);

    // For file-level issues (line 0), show first N lines as context
    if line_number == 0 {
        let context_lines = (options.lines_before + options.lines_after + 1).min(total_lines);
        context.issue_lines = "(file-level issue)".to_string();
        context.before = String::new();
        context.after = lines[..context_lines].join("\n");
        context.full_snippet = format!(">>> File-level issue <<<\n{}", context.after);
        return Ok(context);
    }

    // Extract issue line
    context.issue_lines = lines[idx].to_string();

    // Extract before context
    let start = idx.saturating_sub(options.lines_before);
    if start < idx {
        context.before = lines[start..idx].join("\n");
    }

    // Extract after context
    let end = (idx + 1 + options.lines_after).min(total_lines);
    if idx + 1 < end {
        context.after = lines[idx + 1..end].join("\n");
    }

    // Build full snippet
    context.full_snippet = format!(
        "{}\n>>> {} <<<\n{}",
        context.before, context.issue_lines, context.after
    );

    Ok(context)
}

/// Detect language from file path
fn detect_language_from_path(path: &str) -> String {
    let path = std::path::Path::new(path);
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| match ext {
            "rs" => "rust",
            "py" | "pyw" => "python",
            "js" | "jsx" | "mjs" => "javascript",
            "ts" | "tsx" | "mts" => "typescript",
            "go" => "go",
            "java" => "java",
            "cpp" | "cc" | "cxx" | "c" | "h" | "hpp" => "cpp",
            "swift" => "swift",
            "kt" | "kts" => "kotlin",
            "rb" => "ruby",
            "php" => "php",
            _ => ext,
        })
        .unwrap_or("unknown")
        .to_string()
}

/// Categorize issue based on lint issue properties
fn categorize_issue(issue: &LintIssue) -> IssueCategory {
    let code = issue.code.as_deref().unwrap_or("").to_lowercase();
    let message = issue.message.to_lowercase();

    CATEGORY_KEYWORDS
        .iter()
        .find(|(_, code_words, msg_words)| {
            code_words.iter().any(|w| code.contains(w))
                || msg_words.iter().any(|w| message.contains(w))
        })
        .map(|(category, _, _)| *category)
        .unwrap_or({
            // Nothing matched: an error is most likely a bug.
            if issue.severity == Severity::Error {
                IssueCategory::Bug
            } else {
                IssueCategory::General
            }
        })
}

/// Words that place an issue in a category, checked against the rule code and
/// the message separately — the two use different vocabulary ("vuln" appears
/// in codes, "vulnerability" in prose).
///
/// Order matters: the first match wins, so the more specific categories come
/// before the general ones.
type CategoryKeywords = (IssueCategory, &'static [&'static str], &'static [&'static str]);
const CATEGORY_KEYWORDS: &[CategoryKeywords] = &[
    (
        IssueCategory::Security,
        &["security", "vuln"],
        &["security", "vulnerability", "injection", "xss"],
    ),
    (
        IssueCategory::Performance,
        &["perf"],
        &["performance", "slow", "optimize"],
    ),
    (
        IssueCategory::Complexity,
        &["complex"],
        &["complexity", "cyclomatic", "cognitive"],
    ),
    (
        IssueCategory::Style,
        &["style", "format"],
        &["naming", "indent"],
    ),
    (IssueCategory::Deprecation, &["deprecated"], &["deprecated"]),
    (
        IssueCategory::Type,
        &["type"],
        &["type mismatch", "type error"],
    ),
    (
        IssueCategory::Documentation,
        &["doc"],
        &["documentation", "missing doc"],
    ),
];

/// Categorize from rule ID patterns
fn categorize_from_rule_id(rule_id: &str) -> IssueCategory {
    let rule_lower = rule_id.to_lowercase();

    // Common linter rule patterns
    if rule_lower.starts_with("sec") || rule_lower.contains("security") {
        IssueCategory::Security
    } else if rule_lower.starts_with("perf") || rule_lower.contains("performance") {
        IssueCategory::Performance
    } else if rule_lower.contains("complex") {
        IssueCategory::Complexity
    } else if rule_lower.starts_with("style") || rule_lower.starts_with("fmt") {
        IssueCategory::Style
    } else if rule_lower.contains("deprecated") {
        IssueCategory::Deprecation
    } else if rule_lower.starts_with("type") {
        IssueCategory::Type
    } else if rule_lower.starts_with("doc") {
        IssueCategory::Documentation
    } else {
        IssueCategory::General
    }
}

/// Parse AI response into fix suggestions
fn parse_ai_response(response: &str, language: &str, line_number: usize) -> Vec<FixSuggestion> {
    // Unified diff is preferred: it says which lines to replace.
    let mut suggestions = parse_unified_diff(response, language).unwrap_or_default();

    // Then fenced blocks, language-tagged first, plain ones as a fallback.
    if suggestions.is_empty() {
        let tagged = format!("```{}\\s*\\n([\\s\\S]*?)\\n```", language);
        suggestions = extract_code_blocks(response, &tagged, language, line_number);
    }
    if suggestions.is_empty() {
        suggestions = extract_code_blocks(
            response,
            r"```\s*\n([\s\S]*?)\n```",
            language,
            line_number,
        );
    }

    // Last resort: the whole reply, if it reads like code at all.
    if suggestions.is_empty() && looks_like_code(response) {
        suggestions.push(FixSuggestion::new(
            response.trim().to_string(),
            line_number,
            line_number,
            language,
        ));
    }

    if let Some(explanation) = extract_explanation(response) {
        for suggestion in &mut suggestions {
            suggestion.explanation = Some(explanation.clone());
        }
    }

    suggestions
}

/// Pull fenced code blocks matching `pattern` out of an AI reply.
///
/// Diff blocks are skipped: `parse_unified_diff` has already had its turn at
/// them, and treating a diff as literal replacement code corrupts the file.
fn extract_code_blocks(
    response: &str,
    pattern: &str,
    language: &str,
    line_number: usize,
) -> Vec<FixSuggestion> {
    let Ok(re) = regex::Regex::new(pattern) else {
        return Vec::new();
    };

    re.captures_iter(response)
        .filter_map(|cap| {
            let code = cap.get(1)?.as_str().trim();
            if code.is_empty() || code.starts_with("@@") || code.starts_with("---") {
                return None;
            }
            // An over-eager model returns the whole file; keep it bounded.
            let limited = limit_code_block_size(code, 5);
            let line_count = limited.lines().count();
            Some(FixSuggestion::new(
                limited,
                line_number,
                line_number + line_count.saturating_sub(1),
                language,
            ))
        })
        .collect()
}

/// Parse unified diff format from AI response
fn parse_unified_diff(response: &str, language: &str) -> Option<Vec<FixSuggestion>> {
    // Look for diff code blocks
    let diff_pattern = r"```diff\s*\n([\s\S]*?)\n```";

    let re = regex::Regex::new(diff_pattern).ok()?;
    let mut suggestions = Vec::new();

    for cap in re.captures_iter(response) {
        if let Some(diff_content) = cap.get(1) {
            if let Some(suggestion) = parse_diff_hunk(diff_content.as_str(), language) {
                suggestions.push(suggestion);
            }
        }
    }

    // Also try to find inline diff (not in code block)
    if suggestions.is_empty() {
        if let Some(suggestion) = parse_diff_hunk(response, language) {
            suggestions.push(suggestion);
        }
    }

    if suggestions.is_empty() {
        None
    } else {
        Some(suggestions)
    }
}

/// The `+`/`-` lines of a hunk, and how many context lines precede them.
struct HunkChanges {
    added: Vec<String>,
    removed: Vec<String>,
    first_change_offset: usize,
}

/// Parse `@@ -start,count +start,count @@` and return the old start line plus
/// the body that follows the header.
fn parse_hunk_header(diff: &str) -> Option<(usize, &str)> {
    let re = regex::Regex::new(r"@@\s*-(\d+)(?:,(\d+))?\s+\+(\d+)(?:,(\d+))?\s*@@").ok()?;
    let cap = re.captures(diff)?;
    let old_start: usize = cap.get(1)?.as_str().parse().ok()?;
    Some((old_start, &diff[cap.get(0)?.end()..]))
}

/// Split a hunk body into added and removed lines.
///
/// Context lines are counted, not collected: they are only needed to know how
/// far into the hunk the first real change sits. `+++`/`---` file headers are
/// not changes.
fn split_hunk_changes(body: &str) -> HunkChanges {
    let mut changes = HunkChanges {
        added: Vec::new(),
        removed: Vec::new(),
        first_change_offset: 0,
    };
    let mut context_before = 0;
    let mut seen_change = false;

    for line in body.lines() {
        let target = if line.starts_with('+') && !line.starts_with("+++") {
            Some(&mut changes.added)
        } else if line.starts_with('-') && !line.starts_with("---") {
            Some(&mut changes.removed)
        } else {
            if line.starts_with(' ') && !seen_change {
                context_before += 1;
            }
            None
        };

        if let Some(bucket) = target {
            if !seen_change {
                seen_change = true;
                changes.first_change_offset = context_before;
            }
            bucket.push(line[1..].to_string());
        }
    }

    changes
}

/// Parse a single diff hunk and extract the fix
fn parse_diff_hunk(diff: &str, language: &str) -> Option<FixSuggestion> {
    let (old_start, body) = parse_hunk_header(diff)?;
    let changes = split_hunk_changes(body);

    if changes.added.is_empty() {
        return None;
    }

    // Context lines before the first +/- shift where the replacement starts.
    let actual_start = old_start + changes.first_change_offset;
    let removed_count = changes.removed.len();

    // A model that returns far more added lines than it removed has pasted
    // surrounding context along with the fix; keep a proportionate slice.
    let fix_code = if removed_count > 0 && changes.added.len() > removed_count * 3 {
        let take = (removed_count * 2).max(3).min(changes.added.len());
        changes.added[..take].join("\n")
    } else {
        changes.added.join("\n")
    };

    let end_line = if removed_count > 0 {
        actual_start + removed_count - 1
    } else {
        actual_start
    };

    Some(FixSuggestion::new(
        fix_code,
        actual_start,
        end_line,
        language,
    ))
}

/// Limit code block size to avoid applying entire file as a fix
/// If the code block has more than max_lines, try to extract only the relevant portion
fn limit_code_block_size(code: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = code.lines().collect();

    if lines.len() <= max_lines {
        return code.to_string();
    }

    // If code is too long, it's likely the AI returned too much
    // Try to find the actual changed portion by looking for common patterns

    // For simple cases, just take the first few lines
    // This is a heuristic - the AI should really return minimal diffs
    lines[..max_lines].join("\n")
}

/// Check if text looks like code
fn looks_like_code(text: &str) -> bool {
    let code_indicators = [
        "fn ",
        "let ",
        "const ",
        "var ",
        "function ",
        "def ",
        "class ",
        "if ",
        "for ",
        "while ",
        "return ",
        "import ",
        "from ",
        "use ",
        "{",
        "}",
        "(",
        ")",
        ";",
        "=>",
        "->",
    ];

    let trimmed = text.trim();
    code_indicators.iter().any(|&ind| trimmed.contains(ind))
}

/// Extract explanation from response
fn extract_explanation(response: &str) -> Option<String> {
    // Look for common explanation patterns
    let patterns = [
        "Note:",
        "Explanation:",
        "Security note:",
        "Bug fix note:",
        "This fix",
        "The change",
    ];

    for pattern in &patterns {
        if let Some(pos) = response.find(pattern) {
            let rest = &response[pos..];
            // Get text until next code block or end
            if let Some(end) = rest.find("```") {
                return Some(rest[..end].trim().to_string());
            } else {
                return Some(rest.trim().to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_suggestion_options_default() {
        let options = SuggestionOptions::default();
        assert_eq!(options.max_suggestions, 3);
        assert!(options.include_explanation);
        assert!(options.skip_with_suggestion);
    }

    #[test]
    fn test_fix_suggestion() {
        let suggestion = FixSuggestion::new("let x = 5;".to_string(), 10, 10, "rust")
            .with_explanation("Fixed unused variable")
            .with_confidence(0.9);

        assert_eq!(suggestion.start_line, 10);
        assert_eq!(suggestion.confidence, Some(0.9));
        assert!(suggestion.explanation.is_some());
    }

    #[test]
    fn test_suggestion_result() {
        let result =
            SuggestionResult::success("W0001", "src/main.rs", 10, "unused variable", vec![], 100);

        assert!(!result.is_success()); // No suggestions

        let result_with_suggestions = SuggestionResult::success(
            "W0001",
            "src/main.rs",
            10,
            "unused variable",
            vec![FixSuggestion::new(
                "let _x = 5;".to_string(),
                10,
                10,
                "rust",
            )],
            100,
        );

        assert!(result_with_suggestions.is_success());
    }

    #[test]
    fn test_suggestions_report() {
        let mut report = SuggestionsReport::new("Mock", "mock-model");

        let success = SuggestionResult::success(
            "W0001",
            "test.rs",
            1,
            "test",
            vec![FixSuggestion::new("code".to_string(), 1, 1, "rust")],
            10,
        );

        let failure = SuggestionResult::failure("W0002", "test.rs", 2, "test", "error");

        report.add_result(success);
        report.add_result(failure);

        assert_eq!(report.total_issues, 2);
        assert_eq!(report.successful, 1);
        assert_eq!(report.failed, 1);
        assert_eq!(report.success_rate(), 0.5);
    }

    #[test]
    fn test_parse_ai_response() {
        let response = r#"
Here's the fix:
```rust
let _x = 5;
```
Note: Added underscore prefix to indicate intentionally unused variable.
"#;

        let suggestions = parse_ai_response(response, "rust", 10);

        assert_eq!(suggestions.len(), 1);
        assert!(suggestions[0].code.contains("let _x"));
        assert!(suggestions[0].explanation.is_some());
    }

    #[test]
    fn test_parse_unified_diff() {
        let response = r#"
Here's the fix:
```diff
@@ -5,1 +5,1 @@
-    let x = 5;
+    let _x = 5;
```
Note: Added underscore prefix.
"#;

        let suggestions = parse_ai_response(response, "rust", 5);

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].start_line, 5);
        assert_eq!(suggestions[0].end_line, 5);
        assert!(suggestions[0].code.contains("let _x"));
    }

    #[test]
    fn test_parse_unified_diff_multiline() {
        let response = r#"
```diff
@@ -10,4 +10,3 @@
 context before
-    old line 1
-    old line 2
+    new line
 context after
```
"#;

        let suggestions = parse_ai_response(response, "python", 10);

        assert_eq!(suggestions.len(), 1);
        // Start line is 11 (10 + 1 context line before the change)
        assert_eq!(suggestions[0].start_line, 11);
        // End line is 12 (start + 2 removed lines - 1)
        assert_eq!(suggestions[0].end_line, 12);
    }

    #[test]
    fn test_parse_diff_with_context() {
        let response = r#"
```diff
@@ -4,3 +4,3 @@
     let y = 10;
-    let x = 5;
+    let _x = 5;
     println!("{}", y);
```
"#;

        let suggestions = parse_ai_response(response, "rust", 5);

        assert_eq!(suggestions.len(), 1);
        // The fix should include the replaced line
        assert!(suggestions[0].code.contains("let _x"));
    }

    #[test]
    fn test_categorize_issue() {
        let mut issue = LintIssue {
            file_path: PathBuf::from("test.rs"),
            line: 1,
            column: None,
            severity: Severity::Warning,
            message: "SQL injection vulnerability detected".to_string(),
            code: Some("SEC001".to_string()),
            source: Some("linthis".to_string()),
            suggestion: None,
            language: None,
            code_line: None,
            context_before: Vec::new(),
            context_after: Vec::new(),
            ignore: None,
        };

        assert_eq!(categorize_issue(&issue), IssueCategory::Security);

        issue.message = "High cyclomatic complexity".to_string();
        issue.code = Some("COMPLEX001".to_string());
        assert_eq!(categorize_issue(&issue), IssueCategory::Complexity);
    }

    #[test]
    fn categorize_falls_through_to_severity() {
        // Nothing in the table matches, so severity decides.
        let mut issue = LintIssue::new(
            PathBuf::from("t.rs"),
            1,
            "something odd happened".into(),
            Severity::Error,
        );
        assert_eq!(categorize_issue(&issue), IssueCategory::Bug);

        issue.severity = Severity::Warning;
        assert_eq!(categorize_issue(&issue), IssueCategory::General);
    }

    #[test]
    fn categorize_checks_code_and_message_separately() {
        // "vuln" is code vocabulary, "vulnerability" is prose — either alone
        // must be enough.
        let by_code = LintIssue::new(PathBuf::from("t.rs"), 1, "bad thing".into(), Severity::Warning)
            .with_code("VULN-3".into());
        assert_eq!(categorize_issue(&by_code), IssueCategory::Security);

        let by_message = LintIssue::new(
            PathBuf::from("t.rs"),
            1,
            "possible xss here".into(),
            Severity::Warning,
        );
        assert_eq!(categorize_issue(&by_message), IssueCategory::Security);
    }

    #[test]
    fn code_block_extraction_skips_diffs_and_blanks() {
        let response = "```rust\n@@ -1 +1 @@\n```\n```rust\nlet x = 1;\n```";
        let blocks = extract_code_blocks(
            response,
            "```rust\\s*\\n([\\s\\S]*?)\\n```",
            "rust",
            7,
        );
        // The diff block is parse_unified_diff's job, not a literal replacement.
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].code, "let x = 1;");
        assert_eq!(blocks[0].start_line, 7);
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language_from_path("src/main.rs"), "rust");
        assert_eq!(detect_language_from_path("src/app.py"), "python");
        assert_eq!(detect_language_from_path("src/index.ts"), "typescript");
    }

    #[test]
    fn test_ai_suggester_mock() {
        use super::super::provider::AiProviderConfig;

        let provider = AiProvider::new(AiProviderConfig::mock());
        let suggester = AiSuggester::with_provider(provider);

        assert!(suggester.is_available());
        assert_eq!(suggester.provider_name(), "Mock");
    }
}
