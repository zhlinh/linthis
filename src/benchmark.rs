// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Benchmark module for comparing linter/formatter performance.
//!
//! This module provides functionality to compare the speed of different
//! linting and formatting tools, particularly ruff vs flake8+black for Python.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

/// Result of a benchmark run
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Name of the tool
    pub tool: String,
    /// Duration for linting
    pub lint_duration: Option<Duration>,
    /// Duration for formatting
    pub format_duration: Option<Duration>,
    /// Number of files processed
    pub file_count: usize,
    /// Whether the tool is available
    pub available: bool,
}

impl BenchmarkResult {
    pub fn new(tool: &str) -> Self {
        Self {
            tool: tool.to_string(),
            lint_duration: None,
            format_duration: None,
            file_count: 0,
            available: false,
        }
    }

    pub fn total_duration(&self) -> Duration {
        let lint = self.lint_duration.unwrap_or(Duration::ZERO);
        let format = self.format_duration.unwrap_or(Duration::ZERO);
        lint + format
    }
}

/// Benchmark comparison results
#[derive(Debug)]
pub struct BenchmarkComparison {
    pub ruff: BenchmarkResult,
    pub legacy: BenchmarkResult,
}

impl BenchmarkComparison {
    /// Calculate speedup factor (legacy_time / ruff_time)
    pub fn speedup(&self) -> Option<f64> {
        let ruff_total = self.ruff.total_duration().as_secs_f64();
        let legacy_total = self.legacy.total_duration().as_secs_f64();

        if ruff_total > 0.0 && legacy_total > 0.0 {
            Some(legacy_total / ruff_total)
        } else {
            None
        }
    }

