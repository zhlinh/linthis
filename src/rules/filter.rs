// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Rule filtering for disabling rules and overriding severity levels.

use std::collections::{HashMap, HashSet};

use super::config::{RulesConfig, SeverityOverride};
use crate::utils::types::LintIssue;

/// Filter for applying rule disable/enable and severity overrides.
#[derive(Debug, Clone)]
pub struct RuleFilter {
    /// Exact rule codes that are disabled.
    disabled_codes: HashSet<String>,
    /// Rule code prefixes that are disabled (e.g., "whitespace" for "whitespace/*").
    disabled_prefixes: Vec<String>,
    /// Severity overrides by rule code.
    severity_overrides: HashMap<String, SeverityOverride>,
}

impl Default for RuleFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleFilter {
    /// Create a new empty rule filter.
    pub fn new() -> Self {
        Self {
            disabled_codes: HashSet::new(),
            disabled_prefixes: Vec::new(),
            severity_overrides: HashMap::new(),
        }
    }

    /// Create a rule filter from a RulesConfig.
    pub fn from_config(config: &RulesConfig) -> Self {
        let mut filter = Self::new();

        // Process disabled rules
        for code in &config.disable {
            if let Some(prefix) = code.strip_suffix("/*") {
                // Prefix pattern like "whitespace/*"
                filter.disabled_prefixes.push(prefix.to_string());
            } else if let Some(prefix) = code.strip_suffix('*') {
                // Prefix pattern like "clippy::needless_*"
                filter.disabled_prefixes.push(prefix.to_string());
            } else {
                // Exact code match
                filter.disabled_codes.insert(code.clone());
            }
        }

        // Process severity overrides
        filter.severity_overrides = config.severity.clone();

        filter
    }

    /// Merge language-specific rules config.
    pub fn merge_language_config(&mut self, lang_rules: Option<&RulesConfig>) {
        if let Some(rules) = lang_rules {
            // Add language-specific disabled rules
            for code in &rules.disable {
                if let Some(prefix) = code.strip_suffix("/*") {
                    if !self.disabled_prefixes.contains(&prefix.to_string()) {
                        self.disabled_prefixes.push(prefix.to_string());
                    }
                } else if let Some(prefix) = code.strip_suffix('*') {
                    if !self.disabled_prefixes.contains(&prefix.to_string()) {
                        self.disabled_prefixes.push(prefix.to_string());
                    }
                } else {
                    self.disabled_codes.insert(code.clone());
                }
            }

            // Add language-specific severity overrides (override global)
            self.severity_overrides.extend(rules.severity.clone());
        }
    }

    /// Check if a rule code is disabled.
    pub fn is_disabled(&self, code: &str) -> bool {
        // Check exact match
        if self.disabled_codes.contains(code) {
            return true;
        }

        // Check severity override set to Off
        if let Some(SeverityOverride::Off) = self.severity_overrides.get(code) {
            return true;
        }

        // Check prefix match
        for prefix in &self.disabled_prefixes {
            if code.starts_with(prefix) {
                return true;
            }
        }

        false
    }

    /// Get the severity override for a rule code, if any.
    pub fn get_severity_override(&self, code: &str) -> Option<SeverityOverride> {
        self.severity_overrides.get(code).copied()
    }

    /// Filter a list of issues, removing disabled rules and applying severity overrides.
    pub fn filter_issues(&self, issues: Vec<LintIssue>) -> Vec<LintIssue> {
        issues
            .into_iter()
            .filter_map(|mut issue| {
                // Get rule code (use "unknown" if not set)
                let code = issue.code.as_deref().unwrap_or("unknown");

                // Check if disabled
                if self.is_disabled(code) {
                    return None;
                }

                // Apply severity override
                if let Some(override_severity) = self.get_severity_override(code) {
                    match override_severity {
                        SeverityOverride::Off => return None,
                        SeverityOverride::Error => issue.severity = crate::utils::types::Severity::Error,
                        SeverityOverride::Warning => issue.severity = crate::utils::types::Severity::Warning,
                        SeverityOverride::Info => issue.severity = crate::utils::types::Severity::Info,
                    }
                }

                Some(issue)
            })
            .collect()
    }

    /// Get statistics about the filter configuration.
    pub fn stats(&self) -> FilterStats {
        FilterStats {
            disabled_codes: self.disabled_codes.len(),
            disabled_prefixes: self.disabled_prefixes.len(),
            severity_overrides: self.severity_overrides.len(),
        }
    }
}

