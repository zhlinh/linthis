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
}

impl TrendAnalysis {
    /// Analyze trends from a list of data points.
    pub fn from_data_points(mut data_points: Vec<TrendDataPoint>) -> Self {
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
            };
        }

        // Calculate average
        let total: usize = data_points.iter().map(|dp| dp.total_issues).sum();
        let average_issues_per_run = total as f64 / data_points.len() as f64;

        // Find best and worst runs
        let best_run = data_points
            .iter()
            .min_by_key(|dp| dp.total_issues)
            .cloned();
        let worst_run = data_points
            .iter()
            .max_by_key(|dp| dp.total_issues)
            .cloned();

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
        }
    }

    /// Format trend analysis as human-readable text.
    pub fn format_human(&self) -> String {
        if self.data_points.is_empty() {
            return "No historical data available for trend analysis.\n".to_string();
        }

        let mut output = String::new();

        output.push_str("=== Code Quality Trends ===\n\n");

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
            output.push_str(&format!(
                "  {}. {} - {} issues ({} errors, {} warnings)\n",
                i + 1,
                dp.timestamp.format("%Y-%m-%d %H:%M"),
                dp.total_issues,
                dp.errors,
                dp.warnings,
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

    // Load and parse files
    result_files
        .into_iter()
        .take(limit)
        .filter_map(|entry| {
            let path = entry.path();
            let content = fs::read_to_string(&path).ok()?;
            let result: RunResult = serde_json::from_str(&content).ok()?;
            Some((path, result))
        })
        .collect()
}

/// Get the most recent result file.
pub fn get_last_result(project_root: &Path) -> Option<(PathBuf, RunResult)> {
    load_historical_results(project_root, 1).into_iter().next()
}

/// Load a result from a specific file path.
pub fn load_result_from_file(path: &Path) -> Option<RunResult> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
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
pub fn analyze_trends(project_root: &Path, limit: usize) -> TrendAnalysis {
    let results = load_historical_results(project_root, limit);

    let data_points: Vec<_> = results
        .into_iter()
        .filter_map(|(path, result)| {
            let filename = path.file_name()?.to_string_lossy().to_string();
            let timestamp = parse_result_timestamp(&filename)?;
            Some(TrendDataPoint::from_result(&result, &path, timestamp))
        })
        .collect();

    TrendAnalysis::from_data_points(data_points)
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
            make_data_point(40, 3),  // Oldest: 40 issues (3 days ago)
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
        }
    }
}
