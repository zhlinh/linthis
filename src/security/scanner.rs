// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Security scanner implementation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::languages::{
    GoSecurityScanner, JavaSecurityScanner, NodeSecurityScanner,
    PythonSecurityScanner, RustSecurityScanner,
};
use super::vulnerability::{Severity, Vulnerability};

/// Language-specific security scanner trait
pub trait LanguageSecurityScanner {
    /// Check if this scanner is available (tool installed)
    fn is_available(&self) -> bool;

    /// Get the scanner name
    fn name(&self) -> &str;

    /// Get the language this scanner handles
    fn language(&self) -> &str;

    /// Check if a project uses this language (based on manifest files)
    fn detect(&self, path: &Path) -> bool;

    /// Run security scan
    fn scan(&self, path: &Path, options: &ScanOptions) -> Result<Vec<Vulnerability>, String>;

    /// Attempt to fix vulnerabilities
    fn fix(&self, path: &Path, vulnerabilities: &[Vulnerability]) -> Result<FixResult, String>;
}

/// Options for security scanning
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanOptions {
    /// Path to scan
    pub path: PathBuf,
    /// Minimum severity to report
    pub severity_threshold: Option<String>,
    /// Include dev dependencies
    pub include_dev: bool,
    /// Specific packages to scan (empty = all)
    pub packages: Vec<String>,
    /// Vulnerability IDs to ignore
    pub ignore: Vec<String>,
    /// Output format (human, json, sarif)
    pub format: String,
    /// Generate SBOM
    pub generate_sbom: bool,
    /// Fail on vulnerabilities meeting threshold
    pub fail_on: Option<String>,
    /// Verbose output
    pub verbose: bool,
}

impl ScanOptions {
    /// Create new scan options with default values
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            format: "human".to_string(),
            ..Default::default()
        }
    }

    /// Get the severity threshold
    pub fn get_severity_threshold(&self) -> Severity {
        self.severity_threshold
            .as_ref()
            .map(|s| Severity::from_str(s))
            .unwrap_or(Severity::Unknown)
    }
}

/// Result of a fix attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixResult {
    /// Vulnerabilities that were fixed
    pub fixed: Vec<String>,
    /// Vulnerabilities that could not be fixed
    pub unfixed: Vec<String>,
    /// Commands that were executed
    pub commands: Vec<String>,
    /// Whether a manual review is needed
    pub needs_review: bool,
    /// Messages for the user
    pub messages: Vec<String>,
}

impl Default for FixResult {
    fn default() -> Self {
        Self {
            fixed: Vec::new(),
            unfixed: Vec::new(),
            commands: Vec::new(),
            needs_review: false,
            messages: Vec::new(),
        }
    }
}

/// Aggregated scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// All detected vulnerabilities
    pub vulnerabilities: Vec<Vulnerability>,
    /// Vulnerabilities by severity
    pub by_severity: HashMap<String, usize>,
    /// Vulnerabilities by language
    pub by_language: HashMap<String, usize>,
    /// Scan duration
    pub duration_ms: u64,
    /// Languages that were scanned
    pub languages_scanned: Vec<String>,
    /// Scanner availability info
    pub scanner_status: HashMap<String, bool>,
    /// Any errors that occurred
    pub errors: Vec<String>,
    /// Total packages analyzed
    pub total_packages: usize,
    /// Packages with vulnerabilities
    pub vulnerable_packages: usize,
}

impl ScanResult {
    /// Create an empty scan result
    pub fn new() -> Self {
        Self {
            vulnerabilities: Vec::new(),
            by_severity: HashMap::new(),
            by_language: HashMap::new(),
            duration_ms: 0,
            languages_scanned: Vec::new(),
            scanner_status: HashMap::new(),
            errors: Vec::new(),
            total_packages: 0,
            vulnerable_packages: 0,
        }
    }

    /// Filter vulnerabilities by severity threshold
    pub fn filter_by_severity(&self, threshold: Severity) -> Vec<&Vulnerability> {
        self.vulnerabilities
            .iter()
            .filter(|v| v.meets_severity_threshold(&threshold))
            .collect()
    }

    /// Get count of critical/high vulnerabilities
    pub fn critical_high_count(&self) -> usize {
        self.vulnerabilities
            .iter()
            .filter(|v| matches!(v.severity(), Severity::Critical | Severity::High))
            .count()
    }

    /// Check if scan found any vulnerabilities meeting threshold
    pub fn has_vulnerabilities_above(&self, threshold: Severity) -> bool {
        self.vulnerabilities
            .iter()
            .any(|v| v.meets_severity_threshold(&threshold))
    }
}

