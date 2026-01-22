// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! License policy configuration and violation checking.

use serde::{Deserialize, Serialize};

use super::scanner::{PackageLicense, ScanResult};
use super::spdx::SpdxLicense;

/// License category for classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LicenseCategory {
    /// Permissive licenses (MIT, Apache, BSD, etc.)
    Permissive,
    /// Weak copyleft (LGPL, MPL, etc.)
    WeakCopyleft,
    /// Strong copyleft (GPL, AGPL, etc.)
    Copyleft,
    /// Proprietary or commercial
    Proprietary,
    /// Unknown license
    Unknown,
}

impl LicenseCategory {
    /// Categorize a license
    pub fn from_license(license: &SpdxLicense) -> Self {
        if license.is_permissive() {
            LicenseCategory::Permissive
        } else if license.is_copyleft() {
            LicenseCategory::Copyleft
        } else if license.is_weak_copyleft() {
            LicenseCategory::WeakCopyleft
        } else if matches!(license, SpdxLicense::Proprietary) {
            LicenseCategory::Proprietary
        } else {
            LicenseCategory::Unknown
        }
    }
}

/// A policy violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyViolation {
    /// Package name
    pub package: String,
    /// Package version
    pub version: String,
    /// License that caused the violation
    pub license: String,
    /// Violation type
    pub violation_type: ViolationType,
    /// Reason for violation
    pub reason: String,
    /// Suggested action
    pub suggestion: Option<String>,
}

/// Type of policy violation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationType {
    /// License is explicitly denied
    Denied,
    /// License is not in allow list
    NotAllowed,
    /// License is unknown
    Unknown,
    /// License triggers warning
    Warning,
}

/// License policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePolicy {
    /// Allowed licenses (if set, only these are allowed)
    #[serde(default)]
    pub allow: Vec<String>,
    /// Denied licenses (always rejected)
    #[serde(default)]
    pub deny: Vec<String>,
    /// Warning licenses (allowed but warned)
    #[serde(default)]
    pub warn: Vec<String>,
    /// Whether to allow unknown licenses
    #[serde(default)]
    pub allow_unknown: bool,
    /// Whether to allow copyleft licenses
    #[serde(default = "default_true")]
    pub allow_copyleft: bool,
    /// Packages to ignore
    #[serde(default)]
    pub ignore_packages: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Default for LicensePolicy {
    fn default() -> Self {
        Self {
            // Common permissive licenses
            allow: vec![
                "MIT".to_string(),
                "Apache-2.0".to_string(),
                "BSD-2-Clause".to_string(),
                "BSD-3-Clause".to_string(),
                "ISC".to_string(),
                "Unlicense".to_string(),
                "CC0-1.0".to_string(),
                "Zlib".to_string(),
                "MPL-2.0".to_string(),
            ],
            deny: vec![],
            warn: vec![
                "LGPL-*".to_string(),
                "GPL-*".to_string(),
                "AGPL-*".to_string(),
            ],
            allow_unknown: false,
            allow_copyleft: true,
            ignore_packages: vec![],
        }
    }
}

impl LicensePolicy {
    /// Create a new empty policy
    pub fn new() -> Self {
        Self {
            allow: vec![],
            deny: vec![],
            warn: vec![],
            allow_unknown: false,
            allow_copyleft: true,
            ignore_packages: vec![],
        }
    }

    /// Create a strict policy (only permissive licenses)
    pub fn strict() -> Self {
        Self {
            allow: vec![
                "MIT".to_string(),
                "Apache-2.0".to_string(),
                "BSD-2-Clause".to_string(),
                "BSD-3-Clause".to_string(),
                "ISC".to_string(),
            ],
            deny: vec![
                "GPL-*".to_string(),
                "AGPL-*".to_string(),
                "LGPL-*".to_string(),
            ],
            warn: vec![],
            allow_unknown: false,
            allow_copyleft: false,
            ignore_packages: vec![],
        }
    }

    /// Create a permissive policy (allow everything except copyleft)
    pub fn permissive() -> Self {
        Self {
            allow: vec![],
            deny: vec![
                "GPL-*".to_string(),
                "AGPL-*".to_string(),
            ],
            warn: vec!["LGPL-*".to_string()],
            allow_unknown: true,
            allow_copyleft: false,
            ignore_packages: vec![],
        }
    }

    /// Load policy from TOML string
    pub fn from_toml(content: &str) -> Result<Self, String> {
        toml::from_str(content)
            .map_err(|e| format!("Failed to parse license policy: {}", e))
    }

