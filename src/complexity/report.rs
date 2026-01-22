// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Complexity report formatting module.

use super::analyzer::AnalysisResult;
use super::metrics::{FileMetrics, MetricLevel};

/// Report format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ComplexityReportFormat {
    /// Human-readable text output
    #[default]
    Human,
    /// JSON output
    Json,
    /// Markdown output
    Markdown,
    /// HTML output with visualization
    Html,
}

impl std::str::FromStr for ComplexityReportFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "human" | "text" => Ok(Self::Human),
            "json" => Ok(Self::Json),
            "markdown" | "md" => Ok(Self::Markdown),
            "html" => Ok(Self::Html),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}

/// Format complexity analysis result
pub fn format_complexity_report(result: &AnalysisResult, format: ComplexityReportFormat) -> String {
    match format {
        ComplexityReportFormat::Human => format_human(result),
        ComplexityReportFormat::Json => format_json(result),
        ComplexityReportFormat::Markdown => format_markdown(result),
        ComplexityReportFormat::Html => format_html(result),
    }
}

fn format_human(result: &AnalysisResult) -> String {
    let mut output = String::new();
    let reset = "\x1b[0m";

    // Header
    output.push_str("\n=== Code Complexity Analysis ===\n\n");

    // Summary
    output.push_str("Summary:\n");
    output.push_str(&format!("  Files analyzed: {}\n", result.summary.total_files));
    output.push_str(&format!("  Functions analyzed: {}\n", result.summary.total_functions));
    output.push_str(&format!("  Total SLOC: {}\n", result.summary.total_sloc));
    output.push_str(&format!(
        "  Average cyclomatic complexity: {:.2}\n",
        result.summary.avg_cyclomatic
    ));
    output.push_str(&format!(
        "  Maximum cyclomatic complexity: {}\n",
        result.summary.max_cyclomatic
    ));
    output.push_str(&format!(
        "  High complexity files: {}\n",
        result.summary.high_complexity_files
    ));
    output.push_str(&format!(
        "  High complexity functions: {}\n",
        result.summary.high_complexity_functions
    ));
    output.push_str(&format!("  Analysis time: {}ms\n\n", result.duration_ms));

    // Language breakdown
    if !result.by_language.is_empty() {
        output.push_str("By Language:\n");
        for (lang, stats) in &result.by_language {
            output.push_str(&format!(
                "  {}: {} files, {} functions, avg complexity: {:.2}\n",
                lang, stats.total_files, stats.total_functions, stats.avg_cyclomatic
            ));
        }
        output.push_str("\n");
    }

    // High complexity files
    let high_complexity: Vec<_> = result
        .files
        .iter()
        .filter(|f| {
            f.metrics.overall_level() == MetricLevel::High
                || f.metrics.overall_level() == MetricLevel::Critical
        })
        .collect();

    if !high_complexity.is_empty() {
        output.push_str("High Complexity Files:\n");
        for file in high_complexity {
            let level = file.metrics.overall_level();
            let color = level.color_code();
            output.push_str(&format!(
                "  {}{}{} - cyclomatic: {}, cognitive: {}, nesting: {}\n",
                color,
                file.path.display(),
                reset,
                file.metrics.cyclomatic,
                file.metrics.cognitive,
                file.metrics.max_nesting
            ));

            // Show high complexity functions
            for func in &file.functions {
                if func.metrics.overall_level() == MetricLevel::High
                    || func.metrics.overall_level() == MetricLevel::Critical
                {
                    let func_color = func.metrics.overall_level().color_code();
                    output.push_str(&format!(
                        "    {}{}(){} (lines {}-{}): cyclomatic={}, cognitive={}\n",
                        func_color,
                        func.name,
                        reset,
                        func.start_line,
                        func.end_line,
                        func.metrics.cyclomatic,
                        func.metrics.cognitive
                    ));
                }
            }
        }
        output.push_str("\n");
    }

    // Top 10 most complex files
    let mut sorted_files: Vec<&FileMetrics> = result.files.iter().collect();
    sorted_files.sort_by(|a, b| b.metrics.cyclomatic.cmp(&a.metrics.cyclomatic));

    output.push_str("Top 10 Most Complex Files:\n");
    for (i, file) in sorted_files.iter().take(10).enumerate() {
        let level = file.metrics.overall_level();
        let color = level.color_code();
        output.push_str(&format!(
            "  {}. {}{}{} - cyclomatic: {}\n",
            i + 1,
            color,
            file.path.display(),
            reset,
            file.metrics.cyclomatic
        ));
    }

    // Errors
    if !result.errors.is_empty() {
        output.push_str("\nErrors:\n");
        for error in &result.errors {
            output.push_str(&format!("  - {}\n", error));
        }
    }

    output
}

