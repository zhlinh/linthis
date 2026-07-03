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
pub mod suppress;
pub mod tools;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

pub use finding::SastFinding;
pub use report::format_sast_report;
pub use scanner::{SastScanOptions, SastScanner};
pub use tools::{BanditScanner, FlawfinderScanner, GosecScanner, OpenGrepScanner, SecretsScanner};

use crate::security::vulnerability::Severity;

/// Info about a SAST tool that was needed but not installed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SastUnavailableTool {
    pub tool: String,
    pub languages: Vec<String>,
    pub install_hint: String,
}

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
    /// Tools that were needed (by language) but not installed
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unavailable_tools: Vec<SastUnavailableTool>,
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

/// Result of scanner selection: scanners to run, per-scanner availability
/// status, and relevant-but-uninstalled tools to report.
type ScannerSelection<'a> = (
    Vec<&'a dyn SastScanner>,
    Vec<(String, bool)>,
    Vec<SastUnavailableTool>,
);

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

    /// Select scanners relevant to the detected languages.
    ///
    /// Returns the available scanners to run, the availability status of every
    /// relevant scanner, and the relevant-but-uninstalled scanners to report.
    fn select_scanners<'a>(
        &'a self,
        detected_langs: &std::collections::HashSet<String>,
    ) -> ScannerSelection<'a> {
        let mut needed_scanners: Vec<&dyn SastScanner> = Vec::new();
        let mut scanner_status = Vec::new();
        let mut unavailable_tools = Vec::new();

        for scanner in &self.scanners {
            let supported = scanner.supported_languages();
            let is_universal = supported.contains(&"*");
            let is_needed = is_universal
                || supported
                    .iter()
                    .any(|lang| detected_langs.contains(&lang.to_string()));

            if !is_needed {
                // Scanner not relevant for these files — skip silently
                continue;
            }

            let available = scanner.is_available();
            scanner_status.push((scanner.name().to_string(), available));

            if available {
                needed_scanners.push(scanner.as_ref());
            } else {
                // Needed but not installed — report
                let relevant_langs: Vec<String> = if is_universal {
                    detected_langs.iter().cloned().collect()
                } else {
                    supported
                        .iter()
                        .filter(|l| detected_langs.contains(&l.to_string()))
                        .map(|l| l.to_string())
                        .collect()
                };
                unavailable_tools.push(SastUnavailableTool {
                    tool: scanner.name().to_string(),
                    languages: relevant_langs,
                    install_hint: scanner.install_hint(),
                });
            }
        }

        (needed_scanners, scanner_status, unavailable_tools)
    }

    /// Run SAST scan across all available scanners.
    ///
    /// Scanners are filtered by language: only scanners that support at least one
    /// language present in the target files are invoked. Unavailable but needed
    /// scanners are reported in `SastResult::unavailable_tools`.
    #[allow(clippy::unnecessary_to_owned)]
    pub fn scan(&self, path: &Path, files: &[PathBuf], options: &SastScanOptions) -> SastResult {
        let start = Instant::now();
        let mut all_findings = Vec::new();
        let mut errors = Vec::new();

        // If path is a file, use its parent as the scan directory
        let (scan_dir, scan_files) = if path.is_file() {
            let parent = path.parent().unwrap_or(Path::new("."));
            (parent.to_path_buf(), vec![path.to_path_buf()])
        } else {
            (path.to_path_buf(), files.to_vec())
        };

        // Detect languages from target files, then select relevant scanners.
        let detected_langs = detect_languages_from_files(&scan_files, &scan_dir);
        let (needed_scanners, scanner_status, unavailable_tools) =
            self.select_scanners(&detected_langs);

        // Run needed + available scanners in parallel
        let scan_dir_ref = &scan_dir;
        let scan_files_ref = &scan_files;
        let options_ref = options;

        let results: Vec<_> = needed_scanners
            .into_par_iter()
            .map(
                |scanner| match scanner.scan(scan_dir_ref, scan_files_ref, options_ref) {
                    Ok(mut findings) => {
                        if let Some(ref threshold) = options_ref.severity_threshold {
                            findings.retain(|f| f.meets_severity_threshold(threshold));
                        }
                        Ok(findings)
                    }
                    Err(e) => Err(format!("{}: {}", scanner.name(), e)),
                },
            )
            .collect();

        for r in results {
            match r {
                Ok(mut findings) => all_findings.append(&mut findings),
                Err(e) => errors.push(e),
            }
        }

        // Drop findings suppressed by an inline `linthis:ignore` directive.
        // Applies uniformly to every SAST tool (secrets, opengrep, bandit, ...).
        all_findings = suppress::filter_suppressed(all_findings, &scan_dir);

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
            unavailable_tools,
            duration_ms,
            errors,
        }
    }
}

