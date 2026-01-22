// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Security report formatting and output.

use colored::Colorize;
use serde::Serialize;

use super::scanner::ScanResult;
use super::vulnerability::{Severity, Vulnerability};

/// Security report output format
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SecurityReportFormat {
    Human,
    Json,
    Sarif,
}

impl SecurityReportFormat {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => SecurityReportFormat::Json,
            "sarif" => SecurityReportFormat::Sarif,
            _ => SecurityReportFormat::Human,
        }
    }
}

/// Security report data structure
#[derive(Debug, Serialize)]
pub struct SecurityReport {
    /// Report version
    pub version: String,
    /// Scan timestamp
    pub timestamp: String,
    /// Scan result
    pub result: ScanResult,
    /// Summary statistics
    pub summary: ReportSummary,
}

/// Summary statistics for the report
#[derive(Debug, Serialize)]
pub struct ReportSummary {
    pub total_vulnerabilities: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub unknown: usize,
    pub fixable: usize,
    pub languages_scanned: usize,
}

impl SecurityReport {
    /// Create a report from scan result
    pub fn from_result(result: ScanResult) -> Self {
        let summary = ReportSummary {
            total_vulnerabilities: result.vulnerabilities.len(),
            critical: result.vulnerabilities.iter().filter(|v| v.severity() == Severity::Critical).count(),
            high: result.vulnerabilities.iter().filter(|v| v.severity() == Severity::High).count(),
            medium: result.vulnerabilities.iter().filter(|v| v.severity() == Severity::Medium).count(),
            low: result.vulnerabilities.iter().filter(|v| v.severity() == Severity::Low).count(),
            unknown: result.vulnerabilities.iter().filter(|v| v.severity() == Severity::Unknown).count(),
            fixable: result.vulnerabilities.iter().filter(|v| v.fix_available).count(),
            languages_scanned: result.languages_scanned.len(),
        };

        Self {
            version: "1.0".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            result,
            summary,
        }
    }
}

/// Format the security report for output
pub fn format_security_report(result: &ScanResult, format: SecurityReportFormat) -> String {
    match format {
        SecurityReportFormat::Human => format_human(result),
        SecurityReportFormat::Json => format_json(result),
        SecurityReportFormat::Sarif => format_sarif(result),
    }
}

fn format_human(result: &ScanResult) -> String {
    let mut output = String::new();

    // Header
    output.push_str(&format!("\n{}\n", "🔒 Security Scan Results".bold()));
    output.push_str(&format!("{}\n\n", "=".repeat(50)));

    // Scanner status
    output.push_str(&format!("{}\n", "Scanner Status:".bold()));
    for (name, available) in &result.scanner_status {
        let status = if *available {
            "✓".green()
        } else {
            "✗".red()
        };
        output.push_str(&format!("  {} {}\n", status, name));
    }
    output.push('\n');

    // Languages scanned
    if !result.languages_scanned.is_empty() {
        output.push_str(&format!(
            "Languages scanned: {}\n\n",
            result.languages_scanned.join(", ")
        ));
    }

    // Summary
    if result.vulnerabilities.is_empty() {
        output.push_str(&format!("{}\n", "✅ No vulnerabilities found!".green().bold()));
    } else {
        // Count by severity
        let critical = result.vulnerabilities.iter().filter(|v| v.severity() == Severity::Critical).count();
        let high = result.vulnerabilities.iter().filter(|v| v.severity() == Severity::High).count();
        let medium = result.vulnerabilities.iter().filter(|v| v.severity() == Severity::Medium).count();
        let low = result.vulnerabilities.iter().filter(|v| v.severity() == Severity::Low).count();

        output.push_str(&format!("{}\n", "Vulnerability Summary:".bold()));
        if critical > 0 {
            output.push_str(&format!("  {} CRITICAL: {}\n", "🔴".red(), critical));
        }
        if high > 0 {
            output.push_str(&format!("  {} HIGH: {}\n", "🟠", high));
        }
        if medium > 0 {
            output.push_str(&format!("  {} MEDIUM: {}\n", "🟡", medium));
        }
        if low > 0 {
            output.push_str(&format!("  {} LOW: {}\n", "🔵", low));
        }
        output.push('\n');

        // Group vulnerabilities by severity
        output.push_str(&format!("{}\n", "Vulnerabilities:".bold()));
        output.push_str(&format!("{}\n", "-".repeat(50)));

        // Sort by severity (critical first)
        let mut vulns: Vec<_> = result.vulnerabilities.iter().collect();
        vulns.sort_by(|a, b| b.severity().cmp(&a.severity()));

        for vuln in vulns {
            output.push_str(&format_vulnerability(vuln));
            output.push('\n');
        }
    }

    // Errors
    if !result.errors.is_empty() {
        output.push_str(&format!("\n{}\n", "Errors:".yellow().bold()));
        for error in &result.errors {
            output.push_str(&format!("  ⚠️  {}\n", error));
        }
    }

    // Duration
    output.push_str(&format!(
        "\nScan completed in {:.2}s\n",
        result.duration_ms as f64 / 1000.0
    ));

    output
}

