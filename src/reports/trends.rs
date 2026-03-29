// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Historical trend analysis for linthis reports.

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::types::{RunResult, Severity};

/// A single data point in the trend timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendDataPoint {
    /// Timestamp of the run.
    pub timestamp: DateTime<Utc>,
    /// Path to the result file.
    pub result_file: String,
    /// Total number of issues.
    pub total_issues: usize,
    /// Number of errors.
    pub errors: usize,
    /// Number of warnings.
    pub warnings: usize,
    /// Total files processed.
    pub total_files: usize,
    /// Files with issues.
    pub files_with_issues: usize,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
    /// Target paths that were scanned (scope identifier).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_paths: Vec<String>,
}

impl TrendDataPoint {
    /// Create a data point from a RunResult and file path.
    pub fn from_result(result: &RunResult, file_path: &Path, timestamp: DateTime<Utc>) -> Self {
        let mut errors = 0;
        let mut warnings = 0;

        for issue in &result.issues {
            match issue.severity {
                Severity::Error => errors += 1,
                Severity::Warning => warnings += 1,
                Severity::Info => {}
            }
        }

        Self {
            timestamp,
            result_file: file_path.to_string_lossy().to_string(),
            total_issues: result.issues.len(),
            errors,
            warnings,
            total_files: result.total_files,
            files_with_issues: result.files_with_issues,
            duration_ms: result.duration_ms,
            target_paths: result.target_paths.clone(),
        }
    }

    /// Get a normalized scope key for grouping results.
    /// Empty target_paths means "project root" scan.
    pub fn scope_key(&self) -> String {
        if self.target_paths.is_empty() {
            ".".to_string()
        } else {
            let mut sorted = self.target_paths.clone();
            sorted.sort();
            sorted.join(",")
        }
    }

    /// Get a human-readable scope label.
    pub fn scope_label(&self) -> String {
        if self.target_paths.is_empty() {
            "project root".to_string()
        } else {
            self.target_paths.join(", ")
        }
    }
}

/// Trend direction indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    /// Issues are decreasing over time.
    Improving,
    /// Issues are relatively stable.
    Stable,
    /// Issues are increasing over time.
    Degrading,
}

impl std::fmt::Display for TrendDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrendDirection::Improving => write!(f, "Improving"),
            TrendDirection::Stable => write!(f, "Stable"),
            TrendDirection::Degrading => write!(f, "Degrading"),
        }
    }
}

/// Aggregated trend analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    /// Data points in chronological order (oldest first).
    pub data_points: Vec<TrendDataPoint>,
    /// Overall trend direction.
    pub trend_direction: TrendDirection,
    /// Percentage change in issues (positive = more issues).
    pub issue_change_percentage: f64,
    /// Average issues per run.
    pub average_issues_per_run: f64,
    /// Best run (lowest issues).
    pub best_run: Option<TrendDataPoint>,
    /// Worst run (highest issues).
    pub worst_run: Option<TrendDataPoint>,
    /// Scope label for this trend analysis (what paths were scanned).
    #[serde(default)]
    pub scope: String,
    /// Number of results excluded due to different scope.
    #[serde(default)]
    pub excluded_count: usize,
}

impl TrendAnalysis {
    /// Analyze trends from a list of data points.
    pub fn from_data_points(data_points: Vec<TrendDataPoint>) -> Self {
        Self::from_data_points_with_scope(data_points, String::new(), 0)
    }

