// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Custom regex-based rule checker.

use regex::Regex;
use std::fs;
use std::path::Path;

use super::config::CustomRule;
use crate::utils::types::{LintIssue, Severity};
use crate::Result;

/// A compiled custom rule with pre-compiled regex pattern.
#[derive(Debug)]
struct CompiledRule {
    /// Original rule configuration.
    rule: CustomRule,
    /// Pre-compiled regex pattern.
    regex: Regex,
}

/// Checker for custom regex-based lint rules.
#[derive(Debug)]
pub struct CustomRulesChecker {
    /// Compiled rules ready for matching.
    rules: Vec<CompiledRule>,
}

impl CustomRulesChecker {
    /// Create a new custom rules checker from a list of custom rules.
    ///
    /// Returns an error if any rule has an invalid regex pattern.
    pub fn new(rules: &[CustomRule]) -> Result<Self> {
        let compiled_rules: Result<Vec<CompiledRule>> = rules
            .iter()
            .filter(|r| r.enabled)
            .map(|rule| {
                let regex = Regex::new(&rule.pattern).map_err(|e| {
                    crate::LintisError::Config(format!(
                        "Invalid regex pattern in rule '{}': {}",
                        rule.code, e
                    ))
                })?;
                Ok(CompiledRule {
                    rule: rule.clone(),
                    regex,
                })
            })
            .collect();

        Ok(Self {
            rules: compiled_rules?,
        })
    }

    /// Check if this checker has any rules.
    pub fn has_rules(&self) -> bool {
        !self.rules.is_empty()
    }

    /// Get the number of active rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Check a file against all custom rules.
    ///
    /// # Arguments
    /// * `path` - Path to the file to check
    /// * `language` - Optional language name for filtering rules
    ///
    /// # Returns
    /// A vector of lint issues found, or an error if the file cannot be read.
    pub fn check(&self, path: &Path, language: Option<&str>) -> Result<Vec<LintIssue>> {
        if self.rules.is_empty() {
            return Ok(Vec::new());
        }

        // Get file extension
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        // Read file content
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                // Skip binary files or files we can't read
                if e.kind() == std::io::ErrorKind::InvalidData {
                    return Ok(Vec::new());
                }
                return Err(crate::LintisError::Io(e));
            }
        };

        let mut issues = Vec::new();

        // Check each rule
        for compiled in &self.rules {
            // Check if rule applies to this file
            if !compiled.rule.applies_to_extension(extension) {
                continue;
            }
            if let Some(lang) = language {
                if !compiled.rule.applies_to_language(lang) {
                    continue;
                }
            }

            // Find all matches
            for (line_num, line) in content.lines().enumerate() {
                for mat in compiled.regex.find_iter(line) {
                    let mut issue = LintIssue::new(
                        path.to_path_buf(),
                        line_num + 1, // 1-indexed
                        compiled.rule.message.clone(),
                        compiled.rule.severity,
                    );
                    issue.code = Some(compiled.rule.code.clone());
                    issue.column = Some(mat.start() + 1); // 1-indexed
                    issue.source = Some("custom".to_string());
                    issue.suggestion = compiled.rule.suggestion.clone();

                    // Add the source line for context
                    issue.code_line = Some(line.to_string());

                    issues.push(issue);
                }
            }
        }

        Ok(issues)
    }

    /// Get information about all active rules.
    pub fn rule_info(&self) -> Vec<RuleInfo> {
        self.rules
            .iter()
            .map(|r| RuleInfo {
                code: r.rule.code.clone(),
                pattern: r.rule.pattern.clone(),
                severity: r.rule.severity,
                has_suggestion: r.rule.suggestion.is_some(),
                extensions: r.rule.extensions.clone(),
                languages: r.rule.languages.clone(),
            })
            .collect()
    }
}

/// Information about a custom rule.
#[derive(Debug, Clone)]
pub struct RuleInfo {
    /// Rule code.
    pub code: String,
    /// Regex pattern.
    pub pattern: String,
    /// Severity level.
    pub severity: Severity,
    /// Whether a suggestion is provided.
    pub has_suggestion: bool,
    /// File extensions filter.
    pub extensions: Vec<String>,
    /// Languages filter.
    pub languages: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::with_suffix(".rs").unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_custom_checker_empty() {
        let checker = CustomRulesChecker::new(&[]).unwrap();
        assert!(!checker.has_rules());
        assert_eq!(checker.rule_count(), 0);
    }

