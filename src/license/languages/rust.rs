// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Rust license scanner using cargo-license or Cargo.lock parsing.

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::license::scanner::{LanguageLicenseScanner, PackageLicense};

/// Rust license scanner
pub struct RustLicenseScanner;

impl RustLicenseScanner {
    pub fn new() -> Self {
        Self
    }

    fn has_cargo_license(&self) -> bool {
        Command::new("cargo")
            .args(["license", "--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn scan_with_cargo_license(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        let output = Command::new("cargo")
            .args(["license", "--json"])
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run cargo license: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("cargo license failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let entries: Vec<CargoLicenseEntry> = serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse cargo license output: {}", e))?;

        Ok(entries
            .into_iter()
            .map(|e| PackageLicense::new(&e.name, &e.version, &e.license.unwrap_or_default(), "crates.io"))
            .collect())
    }

    fn scan_from_metadata(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        let output = Command::new("cargo")
            .args(["metadata", "--format-version", "1", "--no-deps"])
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run cargo metadata: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("cargo metadata failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let metadata: CargoMetadata = serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse cargo metadata: {}", e))?;

        let mut packages = Vec::new();

        // Get all resolved packages
        let full_output = Command::new("cargo")
            .args(["metadata", "--format-version", "1"])
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run cargo metadata: {}", e))?;

        if full_output.status.success() {
            let full_stdout = String::from_utf8_lossy(&full_output.stdout);
            if let Ok(full_metadata) = serde_json::from_str::<CargoMetadata>(&full_stdout) {
                for pkg in full_metadata.packages {
                    let is_direct = metadata.packages.iter().any(|p| p.name == pkg.name);
                    let mut license = PackageLicense::new(
                        &pkg.name,
                        &pkg.version,
                        &pkg.license.unwrap_or_default(),
                        "crates.io",
                    );
                    license.is_direct = is_direct;
                    license.authors = pkg.authors;
                    license.repository = pkg.repository;
                    packages.push(license);
                }
            }
        }

        Ok(packages)
    }
}

impl Default for RustLicenseScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageLicenseScanner for RustLicenseScanner {
    fn name(&self) -> &str {
        "cargo-license"
    }

    fn language(&self) -> &str {
        "rust"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("Cargo.toml").exists()
    }

    fn scan(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        if self.has_cargo_license() {
            self.scan_with_cargo_license(path)
        } else {
            self.scan_from_metadata(path)
        }
    }
}

#[derive(Debug, Deserialize)]
struct CargoLicenseEntry {
    name: String,
    version: String,
    license: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    name: String,
    version: String,
    license: Option<String>,
    #[serde(default)]
    authors: Vec<String>,
    repository: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_scanner_detect() {
        let scanner = RustLicenseScanner::new();
        let temp_dir = tempfile::tempdir().unwrap();

        assert!(!scanner.detect(temp_dir.path()));

        std::fs::write(temp_dir.path().join("Cargo.toml"), "[package]").unwrap();
        assert!(scanner.detect(temp_dir.path()));
    }
}