    /// Analyze trends from data points with scope info.
    pub fn from_data_points_with_scope(
        mut data_points: Vec<TrendDataPoint>,
        scope: String,
        excluded_count: usize,
    ) -> Self {
        // Sort by timestamp (oldest first)
        data_points.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        if data_points.is_empty() {
            return Self {
                data_points: vec![],
                trend_direction: TrendDirection::Stable,
                issue_change_percentage: 0.0,
                average_issues_per_run: 0.0,
                best_run: None,
                worst_run: None,
                scope,
                excluded_count,
            };
        }

        // Calculate average
        let total: usize = data_points.iter().map(|dp| dp.total_issues).sum();
        let average_issues_per_run = total as f64 / data_points.len() as f64;

        // Find best and worst runs
        let best_run = data_points.iter().min_by_key(|dp| dp.total_issues).cloned();
        let worst_run = data_points.iter().max_by_key(|dp| dp.total_issues).cloned();

        // Calculate trend direction and change percentage
        let (trend_direction, issue_change_percentage) = if data_points.len() >= 2 {
            // Compare first half average to second half average
            let mid = data_points.len() / 2;
            let first_half: Vec<_> = data_points[..mid].to_vec();
            let second_half: Vec<_> = data_points[mid..].to_vec();

            let first_avg = first_half.iter().map(|dp| dp.total_issues).sum::<usize>() as f64
                / first_half.len() as f64;
            let second_avg = second_half.iter().map(|dp| dp.total_issues).sum::<usize>() as f64
                / second_half.len() as f64;

            let change_pct = if first_avg > 0.0 {
                ((second_avg - first_avg) / first_avg) * 100.0
            } else if second_avg > 0.0 {
                100.0 // Went from 0 to some issues
            } else {
                0.0 // Both are 0
            };

            let direction = if change_pct < -10.0 {
                TrendDirection::Improving
            } else if change_pct > 10.0 {
                TrendDirection::Degrading
            } else {
                TrendDirection::Stable
            };

            (direction, change_pct)
        } else {
            (TrendDirection::Stable, 0.0)
        };

        Self {
            data_points,
            trend_direction,
            issue_change_percentage,
            average_issues_per_run,
            best_run,
            worst_run,
            scope,
            excluded_count,
        }
    }

    /// Format trend analysis as human-readable text.
    pub fn format_human(&self) -> String {
        if self.data_points.is_empty() {
            return "No historical data available for trend analysis.\n".to_string();
        }

        let mut output = String::new();

        output.push_str("=== Code Quality Trends ===\n\n");

        // Show scope info
        if !self.scope.is_empty() {
            output.push_str(&format!("Scope: {}\n", self.scope));
        }
        if self.excluded_count > 0 {
            output.push_str(&format!(
                "({} runs with different scope excluded)\n",
                self.excluded_count
            ));
        }

        // Summary
        output.push_str(&format!("Runs analyzed: {}\n", self.data_points.len()));
        output.push_str(&format!("Trend: {}\n", self.trend_direction));
        output.push_str(&format!(
            "Issue change: {:+.1}%\n",
            self.issue_change_percentage
        ));
        output.push_str(&format!(
            "Average issues/run: {:.1}\n\n",
            self.average_issues_per_run
        ));

        // Best/Worst runs
        if let Some(ref best) = self.best_run {
            output.push_str(&format!(
                "Best run: {} issues ({})\n",
                best.total_issues,
                best.timestamp.format("%Y-%m-%d %H:%M")
            ));
        }
        if let Some(ref worst) = self.worst_run {
            output.push_str(&format!(
                "Worst run: {} issues ({})\n",
                worst.total_issues,
                worst.timestamp.format("%Y-%m-%d %H:%M")
            ));
        }

        output.push_str("\nRecent runs:\n");
        for (i, dp) in self.data_points.iter().rev().take(5).enumerate() {
            let scope_tag = if dp.target_paths.is_empty() {
                String::new()
            } else {
                format!(" [{}]", dp.target_paths.join(", "))
            };
            output.push_str(&format!(
                "  {}. {} - {} issues ({} errors, {} warnings){}\n",
                i + 1,
                dp.timestamp.format("%Y-%m-%d %H:%M"),
                dp.total_issues,
                dp.errors,
                dp.warnings,
                scope_tag,
            ));
        }

        output
    }

    /// Format trend analysis as JSON.
    pub fn format_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Load historical results from the .linthis/result/ directory.
pub fn load_historical_results(project_root: &Path, limit: usize) -> Vec<(PathBuf, RunResult)> {
    let result_dir = project_root.join(".linthis").join("result");

    if !result_dir.exists() {
        return Vec::new();
    }

    let mut result_files: Vec<_> = fs::read_dir(&result_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with("result-") && name.ends_with(".json")
        })
        .collect();

