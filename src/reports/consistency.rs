// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Team code style consistency analysis for linthis reports.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::utils::types::RunResult;

/// Team code style consistency analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyAnalysis {
    /// Overall consistency score (0-100).
    pub consistency_score: f64,
    /// Issues that appear across multiple files (pattern violations).
    pub repeated_patterns: Vec<RepeatedPattern>,
    /// Files that deviate from team norms.
    pub outlier_files: Vec<OutlierFile>,
    /// Rule violations by frequency.
    pub rule_frequency: HashMap<String, RuleFrequency>,
    /// Total files analyzed.
    pub total_files: usize,
    /// Files with issues.
    pub files_with_issues: usize,
}

/// A pattern that appears across multiple files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepeatedPattern {
    /// Rule code for this pattern.
    pub rule_code: String,
    /// Representative message for this pattern.
    pub message_pattern: String,
    /// Number of times this pattern occurs.
    pub occurrence_count: usize,
    /// Files affected by this pattern.
    pub affected_files: Vec<String>,
    /// Suggested fix (if available).
    pub suggestion: Option<String>,
}

/// A file that deviates significantly from team norms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlierFile {
    /// File path.
    pub path: String,
    /// Number of issues in this file.
    pub issue_count: usize,
    /// Deviation score (how far from average).
    pub deviation_score: f64,
    /// Primary issues in this file (top rule codes).
    pub primary_issues: Vec<String>,
}

/// Frequency statistics for a rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleFrequency {
    /// Rule code.
    pub code: String,
    /// Total occurrences across all files.
    pub total_occurrences: usize,
    /// Number of files with this violation.
    pub file_count: usize,
    /// Whether this is a systematic issue (appears in >30% of files).
    pub is_systematic: bool,
}