/// Statistics about a RuleFilter configuration.
#[derive(Debug, Clone)]
pub struct FilterStats {
    /// Number of exact rule codes disabled.
    pub disabled_codes: usize,
    /// Number of prefix patterns disabled.
    pub disabled_prefixes: usize,
    /// Number of severity overrides.
    pub severity_overrides: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::types::Severity;
    use std::path::PathBuf;

    fn make_issue(code: &str, severity: Severity) -> LintIssue {
        let mut issue = LintIssue::new(
            PathBuf::from("test.rs"),
            1,
            "Test message".to_string(),
            severity,
        );
        issue.code = Some(code.to_string());
        issue
    }

    #[test]
    fn test_filter_disabled_exact() {
        let mut config = RulesConfig::default();
        config.disable.push("E501".to_string());
        config.disable.push("W001".to_string());

        let filter = RuleFilter::from_config(&config);

        assert!(filter.is_disabled("E501"));
        assert!(filter.is_disabled("W001"));
        assert!(!filter.is_disabled("E502"));
    }

    #[test]
    fn test_filter_disabled_prefix() {
        let mut config = RulesConfig::default();
        config.disable.push("whitespace/*".to_string());
        config.disable.push("clippy::needless_*".to_string());

        let filter = RuleFilter::from_config(&config);

        assert!(filter.is_disabled("whitespace/trailing"));
        assert!(filter.is_disabled("whitespace/indent"));
        assert!(filter.is_disabled("clippy::needless_return"));
        assert!(!filter.is_disabled("formatting/indent"));
    }

    #[test]
    fn test_filter_severity_override() {
        let mut config = RulesConfig::default();
        config.severity.insert("W001".to_string(), SeverityOverride::Error);
        config.severity.insert("E001".to_string(), SeverityOverride::Info);
        config.severity.insert("W002".to_string(), SeverityOverride::Off);

        let filter = RuleFilter::from_config(&config);

        assert_eq!(filter.get_severity_override("W001"), Some(SeverityOverride::Error));
        assert_eq!(filter.get_severity_override("E001"), Some(SeverityOverride::Info));
        assert_eq!(filter.get_severity_override("W002"), Some(SeverityOverride::Off));
        assert_eq!(filter.get_severity_override("X001"), None);

        // Off should also count as disabled
        assert!(filter.is_disabled("W002"));
    }

    #[test]
    fn test_filter_issues() {
        let mut config = RulesConfig::default();
        config.disable.push("E501".to_string());
        config.severity.insert("W001".to_string(), SeverityOverride::Error);
        config.severity.insert("W002".to_string(), SeverityOverride::Off);

        let filter = RuleFilter::from_config(&config);

        let issues = vec![
            make_issue("E501", Severity::Error),   // Should be filtered out (disabled)
            make_issue("W001", Severity::Warning), // Should become Error
            make_issue("W002", Severity::Warning), // Should be filtered out (off)
            make_issue("E001", Severity::Error),   // Should remain unchanged
        ];

        let filtered = filter.filter_issues(issues);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].code.as_deref(), Some("W001"));
        assert_eq!(filtered[0].severity, Severity::Error);
        assert_eq!(filtered[1].code.as_deref(), Some("E001"));
        assert_eq!(filtered[1].severity, Severity::Error);
    }

    #[test]
    fn test_merge_language_config() {
        let mut global_config = RulesConfig::default();
        global_config.disable.push("E501".to_string());
        global_config.severity.insert("W001".to_string(), SeverityOverride::Info);

        let mut lang_config = RulesConfig::default();
        lang_config.disable.push("E502".to_string());
        lang_config.severity.insert("W001".to_string(), SeverityOverride::Error); // Override global

        let mut filter = RuleFilter::from_config(&global_config);
        filter.merge_language_config(Some(&lang_config));

        assert!(filter.is_disabled("E501")); // From global
        assert!(filter.is_disabled("E502")); // From language

        // Language override should take precedence
        assert_eq!(filter.get_severity_override("W001"), Some(SeverityOverride::Error));
    }

    #[test]
    fn test_filter_stats() {
        let mut config = RulesConfig::default();
        config.disable.push("E501".to_string());
        config.disable.push("whitespace/*".to_string());
        config.severity.insert("W001".to_string(), SeverityOverride::Error);

        let filter = RuleFilter::from_config(&config);
        let stats = filter.stats();

        assert_eq!(stats.disabled_codes, 1);
        assert_eq!(stats.disabled_prefixes, 1);
        assert_eq!(stats.severity_overrides, 1);
    }
}
