// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Java security scanner using OWASP dependency-check.

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::security::scanner::{FixResult, LanguageSecurityScanner, ScanOptions};
use crate::security::vulnerability::{Advisory, AffectedPackage, Severity, Vulnerability};

/// Java security scanner using OWASP dependency-check
pub struct JavaSecurityScanner;

impl JavaSecurityScanner {
    pub fn new() -> Self {
        Self
    }

    fn check_tool(&self) -> bool {
        // Check for dependency-check CLI or Maven/Gradle plugin
        Command::new("dependency-check")
            .args(["--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
            || self.check_maven_plugin()
            || self.check_gradle_plugin()
    }

    fn check_maven_plugin(&self) -> bool {
        Command::new("mvn")
            .args(["--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn check_gradle_plugin(&self) -> bool {
        Command::new("gradle")
            .args(["--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn parse_dependency_check(&self, json_path: &Path) -> Result<Vec<Vulnerability>, String> {
        let content = std::fs::read_to_string(json_path)
            .map_err(|e| format!("Failed to read dependency-check report: {}", e))?;

        let report: DependencyCheckReport = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse dependency-check report: {}", e))?;

        let mut vulnerabilities = Vec::new();

        for dep in report.dependencies {
            for vuln in dep.vulnerabilities.unwrap_or_default() {
                let advisory = Advisory {
                    id: vuln.name.clone(),
                    aliases: Vec::new(),
                    title: vuln.description.lines().next().unwrap_or("").to_string(),
                    description: vuln.description.clone(),
                    severity: map_cvss_to_severity(vuln.cvss_v3.as_ref().or(vuln.cvss_v2.as_ref())),
                    cvss_score: vuln.cvss_v3.as_ref().or(vuln.cvss_v2.as_ref()).and_then(|c| c.base_score),
                    cvss_vector: vuln.cvss_v3.as_ref().or(vuln.cvss_v2.as_ref()).and_then(|c| c.vector.clone()),
                    url: vuln.references.first().map(|r| r.url.clone()),
                    published: None,
                    updated: None,
                    cwe_ids: vuln.cwes.clone().unwrap_or_default(),
                    references: vuln.references.iter().map(|r| r.url.clone()).collect(),
                };

                let affected = AffectedPackage {
                    name: dep.file_name.clone(),
                    version: dep.version.clone().unwrap_or_default(),
                    ecosystem: "maven".to_string(),
                    affected_versions: vec![],
                    patched_versions: vec![],
                    recommended_version: None,
                    path: vec![dep.file_path.clone()],
                    is_direct: true,
                };

                vulnerabilities.push(Vulnerability::new(
                    advisory,
                    vec![affected],
                    "dependency-check",
                    "java",
                ));
            }
        }

        Ok(vulnerabilities)
    }

    fn run_dependency_check(&self, path: &Path) -> Result<std::path::PathBuf, String> {
        let report_dir = path.join("target").join("dependency-check");
        std::fs::create_dir_all(&report_dir)
            .map_err(|e| format!("Failed to create report directory: {}", e))?;

        let report_path = report_dir.join("dependency-check-report.json");

        // Try dependency-check CLI first
        let cli_result = Command::new("dependency-check")
            .args([
                "--scan", path.to_str().unwrap_or("."),
                "--format", "JSON",
                "--out", report_dir.to_str().unwrap_or("."),
            ])
            .current_dir(path)
            .output();

        if let Ok(output) = cli_result {
            if output.status.success() && report_path.exists() {
                return Ok(report_path);
            }
        }

        // Try Maven plugin
        if path.join("pom.xml").exists() {
            let mvn_result = Command::new("mvn")
                .args([
                    "org.owasp:dependency-check-maven:check",
                    "-DformatsFormat=JSON",
                ])
                .current_dir(path)
                .output();

            if let Ok(output) = mvn_result {
                if output.status.success() {
                    let maven_report = path.join("target/dependency-check-report.json");
                    if maven_report.exists() {
                        return Ok(maven_report);
                    }
                }
            }
        }

        // Try Gradle plugin
        if path.join("build.gradle").exists() || path.join("build.gradle.kts").exists() {
            let gradle_result = Command::new("gradle")
                .args(["dependencyCheckAnalyze", "--info"])
                .current_dir(path)
                .output();

            if let Ok(output) = gradle_result {
                if output.status.success() {
                    let gradle_report = path.join("build/reports/dependency-check-report.json");
                    if gradle_report.exists() {
                        return Ok(gradle_report);
                    }
                }
            }
        }

        Err("Failed to run dependency-check. Install it via: brew install dependency-check".to_string())
    }
}

impl Default for JavaSecurityScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageSecurityScanner for JavaSecurityScanner {
    fn is_available(&self) -> bool {
        self.check_tool()
    }

    fn name(&self) -> &str {
        "dependency-check"
    }

    fn language(&self) -> &str {
        "java"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("pom.xml").exists()
            || path.join("build.gradle").exists()
            || path.join("build.gradle.kts").exists()
    }

    fn scan(&self, path: &Path, _options: &ScanOptions) -> Result<Vec<Vulnerability>, String> {
        let report_path = self.run_dependency_check(path)?;
        self.parse_dependency_check(&report_path)
    }

    fn fix(&self, _path: &Path, vulnerabilities: &[Vulnerability]) -> Result<FixResult, String> {
        let mut result = FixResult::default();

        result.needs_review = true;
        result.messages.push("Java dependency fixes require manual version updates".to_string());
        result.messages.push("Update versions in pom.xml or build.gradle".to_string());

        for vuln in vulnerabilities {
            result.unfixed.push(vuln.advisory.id.clone());
        }

        Ok(result)
    }
}

// dependency-check JSON output structures
#[derive(Debug, Deserialize)]
struct DependencyCheckReport {
    #[serde(default)]
    dependencies: Vec<DependencyEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DependencyEntry {
    file_name: String,
    file_path: String,
    version: Option<String>,
    vulnerabilities: Option<Vec<VulnerabilityEntry>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VulnerabilityEntry {
    name: String,
    description: String,
    cvss_v2: Option<CvssEntry>,
    cvss_v3: Option<CvssEntry>,
    cwes: Option<Vec<String>>,
    #[serde(default)]
    references: Vec<ReferenceEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CvssEntry {
    base_score: Option<f32>,
    vector: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReferenceEntry {
    url: String,
}

fn map_cvss_to_severity(cvss: Option<&CvssEntry>) -> Severity {
    match cvss.and_then(|c| c.base_score) {
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
    fn test_java_scanner_detect() {
        let scanner = JavaSecurityScanner::new();
        let temp_dir = tempfile::tempdir().unwrap();

        assert!(!scanner.detect(temp_dir.path()));

        std::fs::write(temp_dir.path().join("pom.xml"), "<project/>").unwrap();
        assert!(scanner.detect(temp_dir.path()));
    }
}
