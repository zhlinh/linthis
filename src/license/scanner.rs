// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! License scanner implementation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::languages::{
    GoLicenseScanner, JavaLicenseScanner, NodeLicenseScanner,
    PythonLicenseScanner, RustLicenseScanner,
};
use super::spdx::SpdxLicense;

/// Language-specific license scanner trait
pub trait LanguageLicenseScanner {
    /// Get the scanner name
    fn name(&self) -> &str;

    /// Get the language this scanner handles
    fn language(&self) -> &str;

    /// Check if a project uses this language
    fn detect(&self, path: &Path) -> bool;

    /// Scan for package licenses
    fn scan(&self, path: &Path) -> Result<Vec<PackageLicense>, String>;
}

/// License information for a package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageLicense {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// SPDX license identifier(s)
    pub license: SpdxLicense,
    /// Raw license string from manifest
    pub license_text: String,
    /// Ecosystem (npm, crates.io, pypi, etc.)
    pub ecosystem: String,
    /// Path to license file (if found)
    pub license_file: Option<PathBuf>,
    /// Whether this is a direct dependency
    pub is_direct: bool,
    /// License URL
    pub url: Option<String>,
    /// Authors
    pub authors: Vec<String>,
    /// Repository URL
    pub repository: Option<String>,
}

impl PackageLicense {
    /// Create a new package license
    pub fn new(name: &str, version: &str, license: &str, ecosystem: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            license: SpdxLicense::from_str(license),
            license_text: license.to_string(),
            ecosystem: ecosystem.to_string(),
            license_file: None,
            is_direct: true,
            url: None,
            authors: Vec::new(),
            repository: None,
        }
    }
}

/// Options for license scanning
#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    /// Path to scan
    pub path: PathBuf,
    /// Include dev dependencies
    pub include_dev: bool,
    /// Output format
    pub format: String,
    /// Generate SBOM
    pub generate_sbom: bool,
    /// Verbose output
    pub verbose: bool,
}

impl ScanOptions {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            format: "human".to_string(),
            ..Default::default()
        }
    }
}

/// License scan result
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanResult {
    /// All package licenses
    pub packages: Vec<PackageLicense>,
    /// Licenses by type
    pub by_license: HashMap<String, usize>,
    /// Packages by ecosystem
    pub by_ecosystem: HashMap<String, usize>,
    /// Languages scanned
    pub languages_scanned: Vec<String>,
    /// Scan duration in milliseconds
    pub duration_ms: u64,
    /// Any errors that occurred
    pub errors: Vec<String>,
}

impl ScanResult {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all unique licenses
    pub fn unique_licenses(&self) -> Vec<&SpdxLicense> {
        let mut licenses: Vec<_> = self.packages
            .iter()
            .map(|p| &p.license)
            .collect();
        licenses.sort_by(|a, b| a.to_spdx().cmp(b.to_spdx()));
        licenses.dedup_by(|a, b| a.to_spdx() == b.to_spdx());
        licenses
    }

    /// Get packages by license type
    pub fn packages_by_license(&self, license: &SpdxLicense) -> Vec<&PackageLicense> {
        self.packages
            .iter()
            .filter(|p| p.license.to_spdx() == license.to_spdx())
            .collect()
    }

    /// Get count of permissive licenses
    pub fn permissive_count(&self) -> usize {
        self.packages.iter().filter(|p| p.license.is_permissive()).count()
    }

    /// Get count of copyleft licenses
    pub fn copyleft_count(&self) -> usize {
        self.packages.iter().filter(|p| p.license.is_copyleft()).count()
    }

    /// Get count of weak copyleft licenses
    pub fn weak_copyleft_count(&self) -> usize {
        self.packages.iter().filter(|p| p.license.is_weak_copyleft()).count()
    }

    /// Get count of unknown licenses
    pub fn unknown_count(&self) -> usize {
        self.packages
            .iter()
            .filter(|p| matches!(p.license, SpdxLicense::Unknown(_)))
            .count()
    }
}

/// Main license scanner
pub struct LicenseScanner {
    scanners: Vec<Box<dyn LanguageLicenseScanner>>,
}

impl Default for LicenseScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl LicenseScanner {
    /// Create a new license scanner with all language scanners
    pub fn new() -> Self {
        let scanners: Vec<Box<dyn LanguageLicenseScanner>> = vec![
            Box::new(RustLicenseScanner::new()),
            Box::new(NodeLicenseScanner::new()),
            Box::new(PythonLicenseScanner::new()),
            Box::new(GoLicenseScanner::new()),
            Box::new(JavaLicenseScanner::new()),
        ];

        Self { scanners }
    }

    /// Detect which languages are used in the project
    pub fn detect_languages(&self, path: &Path) -> Vec<String> {
        self.scanners
            .iter()
            .filter(|s| s.detect(path))
            .map(|s| s.language().to_string())
            .collect()
    }

    /// Run license scan on a path
    pub fn scan(&self, options: &ScanOptions) -> Result<ScanResult, String> {
        let start = Instant::now();
        let mut result = ScanResult::new();
        let path = &options.path;

        for scanner in &self.scanners {
            if !scanner.detect(path) {
                continue;
            }

            result.languages_scanned.push(scanner.language().to_string());

            match scanner.scan(path) {
                Ok(packages) => {
                    for pkg in packages {
                        // Count by license
                        *result.by_license
                            .entry(pkg.license.to_spdx().to_string())
                            .or_insert(0) += 1;

                        // Count by ecosystem
                        *result.by_ecosystem
                            .entry(pkg.ecosystem.clone())
                            .or_insert(0) += 1;

                        result.packages.push(pkg);
                    }
                }
                Err(e) => {
                    result.errors.push(format!("{}: {}", scanner.name(), e));
                }
            }
        }

        result.duration_ms = start.elapsed().as_millis() as u64;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_options_default() {
        let options = ScanOptions::default();
        assert!(!options.include_dev);
        assert!(!options.generate_sbom);
    }

    #[test]
    fn test_scan_result_default() {
        let result = ScanResult::new();
        assert!(result.packages.is_empty());
        assert_eq!(result.duration_ms, 0);
    }

    #[test]
    fn test_package_license_new() {
        let pkg = PackageLicense::new("test-pkg", "1.0.0", "MIT", "crates.io");
        assert_eq!(pkg.name, "test-pkg");
        assert_eq!(pkg.version, "1.0.0");
        assert_eq!(pkg.license, SpdxLicense::MIT);
    }

    #[test]
    fn test_license_scanner_creation() {
        let scanner = LicenseScanner::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let languages = scanner.detect_languages(temp_dir.path());
        assert!(languages.is_empty());
    }
}