    // Sort by modification time (newest first)
    result_files.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|m| m.modified()).ok();
        let b_time = b.metadata().and_then(|m| m.modified()).ok();
        b_time.cmp(&a_time)
    });

    // Load and parse files (supports both legacy and unified formats)
    result_files
        .into_iter()
        .take(limit)
        .filter_map(|entry| {
            let path = entry.path();
            let result = load_result_from_file(&path)?;
            Some((path, result))
        })
        .collect()
}

/// Get the most recent result file.
pub fn get_last_result(project_root: &Path) -> Option<(PathBuf, RunResult)> {
    load_historical_results(project_root, 1).into_iter().next()
}

/// Load a result from a specific file path.
///
/// Supports both the legacy flat format and the new unified format
/// (with `checks_run`, `lint`, `security`, `complexity` top-level keys).
pub fn load_result_from_file(path: &Path) -> Option<RunResult> {
    let content = fs::read_to_string(path).ok()?;

    // Try legacy format first (direct RunResult deserialization)
    if let Ok(result) = serde_json::from_str::<RunResult>(&content) {
        return Some(result);
    }

    // Try new unified format
    if let Ok(unified) = serde_json::from_str::<serde_json::Value>(&content) {
        let mut result = RunResult::new();

        // Extract lint data
        if let Some(lint) = unified.get("lint") {
            result.total_files = lint.get("total_files").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            if let Some(issues) = lint.get("issues").and_then(|v| v.as_array()) {
                for iv in issues {
                    let file_str = iv.get("file").or_else(|| iv.get("file_path"))
                        .and_then(|v| v.as_str()).unwrap_or("");
                    let line = iv.get("line").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let message = iv.get("message").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let severity = match iv.get("severity").and_then(|v| v.as_str()).unwrap_or("error") {
                        "error" => crate::utils::types::Severity::Error,
                        "warning" => crate::utils::types::Severity::Warning,
                        _ => crate::utils::types::Severity::Info,
                    };
                    let mut issue = crate::utils::types::LintIssue::new(
                        std::path::PathBuf::from(file_str), line, message, severity,
                    );
                    if let Some(col) = iv.get("column").and_then(|v| v.as_u64()) {
                        issue = issue.with_column(col as usize);
                    }
                    if let Some(code) = iv.get("code").and_then(|v| v.as_str()) {
                        issue = issue.with_code(code.to_string());
                    }
                    if let Some(source) = iv.get("source").and_then(|v| v.as_str()) {
                        issue = issue.with_source(source.to_string());
                    }
                    if let Some(suggestion) = iv.get("suggestion").and_then(|v| v.as_str()) {
                        issue = issue.with_suggestion(suggestion.to_string());
                    }
                    result.issues.push(issue);
                }
            }
            result.files_with_issues = lint.get("files_with_issues").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            result.files_formatted = lint.get("files_formatted").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        }

        result.exit_code = unified.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        result.duration_ms = unified.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0);

        if let Some(checks) = unified.get("checks_run").and_then(|v| v.as_array()) {
            result.checks_run = checks.iter().filter_map(|v| v.as_str().map(String::from)).collect();
        }
        if let Some(paths) = unified.get("target_paths").and_then(|v| v.as_array()) {
            result.target_paths = paths.iter().filter_map(|v| v.as_str().map(String::from)).collect();
        }

        // Note: security/complexity data in unified format uses CheckResultView
        // which doesn't directly map to SastResult/AnalysisResult.
        // For report/trend purposes, lint issues are the primary data.

        result.count_files_with_issues();
        return Some(result);
    }

    None
}

/// Parse timestamp from result filename (result-20260118-172630.json).
pub fn parse_result_timestamp(filename: &str) -> Option<DateTime<Utc>> {
    let stem = filename
        .trim_start_matches("result-")
        .trim_end_matches(".json");

    // Parse YYYYMMDD-HHMMSS format
    NaiveDateTime::parse_from_str(stem, "%Y%m%d-%H%M%S")
        .ok()
        .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
}

