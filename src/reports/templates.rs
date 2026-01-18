// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Embedded HTML/CSS/SVG templates for linthis reports.

/// Embedded CSS for the HTML report.
pub fn report_css() -> &'static str {
    r#"
:root {
    --color-error: #dc3545;
    --color-warning: #ffc107;
    --color-info: #17a2b8;
    --color-success: #28a745;
    --color-bg: #ffffff;
    --color-bg-secondary: #f8f9fa;
    --color-text: #212529;
    --color-text-muted: #6c757d;
    --color-border: #dee2e6;
}

* { box-sizing: border-box; margin: 0; padding: 0; }

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
    line-height: 1.6;
    color: var(--color-text);
    background: var(--color-bg);
}

.container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem;
}

header {
    margin-bottom: 2rem;
    padding-bottom: 1rem;
    border-bottom: 2px solid var(--color-border);
}

header h1 {
    font-size: 2rem;
    color: #333;
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

header h1::before {
    content: '📊';
}

.generated-at {
    color: var(--color-text-muted);
    font-size: 0.9rem;
    margin-top: 0.5rem;
}

section {
    margin-bottom: 2rem;
}

section h2 {
    font-size: 1.4rem;
    margin-bottom: 1rem;
    color: #333;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--color-border);
}

/* Stats Grid */
.stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 1rem;
    margin-bottom: 1.5rem;
}

.stat-card {
    background: var(--color-bg-secondary);
    border-radius: 8px;
    padding: 1.5rem;
    text-align: center;
    border: 1px solid var(--color-border);
}

.stat-value {
    font-size: 2.5rem;
    font-weight: bold;
    line-height: 1.2;
}

.stat-label {
    color: var(--color-text-muted);
    font-size: 0.85rem;
    margin-top: 0.25rem;
}

.stat-value.error { color: var(--color-error); }
.stat-value.warning { color: var(--color-warning); }
.stat-value.info { color: var(--color-info); }
.stat-value.success { color: var(--color-success); }

/* Issue List */
.issue-list {
    list-style: none;
}

.issue-item {
    padding: 1rem;
    margin-bottom: 0.5rem;
    border-left: 4px solid;
    background: var(--color-bg-secondary);
    border-radius: 0 4px 4px 0;
}

.issue-item.error { border-color: var(--color-error); }
.issue-item.warning { border-color: var(--color-warning); }
.issue-item.info { border-color: var(--color-info); }

.issue-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 0.5rem;
}

.issue-location {
    font-family: 'SF Mono', Monaco, 'Courier New', monospace;
    font-size: 0.85rem;
    color: var(--color-text-muted);
}

.issue-message {
    font-size: 0.95rem;
}

.issue-suggestion {
    margin-top: 0.5rem;
    padding: 0.5rem;
    background: rgba(40, 167, 69, 0.1);
    border-radius: 4px;
    font-size: 0.85rem;
    color: #155724;
}

/* Badges */
.badge {
    display: inline-block;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 500;
}

