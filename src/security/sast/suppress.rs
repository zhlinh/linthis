// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Inline `linthis:ignore` suppression for SAST findings.
//!
//! A source line can carry a directive that suppresses security findings on
//! that line (or, with `-next-line`, on the following line). This applies
//! uniformly to every SAST tool (secrets, opengrep, bandit, gosec, flawfinder).
//!
//! # Syntax
//!
//! - `# linthis:ignore security`               — suppress all security findings on the line
//! - `# linthis:ignore secrets`                — suppress findings from one tool
//! - `# linthis:ignore secrets/aws-access-key` — suppress one specific rule
//! - `// linthis:ignore opengrep`              — any comment style works
//! - `# linthis:ignore-next-line security`     — apply to the next line instead
//!
//! The comment marker (`#`, `//`, `/* */`, ...) is irrelevant — only the
//! `linthis:ignore` token and its target are parsed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::finding::SastFinding;

/// Target keyword that matches every security finding, regardless of tool.
const TARGET_ALL: &str = "security";

/// Prefix stripped from a finding's `source` to derive its short tool name
/// (e.g. `linthis-secrets` -> `secrets`).
const SOURCE_PREFIX: &str = "linthis-";

/// Remove findings suppressed by an inline `linthis:ignore` directive.
///
/// Each finding is anchored to a source line; the file is read once (results
/// cached) and the finding is dropped when the directive on its own line, or a
/// `-next-line` directive on the preceding line, targets it. Files that cannot
/// be read are treated as having no directives (finding is kept).
///
/// `scan_dir` is used to resolve findings that carry a path relative to the
/// scan root (e.g. tool output paths).
pub fn filter_suppressed(findings: Vec<SastFinding>, scan_dir: &Path) -> Vec<SastFinding> {
    // Cache file contents (split into lines) keyed by the finding's path.
    // `None` means the file could not be read.
    let mut cache: HashMap<PathBuf, Option<Vec<String>>> = HashMap::new();

    findings
        .into_iter()
        .filter(|finding| {
            let lines = cache
                .entry(finding.file_path.clone())
                .or_insert_with(|| read_lines(&finding.file_path, scan_dir));

            match lines {
                Some(lines) => !is_suppressed(finding, lines),
                None => true,
            }
        })
        .collect()
}

/// Read a finding's file into lines, trying the path as given and then relative
/// to `scan_dir` (tools may report paths relative to the scan root).
fn read_lines(file_path: &Path, scan_dir: &Path) -> Option<Vec<String>> {
    let content = std::fs::read_to_string(file_path)
        .or_else(|_| std::fs::read_to_string(scan_dir.join(file_path)))
        .ok()?;
    Some(content.lines().map(|l| l.to_string()).collect())
}

/// Determine whether a finding is suppressed by a directive on its own line or a
/// `-next-line` directive on the preceding line.
fn is_suppressed(finding: &SastFinding, lines: &[String]) -> bool {
    // Finding lines are 1-based.
    let Some(idx) = finding.line.checked_sub(1) else {
        return false;
    };

    // Same-line directive.
    if let Some(line) = lines.get(idx) {
        if let Some(target) = parse_ignore_directive(line) {
            if target_matches(&target, finding) {
                return true;
            }
        }
    }

    // Preceding line: the explicit `-next-line` form, or a bare directive
    // sitting alone on a comment line. The latter has no code of its own to
    // suppress, and putting the hint linthis prints on the line above the
    // finding is what people reach for first, so read it as the next-line form.
    if let Some(prev_idx) = idx.checked_sub(1) {
        if let Some(prev) = lines.get(prev_idx) {
            let target = parse_ignore_next_line_directive(prev)
                .or_else(|| is_directive_only_line(prev).then(|| parse_ignore_directive(prev))?);
            if let Some(target) = target {
                if target_matches(&target, finding) {
                    return true;
                }
            }
        }
    }

    false
}

