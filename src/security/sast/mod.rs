// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! SAST (Static Application Security Testing) module.
//!
//! Provides source code security analysis by integrating with multiple SAST tools:
//!
//! - **OpenGrep/Semgrep**: Multi-language (30+ languages), YAML-based rules
//! - **Bandit**: Python-specific, 68+ security checks
//! - **Gosec**: Go-specific, 50+ rules with CWE mapping
//! - **Flawfinder**: C/C++ lexical security scanning
//!
//! Tools are detected at runtime. Available tools run in parallel,
//! unavailable tools are skipped with a warning.

pub mod finding;
pub mod report;
pub mod scanner;
pub mod tools;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

pub use finding::SastFinding;
pub use report::format_sast_report;
pub use scanner::{SastScanOptions, SastScanner};
pub use tools::{BanditScanner, FlawfinderScanner, GosecScanner, OpenGrepScanner, SecretsScanner};

use crate::security::vulnerability::Severity;

/// Aggregated SAST scan result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SastResult {
    /// All findings from all scanners
    pub findings: Vec<SastFinding>,
    /// Findings grouped by severity
    pub by_severity: HashMap<String, usize>,
    /// Findings grouped by tool
    pub by_tool: HashMap<String, usize>,
    /// Scanner availability status (name -> available)
    pub scanner_status: Vec<(String, bool)>,
    /// Scan duration in milliseconds
    pub duration_ms: u64,
    /// Any errors that occurred
    pub errors: Vec<String>,
}

impl SastResult {
    /// Get count of critical + high findings
    pub fn critical_high_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| matches!(f.severity, Severity::Critical | Severity::High))
            .count()
    }

    /// Check if any findings meet the severity threshold
    pub fn has_findings_above(&self, threshold: Severity) -> bool {
        self.findings
            .iter()
            .any(|f| f.severity.meets_threshold(&threshold))
    }
}

/// SAST aggregator that manages and dispatches to all SAST scanners.
pub struct SastAggregator {
    scanners: Vec<Box<dyn SastScanner>>,
}

impl Default for SastAggregator {
    fn default() -> Self {
        Self::new()
    }
}

impl SastAggregator {
    /// Create a new aggregator with all registered SAST scanners.
    pub fn new() -> Self {
        Self::with_config(None)
    }

    /// Create aggregator with optional config path for secrets scanner.
    pub fn with_config(config_path: Option<&Path>) -> Self {
        let scanners: Vec<Box<dyn SastScanner>> = vec![
            Box::new(SecretsScanner::with_config(config_path)),
            Box::new(OpenGrepScanner::new()),
            Box::new(BanditScanner::new()),
            Box::new(GosecScanner::new()),
            Box::new(FlawfinderScanner::new()),
        ];
        Self { scanners }
    }

    /// Get scanner availability information.
    pub fn available_scanners(&self) -> Vec<(&str, bool, &[&str])> {
        self.scanners
            .iter()
            .map(|s| (s.name(), s.is_available(), s.supported_languages()))
            .collect()
    }

    /// Run SAST scan across all available scanners.
    pub fn scan(
        &self,
        path: &Path,
        files: &[PathBuf],
        options: &SastScanOptions,
    ) -> SastResult {
        let start = Instant::now();
        let mut all_findings = Vec::new();
        let mut scanner_status = Vec::new();
        let mut errors = Vec::new();

        // If path is a file, use its parent as the scan directory
        // and pass the file as a specific target
        let (scan_dir, scan_files) = if path.is_file() {
            let parent = path.parent().unwrap_or(Path::new("."));
            let file_list = vec![path.to_path_buf()];
            (parent.to_path_buf(), file_list)
        } else {
            (path.to_path_buf(), files.to_vec())
        };

        for scanner in &self.scanners {
            let available = scanner.is_available();
            scanner_status.push((scanner.name().to_string(), available));

            if !available {
                continue;
            }

            match scanner.scan(&scan_dir, &scan_files, options) {
                Ok(mut findings) => {
                    // Apply severity filter if set
                    if let Some(ref threshold) = options.severity_threshold {
                        findings.retain(|f| f.meets_severity_threshold(threshold));
                    }
                    all_findings.append(&mut findings);
                }
                Err(e) => {
                    errors.push(format!("{}: {}", scanner.name(), e));
                }
            }
        }

        // Sort findings: critical first, then by file/line
        all_findings.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then_with(|| a.file_path.cmp(&b.file_path))
                .then_with(|| a.line.cmp(&b.line))
        });

        // Build severity counts
        let mut by_severity = HashMap::new();
        for f in &all_findings {
            *by_severity.entry(f.severity.to_string()).or_insert(0) += 1;
        }

        // Build tool counts
        let mut by_tool = HashMap::new();
        for f in &all_findings {
            *by_tool.entry(f.source.clone()).or_insert(0) += 1;
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        SastResult {
            findings: all_findings,
            by_severity,
            by_tool,
            scanner_status,
            duration_ms,
            errors,
        }
    }
}
