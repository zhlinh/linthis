// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Configuration types for custom rules, rule disabling, and severity overrides.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::utils::types::Severity;

/// Configuration for custom rules, disabled rules, and severity overrides.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RulesConfig {
    /// Custom regex-based rules.
    #[serde(default)]
    pub custom: Vec<CustomRule>,

    /// Rule codes to disable (supports "prefix/*" patterns for prefix matching).
    #[serde(default)]
    pub disable: Vec<String>,

    /// Severity overrides: rule code -> new severity.
    #[serde(default)]
    pub severity: HashMap<String, SeverityOverride>,
}

impl RulesConfig {
    /// Create a new empty RulesConfig.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if there are any custom rules defined.
    pub fn has_custom_rules(&self) -> bool {
        !self.custom.is_empty()
    }

    /// Check if there are any rule modifications (disable or severity).
    pub fn has_modifications(&self) -> bool {
        !self.disable.is_empty() || !self.severity.is_empty()
    }

    /// Merge another RulesConfig into this one.
    pub fn merge(&mut self, other: RulesConfig) {
        self.custom.extend(other.custom);
        self.disable.extend(other.disable);
        self.severity.extend(other.severity);
    }
}

/// A custom regex-based lint rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRule {
    /// Unique rule code (e.g., "custom/no-fixme").
    pub code: String,

    /// Regex pattern to match in source code.
    pub pattern: String,

    /// Error message to display when pattern matches.
    pub message: String,

    /// Severity level for this rule.
    #[serde(default = "default_warning")]
    pub severity: Severity,

    /// Optional suggestion for fixing the issue.
    #[serde(default)]
    pub suggestion: Option<String>,

    /// File extensions to check (e.g., ["rs", "py"]). Empty means all files.
    #[serde(default)]
    pub extensions: Vec<String>,

    /// Languages to check (e.g., ["rust", "python"]). Empty means all languages.
    #[serde(default)]
    pub languages: Vec<String>,

    /// Whether this rule is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_warning() -> Severity {
    Severity::Warning
}

fn default_true() -> bool {
    true
}

impl CustomRule {
    /// Create a new custom rule with required fields.
    pub fn new(code: impl Into<String>, pattern: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            pattern: pattern.into(),
            message: message.into(),
            severity: Severity::Warning,
            suggestion: None,
            extensions: Vec::new(),
            languages: Vec::new(),
            enabled: true,
        }
    }

    /// Set the severity level.
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Set the suggestion text.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Set the file extensions filter.
    pub fn with_extensions(mut self, extensions: Vec<String>) -> Self {
        self.extensions = extensions;
        self
    }

    /// Set the languages filter.
    pub fn with_languages(mut self, languages: Vec<String>) -> Self {
        self.languages = languages;
        self
    }

    /// Check if this rule applies to a given file extension.
    pub fn applies_to_extension(&self, ext: &str) -> bool {
        self.extensions.is_empty() || self.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext))
    }

    /// Check if this rule applies to a given language.
    pub fn applies_to_language(&self, lang: &str) -> bool {
        self.languages.is_empty() || self.languages.iter().any(|l| l.eq_ignore_ascii_case(lang))
    }
}

/// Severity override for a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeverityOverride {
    /// Treat as error.
    Error,
    /// Treat as warning.
    Warning,
    /// Treat as info.
    Info,
    /// Disable the rule entirely.
    Off,
}

impl SeverityOverride {
    /// Convert to `Option<Severity>`, returning None for Off.
    pub fn to_severity(self) -> Option<Severity> {
        match self {
            SeverityOverride::Error => Some(Severity::Error),
            SeverityOverride::Warning => Some(Severity::Warning),
            SeverityOverride::Info => Some(Severity::Info),
            SeverityOverride::Off => None,
        }
    }
}

impl From<SeverityOverride> for Option<Severity> {
    fn from(override_val: SeverityOverride) -> Self {
        override_val.to_severity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rules_config_default() {
        let config = RulesConfig::default();
        assert!(config.custom.is_empty());
        assert!(config.disable.is_empty());
        assert!(config.severity.is_empty());
        assert!(!config.has_custom_rules());
        assert!(!config.has_modifications());
    }

    #[test]
    fn test_rules_config_merge() {
        let mut config1 = RulesConfig::default();
        config1.disable.push("E001".to_string());

        let mut config2 = RulesConfig::default();
        config2.disable.push("W001".to_string());
        config2.custom.push(CustomRule::new("custom/test", "TODO", "Found TODO"));

        config1.merge(config2);

        assert_eq!(config1.disable.len(), 2);
        assert_eq!(config1.custom.len(), 1);
    }

    #[test]
    fn test_custom_rule_builder() {
        let rule = CustomRule::new("custom/no-print", r"print\(", "No print statements")
            .with_severity(Severity::Error)
            .with_suggestion("Use logging instead")
            .with_extensions(vec!["py".to_string()])
            .with_languages(vec!["python".to_string()]);

        assert_eq!(rule.code, "custom/no-print");
        assert_eq!(rule.severity, Severity::Error);
        assert!(rule.suggestion.is_some());
        assert!(rule.applies_to_extension("py"));
        assert!(!rule.applies_to_extension("rs"));
        assert!(rule.applies_to_language("python"));
        assert!(!rule.applies_to_language("rust"));
    }

    #[test]
    fn test_custom_rule_applies_to_all() {
        let rule = CustomRule::new("custom/test", "test", "Test message");

        // Empty filters should match all
        assert!(rule.applies_to_extension("rs"));
        assert!(rule.applies_to_extension("py"));
        assert!(rule.applies_to_language("rust"));
        assert!(rule.applies_to_language("python"));
    }

    #[test]
    fn test_severity_override_to_severity() {
        assert_eq!(SeverityOverride::Error.to_severity(), Some(Severity::Error));
        assert_eq!(SeverityOverride::Warning.to_severity(), Some(Severity::Warning));
        assert_eq!(SeverityOverride::Info.to_severity(), Some(Severity::Info));
        assert_eq!(SeverityOverride::Off.to_severity(), None);
    }

    #[test]
    fn test_deserialize_rules_config() {
        let toml = r#"
            disable = ["E501", "W001"]

            [severity]
            "E001" = "error"
            "W002" = "off"

            [[custom]]
            code = "custom/no-todo"
            pattern = "TODO"
            message = "Found TODO comment"
            severity = "warning"
        "#;

        let config: RulesConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.disable.len(), 2);
        assert_eq!(config.severity.len(), 2);
        assert_eq!(config.custom.len(), 1);
        assert_eq!(config.custom[0].code, "custom/no-todo");
    }
}