/// Comment markers a directive-only line may open with, longest first so that
/// `///` is not mistaken for `//` with a stray slash.
const COMMENT_MARKERS: &[&str] = &[
    "<!--", "///", "//", "/*", "--", "#", ";", "%", "*",
];

/// Is this line nothing but a comment holding a `linthis:ignore` directive?
///
/// Whitespace and one leading comment marker are stripped; anything else
/// before the directive (i.e. actual code) makes it a same-line directive.
fn is_directive_only_line(line: &str) -> bool {
    let mut rest = line.trim_start();
    for marker in COMMENT_MARKERS {
        if let Some(stripped) = rest.strip_prefix(marker) {
            rest = stripped.trim_start();
            break;
        }
    }
    rest.starts_with("linthis:ignore")
}

/// Check whether an ignore `target` applies to a finding.
///
/// Matches when the target is the catch-all `security`, the finding's tool name
/// (source, with any `linthis-` prefix stripped), or the exact rule id.
fn target_matches(target: &str, finding: &SastFinding) -> bool {
    if target == TARGET_ALL {
        return true;
    }

    let tool = finding
        .source
        .strip_prefix(SOURCE_PREFIX)
        .unwrap_or(&finding.source);

    target == tool || target == finding.source || target == finding.rule_id
}

/// Parse an inline ignore directive from a line.
///
/// Returns the ignore target, or `None` if the line has no `linthis:ignore`
/// directive (or it is the distinct `-next-line` variant).
pub fn parse_ignore_directive(line: &str) -> Option<String> {
    let marker = "linthis:ignore";
    let pos = line.find(marker)?;

    let after_marker = &line[pos + marker.len()..];
    // Reject "linthis:ignore-next-line" — that's a different directive.
    if after_marker.starts_with("-next-line") {
        return None;
    }

    extract_ignore_target(after_marker)
}

/// Parse an `ignore-next-line` directive from a line.
///
/// Returns the ignore target, or `None` if not found.
pub fn parse_ignore_next_line_directive(line: &str) -> Option<String> {
    let marker = "linthis:ignore-next-line";
    let pos = line.find(marker)?;
    let after_marker = &line[pos + marker.len()..];
    extract_ignore_target(after_marker)
}

