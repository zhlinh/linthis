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

    // Header
    output.push_str(&format!(
        "\n{}\n",
        "🔍 SAST Source Code Security Scan Results".bold()
    ));

    // Scanner status
    for (name, available) in &result.scanner_status {
        let status = if *available {
            "✓".green().to_string()
        } else {
            "✗".red().to_string()
        };
        output.push_str(&format!("  {} {}\n", status, name));
    }
    output.push('\n');

    if result.findings.is_empty() {
        output.push_str(&format!("{}\n", "  ✅ No security issues found.".green()));
        return output;
    }

    // Group findings by severity
    let critical: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Critical)
        .collect();
    let high: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.severity == Severity::High)
        .collect();
    let medium: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Medium)
        .collect();
    let low: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Low)
        .collect();

    // Summary counts
    output.push_str(&format!(
        "  Found {} issue(s): {} critical, {} high, {} medium, {} low\n\n",
        result.findings.len(),
        critical.len(),
        high.len(),
        medium.len(),
        low.len(),
    ));

    // Print findings grouped by severity
    let all_groups: Vec<(&str, &Vec<&SastFinding>)> = vec![
        ("CRITICAL", &critical),
        ("HIGH", &high),
        ("MEDIUM", &medium),
        ("LOW", &low),
    ];

    for (label, findings) in all_groups {
        if findings.is_empty() {
            continue;
        }
        for finding in findings.iter() {
            let colored_label = match label {
                "CRITICAL" => format!("[{}]", label).red().bold().to_string(),
                "HIGH" => format!("[{}]", label).red().to_string(),
                "MEDIUM" => format!("[{}]", label).yellow().to_string(),
                _ => format!("[{}]", label).cyan().to_string(),
            };

            output.push_str(&format!("  {} {}\n", colored_label, finding.message));
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
    }

    // Unavailable tools warning
    let unavailable: Vec<_> = result
        .scanner_status
        .iter()
        .filter(|(_, available)| !*available)
        .map(|(name, _)| name.as_str())
        .collect();

    if !unavailable.is_empty() {
        output.push_str(&format!(
            "  {} {} SAST tool(s) not available: {}\n",
            "⚠".yellow(),
            unavailable.len(),
            unavailable.join(", ")
        ));
    }

    output
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