fn format_vulnerability(vuln: &Vulnerability) -> String {
    let mut output = String::new();

    let severity_colored = match vuln.severity() {
        Severity::Critical => vuln.severity().to_string().red().bold().to_string(),
        Severity::High => vuln.severity().to_string().red().to_string(),
        Severity::Medium => vuln.severity().to_string().yellow().to_string(),
        Severity::Low => vuln.severity().to_string().cyan().to_string(),
        _ => vuln.severity().to_string(),
    };

    output.push_str(&format!(
        "{} {} [{}]\n",
        vuln.advisory.id.bold(),
        severity_colored,
        vuln.language
    ));

    output.push_str(&format!("  {}\n", vuln.advisory.title));

    for pkg in &vuln.affected_packages {
        output.push_str(&format!(
            "  📦 {} @ {}\n",
            pkg.name.cyan(),
            pkg.version
        ));

        if let Some(ref recommended) = pkg.recommended_version {
            output.push_str(&format!("     └─ Fix: Upgrade to {}\n", recommended.green()));
        } else if !pkg.patched_versions.is_empty() {
            output.push_str(&format!(
                "     └─ Fix: Upgrade to one of: {}\n",
                pkg.patched_versions.join(", ").green()
            ));
        } else {
            output.push_str(&format!("     └─ {}\n", "No fix available".yellow()));
        }
    }

    if let Some(ref url) = vuln.advisory.url {
        output.push_str(&format!("  🔗 {}\n", url.dimmed()));
    }

    output
}

fn format_json(result: &ScanResult) -> String {
    let report = SecurityReport::from_result(result.clone());
    serde_json::to_string_pretty(&report).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

fn format_sarif(result: &ScanResult) -> String {
    // SARIF (Static Analysis Results Interchange Format) output
    let sarif = SarifReport::from_result(result);
    serde_json::to_string_pretty(&sarif).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

/// SARIF report format for integration with security tools
#[derive(Debug, Serialize)]
struct SarifReport {
    #[serde(rename = "$schema")]
    schema: String,
    version: String,
    runs: Vec<SarifRun>,
}

#[derive(Debug, Serialize)]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Debug, Serialize)]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Debug, Serialize)]
struct SarifDriver {
    name: String,
    version: String,
    #[serde(rename = "informationUri")]
    information_uri: String,
    rules: Vec<SarifRule>,
}

#[derive(Debug, Serialize)]
struct SarifRule {
    id: String,
    name: String,
    #[serde(rename = "shortDescription")]
    short_description: SarifMessage,
    #[serde(rename = "fullDescription")]
    full_description: SarifMessage,
    #[serde(rename = "defaultConfiguration")]
    default_configuration: SarifConfiguration,
}

#[derive(Debug, Serialize)]
struct SarifConfiguration {
    level: String,
}

#[derive(Debug, Serialize)]
struct SarifMessage {
    text: String,
}

#[derive(Debug, Serialize)]
struct SarifResult {
    #[serde(rename = "ruleId")]
    rule_id: String,
    level: String,
    message: SarifMessage,
}

impl SarifReport {
    fn from_result(result: &ScanResult) -> Self {
        let mut rules = Vec::new();
        let mut results = Vec::new();

        for vuln in &result.vulnerabilities {
            rules.push(SarifRule {
                id: vuln.advisory.id.clone(),
                name: vuln.advisory.title.clone(),
                short_description: SarifMessage {
                    text: vuln.advisory.title.clone(),
                },
                full_description: SarifMessage {
                    text: vuln.advisory.description.clone(),
                },
                default_configuration: SarifConfiguration {
                    level: severity_to_sarif_level(vuln.severity()),
                },
            });

            results.push(SarifResult {
                rule_id: vuln.advisory.id.clone(),
                level: severity_to_sarif_level(vuln.severity()),
                message: SarifMessage {
                    text: format!(
                        "{} in {}",
                        vuln.advisory.title,
                        vuln.affected_packages
                            .iter()
                            .map(|p| format!("{} {}", p.name, p.version))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                },
            });
        }

        SarifReport {
            schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json".to_string(),
            version: "2.1.0".to_string(),
            runs: vec![SarifRun {
                tool: SarifTool {
                    driver: SarifDriver {
                        name: "linthis-security".to_string(),
                        version: env!("CARGO_PKG_VERSION").to_string(),
                        information_uri: "https://github.com/zhlinh/linthis".to_string(),
                        rules,
                    },
                },
                results,
            }],
        }
    }
}

fn severity_to_sarif_level(severity: Severity) -> String {
    match severity {
        Severity::Critical | Severity::High => "error".to_string(),
        Severity::Medium => "warning".to_string(),
        Severity::Low | Severity::None | Severity::Unknown => "note".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_format() {
        let result = ScanResult::new();
        let human_output = format_security_report(&result, SecurityReportFormat::Human);
        assert!(human_output.contains("Security Scan Results"));

        let json_output = format_security_report(&result, SecurityReportFormat::Json);
        assert!(json_output.contains("\"version\""));
    }
}