/// Extract the target token from text following a `linthis:ignore` marker.
fn extract_ignore_target(after_marker: &str) -> Option<String> {
    let trimmed = after_marker.trim();
    if trimmed.is_empty() {
        return None;
    }

    let target = trimmed
        .split_whitespace()
        .next()?
        .trim_end_matches("*/")
        .trim();

    if target.is_empty() {
        return None;
    }

    Some(target.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::vulnerability::Severity;

    fn finding(source: &str, rule_id: &str, line: usize) -> SastFinding {
        SastFinding {
            rule_id: rule_id.to_string(),
            severity: Severity::High,
            message: "test".to_string(),
            file_path: PathBuf::from("test.py"),
            line,
            column: None,
            end_line: None,
            end_column: None,
            code_snippet: None,
            fix_suggestion: None,
            category: "test".to_string(),
            cwe_ids: Vec::new(),
            source: source.to_string(),
            language: "python".to_string(),
        }
    }

    #[test]
    fn bare_directive_alone_on_its_own_line_covers_the_next_line() {
        // The hint linthis prints is `// linthis:ignore <rule>`; users put it
        // on the line above the finding.
        let lines: Vec<String> = [
            "credentials {",
            "    // linthis:ignore secrets/generic-password",
            "    password = \"hunter2\"",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        let f = finding("linthis-secrets", "secrets/generic-password", 3);
        assert!(is_suppressed(&f, &lines));

        // Still scoped to one rule.
        let other = finding("linthis-secrets", "secrets/aws-access-key", 3);
        assert!(!is_suppressed(&other, &lines));
    }

    #[test]
    fn directive_trailing_real_code_stays_same_line_only() {
        let lines: Vec<String> = [
            "x = 1  // linthis:ignore security",
            "password = \"hunter2\"",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        assert!(is_suppressed(&finding("linthis-secrets", "r", 1), &lines));
        assert!(!is_suppressed(&finding("linthis-secrets", "r", 2), &lines));
    }

    #[test]
    fn recognises_directive_only_lines_across_comment_styles() {
        for line in [
            "// linthis:ignore security",
            "  # linthis:ignore security",
            "/* linthis:ignore security */",
            "-- linthis:ignore security",
            " * linthis:ignore security",
            "<!-- linthis:ignore security -->",
        ] {
            assert!(is_directive_only_line(line), "{line}");
        }
        for line in ["x = 1 // linthis:ignore security", "let y = 2;  # linthis:ignore x"] {
            assert!(!is_directive_only_line(line), "{line}");
        }
    }

    #[test]
    fn parses_same_line_target() {
        assert_eq!(
            parse_ignore_directive("x = 1  # linthis:ignore security"),
            Some("security".to_string())
        );
        assert_eq!(
            parse_ignore_directive("x = 1  // linthis:ignore opengrep"),
            Some("opengrep".to_string())
        );
        assert_eq!(parse_ignore_directive("x = 1  # no directive"), None);
    }

    #[test]
    fn next_line_marker_is_not_a_same_line_directive() {
        assert_eq!(
            parse_ignore_directive("# linthis:ignore-next-line security"),
            None
        );
        assert_eq!(
            parse_ignore_next_line_directive("# linthis:ignore-next-line security"),
            Some("security".to_string())
        );
    }

    #[test]
    fn extracts_target_from_block_comment() {
        assert_eq!(
            parse_ignore_directive("x = 1  /* linthis:ignore bandit */"),
            Some("bandit".to_string())
        );
    }

    #[test]
    fn security_target_matches_any_tool() {
        assert!(target_matches(
            "security",
            &finding("opengrep", "python.audit.exec", 1)
        ));
        assert!(target_matches(
            "security",
            &finding("linthis-secrets", "secrets/aws-access-key", 1)
        ));
    }

    #[test]
    fn tool_target_matches_by_short_and_full_source() {
        // linthis-secrets -> secrets
        assert!(target_matches(
            "secrets",
            &finding("linthis-secrets", "secrets/aws-access-key", 1)
        ));
        assert!(target_matches(
            "linthis-secrets",
            &finding("linthis-secrets", "secrets/aws-access-key", 1)
        ));
        assert!(target_matches(
            "opengrep",
            &finding("opengrep", "python.audit.exec", 1)
        ));
        // Wrong tool does not match.
        assert!(!target_matches(
            "bandit",
            &finding("opengrep", "python.audit.exec", 1)
        ));
    }

    #[test]
    fn rule_id_target_matches_exact_rule_only() {
        let f = finding("opengrep", "python.audit.exec", 1);
        assert!(target_matches("python.audit.exec", &f));
        assert!(!target_matches("python.audit.other", &f));
    }

    #[test]
    fn suppresses_on_same_line() {
        let lines: Vec<String> = vec!["exec(cmd)  # linthis:ignore opengrep".to_string()];
        let f = finding("opengrep", "python.audit.exec", 1);
        assert!(is_suppressed(&f, &lines));
    }

    #[test]
    fn suppresses_on_next_line() {
        let lines: Vec<String> = vec![
            "# linthis:ignore-next-line security".to_string(),
            "exec(cmd)".to_string(),
        ];
        let f = finding("opengrep", "python.audit.exec", 2);
        assert!(is_suppressed(&f, &lines));
    }

    #[test]
    fn does_not_suppress_unrelated_tool() {
        let lines: Vec<String> = vec!["exec(cmd)  # linthis:ignore bandit".to_string()];
        let f = finding("opengrep", "python.audit.exec", 1);
        assert!(!is_suppressed(&f, &lines));
    }

    #[test]
    fn does_not_suppress_without_directive() {
        let lines: Vec<String> = vec!["exec(cmd)".to_string()];
        let f = finding("opengrep", "python.audit.exec", 1);
        assert!(!is_suppressed(&f, &lines));
    }
}