    /// Check a scan result against this policy
    pub fn check(&self, result: &ScanResult) -> Vec<PolicyViolation> {
        let mut violations = Vec::new();

        for pkg in &result.packages {
            // Skip ignored packages
            if self.ignore_packages.contains(&pkg.name) {
                continue;
            }

            if let Some(violation) = self.check_package(pkg) {
                violations.push(violation);
            }
        }

        violations
    }

    /// Check a single package against this policy
    pub fn check_package(&self, pkg: &PackageLicense) -> Option<PolicyViolation> {
        let license_str = pkg.license.to_spdx();

        // Check deny list first
        for pattern in &self.deny {
            if pkg.license.matches_pattern(pattern) {
                return Some(PolicyViolation {
                    package: pkg.name.clone(),
                    version: pkg.version.clone(),
                    license: license_str.to_string(),
                    violation_type: ViolationType::Denied,
                    reason: format!("License '{}' matches denied pattern '{}'", license_str, pattern),
                    suggestion: Some("Find an alternative package with a compatible license".to_string()),
                });
            }
        }

        // Check unknown licenses
        if matches!(pkg.license, SpdxLicense::Unknown(_)) && !self.allow_unknown {
            return Some(PolicyViolation {
                package: pkg.name.clone(),
                version: pkg.version.clone(),
                license: license_str.to_string(),
                violation_type: ViolationType::Unknown,
                reason: format!("License '{}' is unknown", license_str),
                suggestion: Some("Review the package license manually".to_string()),
            });
        }

        // Check copyleft
        if pkg.license.is_copyleft() && !self.allow_copyleft {
            return Some(PolicyViolation {
                package: pkg.name.clone(),
                version: pkg.version.clone(),
                license: license_str.to_string(),
                violation_type: ViolationType::Denied,
                reason: format!("Copyleft license '{}' is not allowed", license_str),
                suggestion: Some("Find an alternative package with a permissive license".to_string()),
            });
        }

        // Check warning list
        for pattern in &self.warn {
            if pkg.license.matches_pattern(pattern) {
                return Some(PolicyViolation {
                    package: pkg.name.clone(),
                    version: pkg.version.clone(),
                    license: license_str.to_string(),
                    violation_type: ViolationType::Warning,
                    reason: format!("License '{}' matches warning pattern '{}'", license_str, pattern),
                    suggestion: Some("Review license terms for compliance".to_string()),
                });
            }
        }

        // Check allow list (if not empty)
        if !self.allow.is_empty() {
            let allowed = self.allow.iter().any(|pattern| pkg.license.matches_pattern(pattern));
            if !allowed {
                return Some(PolicyViolation {
                    package: pkg.name.clone(),
                    version: pkg.version.clone(),
                    license: license_str.to_string(),
                    violation_type: ViolationType::NotAllowed,
                    reason: format!("License '{}' is not in allow list", license_str),
                    suggestion: Some("Add license to allow list or find alternative package".to_string()),
                });
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_license_category() {
        assert_eq!(
            LicenseCategory::from_license(&SpdxLicense::MIT),
            LicenseCategory::Permissive
        );
        assert_eq!(
            LicenseCategory::from_license(&SpdxLicense::GPL3),
            LicenseCategory::Copyleft
        );
        assert_eq!(
            LicenseCategory::from_license(&SpdxLicense::LGPL3),
            LicenseCategory::WeakCopyleft
        );
    }

    #[test]
    fn test_policy_default() {
        let policy = LicensePolicy::default();
        assert!(!policy.allow.is_empty());
        assert!(policy.deny.is_empty());
    }

    #[test]
    fn test_policy_check_allowed() {
        let policy = LicensePolicy::default();
        let pkg = PackageLicense::new("test", "1.0.0", "MIT", "crates.io");
        assert!(policy.check_package(&pkg).is_none());
    }

    #[test]
    fn test_policy_check_denied() {
        let mut policy = LicensePolicy::default();
        policy.deny.push("GPL-*".to_string());
        policy.allow_copyleft = false;

        let pkg = PackageLicense::new("test", "1.0.0", "GPL-3.0", "crates.io");
        let violation = policy.check_package(&pkg);
        assert!(violation.is_some());
        assert_eq!(violation.unwrap().violation_type, ViolationType::Denied);
    }

    #[test]
    fn test_policy_strict() {
        let policy = LicensePolicy::strict();
        assert!(!policy.allow_copyleft);

        let pkg = PackageLicense::new("test", "1.0.0", "GPL-3.0", "crates.io");
        let violation = policy.check_package(&pkg);
        assert!(violation.is_some());
    }
}