.badge-error { background: var(--color-error); color: white; }
.badge-warning { background: var(--color-warning); color: #333; }
.badge-info { background: var(--color-info); color: white; }

/* Tables */
table {
    width: 100%;
    border-collapse: collapse;
    margin: 1rem 0;
}

th, td {
    padding: 0.75rem;
    text-align: left;
    border-bottom: 1px solid var(--color-border);
}

th {
    background: var(--color-bg-secondary);
    font-weight: 600;
}

tr:hover {
    background: var(--color-bg-secondary);
}

/* Charts */
.chart-container {
    margin: 1.5rem 0;
    overflow-x: auto;
}

.chart {
    max-width: 100%;
    height: auto;
}

.bar { fill: #4a90d9; }
.bar:hover { fill: #357abd; }
.bar-label { font-size: 11px; fill: #333; }
.bar-value { font-size: 10px; fill: #666; }
.chart-title { font-size: 14px; font-weight: 600; fill: #333; }
.axis-line { stroke: #ccc; stroke-width: 1; }
.axis-label { font-size: 10px; fill: #666; }

.trend-line { fill: none; stroke: #4a90d9; stroke-width: 2; }
.trend-point { fill: #4a90d9; }
.trend-point:hover { fill: #357abd; r: 6; }

/* Trend Indicator */
.trend-indicator {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-size: 0.85rem;
    font-weight: 500;
}

.trend-improving {
    background: rgba(40, 167, 69, 0.1);
    color: var(--color-success);
}

.trend-stable {
    background: rgba(23, 162, 184, 0.1);
    color: var(--color-info);
}

.trend-degrading {
    background: rgba(220, 53, 69, 0.1);
    color: var(--color-error);
}

/* Collapsible Sections */
.collapsible {
    cursor: pointer;
    user-select: none;
}

.collapsible::after {
    content: ' ▼';
    font-size: 0.8em;
}

.collapsible.collapsed::after {
    content: ' ▶';
}

.collapse-content {
    overflow: hidden;
    max-height: 2000px;
    transition: max-height 0.3s ease;
}

.collapse-content.collapsed {
    max-height: 0;
}

/* Footer */
footer {
    margin-top: 3rem;
    padding-top: 1rem;
    border-top: 1px solid var(--color-border);
    text-align: center;
    color: var(--color-text-muted);
    font-size: 0.85rem;
}

/* Dark Mode */
@media (prefers-color-scheme: dark) {
    :root {
        --color-bg: #1a1a1a;
        --color-bg-secondary: #2d2d2d;
        --color-text: #e0e0e0;
        --color-text-muted: #999;
        --color-border: #404040;
    }

    header h1, section h2 { color: #e0e0e0; }
    .bar-label, .chart-title { fill: #e0e0e0; }
    .bar-value, .axis-label { fill: #999; }
    .issue-suggestion { background: rgba(40, 167, 69, 0.2); color: #8fd19e; }
}

/* Print Styles */
@media print {
    body { background: white; }
    .container { max-width: none; padding: 1rem; }
    .collapsible::after { display: none; }
    .collapse-content { max-height: none !important; }
}
"#
}

/// Minimal JavaScript for the HTML report (collapsible sections).
pub fn report_js() -> &'static str {
    r#"
document.addEventListener('DOMContentLoaded', function() {
    document.querySelectorAll('.collapsible').forEach(function(header) {
        header.addEventListener('click', function() {
            this.classList.toggle('collapsed');
            var content = this.nextElementSibling;
            if (content && content.classList.contains('collapse-content')) {
                content.classList.toggle('collapsed');
            }
        });
    });
});
"#
}

/// Generate the main HTML report structure.
pub fn html_report_template(
    title: &str,
    summary_html: &str,
    statistics_html: &str,
    issues_html: &str,
    trends_html: &str,
    timestamp: &str,
) -> String {
    let css = report_css();
    let js = report_js();

    let trends_section = if trends_html.is_empty() {
        String::new()
    } else {
        format!(
            r#"<section id="trends">
            <h2>Code Quality Trends</h2>
            {}
        </section>"#,
            trends_html
        )
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <style>{css}</style>
</head>
<body>
    <div class="container">
        <header>
            <h1>Linthis Report</h1>
            <p class="generated-at">Generated: {timestamp}</p>
        </header>

        <section id="summary">
            <h2>Summary</h2>
            {summary_html}
        </section>

        <section id="statistics">
            <h2>Statistics</h2>
            {statistics_html}
        </section>

        {trends_section}

        <section id="issues">
            <h2 class="collapsible">Issues ({issue_count})</h2>
            <div class="collapse-content">
                {issues_html}
            </div>
        </section>

        <footer>
            Generated by <strong>linthis</strong> v{version}
        </footer>
    </div>
    <script>{js}</script>
</body>
</html>"#,
        title = title,
        css = css,
        timestamp = timestamp,
        summary_html = summary_html,
        statistics_html = statistics_html,
        trends_section = trends_section,
        issue_count = count_issues(issues_html),
        issues_html = issues_html,
        js = js,
        version = env!("CARGO_PKG_VERSION"),
    )
}

/// Count issues in the HTML (simple heuristic).
fn count_issues(issues_html: &str) -> usize {
    issues_html.matches("issue-item").count()
}

/// Generate a bar chart as inline SVG.
pub fn generate_bar_chart_svg(
    data: &[(String, usize)],
    width: usize,
    height: usize,
    title: &str,
) -> String {
    if data.is_empty() {
        return String::new();
    }

    let max_value = data.iter().map(|(_, v)| *v).max().unwrap_or(1).max(1);
    let bar_width = (width - 60) / data.len().max(1);
    let chart_height = height - 60;
    let scale = chart_height as f64 / max_value as f64;

    let mut bars = String::new();
    for (i, (label, value)) in data.iter().enumerate() {
        let bar_height = (*value as f64 * scale) as usize;
        let x = 40 + i * bar_width;
        let y = 30 + chart_height - bar_height;
        let label_truncated = truncate_label(label, 8);

        bars.push_str(&format!(
            r#"<rect class="bar" x="{x}" y="{y}" width="{bw}" height="{bh}" rx="2">
                <title>{label}: {value}</title>
            </rect>
            <text class="bar-value" x="{lx}" y="{vy}" text-anchor="middle">{value}</text>
            <text class="bar-label" x="{lx}" y="{ly}" text-anchor="middle">{label_short}</text>"#,
            x = x + 2,
            y = y,
            bw = bar_width.saturating_sub(4),
            bh = bar_height,
            lx = x + bar_width / 2,
            vy = y.saturating_sub(5),
            ly = height - 10,
            label = label,
            value = value,
            label_short = label_truncated,
        ));
    }

    format!(
        r#"<div class="chart-container">
        <svg viewBox="0 0 {w} {h}" class="chart" role="img" aria-label="{title}">
            <text x="{tw}" y="20" text-anchor="middle" class="chart-title">{title}</text>
            <line class="axis-line" x1="40" y1="30" x2="40" y2="{ah}"/>
            <line class="axis-line" x1="40" y1="{ah}" x2="{aw}" y2="{ah}"/>
            {bars}
        </svg>
    </div>"#,
        w = width,
        h = height,
        tw = width / 2,
        title = title,
        ah = 30 + chart_height,
        aw = width - 20,
        bars = bars,
    )
}

/// Generate a trend line chart as inline SVG.
pub fn generate_trend_chart_svg(
    data: &[(String, usize)], // (label, value)
    width: usize,
    height: usize,
    title: &str,
) -> String {
    if data.len() < 2 {
        return String::new();
    }

    let max_value = data.iter().map(|(_, v)| *v).max().unwrap_or(1).max(1);
    let chart_width = width - 80;
    let chart_height = height - 60;
    let x_step = chart_width / (data.len() - 1).max(1);
    let y_scale = chart_height as f64 / max_value as f64;

    // Build path for trend line
    let mut path_data = String::new();
    let mut points = String::new();

    for (i, (label, value)) in data.iter().enumerate() {
        let x = 50 + i * x_step;
        let y = 30 + chart_height - (*value as f64 * y_scale) as usize;
        let cmd = if i == 0 { "M" } else { "L" };
        path_data.push_str(&format!("{} {} {} ", cmd, x, y));

        points.push_str(&format!(
            r#"<circle class="trend-point" cx="{x}" cy="{y}" r="4">
                <title>{label}: {value}</title>
            </circle>"#,
            x = x,
            y = y,
            label = label,
            value = value,
        ));
    }

    // Y-axis labels
    let mut y_labels = String::new();
    for i in 0..=4 {
        let value = (max_value * i) / 4;
        let y = 30 + chart_height - (value as f64 * y_scale) as usize;
        y_labels.push_str(&format!(
            r#"<text class="axis-label" x="35" y="{y}" text-anchor="end">{value}</text>"#,
            y = y + 4,
            value = value,
        ));
    }

    format!(
        r#"<div class="chart-container">
        <svg viewBox="0 0 {w} {h}" class="chart" role="img" aria-label="{title}">
            <text x="{tw}" y="20" text-anchor="middle" class="chart-title">{title}</text>
            <line class="axis-line" x1="50" y1="30" x2="50" y2="{ah}"/>
            <line class="axis-line" x1="50" y1="{ah}" x2="{aw}" y2="{ah}"/>
            {y_labels}
            <path class="trend-line" d="{path}"/>
            {points}
        </svg>
    </div>"#,
        w = width,
        h = height,
        tw = width / 2,
        title = title,
        ah = 30 + chart_height,
        aw = width - 30,
        y_labels = y_labels,
        path = path_data,
        points = points,
    )
}

/// Truncate a label to a maximum length.
fn truncate_label(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_css_not_empty() {
        let css = report_css();
        assert!(!css.is_empty());
        assert!(css.contains("--color-error"));
        assert!(css.contains("@media (prefers-color-scheme: dark)"));
    }

    #[test]
    fn test_generate_bar_chart_svg() {
        let data = vec![
            ("Rust".to_string(), 10),
            ("Python".to_string(), 5),
            ("Go".to_string(), 3),
        ];
        let svg = generate_bar_chart_svg(&data, 400, 200, "Issues by Language");
        assert!(svg.contains("chart-container"));
        assert!(svg.contains("Rust"));
        assert!(svg.contains("10"));
    }

    #[test]
    fn test_generate_trend_chart_svg() {
        let data = vec![
            ("Run 1".to_string(), 10),
            ("Run 2".to_string(), 8),
            ("Run 3".to_string(), 5),
        ];
        let svg = generate_trend_chart_svg(&data, 400, 200, "Issue Trend");
        assert!(svg.contains("trend-line"));
        assert!(svg.contains("trend-point"));
    }

    #[test]
    fn test_html_report_template() {
        let html = html_report_template(
            "Test Report",
            "<p>Summary</p>",
            "<p>Stats</p>",
            "<p>Issues</p>",
            "",
            "2026-01-18 12:00:00",
        );
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Test Report"));
        assert!(html.contains("Summary"));
        assert!(html.contains("linthis"));
    }

    #[test]
    fn test_truncate_label() {
        assert_eq!(truncate_label("short", 10), "short");
        assert_eq!(truncate_label("verylongname", 8), "verylon…");
    }
}
