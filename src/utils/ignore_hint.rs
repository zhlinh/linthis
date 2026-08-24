// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! How to silence a lint issue, attached to the issue itself.
//!
//! Every issue linthis reports can be ignored two ways: from the CLI, which
//! writes `.linthisignore`, and in the code, with the comment the responsible
//! tool understands. Both are printed next to the issue and serialized into
//! the JSON result so nobody has to remember the commands.
//!
//! What the code half looks like depends on who found the issue:
//!
//! - an external linter → that linter's own pragma (`# noqa: E501`, …)
//! - linthis's security scan → `linthis:ignore <rule-id>`
//! - linthis's complexity check and custom rules → nothing; they have no
//!   inline form, and a plausible-looking `# noqa: linthis-complexity` would
//!   simply not work

use serde::{Deserialize, Serialize};

use crate::utils::types::LintIssue;
use crate::Language;

/// Source prefix used when a security finding is merged into the issue list.
const SECURITY_PREFIX: &str = "security/";

/// Sources that are linthis's own checks and have no inline pragma.
const CLI_ONLY_SOURCES: &[&str] = &["linthis-complexity", "custom"];

/// The two ways to ignore an issue. Either half may be absent: an issue with
/// no rule code cannot be named in `.linthisignore`, and linthis's own checks
/// have no inline form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IgnoreHint {
    /// Command that adds the rule to `.linthisignore`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli: Option<String>,
    /// Comment that suppresses this issue where it stands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

impl IgnoreHint {
    fn is_empty(&self) -> bool {
        self.cli.is_none() && self.comment.is_none()
    }
}

/// Build the ignore hint for one issue, or `None` when neither half applies.
pub fn hint_for(issue: &LintIssue) -> Option<IgnoreHint> {
    let hint = IgnoreHint {
        cli: cli_hint(issue),
        comment: comment_hint(issue),
    };
    (!hint.is_empty()).then_some(hint)
}

/// Attach hints to every issue. Idempotent, so it is safe to call again on a
/// result loaded back from JSON.
pub fn attach(issues: &mut [LintIssue]) {
    for issue in issues {
        issue.ignore = hint_for(issue);
    }
}

/// `linthis ignore add "rule:<code>"`, when the issue carries a rule code.
fn cli_hint(issue: &LintIssue) -> Option<String> {
    let code = issue.code.as_deref()?.trim();
    (!code.is_empty()).then(|| format!("linthis ignore add \"rule:{code}\""))
}

/// The comment that suppresses this issue in the source file.
fn comment_hint(issue: &LintIssue) -> Option<String> {
    let source = issue.source.as_deref().unwrap_or("");

    if CLI_ONLY_SOURCES.contains(&source) {
        return None;
    }

    if source.starts_with(SECURITY_PREFIX) {
        // `linthis:ignore <target>` matches on the exact rule id, so an issue
        // without a code has no precise directive to offer.
        let rule = issue.code.as_deref()?.trim();
        if rule.is_empty() {
            return None;
        }
        return Some(format!("{} linthis:ignore {rule}", line_comment(issue)));
    }

    Some(crate::interactive::suppression_comment(issue))
}

/// Line-comment marker for the issue's language.
fn line_comment(issue: &LintIssue) -> &'static str {
    let lang = issue
        .language
        .or_else(|| Language::from_path(&issue.file_path));
    match lang {
        Some(Language::Python | Language::Shell | Language::Ruby) => "#",
        Some(Language::Lua) => "--",
        _ => "//",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::types::Severity;
    use std::path::PathBuf;

    fn issue(file: &str, source: &str, code: Option<&str>) -> LintIssue {
        let mut i = LintIssue::new(PathBuf::from(file), 1, "boom".into(), Severity::Error);
        i.source = Some(source.into());
        i.code = code.map(str::to_string);
        i
    }

    #[test]
    fn external_linter_gets_both_halves() {
        let hint = hint_for(&issue("a.py", "ruff", Some("E501"))).unwrap();
        assert_eq!(
            hint.cli.as_deref(),
            Some("linthis ignore add \"rule:E501\"")
        );
        assert_eq!(hint.comment.as_deref(), Some("# noqa: E501"));
    }

    #[test]
    fn security_findings_use_linthis_ignore() {
        let hint = hint_for(&issue("a.py", "security/secrets", Some("aws-key"))).unwrap();
        // Matches suppress::target_matches, which compares against the rule id.
        assert_eq!(hint.comment.as_deref(), Some("# linthis:ignore aws-key"));

        let c_like = hint_for(&issue("a.cpp", "security/opengrep", Some("sql-inject"))).unwrap();
        assert_eq!(
            c_like.comment.as_deref(),
            Some("// linthis:ignore sql-inject")
        );
    }

    #[test]
    fn linthis_own_checks_offer_no_fake_pragma() {
        // `# noqa: linthis-complexity` would look right and do nothing.
        let cx = hint_for(&issue("a.py", "linthis-complexity", Some("linthis-complexity")))
            .unwrap();
        assert!(cx.comment.is_none());
        assert_eq!(
            cx.cli.as_deref(),
            Some("linthis ignore add \"rule:linthis-complexity\"")
        );

        let custom = hint_for(&issue("a.py", "custom", Some("custom/no-todo"))).unwrap();
        assert!(custom.comment.is_none());
        assert!(custom.cli.is_some());
    }

    #[test]
    fn missing_code_drops_the_cli_half() {
        let hint = hint_for(&issue("a.cpp", "clang-tidy", None)).unwrap();
        assert!(hint.cli.is_none());
        // The language still has a generic form worth showing.
        assert_eq!(hint.comment.as_deref(), Some("// NOLINT"));
    }

    #[test]
    fn security_without_a_rule_id_yields_nothing() {
        assert!(hint_for(&issue("a.py", "security/opengrep", None)).is_none());
    }

    #[test]
    fn attach_is_idempotent() {
        let mut issues = vec![issue("a.py", "ruff", Some("E501"))];
        attach(&mut issues);
        let first = issues[0].ignore.clone();
        attach(&mut issues);
        assert_eq!(issues[0].ignore, first);
    }
}
