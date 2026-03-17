//! Auto-fix generation for review issues.
//!
//! Bridges review issues → AI fix suggestions → file patches.
//! Uses `AiSuggester` to generate fixes and applies them to source files.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ai::{AiProvider, AiProviderConfig, AiSuggester, FixSuggestion, SuggestionOptions};
use crate::review::{AutoFix, ReviewIssue, ReviewResult, Severity};

/// Report of fix generation and application
#[derive(Debug)]
pub struct FixReport {
    /// Number of fixes successfully applied
    pub applied: usize,
    /// Number of fixes that failed
    pub failed: usize,
    /// Files that were modified
    pub modified_files: Vec<PathBuf>,
}

/// Generate and apply fixes for Critical and Important review issues.
///
/// Mutates `review_result.auto_fixes` to record what was fixed.
/// Returns a `FixReport` with success/failure counts and modified file paths.
pub fn generate_and_apply_fixes(
    review_result: &mut ReviewResult,
    provider_config: &AiProviderConfig,
) -> FixReport {
    let mut report = FixReport {
        applied: 0,
        failed: 0,
        modified_files: Vec::new(),
    };

    // Filter to Critical + Important issues with a file and line number
    let fixable_issues: Vec<&ReviewIssue> = review_result
        .issues
        .iter()
        .filter(|i| matches!(i.severity, Severity::Critical | Severity::Important))
        .filter(|i| i.line.is_some())
        .collect();

    if fixable_issues.is_empty() {
        return report;
    }

    // Group issues by file
    let mut issues_by_file: HashMap<PathBuf, Vec<&ReviewIssue>> = HashMap::new();
    for issue in &fixable_issues {
        issues_by_file
            .entry(issue.file.clone())
            .or_default()
            .push(issue);
    }

    let suggester = AiSuggester::with_provider(AiProvider::new(provider_config.clone()));
    let options = SuggestionOptions {
        skip_with_suggestion: false,
        ..SuggestionOptions::default()
    };

    for (file_path, issues) in &issues_by_file {
        // Sort issues by line number descending (apply bottom-up)
        let mut sorted_issues = issues.clone();
        sorted_issues.sort_by(|a, b| b.line.unwrap_or(0).cmp(&a.line.unwrap_or(0)));

        let mut file_modified = false;

        for issue in &sorted_issues {
            let line = match issue.line {
                Some(l) => l as usize,
                None => continue,
            };

            // Use the issue category as rule_id for the AI suggester
            let rule_id = &issue.category;
            let message = &issue.message;

            let suggestion_result =
                suggester.suggest_fix_for_file(file_path, line, message, rule_id, &options);

            if !suggestion_result.is_success() {
                eprintln!(
                    "  Failed to generate fix for {}:{} — {}",
                    file_path.display(),
                    line,
                    suggestion_result
                        .error
                        .as_deref()
                        .unwrap_or("no suggestion generated")
                );
                report.failed += 1;
                continue;
            }

            // Take the first suggestion
            let suggestion = match suggestion_result.suggestions.first() {
                Some(s) => s,
                None => {
                    report.failed += 1;
                    continue;
                }
            };

            // Apply the fix
            if apply_fix(file_path, suggestion) {
                review_result.auto_fixes.push(AutoFix {
                    file: file_path.clone(),
                    line: issue.line,
                    description: format!("[{}] {}", issue.category, issue.message),
                });
                file_modified = true;
                report.applied += 1;
            } else {
                eprintln!("  Failed to apply fix for {}:{}", file_path.display(), line);
                report.failed += 1;
            }
        }

        if file_modified {
            report.modified_files.push(file_path.clone());
        }
    }

    report
}

/// Apply a single fix suggestion to a file (line-based replacement).
///
/// Reads the file, replaces lines from `suggestion.start_line` to
/// `suggestion.end_line` with the suggestion code, and writes back.
fn apply_fix(file_path: &Path, suggestion: &FixSuggestion) -> bool {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let lines: Vec<&str> = content.lines().collect();
    let start_idx = suggestion.start_line.saturating_sub(1);
    let end_idx = suggestion.end_line.saturating_sub(1);

    if start_idx >= lines.len() {
        return false;
    }
    let end_idx = end_idx.min(lines.len() - 1);

    let mut new_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();

    // Remove old lines (from end_idx down to start_idx)
    let remove_count = end_idx - start_idx + 1;
    for _ in 0..remove_count {
        if start_idx < new_lines.len() {
            new_lines.remove(start_idx);
        }
    }

    // Insert suggestion lines
    let suggestion_lines: Vec<&str> = suggestion.code.lines().collect();
    for (i, line) in suggestion_lines.iter().enumerate() {
        if start_idx + i <= new_lines.len() {
            new_lines.insert(start_idx + i, line.to_string());
        }
    }

    // Write back, preserving trailing newline
    let new_content = new_lines.join("\n");
    let final_content = if content.ends_with('\n') && !new_content.ends_with('\n') {
        format!("{}\n", new_content)
    } else {
        new_content
    };

    fs::write(file_path, final_content).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::{Assessment, ReviewIssue, ReviewResult, ReviewSummary, Severity};
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_apply_fix_single_line() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "line 1").unwrap();
        writeln!(tmp, "line 2").unwrap();
        writeln!(tmp, "line 3").unwrap();

        let suggestion = FixSuggestion::new("replaced line 2".to_string(), 2, 2, "text");

        assert!(apply_fix(tmp.path(), &suggestion));

        let result = fs::read_to_string(tmp.path()).unwrap();
        assert!(result.contains("replaced line 2"));
        assert!(result.contains("line 1"));
        assert!(result.contains("line 3"));
        assert!(!result.contains("\nline 2\n"));
    }

    #[test]
    fn test_apply_fix_multi_line() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "a").unwrap();
        writeln!(tmp, "b").unwrap();
        writeln!(tmp, "c").unwrap();
        writeln!(tmp, "d").unwrap();

        let suggestion = FixSuggestion::new("X\nY".to_string(), 2, 3, "text");

        assert!(apply_fix(tmp.path(), &suggestion));

        let result = fs::read_to_string(tmp.path()).unwrap();
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[0], "a");
        assert_eq!(lines[1], "X");
        assert_eq!(lines[2], "Y");
        assert_eq!(lines[3], "d");
    }

    #[test]
    fn test_generate_and_apply_fixes_no_fixable() {
        let mut result = ReviewResult {
            summary: ReviewSummary {
                files_reviewed: 1,
                total_issues: 1,
                critical_count: 0,
                important_count: 0,
                minor_count: 1,
                assessment: Assessment::Ready,
                summary_text: String::new(),
            },
            files: vec![],
            issues: vec![ReviewIssue {
                severity: Severity::Minor,
                category: "style".to_string(),
                file: PathBuf::from("test.rs"),
                line: Some(1),
                message: "minor issue".to_string(),
                suggestion: None,
            }],
            base_ref: "main".to_string(),
            head_ref: "feature".to_string(),
            auto_fixes: vec![],
        };

        let config = crate::ai::AiProviderConfig::mock();
        let report = generate_and_apply_fixes(&mut result, &config);

        assert_eq!(report.applied, 0);
        assert_eq!(report.failed, 0);
        assert!(report.modified_files.is_empty());
        assert!(result.auto_fixes.is_empty());
    }
}