    pub fn lint_speedup(&self) -> Option<f64> {
        match (self.ruff.lint_duration, self.legacy.lint_duration) {
            (Some(ruff), Some(legacy)) => {
                let ruff_secs = ruff.as_secs_f64();
                let legacy_secs = legacy.as_secs_f64();
                if ruff_secs > 0.0 {
                    Some(legacy_secs / ruff_secs)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn format_speedup(&self) -> Option<f64> {
        match (self.ruff.format_duration, self.legacy.format_duration) {
            (Some(ruff), Some(legacy)) => {
                let ruff_secs = ruff.as_secs_f64();
                let legacy_secs = legacy.as_secs_f64();
                if ruff_secs > 0.0 {
                    Some(legacy_secs / ruff_secs)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Check if a tool is available
fn is_tool_available(tool: &str) -> bool {
    Command::new(tool)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run ruff check benchmark on files
fn benchmark_ruff_check(files: &[&Path]) -> Option<Duration> {
    if files.is_empty() {
        return Some(Duration::ZERO);
    }

    let start = Instant::now();
    for file in files {
        let _ = Command::new("ruff")
            .args(["check", "--output-format", "json"])
            .arg(file)
            .output();
    }
    Some(start.elapsed())
}

/// Run ruff format benchmark on files (check mode only, no modifications)
fn benchmark_ruff_format(files: &[&Path]) -> Option<Duration> {
    if files.is_empty() {
        return Some(Duration::ZERO);
    }

    let start = Instant::now();
    for file in files {
        let _ = Command::new("ruff")
            .args(["format", "--check"])
            .arg(file)
            .output();
    }
    Some(start.elapsed())
}

/// Run flake8 benchmark on files
fn benchmark_flake8(files: &[&Path]) -> Option<Duration> {
    if files.is_empty() {
        return Some(Duration::ZERO);
    }

    let start = Instant::now();
    for file in files {
        let _ = Command::new("flake8")
            .arg("--format=default")
            .arg(file)
            .output();
    }
    Some(start.elapsed())
}

/// Run black benchmark on files (check mode only, no modifications)
fn benchmark_black(files: &[&Path]) -> Option<Duration> {
    if files.is_empty() {
        return Some(Duration::ZERO);
    }

    let start = Instant::now();
    for file in files {
        let _ = Command::new("black")
            .args(["--check", "--quiet"])
            .arg(file)
            .output();
    }
    Some(start.elapsed())
}

/// Run benchmark comparison for Python files
pub fn run_python_benchmark(files: &[&Path]) -> BenchmarkComparison {
    let file_count = files.len();

    // Benchmark ruff
    let mut ruff_result = BenchmarkResult::new("ruff");
    ruff_result.available = is_tool_available("ruff");
    ruff_result.file_count = file_count;

    if ruff_result.available {
        ruff_result.lint_duration = benchmark_ruff_check(files);
        ruff_result.format_duration = benchmark_ruff_format(files);
    }

    // Benchmark flake8 + black
    let mut legacy_result = BenchmarkResult::new("flake8+black");
    let flake8_available = is_tool_available("flake8");
    let black_available = is_tool_available("black");
    legacy_result.available = flake8_available && black_available;
    legacy_result.file_count = file_count;

    if flake8_available {
        legacy_result.lint_duration = benchmark_flake8(files);
    }
    if black_available {
        legacy_result.format_duration = benchmark_black(files);
    }

    BenchmarkComparison {
        ruff: ruff_result,
        legacy: legacy_result,
    }
}

/// Format duration in milliseconds
fn format_duration_ms(duration: Option<Duration>) -> String {
    match duration {
        Some(d) => format!("{:.0}", d.as_secs_f64() * 1000.0),
        None => "N/A".to_string(),
    }
}

/// Format speedup as a string
fn format_speedup(speedup: Option<f64>) -> String {
    match speedup {
        Some(s) => format!("{:.1}x", s),
        None => "N/A".to_string(),
    }
}

/// Format benchmark results as a table
pub fn format_benchmark_table(comparison: &BenchmarkComparison) -> String {
    let file_count = comparison.ruff.file_count;

    let mut output = String::new();

    output.push_str(&format!(
        "\nPython Linting/Formatting Benchmark ({} files)\n",
        file_count
    ));
    output.push_str("┌──────────────┬─────────────┬─────────────┬──────────────┐\n");
    output.push_str("│ Tool         │ Lint (ms)   │ Format (ms) │ Total (ms)   │\n");
    output.push_str("├──────────────┼─────────────┼─────────────┼──────────────┤\n");

    // Legacy (flake8+black)
    if comparison.legacy.available {
        output.push_str(&format!(
            "│ {:<12} │ {:>11} │ {:>11} │ {:>12} │\n",
            "flake8+black",
            format_duration_ms(comparison.legacy.lint_duration),
            format_duration_ms(comparison.legacy.format_duration),
            format_duration_ms(Some(comparison.legacy.total_duration())),
        ));
    } else {
        output.push_str("│ flake8+black │     N/A     │     N/A     │      N/A     │\n");
    }

    // Ruff
    if comparison.ruff.available {
        output.push_str(&format!(
            "│ {:<12} │ {:>11} │ {:>11} │ {:>12} │\n",
            "ruff",
            format_duration_ms(comparison.ruff.lint_duration),
            format_duration_ms(comparison.ruff.format_duration),
            format_duration_ms(Some(comparison.ruff.total_duration())),
        ));
    } else {
        output.push_str("│ ruff         │     N/A     │     N/A     │      N/A     │\n");
    }

    output.push_str("├──────────────┼─────────────┼─────────────┼──────────────┤\n");

    // Speedup row
    output.push_str(&format!(
        "│ {:<12} │ {:>11} │ {:>11} │ {:>12} │\n",
        "Speedup",
        format_speedup(comparison.lint_speedup()),
        format_speedup(comparison.format_speedup()),
        format_speedup(comparison.speedup()),
    ));

    output.push_str("└──────────────┴─────────────┴─────────────┴──────────────┘\n");

    // Tool availability notes
    if !comparison.ruff.available {
        output.push_str("\n⚠ ruff not installed. Install with: pip install ruff\n");
    }
    if !comparison.legacy.available {
        if !is_tool_available("flake8") {
            output.push_str("\n⚠ flake8 not installed. Install with: pip install flake8\n");
        }
        if !is_tool_available("black") {
            output.push_str("\n⚠ black not installed. Install with: pip install black\n");
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_result_total_duration() {
        let mut result = BenchmarkResult::new("test");
        result.lint_duration = Some(Duration::from_millis(100));
        result.format_duration = Some(Duration::from_millis(50));

        assert_eq!(result.total_duration(), Duration::from_millis(150));
    }

    #[test]
    fn test_speedup_calculation() {
        let mut ruff = BenchmarkResult::new("ruff");
        ruff.lint_duration = Some(Duration::from_millis(100));
        ruff.format_duration = Some(Duration::from_millis(50));
        ruff.available = true;

        let mut legacy = BenchmarkResult::new("flake8+black");
        legacy.lint_duration = Some(Duration::from_millis(1000));
        legacy.format_duration = Some(Duration::from_millis(500));
        legacy.available = true;

        let comparison = BenchmarkComparison { ruff, legacy };

        // Total speedup: 1500ms / 150ms = 10x
        assert!((comparison.speedup().unwrap() - 10.0).abs() < 0.1);

        // Lint speedup: 1000ms / 100ms = 10x
        assert!((comparison.lint_speedup().unwrap() - 10.0).abs() < 0.1);

        // Format speedup: 500ms / 50ms = 10x
        assert!((comparison.format_speedup().unwrap() - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_format_duration_ms() {
        assert_eq!(format_duration_ms(Some(Duration::from_millis(100))), "100");
        assert_eq!(format_duration_ms(Some(Duration::from_secs(1))), "1000");
        assert_eq!(format_duration_ms(None), "N/A");
    }

    #[test]
    fn test_format_speedup() {
        assert_eq!(format_speedup(Some(10.0)), "10.0x");
        assert_eq!(format_speedup(Some(1.5)), "1.5x");
        assert_eq!(format_speedup(None), "N/A");
    }
}
