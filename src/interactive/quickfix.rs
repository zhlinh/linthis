// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Vim quickfix format output generation.
//!
//! Generates output in the standard vim quickfix format:
//! ```text
//! file:line:column:message
//! ```
//!
//! This can be used with:
//! - `vim -q quickfix.txt` to open vim with the quickfix list
//! - `:cfile quickfix.txt` in vim to load the quickfix list
//! - Other editors that support the quickfix format

use crate::utils::types::{LintIssue, RunResult, Severity};
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Generate quickfix format string from a list of issues
pub fn generate_quickfix(issues: &[LintIssue]) -> String {
    issues
        .iter()
        .map(format_issue_quickfix)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Generate quickfix format string from a RunResult
pub fn generate_quickfix_from_result(result: &RunResult) -> String {
    generate_quickfix(&result.issues)
}

/// Format a single issue in quickfix format
///
/// Format: `file:line:column:severity:message (code)`
fn format_issue_quickfix(issue: &LintIssue) -> String {
    let file = issue.file_path.display();
    let line = issue.line;
    let col = issue.column.unwrap_or(1);

    // Severity prefix for vim
    let severity = match issue.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    };

    // Build message with optional code
    let message = if let Some(ref code) = issue.code {
        format!("{} ({}) [{}]", issue.message, code, severity)
    } else {
        format!("{} [{}]", issue.message, severity)
    };

    // Escape special characters in message
    let message = message.replace('\n', " ").replace('\r', "");

    format!("{}:{}:{}:{}", file, line, col, message)
}

/// Write quickfix format to a file
///
/// # Arguments
/// * `issues` - List of lint issues
/// * `path` - Path to write the quickfix file
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(String)` on failure
pub fn write_quickfix_file(issues: &[LintIssue], path: &Path) -> super::InteractiveResult<()> {
    use super::InteractiveError;

    let content = generate_quickfix(issues);

    let mut file = File::create(path).map_err(|e| {
        InteractiveError::QuickfixWrite(format!("Failed to create file: {}", e))
    })?;

    file.write_all(content.as_bytes()).map_err(|e| {
        InteractiveError::QuickfixWrite(format!("Failed to write content: {}", e))
    })?;

    // Ensure trailing newline
    if !content.is_empty() && !content.ends_with('\n') {
        file.write_all(b"\n").map_err(|e| {
            InteractiveError::QuickfixWrite(format!("Failed to write newline: {}", e))
        })?;
    }

    Ok(())
}

/// Get the default quickfix file path
pub fn default_quickfix_path() -> std::path::PathBuf {
    std::path::PathBuf::from(".linthis").join("quickfix.txt")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_issue(file: &str, line: usize, col: Option<usize>, severity: Severity, msg: &str, code: Option<&str>) -> LintIssue {
        let mut issue = LintIssue::new(
            PathBuf::from(file),
            line,
            msg.to_string(),
            severity,
        );
        if let Some(c) = col {
            issue = issue.with_column(c);
        }
        if let Some(code) = code {
            issue = issue.with_code(code.to_string());
        }
        issue
    }

    #[test]
    fn test_format_issue_quickfix_basic() {
        let issue = make_issue("src/main.rs", 42, Some(10), Severity::Error, "unused variable", Some("W0612"));
        let formatted = format_issue_quickfix(&issue);
        assert_eq!(formatted, "src/main.rs:42:10:unused variable (W0612) [error]");
    }

    #[test]
    fn test_format_issue_quickfix_no_column() {
        let issue = make_issue("test.py", 100, None, Severity::Warning, "line too long", Some("E501"));
        let formatted = format_issue_quickfix(&issue);
        assert_eq!(formatted, "test.py:100:1:line too long (E501) [warning]");
    }

    #[test]
    fn test_format_issue_quickfix_no_code() {
        let issue = make_issue("file.cpp", 5, Some(1), Severity::Info, "consider using const", None);
        let formatted = format_issue_quickfix(&issue);
        assert_eq!(formatted, "file.cpp:5:1:consider using const [info]");
    }

    #[test]
    fn test_generate_quickfix_multiple() {
        let issues = vec![
            make_issue("a.rs", 1, Some(1), Severity::Error, "error 1", Some("E001")),
            make_issue("b.rs", 2, Some(5), Severity::Warning, "warning 1", Some("W001")),
        ];
        let output = generate_quickfix(&issues);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("a.rs:1:1:"));
        assert!(lines[1].starts_with("b.rs:2:5:"));
    }

    #[test]
    fn test_generate_quickfix_empty() {
        let issues: Vec<LintIssue> = vec![];
        let output = generate_quickfix(&issues);
        assert!(output.is_empty());
    }

    #[test]
    fn test_format_issue_quickfix_escapes_newlines() {
        let issue = make_issue("test.rs", 1, Some(1), Severity::Error, "line1\nline2\rline3", None);
        let formatted = format_issue_quickfix(&issue);
        assert!(!formatted.contains('\n'));
        assert!(!formatted.contains('\r'));
    }
}
