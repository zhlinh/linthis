// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Issue classification statistics for linthis reports.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::utils::types::{RunResult, Severity};

/// Aggregated statistics for a lint run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportStatistics {
    /// Total issue counts by severity.
    pub severity_counts: SeverityCounts,
    /// Issues grouped by language.
    pub by_language: HashMap<String, usize>,
    /// Issues grouped by linter tool.
    pub by_tool: HashMap<String, usize>,
    /// Issues grouped by rule code.
    pub by_rule: HashMap<String, RuleStats>,
    /// Top N files with most issues.
    pub top_files: Vec<FileStats>,
    /// Summary metrics.
    pub summary: SummaryMetrics,
}

/// Issue counts by severity level.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeverityCounts {
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
}

impl SeverityCounts {
    /// Total number of issues.
    pub fn total(&self) -> usize {
        self.errors + self.warnings + self.info
    }
}

/// Statistics for a specific rule code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleStats {
    pub code: String,
    pub count: usize,
    pub severity: String,
    pub example_message: String,
}

/// Statistics for a specific file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStats {
    pub path: String,
    pub issue_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

/// Summary metrics for the report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryMetrics {
    pub total_files: usize,
    pub files_with_issues: usize,
    pub clean_file_percentage: f64,
    pub issues_per_file: f64,
    pub most_common_rule: Option<String>,
    pub most_problematic_language: Option<String>,
}

impl ReportStatistics {
    /// Compute statistics from a RunResult.
    pub fn from_run_result(result: &RunResult) -> Self {
        let mut severity_counts = SeverityCounts::default();
        let mut by_language: HashMap<String, usize> = HashMap::new();
        let mut by_tool: HashMap<String, usize> = HashMap::new();
        let mut by_rule: HashMap<String, RuleStats> = HashMap::new();
        let mut file_stats: HashMap<String, FileStats> = HashMap::new();

        // Process each issue
        for issue in &result.issues {
            // Count by severity
            match issue.severity {
                Severity::Error => severity_counts.errors += 1,
                Severity::Warning => severity_counts.warnings += 1,
                Severity::Info => severity_counts.info += 1,
            }

            // Count by language
            if let Some(ref lang) = issue.language {
                *by_language.entry(lang.name().to_string()).or_insert(0) += 1;
            }

            // Count by tool
            if let Some(ref source) = issue.source {
                *by_tool.entry(source.clone()).or_insert(0) += 1;
            }

            // Count by rule code
            if let Some(ref code) = issue.code {
                by_rule
                    .entry(code.clone())
                    .and_modify(|stats| stats.count += 1)
                    .or_insert(RuleStats {
                        code: code.clone(),
                        count: 1,
                        severity: format!("{}", issue.severity),
                        example_message: issue.message.clone(),
                    });
            }

            // Track file statistics
            let path_str = issue.file_path.to_string_lossy().to_string();
            let file_stat = file_stats.entry(path_str.clone()).or_insert(FileStats {
                path: path_str,
                issue_count: 0,
                error_count: 0,
                warning_count: 0,
                info_count: 0,
            });
            file_stat.issue_count += 1;
            match issue.severity {
                Severity::Error => file_stat.error_count += 1,
                Severity::Warning => file_stat.warning_count += 1,
                Severity::Info => file_stat.info_count += 1,
            }
        }

        // Get top files by issue count
        let mut top_files: Vec<FileStats> = file_stats.into_values().collect();
        top_files.sort_by(|a, b| b.issue_count.cmp(&a.issue_count));
        top_files.truncate(10); // Top 10 files

        // Find most common rule
        let most_common_rule = by_rule
            .iter()
            .max_by_key(|(_, stats)| stats.count)
            .map(|(code, _)| code.clone());

        // Find most problematic language
        let most_problematic_language = by_language
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang.clone());

        // Calculate summary metrics
        let total_issues = severity_counts.total();
        let clean_files = result.total_files.saturating_sub(result.files_with_issues);
        let clean_file_percentage = if result.total_files > 0 {
            (clean_files as f64 / result.total_files as f64) * 100.0
        } else {
            100.0
        };
        let issues_per_file = if result.files_with_issues > 0 {
            total_issues as f64 / result.files_with_issues as f64
        } else {
            0.0
        };

        let summary = SummaryMetrics {
            total_files: result.total_files,
            files_with_issues: result.files_with_issues,
            clean_file_percentage,
            issues_per_file,
            most_common_rule,
            most_problematic_language,
        };