fn format_json(result: &AnalysisResult) -> String {
    serde_json::to_string_pretty(result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

fn format_markdown(result: &AnalysisResult) -> String {
    let mut output = String::new();

    // Header
    output.push_str("# Code Complexity Analysis Report\n\n");

    // Summary table
    output.push_str("## Summary\n\n");
    output.push_str("| Metric | Value |\n");
    output.push_str("|--------|-------|\n");
    output.push_str(&format!("| Files analyzed | {} |\n", result.summary.total_files));
    output.push_str(&format!(
        "| Functions analyzed | {} |\n",
        result.summary.total_functions
    ));
    output.push_str(&format!("| Total SLOC | {} |\n", result.summary.total_sloc));
    output.push_str(&format!(
        "| Avg cyclomatic complexity | {:.2} |\n",
        result.summary.avg_cyclomatic
    ));
    output.push_str(&format!(
        "| Max cyclomatic complexity | {} |\n",
        result.summary.max_cyclomatic
    ));
    output.push_str(&format!(
        "| High complexity files | {} |\n",
        result.summary.high_complexity_files
    ));
    output.push_str(&format!(
        "| High complexity functions | {} |\n",
        result.summary.high_complexity_functions
    ));
    output.push_str(&format!("| Analysis time | {}ms |\n\n", result.duration_ms));

    // Language breakdown
    if !result.by_language.is_empty() {
        output.push_str("## By Language\n\n");
        output.push_str("| Language | Files | Functions | Avg Complexity |\n");
        output.push_str("|----------|-------|-----------|----------------|\n");
        for (lang, stats) in &result.by_language {
            output.push_str(&format!(
                "| {} | {} | {} | {:.2} |\n",
                lang, stats.total_files, stats.total_functions, stats.avg_cyclomatic
            ));
        }
        output.push_str("\n");
    }

    // High complexity files
    let high_complexity: Vec<_> = result
        .files
        .iter()
        .filter(|f| {
            f.metrics.overall_level() == MetricLevel::High
                || f.metrics.overall_level() == MetricLevel::Critical
        })
        .collect();

    if !high_complexity.is_empty() {
        output.push_str("## High Complexity Files\n\n");
        output.push_str("| File | Cyclomatic | Cognitive | Nesting | Level |\n");
        output.push_str("|------|------------|-----------|---------|-------|\n");
        for file in high_complexity {
            let level = file.metrics.overall_level();
            let emoji = level.emoji();
            output.push_str(&format!(
                "| `{}` | {} | {} | {} | {} |\n",
                file.path.display(),
                file.metrics.cyclomatic,
                file.metrics.cognitive,
                file.metrics.max_nesting,
                emoji
            ));
        }
        output.push_str("\n");
    }

    // Top complex functions
    let mut all_functions: Vec<_> = result
        .files
        .iter()
        .flat_map(|f| f.functions.iter().map(move |func| (f, func)))
        .collect();
    all_functions.sort_by(|a, b| b.1.metrics.cyclomatic.cmp(&a.1.metrics.cyclomatic));

    output.push_str("## Top 20 Most Complex Functions\n\n");
    output.push_str("| Function | File | Lines | Cyclomatic | Cognitive |\n");
    output.push_str("|----------|------|-------|------------|------------|\n");
    for (file, func) in all_functions.iter().take(20) {
        output.push_str(&format!(
            "| `{}` | `{}` | {}-{} | {} | {} |\n",
            func.name,
            file.path.display(),
            func.start_line,
            func.end_line,
            func.metrics.cyclomatic,
            func.metrics.cognitive
        ));
    }

    output
}

fn format_html(result: &AnalysisResult) -> String {
    let mut output = String::new();

    output.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    output.push_str("  <meta charset=\"UTF-8\">\n");
    output.push_str("  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    output.push_str("  <title>Code Complexity Analysis Report</title>\n");
    output.push_str("  <style>\n");
    output.push_str("    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 20px; }\n");
    output.push_str("    h1, h2 { color: #333; }\n");
    output.push_str("    table { border-collapse: collapse; width: 100%; margin: 20px 0; }\n");
    output.push_str("    th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n");
    output.push_str("    th { background-color: #4a5568; color: white; }\n");
    output.push_str("    tr:nth-child(even) { background-color: #f9f9f9; }\n");
    output.push_str("    .good { color: #22c55e; }\n");
    output.push_str("    .warning { color: #eab308; }\n");
    output.push_str("    .high { color: #f97316; }\n");
    output.push_str("    .critical { color: #ef4444; font-weight: bold; }\n");
    output.push_str("    .summary-card { background: #f7fafc; padding: 20px; border-radius: 8px; margin: 20px 0; }\n");
    output.push_str("    .metric { display: inline-block; margin: 10px 20px 10px 0; }\n");
    output.push_str("    .metric-value { font-size: 2em; font-weight: bold; color: #4a5568; }\n");
    output.push_str("    .metric-label { color: #718096; }\n");
    output.push_str("    .bar { height: 20px; background: linear-gradient(90deg, #22c55e 0%, #eab308 50%, #ef4444 100%); border-radius: 4px; }\n");
    output.push_str("    .bar-fill { height: 100%; background: #4a5568; border-radius: 4px; }\n");
    output.push_str("  </style>\n");
    output.push_str("</head>\n<body>\n");

    output.push_str("  <h1>Code Complexity Analysis Report</h1>\n\n");

    // Summary cards
    output.push_str("  <div class=\"summary-card\">\n");
    output.push_str(&format!(
        "    <div class=\"metric\"><div class=\"metric-value\">{}</div><div class=\"metric-label\">Files</div></div>\n",
        result.summary.total_files
    ));
    output.push_str(&format!(
        "    <div class=\"metric\"><div class=\"metric-value\">{}</div><div class=\"metric-label\">Functions</div></div>\n",
        result.summary.total_functions
    ));
    output.push_str(&format!(
        "    <div class=\"metric\"><div class=\"metric-value\">{}</div><div class=\"metric-label\">SLOC</div></div>\n",
        result.summary.total_sloc
    ));
    output.push_str(&format!(
        "    <div class=\"metric\"><div class=\"metric-value\">{:.1}</div><div class=\"metric-label\">Avg Complexity</div></div>\n",
        result.summary.avg_cyclomatic
    ));
    output.push_str(&format!(
        "    <div class=\"metric\"><div class=\"metric-value {}\">{}</div><div class=\"metric-label\">High Complexity Files</div></div>\n",
        if result.summary.high_complexity_files > 0 { "critical" } else { "good" },
        result.summary.high_complexity_files
    ));
    output.push_str("  </div>\n\n");

    // Files table
    output.push_str("  <h2>Files by Complexity</h2>\n");
    output.push_str("  <table>\n");
    output.push_str("    <tr><th>File</th><th>Language</th><th>Cyclomatic</th><th>Cognitive</th><th>Nesting</th><th>SLOC</th></tr>\n");

    let mut sorted_files: Vec<_> = result.files.iter().collect();
    sorted_files.sort_by(|a, b| b.metrics.cyclomatic.cmp(&a.metrics.cyclomatic));

    for file in sorted_files.iter().take(50) {
        let level_class = match file.metrics.overall_level() {
            MetricLevel::Good => "good",
            MetricLevel::Warning => "warning",
            MetricLevel::High => "high",
            MetricLevel::Critical => "critical",
        };
        output.push_str(&format!(
            "    <tr><td class=\"{}\">{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            level_class,
            file.path.display(),
            file.language,
            file.metrics.cyclomatic,
            file.metrics.cognitive,
            file.metrics.max_nesting,
            file.metrics.sloc
        ));
    }
    output.push_str("  </table>\n\n");

    // Functions table
    output.push_str("  <h2>Most Complex Functions</h2>\n");
    output.push_str("  <table>\n");
    output.push_str("    <tr><th>Function</th><th>File</th><th>Lines</th><th>Cyclomatic</th><th>Cognitive</th></tr>\n");

    let mut all_functions: Vec<_> = result
        .files
        .iter()
        .flat_map(|f| f.functions.iter().map(move |func| (f, func)))
        .collect();
    all_functions.sort_by(|a, b| b.1.metrics.cyclomatic.cmp(&a.1.metrics.cyclomatic));

    for (file, func) in all_functions.iter().take(30) {
        let level_class = match func.metrics.overall_level() {
            MetricLevel::Good => "good",
            MetricLevel::Warning => "warning",
            MetricLevel::High => "high",
            MetricLevel::Critical => "critical",
        };
        output.push_str(&format!(
            "    <tr><td class=\"{}\">{}</td><td>{}</td><td>{}-{}</td><td>{}</td><td>{}</td></tr>\n",
            level_class,
            func.name,
            file.path.display(),
            func.start_line,
            func.end_line,
            func.metrics.cyclomatic,
            func.metrics.cognitive
        ));
    }
    output.push_str("  </table>\n\n");

    output.push_str("</body>\n</html>\n");

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::complexity::AnalysisResult;

    #[test]
    fn test_format_parsing() {
        assert_eq!(
            "human".parse::<ComplexityReportFormat>().unwrap(),
            ComplexityReportFormat::Human
        );
        assert_eq!(
            "json".parse::<ComplexityReportFormat>().unwrap(),
            ComplexityReportFormat::Json
        );
        assert_eq!(
            "markdown".parse::<ComplexityReportFormat>().unwrap(),
            ComplexityReportFormat::Markdown
        );
        assert_eq!(
            "html".parse::<ComplexityReportFormat>().unwrap(),
            ComplexityReportFormat::Html
        );
    }

    #[test]
    fn test_format_human() {
        let result = AnalysisResult::new();
        let output = format_complexity_report(&result, ComplexityReportFormat::Human);
        assert!(output.contains("Code Complexity Analysis"));
    }

    #[test]
    fn test_format_json() {
        let result = AnalysisResult::new();
        let output = format_complexity_report(&result, ComplexityReportFormat::Json);
        assert!(output.starts_with('{'));
    }

    #[test]
    fn test_format_markdown() {
        let result = AnalysisResult::new();
        let output = format_complexity_report(&result, ComplexityReportFormat::Markdown);
        assert!(output.contains("# Code Complexity"));
    }

    #[test]
    fn test_format_html() {
        let result = AnalysisResult::new();
        let output = format_complexity_report(&result, ComplexityReportFormat::Html);
        assert!(output.contains("<!DOCTYPE html>"));
    }
}
