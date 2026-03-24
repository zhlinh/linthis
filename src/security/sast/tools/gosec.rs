// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Gosec SAST scanner for Go.
//!
//! Gosec inspects Go source code for security problems by scanning the AST.
//! Apache 2.0 licensed, 50+ rules with CWE mapping and taint analysis.
//!
//! Note: Gosec scans by package directory, not individual files.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::security::sast::finding::SastFinding;
use crate::security::sast::scanner::{SastScanOptions, SastScanner};
use crate::security::vulnerability::Severity;

/// Gosec Go SAST scanner.
pub struct GosecScanner;

impl GosecScanner {
    pub fn new() -> Self {
        Self
    }

    fn parse_output(&self, output: &str) -> Result<Vec<SastFinding>, String> {
        let parsed: GosecOutput = serde_json::from_str(output)
            .map_err(|e| format!("Failed to parse gosec output: {}", e))?;

        let findings = parsed
            .issues
            .into_iter()
            .map(|issue| {
                let severity = match issue.severity.to_uppercase().as_str() {
                    "HIGH" => Severity::High,
                    "MEDIUM" => Severity::Medium,
                    "LOW" => Severity::Low,
                    _ => Severity::Unknown,
                };

                let line = issue.line.parse::<usize>().unwrap_or(0);
                let column = issue.column.parse::<usize>().ok();

                SastFinding {
                    rule_id: issue.rule_id,
                    severity,
                    message: issue.details,
                    file_path: PathBuf::from(&issue.file),
                    line,
                    column,
                    end_line: None,
                    end_column: None,
                    code_snippet: if issue.code.is_empty() {
                        None
                    } else {
                        Some(issue.code)
                    },
                    fix_suggestion: None,
                    category: issue
                        .cwe
                        .as_ref()
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| "security".to_string()),
                    cwe_ids: issue
                        .cwe
                        .map(|c| vec![format!("CWE-{}", c.id)])
                        .unwrap_or_default(),
                    source: "gosec".to_string(),
                    language: "go".to_string(),
                }
            })
            .collect();

        Ok(findings)
    }
}

impl Default for GosecScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl SastScanner for GosecScanner {
    fn name(&self) -> &str {
        "gosec"
    }

    fn supported_languages(&self) -> &[&str] {
        &["go"]
    }

    fn is_available(&self) -> bool {
        Command::new("gosec")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn scan(
        &self,
        path: &Path,
        _files: &[PathBuf],
        options: &SastScanOptions,
    ) -> Result<Vec<SastFinding>, String> {
        // Gosec scans by package directory, not individual files
        let mut args = vec!["-fmt=json".to_string(), "-quiet".to_string()];

        // Add config
        if let Some(ref config) = options.config_path {
            args.push(format!("-conf={}", config.display()));
        }

        // Add severity filter
        if let Some(ref threshold) = options.severity_threshold {
            let sev = match threshold {
                Severity::Critical | Severity::High => "high",
                Severity::Medium => "medium",
                _ => "low",
            };
            args.push(format!("-severity={}", sev));
        }

        // Add exclude rules
        if !options.exclude.is_empty() {
            args.push(format!("-exclude={}", options.exclude.join(",")));
        }

        // Scan all Go packages in the directory
        args.push("./...".to_string());

        let output = Command::new("gosec")
            .args(&args)
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run gosec: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.trim().is_empty() {
            return Ok(Vec::new());
        }

        self.parse_output(&stdout)
    }

    fn install_hint(&self) -> String {
        "Install: go install github.com/securego/gosec/v2/cmd/gosec@latest".to_string()
    }
}

// Gosec JSON output structures
#[derive(Debug, Deserialize)]
struct GosecOutput {
    #[serde(default, rename = "Issues")]
    issues: Vec<GosecIssue>,
}

#[derive(Debug, Deserialize)]
struct GosecIssue {
    rule_id: String,
    details: String,
    file: String,
    #[serde(default)]
    line: String,
    #[serde(default)]
    column: String,
    severity: String,
    #[serde(default)]
    code: String,
    #[serde(default)]
    cwe: Option<GosecCwe>,
}

#[derive(Debug, Deserialize)]
struct GosecCwe {
    #[serde(default, rename = "ID")]
    id: String,
    #[serde(default, rename = "Name")]
    name: String,
}
