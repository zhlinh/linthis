// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! SAST report formatting and output.

use colored::Colorize;
use serde::Serialize;

use super::finding::SastFinding;
use super::SastResult;
use crate::security::report::SecurityReportFormat;
use crate::security::vulnerability::Severity;

/// SAST report summary
#[derive(Debug, Serialize)]
pub struct SastReportSummary {
    pub total_findings: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub scanners_used: Vec<String>,
    pub scanners_unavailable: Vec<String>,
}

/// Format SAST scan results for output.
pub fn format_sast_report(result: &SastResult, format: SecurityReportFormat) -> String {
    match format {
        SecurityReportFormat::Human => format_human(result),
        SecurityReportFormat::Json => format_json(result),
        SecurityReportFormat::Sarif => format_sarif(result),
    }
}

fn format_human(result: &SastResult) -> String {
    let mut output = String::new();

    format_header(&mut output, result);

    if result.findings.is_empty() {
        output.push_str(&format!("{}\n", "  ✅ No security issues found.".green()));
        return output;
    }

    let errors: Vec<_> = result
        .findings
        .iter()
        .filter(|f| matches!(f.severity, Severity::Critical | Severity::High))
        .collect();
    let warnings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Medium)
        .collect();
    let infos: Vec<_> = result
        .findings
        .iter()
        .filter(|f| matches!(f.severity, Severity::Low | Severity::None | Severity::Unknown))
        .collect();

    format_issue_counts(&mut output, result.findings.len(), &errors, &warnings, &infos);
    format_grouped_findings(&mut output, &errors, &warnings, &infos);
    format_summary_line(&mut output, result, &errors, &warnings, &infos);
    save_result_to_disk(&mut output, result);
    format_unavailable_tools(&mut output, result);

    output
}

/// Render the header and scanner status lines.
fn format_header(output: &mut String, result: &SastResult) {
    output.push_str(&format!(
        "\n{}\n",
        "🔍 SAST Source Code Security Scan Results".bold()
    ));

    for (name, available) in &result.scanner_status {
        let status = if *available {
            "✓".green().to_string()
        } else {
            "✗".red().to_string()
        };
        output.push_str(&format!("  {} {}\n", status, name));
    }
    output.push('\n');
}

/// Render the "Found N issue(s)" counts.
fn format_issue_counts(
    output: &mut String,
    total: usize,
    errors: &[&SastFinding],
    warnings: &[&SastFinding],
    infos: &[&SastFinding],
) {
    output.push_str(&format!(
        "  Found {} issue(s): {} error{}, {} warning{}, {} info\n\n",
        total,
        errors.len(),
        if errors.len() == 1 { "" } else { "s" },
        warnings.len(),
        if warnings.len() == 1 { "" } else { "s" },
        infos.len(),
    ));
}

/// Render each finding grouped by severity.
fn format_grouped_findings(
    output: &mut String,
    errors: &[&SastFinding],
    warnings: &[&SastFinding],
    infos: &[&SastFinding],
) {
    let all_groups: Vec<(&str, &[&SastFinding])> = vec![
        ("ERROR", errors),
        ("WARN", warnings),
        ("INFO", infos),
    ];

    let mut issue_num = 0usize;
    for (label, findings) in all_groups {
        for finding in findings {
            issue_num += 1;
            format_single_finding(output, issue_num, label, finding);
        }
    }
}

/// Render a single finding entry.
fn format_single_finding(
    output: &mut String,
    issue_num: usize,
    label: &str,
    finding: &SastFinding,
) {
    let colored_label = match label {
        "ERROR" => format!("[{}]", label).red().bold().to_string(),
        "WARN" => format!("[{}]", label).yellow().to_string(),
        _ => format!("[{}]", label).cyan().to_string(),
    };

    output.push_str(&format!("  {}. {} {}\n", issue_num, colored_label, finding.message));
    output.push_str(&format!(
        "    File: {}:{}\n",
        finding.file_path.display(),
        finding.line
    ));
    output.push_str(&format!("    Rule: {}\n", finding.rule_id));
    output.push_str(&format!("    Tool: {}\n", finding.source));

    if !finding.cwe_ids.is_empty() {
        output.push_str(&format!("    CWE:  {}\n", finding.cwe_ids.join(", ")));
    }

    if let Some(ref snippet) = finding.code_snippet {
        let snippet_str: &str = snippet;
        for code_line in snippet_str.lines().take(3) {
            output.push_str(&format!("    > {}\n", code_line.dimmed()));
        }
    }

    if let Some(ref fix) = finding.fix_suggestion {
        let fix_str: &str = fix;
        output.push_str(&format!("    Fix:  {}\n", fix_str.green()));
    }

    output.push('\n');
}