        Self {
            severity_counts,
            by_language,
            by_tool,
            by_rule,
            top_files,
            summary,
        }
    }

    /// Format statistics as human-readable text.
    pub fn format_human(&self) -> String {
        let mut output = String::new();

        // Summary header
        output.push_str("=== Lint Statistics ===\n\n");

        // Severity counts
        output.push_str("Severity Breakdown:\n");
        output.push_str(&format!("  Errors:   {}\n", self.severity_counts.errors));
        output.push_str(&format!("  Warnings: {}\n", self.severity_counts.warnings));
        output.push_str(&format!("  Info:     {}\n", self.severity_counts.info));
        output.push_str(&format!("  Total:    {}\n\n", self.severity_counts.total()));

        // File summary
        output.push_str("File Summary:\n");
        output.push_str(&format!("  Total files:       {}\n", self.summary.total_files));
        output.push_str(&format!(
            "  Files with issues: {}\n",
            self.summary.files_with_issues
        ));
        output.push_str(&format!(
            "  Clean files:       {:.1}%\n",
            self.summary.clean_file_percentage
        ));
        output.push_str(&format!(
            "  Issues per file:   {:.1}\n\n",
            self.summary.issues_per_file
        ));

        // By language
        if !self.by_language.is_empty() {
            output.push_str("Issues by Language:\n");
            let mut langs: Vec<_> = self.by_language.iter().collect();
            langs.sort_by(|a, b| b.1.cmp(a.1));
            for (lang, count) in langs {
                output.push_str(&format!("  {}: {}\n", lang, count));
            }
            output.push('\n');
        }

        // By tool
        if !self.by_tool.is_empty() {
            output.push_str("Issues by Tool:\n");
            let mut tools: Vec<_> = self.by_tool.iter().collect();
            tools.sort_by(|a, b| b.1.cmp(a.1));
            for (tool, count) in tools {
                output.push_str(&format!("  {}: {}\n", tool, count));
            }
            output.push('\n');
        }

        // Top rules
        if !self.by_rule.is_empty() {
            output.push_str("Top Rule Violations:\n");
            let mut rules: Vec<_> = self.by_rule.values().collect();
            rules.sort_by(|a, b| b.count.cmp(&a.count));
            for rule in rules.iter().take(5) {
                output.push_str(&format!("  {} ({}): {} occurrences\n", rule.code, rule.severity, rule.count));
            }
            output.push('\n');
        }

        // Top problematic files
        if !self.top_files.is_empty() {
            output.push_str("Top Problematic Files:\n");
            for file in self.top_files.iter().take(5) {
                output.push_str(&format!(
                    "  {} - {} issues ({} errors, {} warnings)\n",
                    file.path, file.issue_count, file.error_count, file.warning_count
                ));
            }
        }

        output
    }

    /// Format statistics as JSON.
    pub fn format_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::types::LintIssue;
    use std::path::PathBuf;

    fn make_issue(severity: Severity, lang: &str, tool: &str, code: &str) -> LintIssue {
        use crate::Language;
        let mut issue = LintIssue::new(
            PathBuf::from("test.rs"),
            1,
            "Test message".to_string(),
            severity,
        );
        issue.language = Language::from_name(lang);
        issue.source = Some(tool.to_string());
        issue.code = Some(code.to_string());
        issue
    }

    #[test]
    fn test_statistics_from_run_result() {
        let mut result = RunResult::new();
        result.total_files = 10;
        result.files_with_issues = 3;
        result.add_issue(make_issue(Severity::Error, "rust", "clippy", "E0001"));
        result.add_issue(make_issue(Severity::Warning, "rust", "clippy", "W0001"));
        result.add_issue(make_issue(Severity::Warning, "python", "ruff", "W0001"));
        result.add_issue(make_issue(Severity::Info, "python", "ruff", "I0001"));

        let stats = ReportStatistics::from_run_result(&result);

        assert_eq!(stats.severity_counts.errors, 1);
        assert_eq!(stats.severity_counts.warnings, 2);
        assert_eq!(stats.severity_counts.info, 1);
        assert_eq!(stats.by_language.get("rust"), Some(&2));
        assert_eq!(stats.by_language.get("python"), Some(&2));
        assert_eq!(stats.by_tool.get("clippy"), Some(&2));
        assert_eq!(stats.by_tool.get("ruff"), Some(&2));
        assert_eq!(stats.summary.total_files, 10);
        assert_eq!(stats.summary.files_with_issues, 3);
    }

    #[test]
    fn test_severity_counts_total() {
        let counts = SeverityCounts {
            errors: 5,
            warnings: 10,
            info: 3,
        };
        assert_eq!(counts.total(), 18);
    }

    #[test]
    fn test_format_human() {
        let stats = ReportStatistics {
            severity_counts: SeverityCounts {
                errors: 2,
                warnings: 5,
                info: 1,
            },
            by_language: [("rust".to_string(), 5), ("python".to_string(), 3)]
                .into_iter()
                .collect(),
            by_tool: [("clippy".to_string(), 5), ("ruff".to_string(), 3)]
                .into_iter()
                .collect(),
            by_rule: HashMap::new(),
            top_files: vec![],
            summary: SummaryMetrics {
                total_files: 10,
                files_with_issues: 3,
                clean_file_percentage: 70.0,
                issues_per_file: 2.7,
                most_common_rule: None,
                most_problematic_language: Some("rust".to_string()),
            },
        };

        let output = stats.format_human();
        assert!(output.contains("Errors:   2"));
        assert!(output.contains("Warnings: 5"));
        assert!(output.contains("rust: 5"));
    }
}
