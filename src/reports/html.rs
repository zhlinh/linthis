// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! HTML report generation for linthis.

use chrono::Local;

use super::statistics::ReportStatistics;
use super::templates::{generate_bar_chart_svg, generate_trend_chart_svg, html_report_template};
use super::trends::TrendAnalysis;
use crate::utils::types::{RunResult, Severity};

/// Options for HTML report generation.
#[derive(Debug, Clone, Default)]
pub struct HtmlReportOptions {
    /// Include trend analysis section.
    pub include_trends: bool,
    /// Trend analysis data (if available).
    pub trends: Option<TrendAnalysis>,
}

/// Generate a complete HTML report from a RunResult.
pub fn generate_html_report(result: &RunResult, options: &HtmlReportOptions) -> String {
    let stats = ReportStatistics::from_run_result(result);
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Generate summary section
    let summary_html = generate_summary_html(result, &stats);

    // Generate statistics section
    let statistics_html = generate_statistics_html(&stats);

    // Generate issues section
    let issues_html = generate_issues_html(result);

    // Generate trends section (if requested)
    let trends_html = if options.include_trends {
        if let Some(ref trends) = options.trends {
            generate_trends_html(trends)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    html_report_template(
        "Linthis Lint Report",
        &summary_html,
        &statistics_html,
        &issues_html,
        &trends_html,
        &timestamp,
    )
}

/// Generate the summary section HTML.
fn generate_summary_html(result: &RunResult, stats: &ReportStatistics) -> String {
    let exit_status = match result.exit_code {
        0 => r#"<span class="stat-value success">✓ Passed</span>"#,
        1 => r#"<span class="stat-value error">✗ Failed (Errors)</span>"#,
        2 => r#"<span class="stat-value error">✗ Failed (Format)</span>"#,
        3 => r#"<span class="stat-value warning">⚠ Warnings</span>"#,
        _ => r#"<span class="stat-value">Unknown</span>"#,
    };

    format!(
        r#"<div class="stats-grid">
        <div class="stat-card">
            <div class="stat-value">{total_files}</div>
            <div class="stat-label">Total Files</div>
        </div>
        <div class="stat-card">
            <div class="stat-value error">{errors}</div>
            <div class="stat-label">Errors</div>
        </div>
        <div class="stat-card">
            <div class="stat-value warning">{warnings}</div>
            <div class="stat-label">Warnings</div>
        </div>
        <div class="stat-card">
            <div class="stat-value info">{info}</div>
            <div class="stat-label">Info</div>
        </div>
        <div class="stat-card">
            <div class="stat-value success">{clean_pct:.1}%</div>
            <div class="stat-label">Clean Files</div>
        </div>
        <div class="stat-card">
            {exit_status}
            <div class="stat-label">Status</div>
        </div>
    </div>
    <p><strong>Duration:</strong> {duration}ms | <strong>Files with issues:</strong> {files_with_issues}</p>"#,
        total_files = result.total_files,
        errors = stats.severity_counts.errors,
        warnings = stats.severity_counts.warnings,
        info = stats.severity_counts.info,
        clean_pct = stats.summary.clean_file_percentage,
        exit_status = exit_status,
        duration = result.duration_ms,
        files_with_issues = result.files_with_issues,
    )
}

/// Generate the statistics section HTML.
fn generate_statistics_html(stats: &ReportStatistics) -> String {
    let mut html = String::new();

    // Issues by language chart
    if !stats.by_language.is_empty() {
        let mut lang_data: Vec<_> = stats.by_language.iter().collect();
        lang_data.sort_by(|a, b| b.1.cmp(a.1));
        let chart_data: Vec<_> = lang_data
            .into_iter()
            .take(8)
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        html.push_str(&generate_bar_chart_svg(
            &chart_data,
            500,
            200,
            "Issues by Language",
        ));
    }

    // Issues by tool chart
    if !stats.by_tool.is_empty() {
        let mut tool_data: Vec<_> = stats.by_tool.iter().collect();
        tool_data.sort_by(|a, b| b.1.cmp(a.1));
        let chart_data: Vec<_> = tool_data
            .into_iter()
            .take(8)
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        html.push_str(&generate_bar_chart_svg(
            &chart_data,
            500,
            200,
            "Issues by Tool",
        ));
    }

    // Top rules table
    if !stats.by_rule.is_empty() {
        html.push_str("<h3>Top Rule Violations</h3>");
        html.push_str("<table><thead><tr><th>Rule</th><th>Severity</th><th>Count</th><th>Example</th></tr></thead><tbody>");

        let mut rules: Vec<_> = stats.by_rule.values().collect();
        rules.sort_by(|a, b| b.count.cmp(&a.count));

        for rule in rules.iter().take(10) {
            let severity_badge = match rule.severity.as_str() {
                "error" => r#"<span class="badge badge-error">error</span>"#,
                "warning" => r#"<span class="badge badge-warning">warning</span>"#,
                _ => r#"<span class="badge badge-info">info</span>"#,
            };
            let message_truncated = truncate_message(&rule.example_message, 60);
            html.push_str(&format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td></tr>",
                html_escape(&rule.code),
                severity_badge,
                rule.count,
                html_escape(&message_truncated),
            ));
        }
        html.push_str("</tbody></table>");
    }

    // Top problematic files
    if !stats.top_files.is_empty() {
        html.push_str("<h3>Top Problematic Files</h3>");
        html.push_str("<table><thead><tr><th>File</th><th>Total</th><th>Errors</th><th>Warnings</th></tr></thead><tbody>");

        for file in stats.top_files.iter().take(10) {
            html.push_str(&format!(
                "<tr><td><code>{}</code></td><td>{}</td><td class=\"error\">{}</td><td class=\"warning\">{}</td></tr>",
                html_escape(&file.path),
                file.issue_count,
                file.error_count,
                file.warning_count,
            ));
        }
        html.push_str("</tbody></table>");
    }

    html
}

/// Generate the issues section HTML.
fn generate_issues_html(result: &RunResult) -> String {
    if result.issues.is_empty() {
        return "<p>No issues found. Great job!</p>".to_string();
    }

    let mut html = String::new();
    html.push_str("<ul class=\"issue-list\">");

    for issue in &result.issues {
        let severity_class = match issue.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        };

        let severity_badge = match issue.severity {
            Severity::Error => r#"<span class="badge badge-error">error</span>"#,
            Severity::Warning => r#"<span class="badge badge-warning">warning</span>"#,
            Severity::Info => r#"<span class="badge badge-info">info</span>"#,
        };

        let location = format!(
            "{}:{}{}",
            issue.file_path.display(),
            issue.line,
            issue
                .column
                .map(|c| format!(":{}", c))
                .unwrap_or_default()
        );

        let rule_code = issue
            .code
            .as_ref()
            .map(|c| format!(" <code>[{}]</code>", html_escape(c)))
            .unwrap_or_default();

        let source = issue
            .source
            .as_ref()
            .map(|s| format!(" ({})", html_escape(s)))
            .unwrap_or_default();

        let suggestion = issue
            .suggestion
            .as_ref()
            .map(|s| {
                format!(
                    r#"<div class="issue-suggestion">💡 {}</div>"#,
                    html_escape(s)
                )
            })
            .unwrap_or_default();

        html.push_str(&format!(
            r#"<li class="issue-item {severity_class}">
            <div class="issue-header">
                <span class="issue-location">{location}</span>
                {severity_badge}{rule_code}{source}
            </div>
            <div class="issue-message">{message}</div>
            {suggestion}
        </li>"#,
            severity_class = severity_class,
            location = html_escape(&location),
            severity_badge = severity_badge,
            rule_code = rule_code,
            source = source,
            message = html_escape(&issue.message),
            suggestion = suggestion,
        ));
    }

    html.push_str("</ul>");
    html
}

