// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! SPDX license identifier parsing and handling.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Common SPDX license identifiers
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpdxLicense {
    // Permissive licenses
    MIT,
    Apache2,
    BSD2Clause,
    BSD3Clause,
    ISC,
    Unlicense,
    CC0,
    WTFPL,
    Zlib,

    // Weak copyleft
    LGPL21,
    LGPL21Plus,
    LGPL3,
    LGPL3Plus,
    MPL2,
    EPL1,
    EPL2,

    // Strong copyleft
    GPL2,
    GPL2Plus,
    GPL3,
    GPL3Plus,
    AGPL3,
    AGPL3Plus,

    // Other
    Proprietary,
    Unknown(String),
    Custom(String),
}

impl SpdxLicense {
    /// Parse SPDX identifier string
    pub fn parse_license(s: &str) -> Self {
        let normalized = s.trim().to_uppercase();
        Self::parse_permissive_license(&normalized)
            .or_else(|| Self::parse_copyleft_license(&normalized))
            .unwrap_or_else(|| Self::parse_other_license(s, &normalized))
    }

    fn parse_common_permissive_license(normalized: &str) -> Option<Self> {
        match normalized {
            "MIT" => Some(SpdxLicense::MIT),
            "APACHE-2.0" | "APACHE 2.0" | "APACHE2" => Some(SpdxLicense::Apache2),
            "BSD-2-CLAUSE" | "BSD 2-CLAUSE" => Some(SpdxLicense::BSD2Clause),
            "BSD-3-CLAUSE" | "BSD 3-CLAUSE" => Some(SpdxLicense::BSD3Clause),
            "ISC" => Some(SpdxLicense::ISC),
            _ => None,
        }
    }

    fn parse_public_domain_license(normalized: &str) -> Option<Self> {
        match normalized {
            "UNLICENSE" | "THE UNLICENSE" => Some(SpdxLicense::Unlicense),
            "CC0-1.0" | "CC0" => Some(SpdxLicense::CC0),
            "WTFPL" => Some(SpdxLicense::WTFPL),
            "ZLIB" => Some(SpdxLicense::Zlib),
            _ => None,
        }
    }

    fn parse_permissive_license(normalized: &str) -> Option<Self> {
        Self::parse_common_permissive_license(normalized)
            .or_else(|| Self::parse_public_domain_license(normalized))
    }

    fn parse_weak_copyleft_license(normalized: &str) -> Option<Self> {
        match normalized {
            "LGPL-2.1" | "LGPL-2.1-ONLY" => Some(SpdxLicense::LGPL21),
            "LGPL-2.1+" | "LGPL-2.1-OR-LATER" => Some(SpdxLicense::LGPL21Plus),
            "LGPL-3.0" | "LGPL-3.0-ONLY" => Some(SpdxLicense::LGPL3),
            "LGPL-3.0+" | "LGPL-3.0-OR-LATER" => Some(SpdxLicense::LGPL3Plus),
            "MPL-2.0" => Some(SpdxLicense::MPL2),
            "EPL-1.0" => Some(SpdxLicense::EPL1),
            "EPL-2.0" => Some(SpdxLicense::EPL2),
            _ => None,
        }
    }

    fn parse_strong_copyleft_license(normalized: &str) -> Option<Self> {
        match normalized {
            "GPL-2.0" | "GPL-2.0-ONLY" => Some(SpdxLicense::GPL2),
            "GPL-2.0+" | "GPL-2.0-OR-LATER" => Some(SpdxLicense::GPL2Plus),
            "GPL-3.0" | "GPL-3.0-ONLY" => Some(SpdxLicense::GPL3),
            "GPL-3.0+" | "GPL-3.0-OR-LATER" => Some(SpdxLicense::GPL3Plus),
            "AGPL-3.0" | "AGPL-3.0-ONLY" => Some(SpdxLicense::AGPL3),
            "AGPL-3.0+" | "AGPL-3.0-OR-LATER" => Some(SpdxLicense::AGPL3Plus),
            _ => None,
        }
    }

    fn parse_copyleft_license(normalized: &str) -> Option<Self> {
        Self::parse_weak_copyleft_license(normalized)
            .or_else(|| Self::parse_strong_copyleft_license(normalized))
    }

    fn parse_other_license(original: &str, normalized: &str) -> Self {
        match normalized {
            "PROPRIETARY" | "COMMERCIAL" => SpdxLicense::Proprietary,
            _ => SpdxLicense::Unknown(original.to_string()),
        }
    }

