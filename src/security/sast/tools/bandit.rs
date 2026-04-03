// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Bandit SAST scanner for Python.
//!
//! Bandit is a tool designed to find common security issues in Python code.
//! Apache 2.0 licensed, 68+ built-in security checks.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::security::sast::finding::SastFinding;
use crate::security::sast::scanner::{SastScanOptions, SastScanner};
use crate::security::vulnerability::Severity;

/// Bandit Python SAST scanner.
pub struct BanditScanner;

impl BanditScanner {
    pub fn new() -> Self {
        Self
    }

    fn parse_output(&self, output: &str) -> Result<Vec<SastFinding>, String> {
        let parsed: BanditOutput = serde_json::from_str(output)
            .map_err(|e| format!("Failed to parse bandit output: {}", e))?;

        let findings = parsed
            .results
            .into_iter()
            .map(|r| {
                let severity = match r.issue_severity.to_uppercase().as_str() {
                    "HIGH" => Severity::High,
                    "MEDIUM" => Severity::Medium,
                    "LOW" => Severity::Low,
                    _ => Severity::Unknown,
                };

                SastFinding {
                    rule_id: r.test_id,
                    severity,
                    message: r.issue_text,
                    file_path: PathBuf::from(r.filename),
                    line: r.line_number,
                    column: Some(r.col_offset.unwrap_or(0)),
                    end_line: r
                        .end_col_offset
                        .map(|_| r.line_range.last().copied().unwrap_or(r.line_number)),
                    end_column: r.end_col_offset,
                    code_snippet: if r.code.is_empty() {
                        None
                    } else {
                        Some(r.code)
                    },
                    fix_suggestion: None,
                    category: r.test_name,
                    cwe_ids: r
                        .issue_cwe
                        .map(|c| vec![format!("CWE-{}", c.id)])
                        .unwrap_or_default(),
                    source: "bandit".to_string(),
                    language: "python".to_string(),
                }
            })
            .collect();

        Ok(findings)
    }
}

impl Default for BanditScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl SastScanner for BanditScanner {
    fn name(&self) -> &str {
        "bandit"
    }

    fn supported_languages(&self) -> &[&str] {
        &["python"]
    }

    fn is_available(&self) -> bool {
        Command::new("bandit")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn scan(
        &self,
        path: &Path,
        files: &[PathBuf],
        options: &SastScanOptions,
    ) -> Result<Vec<SastFinding>, String> {
        let mut args = vec!["-f".to_string(), "json".to_string()];

        // Add severity filter
        if let Some(ref threshold) = options.severity_threshold {
            let level = match threshold {
                Severity::Critical | Severity::High => "high",
                Severity::Medium => "medium",
                _ => "low",
            };
            args.push("-ll".to_string());
            args.push(level.to_string());
        }

        // Add config file
        if let Some(ref config) = options.config_path {
            args.push("-c".to_string());
            args.push(config.to_string_lossy().to_string());
        }

        // Add skip rules
        for rule in &options.exclude {
            args.push("-s".to_string());
            args.push(rule.clone());
        }

        // Add files or recursive scan
        if files.is_empty() {
            args.push("-r".to_string());
            args.push(path.to_string_lossy().to_string());
        } else {
            for f in files {
                args.push(f.to_string_lossy().to_string());
            }
        }

        let mut cmd = Command::new("bandit");
        cmd.args(&args);
        if path.is_dir() {
            cmd.current_dir(path);
        }
        let output = cmd
            .output()
            .map_err(|e| format!("Failed to run bandit: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.trim().is_empty() {
            return Ok(Vec::new());
        }

        self.parse_output(&stdout)
    }

    fn install_hint(&self) -> String {
        crate::python_tool_install_hint("bandit")
    }
}

// Bandit JSON output structures
#[derive(Debug, Deserialize)]
struct BanditOutput {
    #[serde(default)]
    results: Vec<BanditResult>,
}

#[derive(Debug, Deserialize)]
struct BanditResult {
    test_id: String,
    test_name: String,
    filename: String,
    line_number: usize,
    #[serde(default)]
    col_offset: Option<usize>,
    #[serde(default)]
    end_col_offset: Option<usize>,
    #[serde(default)]
    line_range: Vec<usize>,
    issue_text: String,
    issue_severity: String,
    #[serde(default)]
    issue_cwe: Option<BanditCwe>,
    #[serde(default)]
    code: String,
}

#[derive(Debug, Deserialize)]
struct BanditCwe {
    id: u32,
}