/// Detect programming languages from a list of files (by extension).
fn detect_languages_from_files(
    files: &[PathBuf],
    scan_dir: &Path,
) -> std::collections::HashSet<String> {
    let mut langs = std::collections::HashSet::new();

    let file_list: Vec<PathBuf> = if files.is_empty() {
        // If no specific files, walk the directory for common extensions
        walkdir::WalkDir::new(scan_dir)
            .max_depth(5)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.into_path())
            .collect()
    } else {
        files.to_vec()
    };

    for file in &file_list {
        if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
            if let Some(lang) = ext_to_language(ext) {
                langs.insert(lang.to_string());
            }
        }
    }

    langs
}

/// Map a file extension to a language identifier used for scanner selection.
fn ext_to_language(ext: &str) -> Option<&'static str> {
    let lang = match ext {
        "py" | "pyw" => "python",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "ts" | "tsx" => "typescript",
        "go" => "go",
        "rs" => "rust",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => "cpp",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "scala" => "scala",
        "cs" => "csharp",
        _ => return None,
    };
    Some(lang)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Count aggregator findings for the built-in AWS-key secrets rule (which is
    /// always available, so the test does not depend on external SAST tools).
    fn aws_findings(dir: &Path, file: &Path) -> usize {
        let result = SastAggregator::new().scan(
            dir,
            &[file.to_path_buf()],
            &SastScanOptions::default(),
        );
        result
            .findings
            .iter()
            .filter(|f| f.rule_id == "secrets/aws-access-key")
            .count()
    }

    #[test]
    fn inline_ignore_suppresses_findings_across_tools() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("config.py");
        // A valid AWS Access Key ID shape (AKIA + 16 chars); repeated so each
        // line is a distinct finding candidate.
        // Split so this test source itself carries no matching key literal;
        // the joined runtime value still triggers the secrets scanner.
        let key = concat!("AKIA", "2K7BQ4RN5VXZ9WJ3");
        fs::write(
            &file,
            format!(
                "K1 = \"{key}\"\n\
                 K2 = \"{key}\"  # linthis:ignore secrets\n\
                 K3 = \"{key}\"  # linthis:ignore security\n\
                 K4 = \"{key}\"  # linthis:ignore secrets/aws-access-key\n\
                 # linthis:ignore-next-line security\n\
                 K5 = \"{key}\"\n"
            ),
        )
        .unwrap();

        // Only K1 (no directive) should remain; the other four are suppressed by
        // tool name, the catch-all `security`, an exact rule id, and next-line.
        assert_eq!(aws_findings(temp.path(), &file), 1);
    }

    #[test]
    fn unrelated_directive_does_not_suppress() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("config.py");
        // Split so this test source itself carries no matching key literal;
        // the joined runtime value still triggers the secrets scanner.
        let key = concat!("AKIA", "2K7BQ4RN5VXZ9WJ3");
        // `opengrep` target must not suppress a secrets finding.
        fs::write(&file, format!("K = \"{key}\"  # linthis:ignore opengrep\n")).unwrap();

        assert_eq!(aws_findings(temp.path(), &file), 1);
    }
}
