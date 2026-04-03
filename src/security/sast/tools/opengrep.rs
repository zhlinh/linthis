// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! OpenGrep/Semgrep SAST scanner integration.
//!
//! Prefers OpenGrep (fully open-source fork) over Semgrep CE.
//! Supports 30+ languages with YAML-based rules.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::security::sast::finding::SastFinding;
use crate::security::sast::scanner::{SastScanOptions, SastScanner};
use crate::security::vulnerability::Severity;

/// OpenGrep/Semgrep SAST scanner.
///
/// Tries `opengrep` first, falls back to `semgrep` if not found.
pub struct OpenGrepScanner {
    /// The actual binary name to use ("opengrep" or "semgrep")
    binary: Option<String>,
}

impl OpenGrepScanner {
    pub fn new() -> Self {
        let binary = if Self::check_binary("opengrep") {
            Some("opengrep".to_string())
        } else if Self::check_binary("semgrep") {
            Some("semgrep".to_string())
        } else {
            None
        };
        Self { binary }
    }

    fn check_binary(name: &str) -> bool {
        Command::new(name)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn binary_name(&self) -> &str {
        self.binary.as_deref().unwrap_or("opengrep")
    }

    fn parse_output(&self, output: &str) -> Result<Vec<SastFinding>, String> {
        let parsed: SemgrepOutput = serde_json::from_str(output)
            .map_err(|e| format!("Failed to parse {} output: {}", self.binary_name(), e))?;

        let findings = parsed
            .results
            .into_iter()
            .map(|r| {
                let severity = match r.extra.severity.to_lowercase().as_str() {
                    "error" => Severity::High,
                    "warning" => Severity::Medium,
                    "info" => Severity::Low,
                    _ => Severity::Unknown,
                };

                SastFinding {
                    rule_id: r.check_id,
                    severity,
                    message: r.extra.message,
                    file_path: PathBuf::from(r.path),
                    line: r.start.line,
                    column: Some(r.start.col),
                    end_line: Some(r.end.line),
                    end_column: Some(r.end.col),
                    code_snippet: if r.extra.lines.is_empty() {
                        None
                    } else {
                        Some(r.extra.lines)
                    },
                    fix_suggestion: r.extra.fix,
                    category: r
                        .extra
                        .metadata
                        .as_ref()
                        .and_then(|m| m.category.clone())
                        .unwrap_or_else(|| "security".to_string()),
                    cwe_ids: r
                        .extra
                        .metadata
                        .as_ref()
                        .map(|m| m.cwe.clone())
                        .unwrap_or_default(),
                    source: self.binary_name().to_string(),
                    language: r
                        .extra
                        .metadata
                        .as_ref()
                        .and_then(|m| m.technology.first().cloned())
                        .unwrap_or_default(),
                }
            })
            .collect();

        Ok(findings)
    }
}

impl Default for OpenGrepScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl SastScanner for OpenGrepScanner {
    fn name(&self) -> &str {
        self.binary.as_deref().unwrap_or("opengrep")
    }

    fn supported_languages(&self) -> &[&str] {
        &[
            "python",
            "javascript",
            "typescript",
            "go",
            "rust",
            "java",
            "kotlin",
            "c",
            "cpp",
            "csharp",
            "ruby",
            "php",
            "swift",
            "scala",
        ]
    }

    fn is_available(&self) -> bool {
        self.binary.is_some()
    }

    fn scan(
        &self,
        path: &Path,
        files: &[PathBuf],
        options: &SastScanOptions,
    ) -> Result<Vec<SastFinding>, String> {
        let bin = self.binary.as_deref().ok_or_else(|| {
            format!(
                "Neither opengrep nor semgrep is installed. {}",
                self.install_hint()
            )
        })?;

        let mut args = vec!["scan", "--json", "--quiet"];

        // Add config/rules if specified
        if let Some(ref config) = options.config_path {
            args.push("--config");
            args.push(config.to_str().unwrap_or("."));
        } else {
            // Use auto config (community rules)
            args.push("--config");
            args.push("auto");
        }

        // Add severity filter
        if let Some(ref threshold) = options.severity_threshold {
            let sev = match threshold {
                Severity::Critical | Severity::High => "ERROR",
                Severity::Medium => "WARNING",
                _ => "INFO",
            };
            args.push("--severity");
            args.push(sev);
        }

        // Add exclude rules
        for rule in &options.exclude {
            args.push("--exclude-rule");
            args.push(rule);
        }

        // Build the command
        let mut cmd = Command::new(bin);
        cmd.args(&args).current_dir(path);

        // Add specific files or scan whole project
        if !files.is_empty() {
            for f in files {
                cmd.arg(f);
            }
        }

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to run {}: {}", bin, e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.trim().is_empty() {
            return Ok(Vec::new());
        }

        self.parse_output(&stdout)
    }

    fn install_hint(&self) -> String {
        let hint = crate::python_tool_install_hint("opengrep");
        format!("{} (or semgrep)", hint)
    }
}

// Semgrep/OpenGrep JSON output structures
#[derive(Debug, Deserialize)]
struct SemgrepOutput {
    #[serde(default)]
    results: Vec<SemgrepResult>,
}

#[derive(Debug, Deserialize)]
struct SemgrepResult {
    check_id: String,
    path: String,
    start: SemgrepPosition,
    end: SemgrepPosition,
    extra: SemgrepExtra,
}

#[derive(Debug, Deserialize)]
struct SemgrepPosition {
    line: usize,
    col: usize,
}

#[derive(Debug, Deserialize)]
struct SemgrepExtra {
    #[serde(default)]
    message: String,
    #[serde(default)]
    severity: String,
    #[serde(default)]
    lines: String,
    #[serde(default)]
    fix: Option<String>,
    #[serde(default)]
    metadata: Option<SemgrepMetadata>,
}

#[derive(Debug, Deserialize)]
struct SemgrepMetadata {
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    cwe: Vec<String>,
    #[serde(default)]
    technology: Vec<String>,
}