impl ConsistencyAnalysis {
    /// Analyze consistency from a RunResult.
    pub fn from_run_result(result: &RunResult) -> Self {
        let mut files_issues: HashMap<String, Vec<String>> = HashMap::new();
        let mut rule_files: HashMap<String, HashSet<String>> = HashMap::new();
        let mut rule_counts: HashMap<String, usize> = HashMap::new();
        let mut rule_messages: HashMap<String, String> = HashMap::new();
        let mut rule_suggestions: HashMap<String, Option<String>> = HashMap::new();

        // Collect data from issues
        for issue in &result.issues {
            let file_path = issue.file_path.to_string_lossy().to_string();
            let rule_code = issue.code.clone().unwrap_or_else(|| "unknown".to_string());

            // Track issues per file
            files_issues
                .entry(file_path.clone())
                .or_default()
                .push(rule_code.clone());

            // Track files per rule
            rule_files
                .entry(rule_code.clone())
                .or_default()
                .insert(file_path);

            // Count rule occurrences
            *rule_counts.entry(rule_code.clone()).or_insert(0) += 1;

            // Store example message and suggestion
            if !rule_messages.contains_key(&rule_code) {
                rule_messages.insert(rule_code.clone(), issue.message.clone());
                rule_suggestions.insert(rule_code, issue.suggestion.clone());
            }
        }

        let total_files = result.total_files;
        let files_with_issues = files_issues.len();

        // Calculate average issues per file (for files with issues)
        let avg_issues_per_file = if files_with_issues > 0 {
            result.issues.len() as f64 / files_with_issues as f64
        } else {
            0.0
        };

        // Calculate standard deviation
        let variance = if files_with_issues > 1 {
            let sum_sq: f64 = files_issues
                .values()
                .map(|issues| {
                    let diff = issues.len() as f64 - avg_issues_per_file;
                    diff * diff
                })
                .sum();
            sum_sq / (files_with_issues - 1) as f64
        } else {
            0.0
        };
        let std_dev = variance.sqrt();

        // Identify outlier files (more than 2 standard deviations above average)
        let outlier_threshold = avg_issues_per_file + 2.0 * std_dev;
        let mut outlier_files: Vec<OutlierFile> = files_issues
            .iter()
            .filter(|(_, issues)| issues.len() as f64 > outlier_threshold && std_dev > 0.0)
            .map(|(path, issues)| {
                let deviation = (issues.len() as f64 - avg_issues_per_file) / std_dev.max(1.0);

                // Get top rules for this file
                let mut rule_counts_file: HashMap<&String, usize> = HashMap::new();
                for rule in issues {
                    *rule_counts_file.entry(rule).or_insert(0) += 1;
                }
                let mut primary: Vec<_> = rule_counts_file.into_iter().collect();
                primary.sort_by(|a, b| b.1.cmp(&a.1));
                let primary_issues: Vec<String> =
                    primary.into_iter().take(3).map(|(r, _)| r.clone()).collect();

                OutlierFile {
                    path: path.clone(),
                    issue_count: issues.len(),
                    deviation_score: deviation,
                    primary_issues,
                }
            })
            .collect();
        outlier_files.sort_by(|a, b| {
            b.deviation_score
                .partial_cmp(&a.deviation_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        outlier_files.truncate(10);

        // Identify repeated patterns (rules appearing in multiple files)
        let systematic_threshold = (total_files as f64 * 0.3).max(2.0) as usize;
        let mut repeated_patterns: Vec<RepeatedPattern> = rule_files
            .iter()
            .filter(|(_, files)| files.len() >= 2)
            .map(|(rule, files)| {
                let count = rule_counts.get(rule).copied().unwrap_or(0);
                let message = rule_messages.get(rule).cloned().unwrap_or_default();
                let suggestion = rule_suggestions.get(rule).and_then(|s| s.clone());
                let mut affected: Vec<_> = files.iter().cloned().collect();
                affected.sort();
                affected.truncate(5); // Limit to 5 files for display

                RepeatedPattern {
                    rule_code: rule.clone(),
                    message_pattern: truncate_message(&message, 80),
                    occurrence_count: count,
                    affected_files: affected,
                    suggestion,
                }
            })
            .collect();
        repeated_patterns.sort_by(|a, b| b.occurrence_count.cmp(&a.occurrence_count));
        repeated_patterns.truncate(10);

        // Build rule frequency map
        let rule_frequency: HashMap<String, RuleFrequency> = rule_files
            .iter()
            .map(|(rule, files)| {
                let freq = RuleFrequency {
                    code: rule.clone(),
                    total_occurrences: rule_counts.get(rule).copied().unwrap_or(0),
                    file_count: files.len(),
                    is_systematic: files.len() >= systematic_threshold,
                };
                (rule.clone(), freq)
            })
            .collect();

        // Calculate consistency score
        // Higher score = more consistent (fewer issues spread across files)
        let consistency_score = calculate_consistency_score(
            total_files,
            files_with_issues,
            outlier_files.len(),
            repeated_patterns.iter().filter(|p| p.affected_files.len() >= 3).count(),
        );

        Self {
            consistency_score,
            repeated_patterns,
            outlier_files,
            rule_frequency,
            total_files,
            files_with_issues,
        }
    }

    /// Format consistency analysis as human-readable text.
    pub fn format_human(&self) -> String {
        let mut output = String::new();

        output.push_str("=== Code Consistency Analysis ===\n\n");

        // Score
        output.push_str(&format!(
            "Consistency Score: {:.1}/100\n",
            self.consistency_score
        ));
        output.push_str(&format!(
            "Files analyzed: {} ({} with issues)\n\n",
            self.total_files, self.files_with_issues
        ));

        // Repeated patterns
        if !self.repeated_patterns.is_empty() {
            output.push_str("Repeated Patterns (same issue across files):\n");
            for pattern in self.repeated_patterns.iter().take(5) {
                output.push_str(&format!(
                    "  [{}] {} occurrences in {} files\n",
                    pattern.rule_code,
                    pattern.occurrence_count,
                    pattern.affected_files.len()
                ));
                output.push_str(&format!("    Example: {}\n", pattern.message_pattern));
                if let Some(ref suggestion) = pattern.suggestion {
                    output.push_str(&format!("    Fix: {}\n", truncate_message(suggestion, 60)));
                }
            }
            output.push('\n');
        }

        // Outlier files
        if !self.outlier_files.is_empty() {
            output.push_str("Outlier Files (significantly more issues than average):\n");
            for file in self.outlier_files.iter().take(5) {
                output.push_str(&format!(
                    "  {} - {} issues (deviation: {:.1}σ)\n",
                    file.path, file.issue_count, file.deviation_score
                ));
                if !file.primary_issues.is_empty() {
                    output.push_str(&format!("    Primary issues: {}\n", file.primary_issues.join(", ")));
                }
            }
            output.push('\n');
        }

        // Systematic issues
        let systematic: Vec<_> = self
            .rule_frequency
            .values()
            .filter(|f| f.is_systematic)
            .collect();
        if !systematic.is_empty() {
            output.push_str("Systematic Issues (>30% of files affected):\n");
            for freq in systematic.iter().take(5) {
                output.push_str(&format!(
                    "  [{}] {} files, {} total occurrences\n",
                    freq.code, freq.file_count, freq.total_occurrences
                ));
            }
        }

        output
    }

    /// Format consistency analysis as JSON.
    pub fn format_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Calculate a consistency score from 0-100.
fn calculate_consistency_score(
    total_files: usize,
    files_with_issues: usize,
    outlier_count: usize,
    widespread_patterns: usize,
) -> f64 {
    if total_files == 0 {
        return 100.0;
    }

    let mut score = 100.0;

    // Deduct for files with issues (up to 40 points)
    let issue_ratio = files_with_issues as f64 / total_files as f64;
    score -= issue_ratio * 40.0;

    // Deduct for outlier files (up to 20 points)
    let outlier_ratio = outlier_count as f64 / total_files.max(1) as f64;
    score -= outlier_ratio * 20.0;

    // Deduct for widespread patterns (up to 20 points)
    let pattern_penalty = (widespread_patterns as f64).min(5.0) * 4.0;
    score -= pattern_penalty;

    score.clamp(0.0, 100.0)
}

/// Truncate a message to a maximum length.
fn truncate_message(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::types::{LintIssue, Severity};
    use std::path::PathBuf;

    fn make_issue(file: &str, code: &str) -> LintIssue {
        let mut issue = LintIssue::new(
            PathBuf::from(file),
            1,
            "Test message".to_string(),
            Severity::Warning,
        );
        issue.code = Some(code.to_string());
        issue
    }

    #[test]
    fn test_consistency_analysis_empty() {
        let result = RunResult::new();
        let analysis = ConsistencyAnalysis::from_run_result(&result);
        assert_eq!(analysis.consistency_score, 100.0);
        assert!(analysis.repeated_patterns.is_empty());
        assert!(analysis.outlier_files.is_empty());
    }

    #[test]
    fn test_consistency_analysis_with_issues() {
        let mut result = RunResult::new();
        result.total_files = 10;

        // Add issues across multiple files with the same rule
        result.add_issue(make_issue("file1.rs", "E0001"));
        result.add_issue(make_issue("file2.rs", "E0001"));
        result.add_issue(make_issue("file3.rs", "E0001"));
        result.add_issue(make_issue("file1.rs", "W0001"));

        result.count_files_with_issues();

        let analysis = ConsistencyAnalysis::from_run_result(&result);

        // Should find E0001 as a repeated pattern
        assert!(!analysis.repeated_patterns.is_empty());
        let e0001 = analysis
            .repeated_patterns
            .iter()
            .find(|p| p.rule_code == "E0001");
        assert!(e0001.is_some());
        assert_eq!(e0001.unwrap().occurrence_count, 3);
    }

    #[test]
    fn test_calculate_consistency_score() {
        // Perfect score
        assert_eq!(calculate_consistency_score(10, 0, 0, 0), 100.0);

        // Half files with issues
        let score = calculate_consistency_score(10, 5, 0, 0);
        assert!(score < 100.0 && score > 50.0);

        // Many outliers and patterns
        let score = calculate_consistency_score(10, 8, 3, 5);
        assert!(score < 50.0);
    }
}
