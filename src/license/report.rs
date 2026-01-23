// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! License report formatting and output.

use colored::Colorize;
use serde::Serialize;

use super::policy::{PolicyViolation, ViolationType};
use super::scanner::ScanResult;
use super::spdx::SpdxLicense;

/// License report output format
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LicenseReportFormat {
    Human,
    Json,
    Spdx,
}

impl LicenseReportFormat {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => LicenseReportFormat::Json,
            "spdx" => LicenseReportFormat::Spdx,
            _ => LicenseReportFormat::Human,
        }
    }
}

/// License report data structure
#[derive(Debug, Serialize)]
pub struct LicenseReport {
    pub version: String,
    pub timestamp: String,
    pub packages: Vec<PackageInfo>,
    pub summary: ReportSummary,
    pub violations: Vec<PolicyViolation>,
}

#[derive(Debug, Serialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub license: String,
    pub ecosystem: String,
}

#[derive(Debug, Serialize)]
pub struct ReportSummary {
    pub total_packages: usize,
    pub permissive: usize,
    pub copyleft: usize,
    pub weak_copyleft: usize,
    pub unknown: usize,
    pub violations: usize,
    pub warnings: usize,
}

impl LicenseReport {
    pub fn from_result(result: &ScanResult, violations: &[PolicyViolation]) -> Self {
        let packages: Vec<PackageInfo> = result
            .packages
            .iter()
            .map(|p| PackageInfo {
                name: p.name.clone(),
                version: p.version.clone(),
                license: p.license.to_spdx().to_string(),
                ecosystem: p.ecosystem.clone(),
            })
            .collect();

        let warnings = violations
            .iter()
            .filter(|v| v.violation_type == ViolationType::Warning)
            .count();

        let summary = ReportSummary {
            total_packages: result.packages.len(),
            permissive: result.permissive_count(),
            copyleft: result.copyleft_count(),
            weak_copyleft: result.weak_copyleft_count(),
            unknown: result.unknown_count(),
            violations: violations.len() - warnings,
            warnings,
        };

        Self {
            version: "1.0".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            packages,
            summary,
            violations: violations.to_vec(),
        }
    }
}

/// Format the license report for output
pub fn format_license_report(
    result: &ScanResult,
    violations: &[PolicyViolation],
    format: LicenseReportFormat,
) -> String {
    match format {
        LicenseReportFormat::Human => format_human(result, violations),
        LicenseReportFormat::Json => format_json(result, violations),
        LicenseReportFormat::Spdx => format_spdx(result),
    }
}

fn format_human(result: &ScanResult, violations: &[PolicyViolation]) -> String {
    let mut output = String::new();

    // Header
    output.push_str(&format!("\n{}\n", "📜 License Compliance Report".bold()));
    output.push_str(&format!("{}\n\n", "=".repeat(50)));

    // Summary
    output.push_str(&format!("{}\n", "Summary:".bold()));
    output.push_str(&format!("  Total packages: {}\n", result.packages.len()));
    output.push_str(&format!("  ✅ Permissive: {}\n", result.permissive_count()));
    output.push_str(&format!("  ⚠️  Weak copyleft: {}\n", result.weak_copyleft_count()));
    output.push_str(&format!("  🔴 Copyleft: {}\n", result.copyleft_count()));
    output.push_str(&format!("  ❓ Unknown: {}\n", result.unknown_count()));
    output.push('\n');

    // Group by license
    output.push_str(&format!("{}\n", "Licenses:".bold()));
    let mut licenses: Vec<_> = result.by_license.iter().collect();
    licenses.sort_by(|a, b| b.1.cmp(a.1));

    for (license, count) in licenses {
        let spdx = SpdxLicense::from_str(license);
        let icon = if spdx.is_permissive() {
            "✅"
        } else if spdx.is_copyleft() {
            "🔴"
        } else if spdx.is_weak_copyleft() {
            "⚠️ "
        } else {
            "❓"
        };
        output.push_str(&format!("  {} {}: {}\n", icon, license, count));
    }
    output.push('\n');

    // Violations
    if !violations.is_empty() {
        let errors: Vec<_> = violations
            .iter()
            .filter(|v| v.violation_type != ViolationType::Warning)
            .collect();
        let warnings: Vec<_> = violations
            .iter()
            .filter(|v| v.violation_type == ViolationType::Warning)
            .collect();

        if !errors.is_empty() {
            output.push_str(&format!("{}\n", "❌ Policy Violations:".red().bold()));
            output.push_str(&format!("{}\n", "-".repeat(50)));

            for violation in errors {
                output.push_str(&format!(
                    "  {} {} @ {}\n",
                    violation.license.red(),
                    violation.package.cyan(),
                    violation.version
                ));
                output.push_str(&format!("    └─ {}\n", violation.reason));
                if let Some(ref suggestion) = violation.suggestion {
                    output.push_str(&format!("    └─ {}\n", suggestion.dimmed()));
                }
            }
            output.push('\n');
        }

        if !warnings.is_empty() {
            output.push_str(&format!("{}\n", "⚠️  Warnings:".yellow().bold()));
            output.push_str(&format!("{}\n", "-".repeat(50)));

            for violation in warnings {
                output.push_str(&format!(
                    "  {} {} @ {}\n",
                    violation.license.yellow(),
                    violation.package.cyan(),
                    violation.version
                ));
                output.push_str(&format!("    └─ {}\n", violation.reason));
            }
            output.push('\n');
        }
    } else {
        output.push_str(&format!("{}\n", "✅ No policy violations!".green().bold()));
    }

    // Duration
    output.push_str(&format!(
        "\nScan completed in {:.2}s\n",
        result.duration_ms as f64 / 1000.0
    ));

    output
}