    /// Get the SPDX identifier string
    pub fn to_spdx(&self) -> &str {
        self.permissive_to_spdx()
            .or_else(|| self.copyleft_to_spdx())
            .unwrap_or_else(|| self.other_to_spdx())
    }

    fn common_permissive_to_spdx(&self) -> Option<&str> {
        match self {
            SpdxLicense::MIT => Some("MIT"),
            SpdxLicense::Apache2 => Some("Apache-2.0"),
            SpdxLicense::BSD2Clause => Some("BSD-2-Clause"),
            SpdxLicense::BSD3Clause => Some("BSD-3-Clause"),
            SpdxLicense::ISC => Some("ISC"),
            _ => None,
        }
    }

    fn public_domain_to_spdx(&self) -> Option<&str> {
        match self {
            SpdxLicense::Unlicense => Some("Unlicense"),
            SpdxLicense::CC0 => Some("CC0-1.0"),
            SpdxLicense::WTFPL => Some("WTFPL"),
            SpdxLicense::Zlib => Some("Zlib"),
            _ => None,
        }
    }

    fn permissive_to_spdx(&self) -> Option<&str> {
        self.common_permissive_to_spdx()
            .or_else(|| self.public_domain_to_spdx())
    }

    fn weak_copyleft_to_spdx(&self) -> Option<&str> {
        match self {
            SpdxLicense::LGPL21 => Some("LGPL-2.1-only"),
            SpdxLicense::LGPL21Plus => Some("LGPL-2.1-or-later"),
            SpdxLicense::LGPL3 => Some("LGPL-3.0-only"),
            SpdxLicense::LGPL3Plus => Some("LGPL-3.0-or-later"),
            SpdxLicense::MPL2 => Some("MPL-2.0"),
            SpdxLicense::EPL1 => Some("EPL-1.0"),
            SpdxLicense::EPL2 => Some("EPL-2.0"),
            _ => None,
        }
    }

    fn strong_copyleft_to_spdx(&self) -> Option<&str> {
        match self {
            SpdxLicense::GPL2 => Some("GPL-2.0-only"),
            SpdxLicense::GPL2Plus => Some("GPL-2.0-or-later"),
            SpdxLicense::GPL3 => Some("GPL-3.0-only"),
            SpdxLicense::GPL3Plus => Some("GPL-3.0-or-later"),
            SpdxLicense::AGPL3 => Some("AGPL-3.0-only"),
            SpdxLicense::AGPL3Plus => Some("AGPL-3.0-or-later"),
            _ => None,
        }
    }

    fn copyleft_to_spdx(&self) -> Option<&str> {
        self.weak_copyleft_to_spdx()
            .or_else(|| self.strong_copyleft_to_spdx())
    }

    fn other_to_spdx(&self) -> &str {
        match self {
            SpdxLicense::Proprietary => "Proprietary",
            SpdxLicense::Unknown(s) | SpdxLicense::Custom(s) => s,
            _ => "Unknown",
        }
    }

    /// Check if this is a permissive license
    pub fn is_permissive(&self) -> bool {
        matches!(
            self,
            SpdxLicense::MIT
                | SpdxLicense::Apache2
                | SpdxLicense::BSD2Clause
                | SpdxLicense::BSD3Clause
                | SpdxLicense::ISC
                | SpdxLicense::Unlicense
                | SpdxLicense::CC0
                | SpdxLicense::WTFPL
                | SpdxLicense::Zlib
        )
    }

    /// Check if this is a copyleft license
    pub fn is_copyleft(&self) -> bool {
        matches!(
            self,
            SpdxLicense::GPL2
                | SpdxLicense::GPL2Plus
                | SpdxLicense::GPL3
                | SpdxLicense::GPL3Plus
                | SpdxLicense::AGPL3
                | SpdxLicense::AGPL3Plus
        )
    }

    /// Check if this is a weak copyleft license
    pub fn is_weak_copyleft(&self) -> bool {
        matches!(
            self,
            SpdxLicense::LGPL21
                | SpdxLicense::LGPL21Plus
                | SpdxLicense::LGPL3
                | SpdxLicense::LGPL3Plus
                | SpdxLicense::MPL2
                | SpdxLicense::EPL1
                | SpdxLicense::EPL2
        )
    }

