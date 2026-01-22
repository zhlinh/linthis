// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! CLI handler for complexity analysis command.

use std::path::PathBuf;
use std::process::ExitCode;

use linthis::complexity::{
    format_complexity_report, AnalysisOptions, ComplexityAnalyzer, ComplexityReportFormat,
    MetricLevel, Thresholds,
};

/// Options for complexity command
pub struct ComplexityCommandOptions {
    pub path: PathBuf,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub threshold: Option<u32>,
    pub preset: String,
    pub format: String,
    pub with_trends: bool,
    pub trend_count: usize,
    pub only_high: bool,
    pub sort: String,
    pub no_parallel: bool,
    pub fail_on_high: bool,
    pub verbose: bool,
}

/// Handle the complexity analysis command
pub fn handle_complexity_command(options: ComplexityCommandOptions) -> ExitCode {
    if options.verbose {
        println!("Analyzing code complexity in: {}", options.path.display());
    }

    // Create analyzer
    let analyzer = ComplexityAnalyzer::new();

    // Build analysis options
    let mut analysis_options = AnalysisOptions::new(options.path.clone());
    analysis_options.include = options.include.unwrap_or_default();
    analysis_options.exclude = options.exclude.unwrap_or_default();
    analysis_options.threshold = options.threshold;
    analysis_options.format = options.format.clone();
    analysis_options.with_trends = options.with_trends;
    analysis_options.trend_count = options.trend_count;
    analysis_options.verbose = options.verbose;
    analysis_options.parallel = !options.no_parallel;

    // Run analysis
    let mut result = match analyzer.analyze(&analysis_options) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error during analysis: {}", e);
            return ExitCode::FAILURE;
        }
    };

    // Apply threshold preset
    result.thresholds = match options.preset.as_str() {
        "strict" => Thresholds::strict(),
        "lenient" => Thresholds::lenient(),
        _ => Thresholds::default(),
    };

    // Override with custom threshold if provided
    if let Some(threshold) = options.threshold {
        result.thresholds.cyclomatic.good = threshold;
        result.thresholds.cyclomatic.warning = threshold * 2;
        result.thresholds.cyclomatic.high = threshold * 5;
    }

    // Parse output format
    let format = options.format.parse::<ComplexityReportFormat>().unwrap_or_default();

    // Filter to only high complexity if requested
    if options.only_high {
        result.files.retain(|f| {
            f.metrics.overall_level() == MetricLevel::High
                || f.metrics.overall_level() == MetricLevel::Critical
        });
    }

    // Sort results
    match options.sort.as_str() {
        "cognitive" => {
            result.files.sort_by(|a, b| b.metrics.cognitive.cmp(&a.metrics.cognitive));
        }
        "lines" | "loc" => {
            result.files.sort_by(|a, b| b.metrics.loc.cmp(&a.metrics.loc));
        }
        "name" => {
            result.files.sort_by(|a, b| a.path.cmp(&b.path));
        }
        _ => {
            // Default: cyclomatic
            result.files.sort_by(|a, b| b.metrics.cyclomatic.cmp(&a.metrics.cyclomatic));
        }
    }

    // Generate and print report
    let report = format_complexity_report(&result, format);
    println!("{}", report);

    // Check for failure condition
    if options.fail_on_high && result.summary.high_complexity_files > 0 {
        eprintln!(
            "\nError: {} file(s) exceed complexity threshold",
            result.summary.high_complexity_files
        );
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complexity_command_options() {
        let options = ComplexityCommandOptions {
            path: PathBuf::from("."),
            include: None,
            exclude: None,
            threshold: Some(10),
            preset: "default".to_string(),
            format: "human".to_string(),
            with_trends: false,
            trend_count: 10,
            only_high: false,
            sort: "cyclomatic".to_string(),
            no_parallel: false,
            fail_on_high: false,
            verbose: false,
        };

        assert_eq!(options.threshold, Some(10));
        assert_eq!(options.preset, "default");
    }
}