/// Generate the trends section HTML.
fn generate_trends_html(trends: &TrendAnalysis) -> String {
    if trends.data_points.is_empty() {
        return "<p>Not enough historical data for trend analysis.</p>".to_string();
    }

    let mut html = String::new();

    // Trend indicator
    let trend_class = match trends.trend_direction {
        super::trends::TrendDirection::Improving => "trend-improving",
        super::trends::TrendDirection::Stable => "trend-stable",
        super::trends::TrendDirection::Degrading => "trend-degrading",
    };

    let trend_text = match trends.trend_direction {
        super::trends::TrendDirection::Improving => "↓ Improving",
        super::trends::TrendDirection::Stable => "→ Stable",
        super::trends::TrendDirection::Degrading => "↑ Degrading",
    };

    html.push_str(&format!(
        r#"<div class="stats-grid">
        <div class="stat-card">
            <div class="trend-indicator {trend_class}">{trend_text}</div>
            <div class="stat-label">Trend Direction</div>
        </div>
        <div class="stat-card">
            <div class="stat-value">{change:+.1}%</div>
            <div class="stat-label">Issue Change</div>
        </div>
        <div class="stat-card">
            <div class="stat-value">{avg:.1}</div>
            <div class="stat-label">Avg Issues/Run</div>
        </div>
        <div class="stat-card">
            <div class="stat-value">{runs}</div>
            <div class="stat-label">Runs Analyzed</div>
        </div>
    </div>"#,
        trend_class = trend_class,
        trend_text = trend_text,
        change = trends.issue_change_percentage,
        avg = trends.average_issues_per_run,
        runs = trends.data_points.len(),
    ));

    // Trend chart
    let chart_data: Vec<_> = trends
        .data_points
        .iter()
        .enumerate()
        .map(|(i, dp)| (format!("#{}", i + 1), dp.total_issues))
        .collect();

    html.push_str(&generate_trend_chart_svg(
        &chart_data,
        600,
        250,
        "Issues Over Time",
    ));

    html
}

/// Escape HTML special characters.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
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
    use crate::utils::types::LintIssue;
    use std::path::PathBuf;

    #[test]
    fn test_generate_html_report_empty() {
        let result = RunResult::new();
        let options = HtmlReportOptions::default();
        let html = generate_html_report(&result, &options);

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Linthis Report"));
        assert!(html.contains("No issues found"));
    }

    #[test]
    fn test_generate_html_report_with_issues() {
        let mut result = RunResult::new();
        result.total_files = 10;
        result.files_with_issues = 2;

        let mut issue = LintIssue::new(
            PathBuf::from("test.rs"),
            10,
            "Unused variable".to_string(),
            Severity::Warning,
        );
        issue.code = Some("W0001".to_string());
        issue.source = Some("clippy".to_string());
        result.add_issue(issue);

        let options = HtmlReportOptions::default();
        let html = generate_html_report(&result, &options);

        assert!(html.contains("test.rs:10"));
        assert!(html.contains("Unused variable"));
        assert!(html.contains("W0001"));
        assert!(html.contains("clippy"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape(r#"say "hi""#), "say &quot;hi&quot;");
    }

    #[test]
    fn test_truncate_message() {
        assert_eq!(truncate_message("short", 10), "short");
        assert_eq!(truncate_message("this is a long message", 10), "this is a…");
    }
}
