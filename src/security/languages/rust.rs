// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Rust security scanner using cargo-audit.

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::security::scanner::{FixResult, LanguageSecurityScanner, ScanOptions};
use crate::security::vulnerability::{Advisory, AffectedPackage, Severity, Vulnerability};

/// Rust security scanner using cargo-audit
pub struct RustSecurityScanner {
    #[allow(dead_code)]
    tool_available: Option<bool>,
}

impl RustSecurityScanner {
    pub fn new() -> Self {
        Self { tool_available: None }
    }

    fn check_tool(&self) -> bool {
        Command::new("cargo")
            .args(["audit", "--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn parse_audit_output(&self, output: &str) -> Result<Vec<Vulnerability>, String> {
        let audit_result: CargoAuditOutput = serde_json::from_str(output)
            .map_err(|e| format!("Failed to parse cargo-audit output: {}", e))?;

        let mut vulnerabilities = Vec::new();

        for vuln in audit_result.vulnerabilities.list {
            let advisory = Advisory {
                id: vuln.advisory.id.clone(),
                aliases: vuln.advisory.aliases.clone(),
                title: vuln.advisory.title.clone(),
                description: vuln.advisory.description.clone(),
                severity: map_rustsec_severity(&vuln.advisory.cvss),
                cvss_score: vuln.advisory.cvss.as_ref().and_then(|c| c.score()),
                cvss_vector: vuln.advisory.cvss.as_ref().map(|c| c.to_string()),
                url: vuln.advisory.url.clone(),
                published: vuln.advisory.date.clone(),
                updated: None,
                cwe_ids: Vec::new(),
                references: vuln.advisory.references.clone(),
            };

            let affected = AffectedPackage {
                name: vuln.package.name.clone(),
                version: vuln.package.version.clone(),
                ecosystem: "crates.io".to_string(),
                affected_versions: vuln.versions.affected.clone(),
                patched_versions: vuln.versions.patched.clone(),
                recommended_version: vuln.versions.patched.first().cloned(),
                path: vec![vuln.package.name.clone()],
                is_direct: true, // cargo-audit doesn't distinguish
            };

            vulnerabilities.push(Vulnerability::new(
                advisory,
                vec![affected],
                "cargo-audit",
                "rust",
            ));
        }

        Ok(vulnerabilities)
    }
}

impl Default for RustSecurityScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageSecurityScanner for RustSecurityScanner {
    fn is_available(&self) -> bool {
        self.check_tool()
    }

    fn name(&self) -> &str {
        "cargo-audit"
    }

    fn language(&self) -> &str {
        "rust"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("Cargo.toml").exists() || path.join("Cargo.lock").exists()
    }

    fn scan(&self, path: &Path, _options: &ScanOptions) -> Result<Vec<Vulnerability>, String> {
        let output = Command::new("cargo")
            .args(["audit", "--json"])
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run cargo-audit: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.trim().is_empty() {
            // No vulnerabilities or tool error
            if output.status.success() {
                return Ok(Vec::new());
            }
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("cargo-audit failed: {}", stderr));
        }

        self.parse_audit_output(&stdout)
    }

    fn fix(&self, path: &Path, vulnerabilities: &[Vulnerability]) -> Result<FixResult, String> {
        let mut result = FixResult::default();

        // cargo-audit can auto-fix with `cargo audit fix`
        let output = Command::new("cargo")
            .args(["audit", "fix", "--dry-run"])
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run cargo audit fix: {}", e))?;

        if output.status.success() {
            result.commands.push("cargo audit fix".to_string());
            result.messages.push("Run 'cargo audit fix' to apply fixes".to_string());

            for vuln in vulnerabilities {
                if vuln.fix_available {
                    result.fixed.push(vuln.advisory.id.clone());
                } else {
                    result.unfixed.push(vuln.advisory.id.clone());
                }
            }
        } else {
            result.needs_review = true;
            result.messages.push("Some vulnerabilities require manual review".to_string());
            for vuln in vulnerabilities {
                result.unfixed.push(vuln.advisory.id.clone());
            }
        }

        Ok(result)
    }
}

// cargo-audit JSON output structures
#[derive(Debug, Deserialize)]
struct CargoAuditOutput {
    vulnerabilities: VulnerabilitiesSection,
}

#[derive(Debug, Deserialize)]
struct VulnerabilitiesSection {
    list: Vec<CargoVulnerability>,
    #[allow(dead_code)]
    count: usize,
}

#[derive(Debug, Deserialize)]
struct CargoVulnerability {
    advisory: CargoAdvisory,
    package: CargoPackage,
    versions: CargoVersions,
}

#[derive(Debug, Deserialize)]
struct CargoAdvisory {
    id: String,
    title: String,
    description: String,
    date: Option<String>,
    url: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    references: Vec<String>,
    cvss: Option<CvssInfo>,
}

#[derive(Debug, Deserialize)]
struct CvssInfo {
    vector: Option<String>,
    score: Option<f32>,
}

impl CvssInfo {
    fn score(&self) -> Option<f32> {
        self.score
    }
}

impl std::fmt::Display for CvssInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref vector) = self.vector {
            write!(f, "{}", vector)
        } else {
            write!(f, "N/A")
        }
    }
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct CargoVersions {
    #[serde(default)]
    affected: Vec<String>,
    #[serde(default)]
    patched: Vec<String>,
}

fn map_rustsec_severity(cvss: &Option<CvssInfo>) -> Severity {
    match cvss.as_ref().and_then(|c| c.score) {
        Some(score) if score >= 9.0 => Severity::Critical,
        Some(score) if score >= 7.0 => Severity::High,
        Some(score) if score >= 4.0 => Severity::Medium,
        Some(score) if score > 0.0 => Severity::Low,
        _ => Severity::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_scanner_detect() {
        let scanner = RustSecurityScanner::new();
        let temp_dir = tempfile::tempdir().unwrap();

        // Should not detect without Cargo.toml
        assert!(!scanner.detect(temp_dir.path()));

        // Should detect with Cargo.toml
        std::fs::write(temp_dir.path().join("Cargo.toml"), "[package]").unwrap();
        assert!(scanner.detect(temp_dir.path()));
    }
}