/// Build trend analysis from historical results.
/// Automatically groups by scope: uses the most recent run's scope as reference,
/// and only compares results with the same scope.
pub fn analyze_trends(project_root: &Path, limit: usize) -> TrendAnalysis {
    // Load more results than requested so we can filter by scope and still have enough
    let results = load_historical_results(project_root, limit * 3);

    let all_data_points: Vec<_> = results
        .into_iter()
        .filter_map(|(path, result)| {
            let filename = path.file_name()?.to_string_lossy().to_string();
            let timestamp = parse_result_timestamp(&filename)?;
            Some(TrendDataPoint::from_result(&result, &path, timestamp))
        })
        .collect();

    if all_data_points.is_empty() {
        return TrendAnalysis::from_data_points(vec![]);
    }

    // Find the most recent run's scope as reference
    let most_recent = all_data_points
        .iter()
        .max_by_key(|dp| dp.timestamp)
        .unwrap();
    let reference_scope = most_recent.scope_key();
    let scope_label = most_recent.scope_label();

    // Filter to only include results with matching scope
    let (matching, excluded): (Vec<_>, Vec<_>) = all_data_points
        .into_iter()
        .partition(|dp| dp.scope_key() == reference_scope);

    let excluded_count = excluded.len();

    // Apply the original limit
    let matching: Vec<_> = matching.into_iter().take(limit).collect();

    TrendAnalysis::from_data_points_with_scope(matching, scope_label, excluded_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trend_direction_display() {
        assert_eq!(format!("{}", TrendDirection::Improving), "Improving");
        assert_eq!(format!("{}", TrendDirection::Stable), "Stable");
        assert_eq!(format!("{}", TrendDirection::Degrading), "Degrading");
    }

    #[test]
    fn test_parse_result_timestamp() {
        let ts = parse_result_timestamp("result-20260118-172630.json");
        assert!(ts.is_some());
        let ts = ts.unwrap();
        assert_eq!(ts.format("%Y-%m-%d").to_string(), "2026-01-18");
    }

    #[test]
    fn test_trend_analysis_empty() {
        let analysis = TrendAnalysis::from_data_points(vec![]);
        assert_eq!(analysis.data_points.len(), 0);
        assert_eq!(analysis.trend_direction, TrendDirection::Stable);
    }

    #[test]
    fn test_trend_analysis_improving() {
        // Higher days_offset = older (further back in time)
        let data_points = vec![
            make_data_point(100, 3), // Oldest: 100 issues (3 days ago)
            make_data_point(80, 2),
            make_data_point(60, 1),
            make_data_point(40, 0), // Newest: 40 issues (today) - 60% reduction
        ];
        let analysis = TrendAnalysis::from_data_points(data_points);
        assert_eq!(analysis.trend_direction, TrendDirection::Improving);
        assert!(analysis.issue_change_percentage < 0.0);
    }

    #[test]
    fn test_trend_analysis_degrading() {
        // Higher days_offset = older (further back in time)
        let data_points = vec![
            make_data_point(40, 3), // Oldest: 40 issues (3 days ago)
            make_data_point(60, 2),
            make_data_point(80, 1),
            make_data_point(100, 0), // Newest: 100 issues (today) - 150% increase
        ];
        let analysis = TrendAnalysis::from_data_points(data_points);
        assert_eq!(analysis.trend_direction, TrendDirection::Degrading);
        assert!(analysis.issue_change_percentage > 0.0);
    }

    fn make_data_point(issues: usize, days_offset: i64) -> TrendDataPoint {
        TrendDataPoint {
            timestamp: Utc::now() - chrono::Duration::days(days_offset),
            result_file: format!("result-{}.json", days_offset),
            total_issues: issues,
            errors: issues / 2,
            warnings: issues / 2,
            total_files: 100,
            files_with_issues: issues / 10,
            duration_ms: 1000,
            target_paths: vec![],
        }
    }
}
