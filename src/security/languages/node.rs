// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Node.js/npm security scanner using npm audit.

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::security::scanner::{FixResult, LanguageSecurityScanner, ScanOptions};
use crate::security::vulnerability::{Advisory, AffectedPackage, Severity, Vulnerability};

/// Node.js security scanner using npm audit
pub struct NodeSecurityScanner;

impl NodeSecurityScanner {
    pub fn new() -> Self {
        Self
    }

    fn check_tool(&self) -> bool {
        Command::new("npm")
            .args(["--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn parse_npm_audit(&self, output: &str) -> Result<Vec<Vulnerability>, String> {
        let audit: NpmAuditOutput = serde_json::from_str(output)
            .map_err(|e| format!("Failed to parse npm audit output: {}", e))?;

        let mut vulnerabilities = Vec::new();

        for (_, vuln) in audit.vulnerabilities {
            for via in &vuln.via {
                if let ViaEntry::Advisory(adv) = via {
                    let advisory = Advisory {
                        id: format!("GHSA-{}", adv.source),
                        aliases: vec![],
                        title: adv.title.clone(),
                        description: adv.title.clone(), // npm audit doesn't provide detailed description
                        severity: Severity::from_str(&adv.severity),
                        cvss_score: adv.cvss.as_ref().and_then(|c| c.score),
                        cvss_vector: adv.cvss.as_ref().and_then(|c| c.vector_string.clone()),
                        url: Some(adv.url.clone()),
                        published: None,
                        updated: None,
                        cwe_ids: adv.cwe.clone(),
                        references: vec![adv.url.clone()],
                    };

                    let affected = AffectedPackage {
                        name: vuln.name.clone(),
                        version: vuln.range.clone(),
                        ecosystem: "npm".to_string(),
                        affected_versions: vec![adv.range.clone()],
                        patched_versions: vuln.fix_available
                            .as_ref()
                            .map(|f| vec![f.version.clone()])
                            .unwrap_or_default(),
                        recommended_version: vuln.fix_available.as_ref().map(|f| f.version.clone()),
                        path: vuln.nodes.clone(),
                        is_direct: vuln.is_direct,
                    };

                    vulnerabilities.push(Vulnerability::new(
                        advisory,
                        vec![affected],
                        "npm-audit",
                        "javascript",
                    ));
                }
            }
        }

        Ok(vulnerabilities)
    }
}

impl Default for NodeSecurityScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageSecurityScanner for NodeSecurityScanner {
    fn is_available(&self) -> bool {
        self.check_tool()
    }

    fn name(&self) -> &str {
        "npm-audit"
    }

    fn language(&self) -> &str {
        "javascript"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("package.json").exists()
    }

    fn scan(&self, path: &Path, options: &ScanOptions) -> Result<Vec<Vulnerability>, String> {
        let mut args = vec!["audit", "--json"];

        if !options.include_dev {
            args.push("--omit=dev");
        }

        let output = Command::new("npm")
            .args(&args)
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run npm audit: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.trim().is_empty() {
            return Ok(Vec::new());
        }

        self.parse_npm_audit(&stdout)
    }

    fn fix(&self, path: &Path, vulnerabilities: &[Vulnerability]) -> Result<FixResult, String> {
        let mut result = FixResult::default();

        // npm audit fix
        let output = Command::new("npm")
            .args(["audit", "fix", "--dry-run", "--json"])
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run npm audit fix: {}", e))?;

        result.commands.push("npm audit fix".to_string());

        if output.status.success() {
            result.messages.push("Run 'npm audit fix' to apply fixes".to_string());
            for vuln in vulnerabilities {
                if vuln.fix_available {
                    result.fixed.push(vuln.advisory.id.clone());
                } else {
                    result.unfixed.push(vuln.advisory.id.clone());
                }
            }
        } else {
            result.needs_review = true;
            result.messages.push("Some fixes may require '--force' flag or manual intervention".to_string());
            result.commands.push("npm audit fix --force".to_string());
        }

        Ok(result)
    }
}

// npm audit JSON output structures
#[derive(Debug, Deserialize)]
struct NpmAuditOutput {
    #[serde(default)]
    vulnerabilities: std::collections::HashMap<String, NpmVulnerability>,
}

#[derive(Debug, Deserialize)]
struct NpmVulnerability {
    name: String,
    #[allow(dead_code)]
    severity: String,
    range: String,
    #[serde(default)]
    via: Vec<ViaEntry>,
    #[serde(default)]
    nodes: Vec<String>,
    #[serde(default, rename = "isDirect")]
    is_direct: bool,
    #[serde(rename = "fixAvailable")]
    fix_available: Option<FixAvailable>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ViaEntry {
    Advisory(NpmAdvisory),
    #[allow(dead_code)]
    PackageName(String),
}

#[derive(Debug, Deserialize)]
struct NpmAdvisory {
    source: u64,
    title: String,
    url: String,
    severity: String,
    range: String,
    #[serde(default)]
    cwe: Vec<String>,
    cvss: Option<NpmCvss>,
}

#[derive(Debug, Deserialize)]
struct NpmCvss {
    score: Option<f32>,
    #[serde(rename = "vectorString")]
    vector_string: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FixAvailable {
    #[allow(dead_code)]
    name: String,
    version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_scanner_detect() {
        let scanner = NodeSecurityScanner::new();
        let temp_dir = tempfile::tempdir().unwrap();

        assert!(!scanner.detect(temp_dir.path()));

        std::fs::write(temp_dir.path().join("package.json"), "{}").unwrap();
        assert!(scanner.detect(temp_dir.path()));
    }
}
