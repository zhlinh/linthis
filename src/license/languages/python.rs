// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Python license scanner using pip-licenses or metadata.

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::license::scanner::{LanguageLicenseScanner, PackageLicense};

/// Python license scanner
pub struct PythonLicenseScanner;

impl PythonLicenseScanner {
    pub fn new() -> Self {
        Self
    }

    fn has_pip_licenses(&self) -> bool {
        Command::new("pip-licenses")
            .args(["--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn scan_with_pip_licenses(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        let output = Command::new("pip-licenses")
            .args(["--format=json", "--with-urls", "--with-authors"])
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run pip-licenses: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("pip-licenses failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let entries: Vec<PipLicenseEntry> = serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse pip-licenses output: {}", e))?;

        Ok(entries
            .into_iter()
            .map(|e| {
                let mut pkg = PackageLicense::new(&e.name, &e.version, &e.license, "pypi");
                pkg.url = e.url;
                pkg.authors = e.author.map(|a| vec![a]).unwrap_or_default();
                pkg
            })
            .collect())
    }

    fn scan_from_pip_show(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        // Get list of installed packages
        let output = Command::new("pip")
            .args(["list", "--format=json"])
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run pip list: {}", e))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages: Vec<PipPackage> = serde_json::from_str(&stdout)
            .unwrap_or_default();

        let mut result = Vec::new();
        for pkg in packages {
            let license = self.get_package_license(&pkg.name).unwrap_or_default();
            result.push(PackageLicense::new(&pkg.name, &pkg.version, &license, "pypi"));
        }

        Ok(result)
    }

    fn get_package_license(&self, name: &str) -> Option<String> {
        let output = Command::new("pip")
            .args(["show", name])
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.starts_with("License:") {
                return Some(line.trim_start_matches("License:").trim().to_string());
            }
        }
        None
    }
}

impl Default for PythonLicenseScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageLicenseScanner for PythonLicenseScanner {
    fn name(&self) -> &str {
        "pip-licenses"
    }

    fn language(&self) -> &str {
        "python"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("requirements.txt").exists()
            || path.join("pyproject.toml").exists()
            || path.join("setup.py").exists()
            || path.join("Pipfile").exists()
    }

    fn scan(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        if self.has_pip_licenses() {
            self.scan_with_pip_licenses(path)
        } else {
            self.scan_from_pip_show(path)
        }
    }
}

#[derive(Debug, Deserialize)]
struct PipLicenseEntry {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Version")]
    version: String,
    #[serde(rename = "License")]
    license: String,
    #[serde(rename = "URL")]
    url: Option<String>,
    #[serde(rename = "Author")]
    author: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PipPackage {
    name: String,
    version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_scanner_detect() {
        let scanner = PythonLicenseScanner::new();
        let temp_dir = tempfile::tempdir().unwrap();

        assert!(!scanner.detect(temp_dir.path()));

        std::fs::write(temp_dir.path().join("requirements.txt"), "flask==1.0").unwrap();
        assert!(scanner.detect(temp_dir.path()));
    }
}
