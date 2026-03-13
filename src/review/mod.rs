//! AI-powered code review module.
//!
//! Analyzes git diffs using AI to find code quality, security, and architecture
//! issues. Supports background execution, auto-fix with PR/MR creation, and
//! configurable notifications.

pub mod analyzer;
pub mod background;
pub mod diff;
pub mod notifier;
pub mod platform;
pub mod prompts;
pub mod report;
pub mod reviewer;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

/// Overall assessment of a code review
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Assessment {
    /// Code is ready to merge
    Ready,
    /// Code needs work before merging
    NeedsWork,
    /// Critical issues found that must be fixed
    CriticalIssues,
}

impl fmt::Display for Assessment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Assessment::Ready => write!(f, "Ready"),
            Assessment::NeedsWork => write!(f, "Needs Work"),
            Assessment::CriticalIssues => write!(f, "Critical Issues"),
        }
    }
}

/// Severity level for a review issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    /// Must fix before merging
    Critical,
    /// Should fix, may block merge
    Important,
    /// Nice to fix, won't block merge
    Minor,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Critical => write!(f, "Critical"),
            Severity::Important => write!(f, "Important"),
            Severity::Minor => write!(f, "Minor"),
        }
    }
}

/// A single issue found during code review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssue {
    /// Severity of the issue
    pub severity: Severity,
    /// Category (e.g., "security", "performance", "style")
    pub category: String,
    /// File path where the issue was found
    pub file: PathBuf,
    /// Line number (if applicable)
    pub line: Option<u32>,
    /// Description of the issue
    pub message: String,
    /// Suggested fix (if available)
    pub suggestion: Option<String>,
}

impl fmt::Display for ReviewIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} ({}): {}",
            self.severity,
            self.file.display(),
            self.category,
            self.message
        )
    }
}

/// Summary statistics for a review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewSummary {
    /// Total number of files reviewed
    pub files_reviewed: usize,
    /// Total number of issues found
    pub total_issues: usize,
    /// Number of critical issues
    pub critical_count: usize,
    /// Number of important issues
    pub important_count: usize,
    /// Number of minor issues
    pub minor_count: usize,
    /// Overall assessment
    pub assessment: Assessment,
    /// AI-generated summary text
    pub summary_text: String,
}

impl fmt::Display for ReviewSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Review: {} ({} files, {} issues: {} critical, {} important, {} minor)",
            self.assessment,
            self.files_reviewed,
            self.total_issues,
            self.critical_count,
            self.important_count,
            self.minor_count
        )
    }
}

/// Status of a file in the diff
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileStatus {
    /// Newly added file
    Added,
    /// Modified existing file
    Modified,
    /// Deleted file
    Deleted,
    /// Renamed file (with original path)
    Renamed { from: PathBuf },
}

impl fmt::Display for FileStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileStatus::Added => write!(f, "Added"),
            FileStatus::Modified => write!(f, "Modified"),
            FileStatus::Deleted => write!(f, "Deleted"),
            FileStatus::Renamed { from } => {
                write!(f, "Renamed from {}", from.display())
            }
        }
    }
}

/// A file that was reviewed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewedFile {
    /// Path to the file
    pub path: PathBuf,
    /// File status in the diff
    pub status: FileStatus,
    /// Issues found in this file
    pub issues: Vec<ReviewIssue>,
}

impl fmt::Display for ReviewedFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}, {} issues)",
            self.path.display(),
            self.status,
            self.issues.len()
        )
    }
}

/// Complete result of a code review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewResult {
    /// Summary of the review
    pub summary: ReviewSummary,
    /// Files that were reviewed
    pub files: Vec<ReviewedFile>,
    /// All issues found across all files
    pub issues: Vec<ReviewIssue>,
    /// Base ref used for the diff
    pub base_ref: String,
    /// Head ref used for the diff
    pub head_ref: String,
}

impl ReviewResult {
    /// Count issues grouped by severity
    pub fn count_by_severity(&self) -> HashMap<Severity, usize> {
        let mut counts = HashMap::new();
        for issue in &self.issues {
            *counts.entry(issue.severity).or_insert(0) += 1;
        }
        counts
    }
}

impl fmt::Display for ReviewResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Review {}..{}: {}",
            self.base_ref, self.head_ref, self.summary
        )
    }
}
