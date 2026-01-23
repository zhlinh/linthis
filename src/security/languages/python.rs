// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Python security scanner using pip-audit.

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::security::scanner::{FixResult, LanguageSecurityScanner, ScanOptions};
use crate::security::vulnerability::{Advisory, AffectedPackage, Severity, Vulnerability};

/// Python security scanner using pip-audit
pub struct PythonSecurityScanner;

impl PythonSecurityScanner {
    pub fn new() -> Self {
        Self
    }

    fn check_tool(&self) -> bool {
        Command::new("pip-audit")
            .args(["--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn parse_pip_audit(&self, output: &str) -> Result<Vec<Vulnerability>, String> {
        let audit: PipAuditOutput = serde_json::from_str(output)
            .map_err(|e| format!("Failed to parse pip-audit output: {}", e))?;

        let mut vulnerabilities = Vec::new();

        for dep in audit.dependencies {
            for vuln in dep.vulns {
                let advisory = Advisory {
                    id: vuln.id.clone(),
                    aliases: vuln.aliases.clone(),
                    title: vuln.description.lines().next().unwrap_or("").to_string(),
                    description: vuln.description.clone(),
                    severity: Severity::Unknown, // pip-audit doesn't provide severity directly
                    cvss_score: None,
                    cvss_vector: None,
                    url: vuln.fix_versions.first().map(|_| {
                        format!("https://pypi.org/project/{}/", dep.name)
                    }),
                    published: None,
                    updated: None,
                    cwe_ids: Vec::new(),
                    references: Vec::new(),
                };

                let affected = AffectedPackage {
                    name: dep.name.clone(),
                    version: dep.version.clone(),
                    ecosystem: "pypi".to_string(),
                    affected_versions: vec![],
                    patched_versions: vuln.fix_versions.clone(),
                    recommended_version: vuln.fix_versions.first().cloned(),
                    path: vec![dep.name.clone()],
                    is_direct: true,
                };

                vulnerabilities.push(Vulnerability::new(
                    advisory,
                    vec![affected],
                    "pip-audit",
                    "python",
                ));
            }
        }

        Ok(vulnerabilities)
    }
}

impl Default for PythonSecurityScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageSecurityScanner for PythonSecurityScanner {
    fn is_available(&self) -> bool {
        self.check_tool()
    }

    fn name(&self) -> &str {
        "pip-audit"
    }

    fn language(&self) -> &str {
        "python"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("requirements.txt").exists()
            || path.join("pyproject.toml").exists()
            || path.join("setup.py").exists()
            || path.join("Pipfile").exists()
            || path.join("poetry.lock").exists()
    }

    fn scan(&self, path: &Path, _options: &ScanOptions) -> Result<Vec<Vulnerability>, String> {
        let mut args = vec!["--format", "json"];

        // Detect the dependency source
        if path.join("requirements.txt").exists() {
            args.extend(["-r", "requirements.txt"]);
        }

        let output = Command::new("pip-audit")
            .args(&args)
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run pip-audit: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.trim().is_empty() || stdout.trim() == "[]" {
            return Ok(Vec::new());
        }

        self.parse_pip_audit(&stdout)
    }

    fn fix(&self, _path: &Path, vulnerabilities: &[Vulnerability]) -> Result<FixResult, String> {
        let mut result = FixResult::default();

        result.commands.push("pip-audit --fix".to_string());
        result.messages.push("Run 'pip-audit --fix' to attempt automatic fixes".to_string());

        for vuln in vulnerabilities {
            if vuln.fix_available {
                result.fixed.push(vuln.advisory.id.clone());
            } else {
                result.unfixed.push(vuln.advisory.id.clone());
                result.needs_review = true;
            }
        }

        Ok(result)
    }
}

// pip-audit JSON output structures
#[derive(Debug, Deserialize)]
struct PipAuditOutput {
    dependencies: Vec<PipDependency>,
}

#[derive(Debug, Deserialize)]
struct PipDependency {
    name: String,
    version: String,
    #[serde(default)]
    vulns: Vec<PipVulnerability>,
}

#[derive(Debug, Deserialize)]
struct PipVulnerability {
    id: String,
    #[serde(default)]
    aliases: Vec<String>,
    description: String,
    #[serde(default)]
    fix_versions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_scanner_detect() {
        let scanner = PythonSecurityScanner::new();
        let temp_dir = tempfile::tempdir().unwrap();

        assert!(!scanner.detect(temp_dir.path()));

        std::fs::write(temp_dir.path().join("requirements.txt"), "flask==1.0").unwrap();
        assert!(scanner.detect(temp_dir.path()));
    }
}