/// Render the bottom summary line with counts, duration, and failure message.
fn format_summary_line(
    output: &mut String,
    result: &SastResult,
    errors: &[&SastFinding],
    warnings: &[&SastFinding],
    infos: &[&SastFinding],
) {
    let total = result.findings.len();
    if total == 0 {
        return;
    }

    let files_with_issues = {
        let mut files: std::collections::HashSet<String> = std::collections::HashSet::new();
        for f in &result.findings {
            files.insert(f.file_path.to_string_lossy().to_string());
        }
        files.len()
    };
    let error_count = errors.len();

    let duration_str = if result.duration_ms >= 1000 {
        format!("{:.2}s", result.duration_ms as f64 / 1000.0)
    } else {
        format!("{}ms", result.duration_ms)
    };

    output.push_str(&format!(
        "  {} {} issue(s) ({} error{}, {} warning{}, {} info) in {} file(s)\n",
        if error_count > 0 { "✗".red().to_string() } else { "⚠".yellow().to_string() },
        total,
        error_count,
        if error_count == 1 { "" } else { "s" },
        warnings.len(),
        if warnings.len() == 1 { "" } else { "s" },
        infos.len(),
        files_with_issues,
    ));
    output.push_str(&format!("  Done in {}\n", duration_str.cyan()));

    if error_count > 0 {
        output.push_str(&format!(
            "\n  {}\n",
            "✗ Security scan failed due to errors. Fix the issues above.".red().bold()
        ));
    }
}

/// Persist the scan result as JSON to .linthis/result/.
fn save_result_to_disk(output: &mut String, result: &SastResult) {
    use chrono::Local;
    use std::fs::{self, File};
    use std::io::Write;

    let project_root = crate::utils::get_project_root();
    let result_dir = project_root.join(".linthis").join("result");
    if fs::create_dir_all(&result_dir).is_ok() {
        let timestamp = Local::now().format("%Y%m%d-%H%M%S");
        let result_file = result_dir.join(format!("security-{}.json", timestamp));
        if let Ok(json) = serde_json::to_string_pretty(result) {
            if let Ok(mut f) = File::create(&result_file) {
                let _ = writeln!(f, "{}", json);
                output.push_str(&format!(
                    "  {} Results saved to {}\n",
                    "✓".green(),
                    result_file.display()
                ));
            }
        }
    }
}

/// Append a warning about unavailable SAST tools, if any.
fn format_unavailable_tools(output: &mut String, result: &SastResult) {
    let unavailable: Vec<_> = result
        .scanner_status
        .iter()
        .filter(|(_, available)| !*available)
        .map(|(name, _)| name.as_str())
        .collect();

    if !unavailable.is_empty() {
        output.push_str(&format!(
            "\n  {} {} SAST tool(s) not available: {}\n",
            "⚠".yellow(),
            unavailable.len(),
            unavailable.join(", ")
        ));
    }
}

fn format_json(result: &SastResult) -> String {
    serde_json::to_string_pretty(result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

fn format_sarif(result: &SastResult) -> String {
    let sarif = SarifReport {
        version: "2.1.0".to_string(),
        schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json".to_string(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: "linthis-sast".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                },
            },
            results: result.findings.iter().map(|f| {
                SarifResult {
                    rule_id: f.rule_id.clone(),
                    level: match f.severity {
                        Severity::Critical | Severity::High => "error".to_string(),
                        Severity::Medium => "warning".to_string(),
                        _ => "note".to_string(),
                    },
                    message: SarifMessage { text: f.message.clone() },
                    locations: vec![SarifLocation {
                        physical_location: SarifPhysicalLocation {
                            artifact_location: SarifArtifactLocation {
                                uri: f.file_path.to_string_lossy().to_string(),
                            },
                            region: SarifRegion {
                                start_line: f.line,
                                start_column: f.column,
                                end_line: f.end_line,
                                end_column: f.end_column,
                            },
                        },
                    }],
                }
            }).collect(),
        }],
    };

    serde_json::to_string_pretty(&sarif).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

// SARIF output structures
#[derive(Serialize)]
struct SarifReport {
    #[serde(rename = "$schema")]
    schema: String,
    version: String,
    runs: Vec<SarifRun>,
}

#[derive(Serialize)]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Serialize)]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Serialize)]
struct SarifDriver {
    name: String,
    version: String,
}

#[derive(Serialize)]
struct SarifResult {
    #[serde(rename = "ruleId")]
    rule_id: String,
    level: String,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
}

#[derive(Serialize)]
struct SarifMessage {
    text: String,
}

#[derive(Serialize)]
struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    physical_location: SarifPhysicalLocation,
}

#[derive(Serialize)]
struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    artifact_location: SarifArtifactLocation,
    region: SarifRegion,
}

#[derive(Serialize)]
struct SarifArtifactLocation {
    uri: String,
}

#[derive(Serialize)]
struct SarifRegion {
    #[serde(rename = "startLine")]
    start_line: usize,
    #[serde(rename = "startColumn", skip_serializing_if = "Option::is_none")]
    start_column: Option<usize>,
    #[serde(rename = "endLine", skip_serializing_if = "Option::is_none")]
    end_line: Option<usize>,
    #[serde(rename = "endColumn", skip_serializing_if = "Option::is_none")]
    end_column: Option<usize>,
}
