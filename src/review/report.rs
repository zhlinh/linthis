//! Review report generation (Markdown).

use crate::review::{Assessment, ReviewIssue, ReviewResult, Severity};

/// Generate a Markdown report from review results.
pub fn generate_markdown_report(result: &ReviewResult) -> String {
    let mut report = String::new();

    // Title
    report.push_str("# Code Review Report\n\n");

    // Summary table
    let assessment_icon = match result.summary.assessment {
        Assessment::Ready => "✅ Ready",
        Assessment::NeedsWork => "⚠️ Needs Work",
        Assessment::CriticalIssues => "❌ Critical Issues",
    };

    report.push_str("## Summary\n");
    report.push_str("| Metric | Value |\n");
    report.push_str("|--------|-------|\n");
    report.push_str(&format!(
        "| Files Reviewed | {} |\n",
        result.summary.files_reviewed
    ));
    report.push_str(&format!(
        "| Total Issues | {} |\n",
        result.summary.total_issues
    ));
    report.push_str(&format!("| Assessment | {} |\n", assessment_icon));
    report.push('\n');

    if !result.summary.summary_text.is_empty() {
        report.push_str(&format!("{}\n\n", result.summary.summary_text));
    }

    // Changed files table
    if !result.files.is_empty() {
        report.push_str("## Changed Files\n");
        report.push_str("| File | Status | Issues |\n");
        report.push_str("|------|--------|--------|\n");
        for file in &result.files {
            report.push_str(&format!(
                "| {} | {} | {} |\n",
                file.path.display(),
                file.status,
                file.issues.len()
            ));
        }
        report.push('\n');
    }

    // Issues by severity
    let critical: Vec<&ReviewIssue> = result
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Critical)
        .collect();
    let important: Vec<&ReviewIssue> = result
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Important)
        .collect();
    let minor: Vec<&ReviewIssue> = result
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Minor)
        .collect();

    if !result.issues.is_empty() {
        report.push_str("## Issues Found\n\n");

        if !critical.is_empty() {
            report.push_str(&format!("### Critical ({})\n", critical.len()));
            for issue in &critical {
                format_issue(&mut report, issue);
            }
            report.push('\n');
        }

        if !important.is_empty() {
            report.push_str(&format!("### Important ({})\n", important.len()));
            for issue in &important {
                format_issue(&mut report, issue);
            }
            report.push('\n');
        }

        if !minor.is_empty() {
            report.push_str(&format!("### Minor ({})\n", minor.len()));
            for issue in &minor {
                format_issue(&mut report, issue);
            }
            report.push('\n');
        }
    }

    // Auto-fixed issues
    if !result.auto_fixes.is_empty() {
        report.push_str("## Auto-Fixed Issues\n\n");
        report.push_str("The following issues were automatically fixed:\n\n");
        for fix in &result.auto_fixes {
            let location = if let Some(line) = fix.line {
                format!("{}:{}", fix.file.display(), line)
            } else {
                fix.file.display().to_string()
            };
            report.push_str(&format!("- {} — {}\n", location, fix.description));
        }
        report.push('\n');
    }

    // Assessment
    report.push_str("## Assessment\n");
    match result.summary.assessment {
        Assessment::Ready => {
            report.push_str(
                "**Ready** — No critical or important issues found. Code is ready for merge.\n",
            );
        }
        Assessment::NeedsWork => {
            report.push_str(&format!(
                "**Needs Work** — {} critical and {} important issue(s) should be addressed before merge.\n",
                result.summary.critical_count, result.summary.important_count
            ));
        }
        Assessment::CriticalIssues => {
            report.push_str(&format!(
                "**Critical Issues** — {} critical issue(s) must be resolved before merge.\n",
                result.summary.critical_count
            ));
        }
    }

    report
}

fn format_issue(report: &mut String, issue: &ReviewIssue) {
    let location = if let Some(line) = issue.line {
        format!("{}:{}", issue.file.display(), line)
    } else {
        issue.file.display().to_string()
    };

    report.push_str(&format!(
        "- **[{}] {}** — {}\n",
        issue.category, location, issue.message
    ));

    if let Some(ref suggestion) = issue.suggestion {
        report.push_str(&format!("  - Suggestion: {}\n", suggestion));
    }
}

/// Generate a JSON report from review results.
pub fn generate_json_report(result: &ReviewResult) -> String {
    serde_json::to_string_pretty(result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

/// Generate a one-line summary for notifications.
pub fn generate_notification_summary(result: &ReviewResult) -> String {
    format!(
        "Review: {} | {} files | {} critical, {} important, {} minor issues",
        result.summary.assessment,
        result.summary.files_reviewed,
        result.summary.critical_count,
        result.summary.important_count,
        result.summary.minor_count,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::{
        Assessment, FileStatus, ReviewIssue, ReviewResult, ReviewSummary, ReviewedFile, Severity,
    };
    use std::path::PathBuf;

    fn sample_result() -> ReviewResult {
        ReviewResult {
            summary: ReviewSummary {
                files_reviewed: 1,
                total_issues: 2,
                critical_count: 1,
                important_count: 0,
                minor_count: 1,
                assessment: Assessment::CriticalIssues,
                summary_text: String::new(),
            },
            files: vec![ReviewedFile {
                path: PathBuf::from("src/main.rs"),
                status: FileStatus::Modified,
                issues: vec![],
            }],
            issues: vec![
                ReviewIssue {
                    severity: Severity::Critical,
                    category: "security".to_string(),
                    file: PathBuf::from("src/main.rs"),
                    line: Some(42),
                    message: "User input not sanitized".to_string(),
                    suggestion: Some("Use parameterized queries".to_string()),
                },
                ReviewIssue {
                    severity: Severity::Minor,
                    category: "style".to_string(),
                    file: PathBuf::from("src/main.rs"),
                    line: Some(10),
                    message: "Use snake_case".to_string(),
                    suggestion: None,
                },
            ],
            base_ref: "main".to_string(),
            head_ref: "feature".to_string(),
            auto_fixes: vec![],
        }
    }

    #[test]
    fn test_markdown_report_contains_sections() {
        let report = generate_markdown_report(&sample_result());
        assert!(report.contains("# Code Review Report"));
        assert!(report.contains("## Summary"));
        assert!(report.contains("## Changed Files"));
        assert!(report.contains("## Issues Found"));
        assert!(report.contains("### Critical (1)"));
        assert!(report.contains("### Minor (1)"));
        assert!(report.contains("## Assessment"));
    }

    #[test]
    fn test_markdown_report_issue_formatting() {
        let report = generate_markdown_report(&sample_result());
        assert!(report.contains("src/main.rs:42"));
        assert!(report.contains("Suggestion: Use parameterized queries"));
    }

    #[test]
    fn test_notification_summary() {
        let summary = generate_notification_summary(&sample_result());
        assert!(summary.contains("Critical Issues"));
        assert!(summary.contains("1 critical"));
        assert!(summary.contains("1 minor"));
    }

    #[test]
    fn test_empty_result_report() {
        let result = ReviewResult {
            summary: ReviewSummary {
                files_reviewed: 0,
                total_issues: 0,
                critical_count: 0,
                important_count: 0,
                minor_count: 0,
                assessment: Assessment::Ready,
                summary_text: String::new(),
            },
            files: vec![],
            issues: vec![],
            base_ref: "main".to_string(),
            head_ref: "feature".to_string(),
            auto_fixes: vec![],
        };
        let report = generate_markdown_report(&result);
        assert!(report.contains("Ready"));
        assert!(!report.contains("## Issues Found"));
    }
}