    #[test]
    fn test_custom_checker_simple_pattern() {
        let rules = vec![CustomRule::new(
            "custom/no-todo",
            "TODO",
            "Found TODO comment",
        )];

        let checker = CustomRulesChecker::new(&rules).unwrap();
        assert!(checker.has_rules());
        assert_eq!(checker.rule_count(), 1);

        let file = create_test_file("// TODO: fix this\nlet x = 1;\n// Another TODO here");
        let issues = checker.check(file.path(), None).unwrap();

        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].line, 1);
        assert_eq!(issues[0].code.as_deref(), Some("custom/no-todo"));
        assert_eq!(issues[1].line, 3);
    }

    #[test]
    fn test_custom_checker_with_severity() {
        let rules = vec![CustomRule::new("custom/no-fixme", "FIXME|XXX", "Found FIXME/XXX")
            .with_severity(Severity::Error)];

        let checker = CustomRulesChecker::new(&rules).unwrap();

        let file = create_test_file("// FIXME: urgent\n// XXX: hack");
        let issues = checker.check(file.path(), None).unwrap();

        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].severity, Severity::Error);
        assert_eq!(issues[1].severity, Severity::Error);
    }

    #[test]
    fn test_custom_checker_extension_filter() {
        let rules = vec![
            CustomRule::new("custom/py-print", r"print\s*\(", "No print statements")
                .with_extensions(vec!["py".to_string()]),
        ];

        let checker = CustomRulesChecker::new(&rules).unwrap();

        // Create a .rs file (should not match)
        let rs_file = create_test_file("print(\"hello\")");
        let issues = checker.check(rs_file.path(), None).unwrap();
        assert_eq!(issues.len(), 0); // .rs doesn't match .py filter

        // Create a .py file (should match)
        let mut py_file = NamedTempFile::with_suffix(".py").unwrap();
        py_file.write_all(b"print(\"hello\")").unwrap();
        py_file.flush().unwrap();
        let issues = checker.check(py_file.path(), None).unwrap();
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn test_custom_checker_language_filter() {
        let rules = vec![
            CustomRule::new("custom/rust-unwrap", r"\.unwrap\(\)", "Avoid unwrap()")
                .with_languages(vec!["rust".to_string()]),
        ];

        let checker = CustomRulesChecker::new(&rules).unwrap();

        let file = create_test_file("let x = foo.unwrap();");

        // Without language hint, rule applies (empty language filter matches all)
        let issues = checker.check(file.path(), None).unwrap();
        assert_eq!(issues.len(), 1);

        // With matching language
        let issues = checker.check(file.path(), Some("rust")).unwrap();
        assert_eq!(issues.len(), 1);

        // With non-matching language
        let issues = checker.check(file.path(), Some("python")).unwrap();
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn test_custom_checker_disabled_rule() {
        let mut rule = CustomRule::new("custom/disabled", "test", "Test");
        rule.enabled = false;

        let checker = CustomRulesChecker::new(&[rule]).unwrap();
        assert!(!checker.has_rules());
        assert_eq!(checker.rule_count(), 0);
    }

    #[test]
    fn test_custom_checker_invalid_regex() {
        let rules = vec![CustomRule::new(
            "custom/bad-regex",
            "[invalid(regex",
            "Bad pattern",
        )];

        let result = CustomRulesChecker::new(&rules);
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_checker_with_suggestion() {
        let rules = vec![CustomRule::new(
            "custom/no-print",
            r"println!\(",
            "Direct println! found",
        )
        .with_suggestion("Use log::info! or log::debug! instead")];

        let checker = CustomRulesChecker::new(&rules).unwrap();

        let file = create_test_file("fn main() { println!(\"hello\"); }");
        let issues = checker.check(file.path(), None).unwrap();

        assert_eq!(issues.len(), 1);
        assert!(issues[0].suggestion.is_some());
        assert!(issues[0].suggestion.as_ref().unwrap().contains("log::info"));
    }

    #[test]
    fn test_custom_checker_column_tracking() {
        let rules = vec![CustomRule::new("custom/test", "TODO", "Found TODO")];

        let checker = CustomRulesChecker::new(&rules).unwrap();

        let file = create_test_file("   // TODO: test");
        let issues = checker.check(file.path(), None).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].column, Some(7)); // 1-indexed, after "   // "
    }

    #[test]
    fn test_rule_info() {
        let rules = vec![
            CustomRule::new("custom/rule1", "pattern1", "Message 1")
                .with_suggestion("Fix 1")
                .with_extensions(vec!["rs".to_string()]),
            CustomRule::new("custom/rule2", "pattern2", "Message 2")
                .with_severity(Severity::Error),
        ];

        let checker = CustomRulesChecker::new(&rules).unwrap();
        let info = checker.rule_info();

        assert_eq!(info.len(), 2);
        assert_eq!(info[0].code, "custom/rule1");
        assert!(info[0].has_suggestion);
        assert_eq!(info[0].extensions, vec!["rs"]);
        assert_eq!(info[1].severity, Severity::Error);
    }
}
