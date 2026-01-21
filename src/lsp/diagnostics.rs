// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Conversion utilities from linthis types to LSP types.

use crate::utils::types::{LintIssue, Severity};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};

/// Convert linthis Severity to LSP DiagnosticSeverity.
pub fn to_lsp_severity(severity: Severity) -> DiagnosticSeverity {
    match severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Info => DiagnosticSeverity::INFORMATION,
    }
}

/// Convert a LintIssue to an LSP Diagnostic.
pub fn to_diagnostic(issue: &LintIssue) -> Diagnostic {
    // LSP uses 0-indexed line and column numbers
    let line = issue.line.saturating_sub(1) as u32;
    let col = issue.column.unwrap_or(1).saturating_sub(1) as u32;

    // Calculate end position - use the entire line if we don't have precise info
    let end_col = if let Some(code_line) = &issue.code_line {
        code_line.len() as u32
    } else {
        col + 1
    };

    Diagnostic {
        range: Range {
            start: Position {
                line,
                character: col,
            },
            end: Position {
                line,
                character: end_col,
            },
        },
        severity: Some(to_lsp_severity(issue.severity)),
        code: issue.code.clone().map(NumberOrString::String),
        code_description: None,
        source: Some(match &issue.source {
            Some(s) => format!("linthis-{}", s),
            None => "linthis".to_string(),
        }),
        message: format_message(issue),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Format the diagnostic message, including suggestion if available.
fn format_message(issue: &LintIssue) -> String {
    if let Some(ref suggestion) = issue.suggestion {
        format!("{}\n\nSuggestion: {}", issue.message, suggestion)
    } else {
        issue.message.clone()
    }
}

/// Convert a list of LintIssues to LSP Diagnostics.
pub fn to_diagnostics(issues: &[LintIssue]) -> Vec<Diagnostic> {
    issues.iter().map(to_diagnostic).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_severity_mapping() {
        assert_eq!(to_lsp_severity(Severity::Error), DiagnosticSeverity::ERROR);
        assert_eq!(
            to_lsp_severity(Severity::Warning),
            DiagnosticSeverity::WARNING
        );
        assert_eq!(
            to_lsp_severity(Severity::Info),
            DiagnosticSeverity::INFORMATION
        );
    }

    #[test]
    fn test_to_diagnostic() {
        let issue = LintIssue::new(
            PathBuf::from("test.py"),
            10,
            "Unused variable".to_string(),
            Severity::Warning,
        )
        .with_column(5)
        .with_code("W0612".to_string())
        .with_source("ruff".to_string());

        let diag = to_diagnostic(&issue);

        assert_eq!(diag.range.start.line, 9); // 0-indexed
        assert_eq!(diag.range.start.character, 4); // 0-indexed
        assert_eq!(diag.severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(diag.code, Some(NumberOrString::String("W0612".to_string())));
        assert_eq!(diag.source, Some("linthis-ruff".to_string()));
        assert!(diag.message.contains("Unused variable"));
    }

    #[test]
    fn test_diagnostic_with_suggestion() {
        let issue = LintIssue::new(
            PathBuf::from("test.py"),
            1,
            "Error message".to_string(),
            Severity::Error,
        )
        .with_suggestion("Fix it this way".to_string());

        let diag = to_diagnostic(&issue);

        assert!(diag.message.contains("Error message"));
        assert!(diag.message.contains("Suggestion:"));
        assert!(diag.message.contains("Fix it this way"));
    }
}
