// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Java license scanner using pom.xml parsing.

use std::path::Path;
use std::process::Command;

use crate::license::scanner::{LanguageLicenseScanner, PackageLicense};

/// Java license scanner
pub struct JavaLicenseScanner;

impl JavaLicenseScanner {
    pub fn new() -> Self {
        Self
    }

    fn scan_with_maven(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        // Try license-maven-plugin
        let output = Command::new("mvn")
            .args([
                "license:aggregate-third-party-report",
                "-DoutputDirectory=target/licenses",
                "-q",
            ])
            .current_dir(path)
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                return self.parse_license_report(path);
            }
        }

        // Fallback to dependency:tree
        self.scan_from_dependency_tree(path)
    }

    fn parse_license_report(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        let report_path = path.join("target/licenses/THIRD-PARTY.txt");
        if !report_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&report_path)
            .map_err(|e| format!("Failed to read license report: {}", e))?;

        let mut packages = Vec::new();

        for line in content.lines() {
            // Format: (License) groupId:artifactId:version
            if let Some(start) = line.find('(') {
                if let Some(end) = line.find(')') {
                    let license = &line[start + 1..end];
                    let artifact = line[end + 1..].trim();

                    let parts: Vec<&str> = artifact.split(':').collect();
                    if parts.len() >= 3 {
                        let name = format!("{}:{}", parts[0], parts[1]);
                        let version = parts[2];
                        packages.push(PackageLicense::new(&name, version, license, "maven"));
                    }
                }
            }
        }

        Ok(packages)
    }

    fn scan_from_dependency_tree(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        let output = Command::new("mvn")
            .args(["dependency:tree", "-DoutputType=text", "-q"])
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run mvn dependency:tree: {}", e))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            // Parse Maven dependency tree format
            let trimmed = line.trim_start_matches(|c| c == '+' || c == '|' || c == '\\' || c == '-' || c == ' ');
            let parts: Vec<&str> = trimmed.split(':').collect();

            if parts.len() >= 4 {
                let name = format!("{}:{}", parts[0], parts[1]);
                let version = parts[3];
                // Maven doesn't include license in dependency tree, would need to look it up
                packages.push(PackageLicense::new(&name, version, "", "maven"));
            }
        }

        Ok(packages)
    }

    fn scan_with_gradle(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        // Try gradle-license-report plugin
        let output = Command::new("gradle")
            .args(["generateLicenseReport", "--quiet"])
            .current_dir(path)
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                return self.parse_gradle_license_report(path);
            }
        }

        // Fallback to dependencies task
        self.scan_from_gradle_dependencies(path)
    }

    fn parse_gradle_license_report(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        let report_path = path.join("build/reports/dependency-license/index.json");
        if report_path.exists() {
            let content = std::fs::read_to_string(&report_path)
                .map_err(|e| format!("Failed to read Gradle license report: {}", e))?;

            if let Ok(report) = serde_json::from_str::<GradleLicenseReport>(&content) {
                return Ok(report
                    .dependencies
                    .into_iter()
                    .map(|d| {
                        PackageLicense::new(
                            &d.module_name,
                            &d.module_version,
                            &d.module_licenses
                                .first()
                                .map(|l| l.module_license.clone())
                                .unwrap_or_default(),
                            "maven",
                        )
                    })
                    .collect());
            }
        }

        Ok(Vec::new())
    }

    fn scan_from_gradle_dependencies(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        let output = Command::new("gradle")
            .args(["dependencies", "--configuration", "runtimeClasspath", "-q"])
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run gradle dependencies: {}", e))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            // Parse Gradle dependency format: group:name:version
            let trimmed = line.trim_start_matches(|c| c == '+' || c == '|' || c == '\\' || c == '-' || c == ' ');

            if trimmed.contains(':') && !trimmed.starts_with("project") {
                let parts: Vec<&str> = trimmed.split(':').collect();
                if parts.len() >= 3 {
                    // Remove (*) or (c) markers
                    let version = parts[2]
                        .split_whitespace()
                        .next()
                        .unwrap_or(parts[2]);

                    let name = format!("{}:{}", parts[0], parts[1]);
                    packages.push(PackageLicense::new(&name, version, "", "maven"));
                }
            }
        }

        Ok(packages)
    }
}

impl Default for JavaLicenseScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageLicenseScanner for JavaLicenseScanner {
    fn name(&self) -> &str {
        "java-license"
    }

    fn language(&self) -> &str {
        "java"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("pom.xml").exists()
            || path.join("build.gradle").exists()
            || path.join("build.gradle.kts").exists()
    }

    fn scan(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        if path.join("pom.xml").exists() {
            self.scan_with_maven(path)
        } else {
            self.scan_with_gradle(path)
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct GradleLicenseReport {
    dependencies: Vec<GradleDependency>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GradleDependency {
    module_name: String,
    module_version: String,
    #[serde(default)]
    module_licenses: Vec<GradleLicense>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GradleLicense {
    module_license: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_java_scanner_detect() {
        let scanner = JavaLicenseScanner::new();
        let temp_dir = tempfile::tempdir().unwrap();

        assert!(!scanner.detect(temp_dir.path()));

        std::fs::write(temp_dir.path().join("pom.xml"), "<project/>").unwrap();
        assert!(scanner.detect(temp_dir.path()));
    }
}