impl Default for ScanResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Main security scanner that aggregates language-specific scanners
pub struct SecurityScanner {
    scanners: Vec<Box<dyn LanguageSecurityScanner>>,
}

impl Default for SecurityScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityScanner {
    /// Create a new security scanner with all language scanners
    pub fn new() -> Self {
        let scanners: Vec<Box<dyn LanguageSecurityScanner>> = vec![
            Box::new(RustSecurityScanner::new()),
            Box::new(NodeSecurityScanner::new()),
            Box::new(PythonSecurityScanner::new()),
            Box::new(GoSecurityScanner::new()),
            Box::new(JavaSecurityScanner::new()),
        ];

        Self { scanners }
    }

    /// Get available scanners
    pub fn available_scanners(&self) -> Vec<(&str, &str, bool)> {
        self.scanners
            .iter()
            .map(|s| (s.name(), s.language(), s.is_available()))
            .collect()
    }

    /// Detect which languages are used in the project
    pub fn detect_languages(&self, path: &Path) -> Vec<String> {
        self.scanners
            .iter()
            .filter(|s| s.detect(path))
            .map(|s| s.language().to_string())
            .collect()
    }

    /// Run security scan on a path
    pub fn scan(&self, options: &ScanOptions) -> Result<ScanResult, String> {
        let start = Instant::now();
        let mut result = ScanResult::new();
        let path = &options.path;

        // Check which scanners are available and applicable
        for scanner in &self.scanners {
            let is_available = scanner.is_available();
            result.scanner_status.insert(
                scanner.name().to_string(),
                is_available,
            );

            if !is_available {
                continue;
            }

            if !scanner.detect(path) {
                continue;
            }

            result.languages_scanned.push(scanner.language().to_string());

            match scanner.scan(path, options) {
                Ok(vulns) => {
                    for vuln in vulns {
                        // Apply ignore list
                        if options.ignore.contains(&vuln.advisory.id) {
                            continue;
                        }

                        // Count by severity
                        *result.by_severity
                            .entry(vuln.severity().to_string())
                            .or_insert(0) += 1;

                        // Count by language
                        *result.by_language
                            .entry(vuln.language.clone())
                            .or_insert(0) += 1;

                        result.vulnerabilities.push(vuln);
                    }
                }
                Err(e) => {
                    result.errors.push(format!("{}: {}", scanner.name(), e));
                }
            }
        }

        result.duration_ms = start.elapsed().as_millis() as u64;
        result.vulnerable_packages = result.vulnerabilities
            .iter()
            .flat_map(|v| &v.affected_packages)
            .map(|p| format!("{}@{}", p.name, p.version))
            .collect::<std::collections::HashSet<_>>()
            .len();

        Ok(result)
    }

    /// Attempt to fix vulnerabilities
    pub fn fix(&self, path: &Path, result: &ScanResult) -> Result<FixResult, String> {
        let mut fix_result = FixResult::default();

        // Group vulnerabilities by language
        let mut by_language: HashMap<String, Vec<&Vulnerability>> = HashMap::new();
        for vuln in &result.vulnerabilities {
            by_language
                .entry(vuln.language.clone())
                .or_default()
                .push(vuln);
        }

        // Run fix for each language
        for (language, vulns) in by_language {
            if let Some(scanner) = self.scanners.iter().find(|s| s.language() == language) {
                let vuln_refs: Vec<Vulnerability> = vulns.into_iter().cloned().collect();
                match scanner.fix(path, &vuln_refs) {
                    Ok(lang_result) => {
                        fix_result.fixed.extend(lang_result.fixed);
                        fix_result.unfixed.extend(lang_result.unfixed);
                        fix_result.commands.extend(lang_result.commands);
                        fix_result.messages.extend(lang_result.messages);
                        if lang_result.needs_review {
                            fix_result.needs_review = true;
                        }
                    }
                    Err(e) => {
                        fix_result.messages.push(format!("Failed to fix {} vulnerabilities: {}", language, e));
                    }
                }
            }
        }

        Ok(fix_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_options_default() {
        let options = ScanOptions::default();
        assert_eq!(options.format, "");
        assert!(!options.include_dev);
        assert!(options.ignore.is_empty());
    }

    #[test]
    fn test_scan_result_default() {
        let result = ScanResult::new();
        assert!(result.vulnerabilities.is_empty());
        assert_eq!(result.duration_ms, 0);
    }

    #[test]
    fn test_security_scanner_creation() {
        let scanner = SecurityScanner::new();
        let available = scanner.available_scanners();
        assert!(!available.is_empty());
    }
}