fn format_json(result: &ScanResult, violations: &[PolicyViolation]) -> String {
    let report = LicenseReport::from_result(result, violations);
    serde_json::to_string_pretty(&report).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

fn format_spdx(result: &ScanResult) -> String {
    // SPDX SBOM format
    let mut output = String::new();

    output.push_str("SPDXVersion: SPDX-2.3\n");
    output.push_str("DataLicense: CC0-1.0\n");
    output.push_str(&format!("SPDXID: SPDXRef-DOCUMENT\n"));
    output.push_str(&format!("DocumentName: linthis-license-scan\n"));
    output.push_str(&format!(
        "DocumentNamespace: https://linthis.io/spdx/{}\n",
        chrono::Utc::now().format("%Y%m%d%H%M%S")
    ));
    output.push_str(&format!(
        "Creator: Tool: linthis-{}\n",
        env!("CARGO_PKG_VERSION")
    ));
    output.push_str(&format!(
        "Created: {}\n",
        chrono::Utc::now().to_rfc3339()
    ));
    output.push('\n');

    for (i, pkg) in result.packages.iter().enumerate() {
        output.push_str(&format!("##### Package: {} #####\n", pkg.name));
        output.push_str(&format!("PackageName: {}\n", pkg.name));
        output.push_str(&format!("SPDXID: SPDXRef-Package-{}\n", i + 1));
        output.push_str(&format!("PackageVersion: {}\n", pkg.version));
        output.push_str(&format!(
            "PackageDownloadLocation: NOASSERTION\n"
        ));
        output.push_str(&format!("FilesAnalyzed: false\n"));
        output.push_str(&format!(
            "PackageLicenseConcluded: {}\n",
            pkg.license.to_spdx()
        ));
        output.push_str(&format!(
            "PackageLicenseDeclared: {}\n",
            pkg.license_text
        ));
        output.push_str(&format!("PackageCopyrightText: NOASSERTION\n"));
        output.push('\n');
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_format() {
        let result = ScanResult::new();
        let violations = vec![];
        let human_output = format_license_report(&result, &violations, LicenseReportFormat::Human);
        assert!(human_output.contains("License Compliance Report"));

        let json_output = format_license_report(&result, &violations, LicenseReportFormat::Json);
        assert!(json_output.contains("\"version\""));
    }

    #[test]
    fn test_report_format_from_str() {
        assert_eq!(
            LicenseReportFormat::from_str("json"),
            LicenseReportFormat::Json
        );
        assert_eq!(
            LicenseReportFormat::from_str("spdx"),
            LicenseReportFormat::Spdx
        );
        assert_eq!(
            LicenseReportFormat::from_str("human"),
            LicenseReportFormat::Human
        );
    }
}
