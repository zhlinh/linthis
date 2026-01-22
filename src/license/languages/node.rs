// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Node.js license scanner using package.json and node_modules.

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::license::scanner::{LanguageLicenseScanner, PackageLicense};

/// Node.js license scanner
pub struct NodeLicenseScanner;

impl NodeLicenseScanner {
    pub fn new() -> Self {
        Self
    }

    fn scan_with_npm(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        let output = Command::new("npm")
            .args(["ls", "--json", "--all"])
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to run npm ls: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Ok(Vec::new());
        }

        let tree: NpmTree = serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse npm ls output: {}", e))?;

        let mut packages = Vec::new();
        self.collect_packages(&tree.dependencies, &mut packages, true);

        Ok(packages)
    }

    fn collect_packages(
        &self,
        deps: &Option<std::collections::HashMap<String, NpmDependency>>,
        packages: &mut Vec<PackageLicense>,
        is_direct: bool,
    ) {
        if let Some(dependencies) = deps {
            for (name, dep) in dependencies {
                let mut pkg = PackageLicense::new(
                    name,
                    &dep.version.clone().unwrap_or_default(),
                    &self.read_package_license(name).unwrap_or_default(),
                    "npm",
                );
                pkg.is_direct = is_direct;
                packages.push(pkg);

                // Recurse into nested dependencies
                self.collect_packages(&dep.dependencies, packages, false);
            }
        }
    }

    fn read_package_license(&self, _name: &str) -> Option<String> {
        // In a real implementation, this would read from node_modules/name/package.json
        None
    }

    fn scan_from_package_json(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        let package_json = path.join("package.json");
        let content = std::fs::read_to_string(&package_json)
            .map_err(|e| format!("Failed to read package.json: {}", e))?;

        let pkg: PackageJson = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse package.json: {}", e))?;

        let mut packages = Vec::new();

        // Read direct dependencies
        if let Some(deps) = pkg.dependencies {
            for (name, version) in deps {
                let license = self.get_license_from_node_modules(path, &name);
                let mut pkg_license = PackageLicense::new(&name, &version, &license, "npm");
                pkg_license.is_direct = true;
                packages.push(pkg_license);
            }
        }

        Ok(packages)
    }

    fn get_license_from_node_modules(&self, path: &Path, name: &str) -> String {
        let pkg_json = path.join("node_modules").join(name).join("package.json");
        if let Ok(content) = std::fs::read_to_string(&pkg_json) {
            if let Ok(pkg) = serde_json::from_str::<PackageJson>(&content) {
                return pkg.license.unwrap_or_default();
            }
        }
        String::new()
    }
}

impl Default for NodeLicenseScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageLicenseScanner for NodeLicenseScanner {
    fn name(&self) -> &str {
        "npm-license"
    }

    fn language(&self) -> &str {
        "javascript"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("package.json").exists()
    }

    fn scan(&self, path: &Path) -> Result<Vec<PackageLicense>, String> {
        if path.join("node_modules").exists() {
            self.scan_with_npm(path)
        } else {
            self.scan_from_package_json(path)
        }
    }
}

#[derive(Debug, Deserialize)]
struct NpmTree {
    dependencies: Option<std::collections::HashMap<String, NpmDependency>>,
}

#[derive(Debug, Deserialize)]
struct NpmDependency {
    version: Option<String>,
    dependencies: Option<std::collections::HashMap<String, NpmDependency>>,
}

#[derive(Debug, Deserialize)]
struct PackageJson {
    license: Option<String>,
    dependencies: Option<std::collections::HashMap<String, String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_scanner_detect() {
        let scanner = NodeLicenseScanner::new();
        let temp_dir = tempfile::tempdir().unwrap();

        assert!(!scanner.detect(temp_dir.path()));

        std::fs::write(temp_dir.path().join("package.json"), "{}").unwrap();
        assert!(scanner.detect(temp_dir.path()));
    }
}
