// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Flawfinder SAST scanner for C/C++.
//!
//! Flawfinder scans C/C++ source code for potential security flaws using
//! lexical pattern matching. CWE-compatible, GPL-2.0 licensed.
//!
//! Uses `--csv` output for reliable parsing (JSON support is limited).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::security::sast::finding::SastFinding;
use crate::security::sast::scanner::{SastScanOptions, SastScanner};
use crate::security::vulnerability::Severity;

/// Flawfinder C/C++ SAST scanner.
pub struct FlawfinderScanner;

impl FlawfinderScanner {
    pub fn new() -> Self {
        Self
    }

    /// Parse CSV output from flawfinder.
    ///
    /// CSV columns (v2.0.19+):
    /// 0:File, 1:Line, 2:Column, 3:DefaultLevel, 4:Level, 5:Category,
    /// 6:Name, 7:Warning, 8:Suggestion, 9:Note, 10:CWEs, 11:Context,
    /// 12:Fingerprint, 13:ToolVersion, 14:RuleId, 15:HelpUri
    fn parse_csv_output(&self, output: &str) -> Result<Vec<SastFinding>, String> {
        let mut findings = Vec::new();

        for line in output.lines() {
            // Skip header line
            if line.starts_with("File,") || line.trim().is_empty() {
                continue;
            }

            // Parse CSV fields (handle quoted fields)
            let fields = parse_csv_line(line);
            if fields.len() < 8 {
                continue;
            }

            let file_path = fields[0].clone();
            let line_num = fields[1].parse::<usize>().unwrap_or(0);
            let column = fields[2].parse::<usize>().ok();
            // Index 4 is the effective Level (index 3 is DefaultLevel)
            let level = fields.get(4).and_then(|s| s.parse::<u32>().ok()).unwrap_or(
                fields[3].parse::<u32>().unwrap_or(0),
            );
            let category = fields.get(5).cloned().unwrap_or_default();
            let name = fields.get(6).cloned().unwrap_or_default();
            let warning = fields.get(7).cloned().unwrap_or_default();
            let suggestion = fields.get(8).filter(|s| !s.is_empty()).cloned();

            // Parse CWEs (field index 10)
            // Format examples: "CWE-120, CWE-20" or "CWE-119!/CWE-120"
            let cwe_ids = fields
                .get(10)
                .filter(|s| !s.is_empty())
                .map(|s| {
                    s.split(", ")
                        .flat_map(|part| part.split('!'))
                        .map(|s| s.trim_start_matches('/'))
                        .filter(|s| !s.is_empty() && s.starts_with("CWE-"))
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default();

            // Map flawfinder risk level (0-5) to Severity
            let severity = match level {
                5 => Severity::Critical,
                4 => Severity::High,
                3 => Severity::Medium,
                2 => Severity::Low,
                _ => Severity::Low,
            };

            // Code context (field index 11)
            let code_snippet = fields.get(11).filter(|s| !s.is_empty()).cloned();

            let rule_id = if name.is_empty() {
                format!("flawfinder-{}", category)
            } else {
                format!("{}/{}", category, name)
            };

            findings.push(SastFinding {
                rule_id,
                severity,
                message: warning,
                file_path: PathBuf::from(file_path),
                line: line_num,
                column,
                end_line: None,
                end_column: None,
                code_snippet,
                fix_suggestion: suggestion,
                category,
                cwe_ids,
                source: "flawfinder".to_string(),
                language: "c".to_string(), // covers both C and C++
            });
        }

        Ok(findings)
    }
}

impl Default for FlawfinderScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl SastScanner for FlawfinderScanner {
    fn name(&self) -> &str {
        "flawfinder"
    }

    fn supported_languages(&self) -> &[&str] {
        &["c", "cpp"]
    }

    fn is_available(&self) -> bool {
        Command::new("flawfinder")
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
        let mut args = vec![
            "--csv".to_string(),
            "--columns".to_string(),
            "--context".to_string(),
            "--quiet".to_string(),
        ];

        // Add minimum risk level filter
        if let Some(ref threshold) = options.severity_threshold {
            let min_level = match threshold {
                Severity::Critical => "5",
                Severity::High => "4",
                Severity::Medium => "3",
                Severity::Low => "2",
                _ => "1",
            };
            args.push(format!("--minlevel={}", min_level));
        }

        // Add files or scan directory
        if files.is_empty() {
            args.push(path.to_string_lossy().to_string());
        } else {
            for f in files {
                args.push(f.to_string_lossy().to_string());
            }
        }

        let output = Command::new("flawfinder")
            .args(&args)
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run flawfinder: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.trim().is_empty() {
            return Ok(Vec::new());
        }

        self.parse_csv_output(&stdout)
    }

    fn install_hint(&self) -> String {
        "Install: pip install flawfinder".to_string()
    }
}

/// Parse a CSV line handling quoted fields with commas inside.
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                fields.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(ch),
        }
    }
    fields.push(current.trim().to_string());
    fields
}
