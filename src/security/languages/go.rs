// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Go security scanner using govulncheck.

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::security::scanner::{FixResult, LanguageSecurityScanner, ScanOptions};
use crate::security::vulnerability::{Advisory, AffectedPackage, Severity, Vulnerability};

/// Go security scanner using govulncheck
pub struct GoSecurityScanner;

impl GoSecurityScanner {
    pub fn new() -> Self {
        Self
    }

    fn check_tool(&self) -> bool {
        Command::new("govulncheck")
            .args(["-version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn parse_govulncheck(&self, output: &str) -> Result<Vec<Vulnerability>, String> {
        let mut vulnerabilities = Vec::new();

        // govulncheck outputs JSON lines
        for line in output.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(entry) = serde_json::from_str::<GovulncheckEntry>(line) {
                if let Some(finding) = entry.finding {
                    if let Some(osv) = entry.osv {
                        let advisory = Advisory {
                            id: osv.id.clone(),
                            aliases: osv.aliases.clone(),
                            title: osv.summary.clone(),
                            description: osv.details.clone(),
                            severity: map_osv_severity(&osv.severity),
                            cvss_score: None,
                            cvss_vector: None,
                            url: osv.references.first().map(|r| r.url.clone()),
                            published: osv.published.clone(),
                            updated: osv.modified.clone(),
                            cwe_ids: osv.database_specific
                                .as_ref()
                                .and_then(|d| d.cwe_ids.clone())
                                .unwrap_or_default(),
                            references: osv.references.iter().map(|r| r.url.clone()).collect(),
                        };

                        let affected_packages: Vec<AffectedPackage> = osv.affected
                            .iter()
                            .map(|a| {
                                let fixed = a.ranges.iter()
                                    .flat_map(|r| r.events.iter())
                                    .filter_map(|e| e.fixed.clone())
                                    .collect::<Vec<_>>();

                                AffectedPackage {
                                    name: a.package.name.clone(),
                                    version: finding.trace.first()
                                        .map(|t| t.version.clone().unwrap_or_default())
                                        .unwrap_or_default(),
                                    ecosystem: a.package.ecosystem.clone(),
                                    affected_versions: vec![],
                                    patched_versions: fixed.clone(),
                                    recommended_version: fixed.first().cloned(),
                                    path: finding.trace.iter()
                                        .filter_map(|t| t.module.clone())
                                        .collect(),
                                    is_direct: finding.trace.len() <= 2,
                                }
                            })
                            .collect();

                        if !affected_packages.is_empty() {
                            vulnerabilities.push(Vulnerability::new(
                                advisory,
                                affected_packages,
                                "govulncheck",
                                "go",
                            ));
                        }
                    }
                }
            }
        }

        Ok(vulnerabilities)
    }
}

impl Default for GoSecurityScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageSecurityScanner for GoSecurityScanner {
    fn is_available(&self) -> bool {
        self.check_tool()
    }

    fn name(&self) -> &str {
        "govulncheck"
    }

    fn language(&self) -> &str {
        "go"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("go.mod").exists()
    }

    fn scan(&self, path: &Path, _options: &ScanOptions) -> Result<Vec<Vulnerability>, String> {
        let output = Command::new("govulncheck")
            .args(["-json", "./..."])
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run govulncheck: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.trim().is_empty() {
            return Ok(Vec::new());
        }

        self.parse_govulncheck(&stdout)
    }

    fn fix(&self, _path: &Path, vulnerabilities: &[Vulnerability]) -> Result<FixResult, String> {
        let mut result = FixResult::default();

        result.commands.push("go get -u ./...".to_string());
        result.messages.push("Run 'go get -u ./...' to update dependencies".to_string());
        result.needs_review = true;

        for vuln in vulnerabilities {
            if vuln.fix_available {
                // Suggest specific upgrade commands
                for pkg in &vuln.affected_packages {
                    if let Some(ref version) = pkg.recommended_version {
                        result.commands.push(format!("go get {}@{}", pkg.name, version));
                    }
                }
                result.fixed.push(vuln.advisory.id.clone());
            } else {
                result.unfixed.push(vuln.advisory.id.clone());
            }
        }

        Ok(result)
    }
}

// govulncheck JSON output structures
#[derive(Debug, Deserialize)]
struct GovulncheckEntry {
    finding: Option<Finding>,
    osv: Option<OsvEntry>,
}

#[derive(Debug, Deserialize)]
struct Finding {
    #[serde(default)]
    trace: Vec<TraceEntry>,
}

#[derive(Debug, Deserialize)]
struct TraceEntry {
    module: Option<String>,
    version: Option<String>,
    #[allow(dead_code)]
    package: Option<String>,
    #[allow(dead_code)]
    function: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OsvEntry {
    id: String,
    #[serde(default)]
    aliases: Vec<String>,
    summary: String,
    details: String,
    #[serde(default)]
    severity: Vec<OsvSeverity>,
    #[serde(default)]
    affected: Vec<OsvAffected>,
    #[serde(default)]
    references: Vec<OsvReference>,
    published: Option<String>,
    modified: Option<String>,
    database_specific: Option<DatabaseSpecific>,
}

#[derive(Debug, Deserialize)]
struct OsvSeverity {
    #[serde(rename = "type")]
    severity_type: String,
    score: String,
}

#[derive(Debug, Deserialize)]
struct OsvAffected {
    package: OsvPackage,
    #[serde(default)]
    ranges: Vec<OsvRange>,
}

#[derive(Debug, Deserialize)]
struct OsvPackage {
    name: String,
    ecosystem: String,
}

#[derive(Debug, Deserialize)]
struct OsvRange {
    #[serde(default)]
    events: Vec<OsvEvent>,
}

#[derive(Debug, Deserialize)]
struct OsvEvent {
    #[allow(dead_code)]
    introduced: Option<String>,
    fixed: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OsvReference {
    url: String,
}

#[derive(Debug, Deserialize)]
struct DatabaseSpecific {
    cwe_ids: Option<Vec<String>>,
}

fn map_osv_severity(severity: &[OsvSeverity]) -> Severity {
    for s in severity {
        if s.severity_type == "CVSS_V3" {
            if let Ok(score) = s.score.parse::<f32>() {
                return match score {
                    s if s >= 9.0 => Severity::Critical,
                    s if s >= 7.0 => Severity::High,
                    s if s >= 4.0 => Severity::Medium,
                    s if s > 0.0 => Severity::Low,
                    _ => Severity::Unknown,
                };
            }
        }
    }
    Severity::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_scanner_detect() {
        let scanner = GoSecurityScanner::new();
        let temp_dir = tempfile::tempdir().unwrap();

        assert!(!scanner.detect(temp_dir.path()));

        std::fs::write(temp_dir.path().join("go.mod"), "module test").unwrap();
        assert!(scanner.detect(temp_dir.path()));
    }
}
