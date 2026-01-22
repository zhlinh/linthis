// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Go license scanner using go-licenses or manual detection.

use std::path::Path;
use std::process::Command;

use crate::license::scanner::{LanguageLicenseScanner, PackageLicense};

/// Go license scanner
pub struct GoLicenseScanner;

impl GoLicenseScanner {
    pub fn new() -> Self {
        Self
    }

    fn has_go_licenses(&self) -> bool {
        Command::new("go-licenses")
            .args(["version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn scan_with_go_licenses(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        let output = Command::new("go-licenses")
            .args(["csv", "./..."])
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run go-licenses: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("go-licenses failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                let module = parts[0];
                let license = parts.get(1).unwrap_or(&"");

                // Extract name and version from module path
                let (name, version) = if module.contains('@') {
                    let split: Vec<&str> = module.split('@').collect();
                    (split[0].to_string(), split.get(1).unwrap_or(&"").to_string())
                } else {
                    (module.to_string(), String::new())
                };

                packages.push(PackageLicense::new(&name, &version, license, "go"));
            }
        }

        Ok(packages)
    }

    fn scan_from_go_mod(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        let output = Command::new("go")
            .args(["list", "-m", "-json", "all"])
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run go list: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("go list failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        // Parse JSON stream (one object per line)
        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Try to parse each line as a complete JSON object
            // go list -m -json outputs multiple JSON objects, not an array
            if let Ok(module) = serde_json::from_str::<GoModule>(line) {
                // Skip main module
                if module.main.unwrap_or(false) {
                    continue;
                }

                let license = self.detect_license_for_module(&module.path, path);
                packages.push(PackageLicense::new(
                    &module.path,
                    &module.version.unwrap_or_default(),
                    &license,
                    "go",
                ));
            }
        }

        Ok(packages)
    }

    fn detect_license_for_module(&self, _module: &str, _project_path: &Path) -> String {
        // In a real implementation, this would check the module cache
        // or fetch license info from pkg.go.dev
        String::new()
    }
}

impl Default for GoLicenseScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageLicenseScanner for GoLicenseScanner {
    fn name(&self) -> &str {
        "go-licenses"
    }

    fn language(&self) -> &str {
        "go"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("go.mod").exists()
    }

    fn scan(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        if self.has_go_licenses() {
            self.scan_with_go_licenses(path)
        } else {
            self.scan_from_go_mod(path)
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct GoModule {
    #[serde(rename = "Path")]
    path: String,
    #[serde(rename = "Version")]
    version: Option<String>,
    #[serde(rename = "Main")]
    main: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_scanner_detect() {
        let scanner = GoLicenseScanner::new();
        let temp_dir = tempfile::tempdir().unwrap();

        assert!(!scanner.detect(temp_dir.path()));

        std::fs::write(temp_dir.path().join("go.mod"), "module test").unwrap();
        assert!(scanner.detect(temp_dir.path()));
    }
}