    /// Check if this license matches a pattern
    pub fn matches_pattern(&self, pattern: &str) -> bool {
        let pattern_upper = pattern.to_uppercase();
        let spdx = self.to_spdx().to_uppercase();

        if pattern_upper.ends_with('*') {
            let prefix = &pattern_upper[..pattern_upper.len() - 1];
            spdx.starts_with(prefix)
        } else if pattern_upper.contains('*') {
            // Simple glob matching
            let parts: Vec<&str> = pattern_upper.split('*').collect();
            if parts.len() == 2 {
                spdx.starts_with(parts[0]) && spdx.ends_with(parts[1])
            } else {
                spdx == pattern_upper
            }
        } else {
            spdx == pattern_upper
        }
    }
}

impl fmt::Display for SpdxLicense {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_spdx())
    }
}

impl Default for SpdxLicense {
    fn default() -> Self {
        SpdxLicense::Unknown("Unknown".to_string())
    }
}

/// Parsed SPDX expression (supports OR and AND)
#[derive(Debug, Clone)]
pub enum SpdxExpression {
    License(SpdxLicense),
    Or(Box<SpdxExpression>, Box<SpdxExpression>),
    And(Box<SpdxExpression>, Box<SpdxExpression>),
    With(Box<SpdxExpression>, String), // License WITH exception
}

impl SpdxExpression {
    /// Get all licenses in this expression
    pub fn all_licenses(&self) -> Vec<&SpdxLicense> {
        match self {
            SpdxExpression::License(l) => vec![l],
            SpdxExpression::Or(a, b) | SpdxExpression::And(a, b) => {
                let mut result = a.all_licenses();
                result.extend(b.all_licenses());
                result
            }
            SpdxExpression::With(expr, _) => expr.all_licenses(),
        }
    }

    /// Check if any license in this expression matches a pattern
    pub fn any_matches(&self, pattern: &str) -> bool {
        self.all_licenses()
            .iter()
            .any(|l| l.matches_pattern(pattern))
    }
}

/// Parse SPDX license expression string
pub fn parse_spdx_expression(s: &str) -> SpdxExpression {
    let s = s.trim();

    // Handle WITH exception
    if let Some(with_pos) = s.to_uppercase().find(" WITH ") {
        let license_part = &s[..with_pos];
        let exception = &s[with_pos + 6..];
        return SpdxExpression::With(
            Box::new(parse_spdx_expression(license_part)),
            exception.trim().to_string(),
        );
    }

    // Handle OR
    if let Some(or_pos) = s.to_uppercase().find(" OR ") {
        let left = &s[..or_pos];
        let right = &s[or_pos + 4..];
        return SpdxExpression::Or(
            Box::new(parse_spdx_expression(left)),
            Box::new(parse_spdx_expression(right)),
        );
    }

    // Handle AND
    if let Some(and_pos) = s.to_uppercase().find(" AND ") {
        let left = &s[..and_pos];
        let right = &s[and_pos + 5..];
        return SpdxExpression::And(
            Box::new(parse_spdx_expression(left)),
            Box::new(parse_spdx_expression(right)),
        );
    }

    // Remove parentheses
    let s = s.trim_start_matches('(').trim_end_matches(')').trim();

    SpdxExpression::License(SpdxLicense::parse_license(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spdx_from_str() {
        assert_eq!(SpdxLicense::parse_license("MIT"), SpdxLicense::MIT);
        assert_eq!(
            SpdxLicense::parse_license("Apache-2.0"),
            SpdxLicense::Apache2
        );
        assert_eq!(SpdxLicense::parse_license("GPL-3.0"), SpdxLicense::GPL3);
    }

    #[test]
    fn test_spdx_is_permissive() {
        assert!(SpdxLicense::MIT.is_permissive());
        assert!(SpdxLicense::Apache2.is_permissive());
        assert!(!SpdxLicense::GPL3.is_permissive());
    }

    #[test]
    fn test_spdx_is_copyleft() {
        assert!(SpdxLicense::GPL3.is_copyleft());
        assert!(SpdxLicense::AGPL3.is_copyleft());
        assert!(!SpdxLicense::MIT.is_copyleft());
    }

    #[test]
    fn test_pattern_matching() {
        assert!(SpdxLicense::GPL3.matches_pattern("GPL-*"));
        assert!(SpdxLicense::LGPL3.matches_pattern("LGPL-*"));
        assert!(!SpdxLicense::MIT.matches_pattern("GPL-*"));
    }

    #[test]
    fn test_parse_expression() {
        let expr = parse_spdx_expression("MIT OR Apache-2.0");
        let licenses = expr.all_licenses();
        assert_eq!(licenses.len(), 2);

        let expr = parse_spdx_expression("MIT AND BSD-3-Clause");
        let licenses = expr.all_licenses();
        assert_eq!(licenses.len(), 2);
    }
}
