// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! SAST finding data structure for source code security analysis results.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::security::vulnerability::Severity;

/// A security finding from SAST (Static Application Security Testing) analysis.
///
/// Unlike `Vulnerability` which represents dependency/supply-chain issues,
/// `SastFinding` represents security issues found directly in source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SastFinding {
    /// Rule identifier (e.g., "python.lang.security.audit.exec-detected")
    pub rule_id: String,
    /// Severity level
    pub severity: Severity,
    /// Human-readable description of the issue
    pub message: String,
    /// Source file path
    pub file_path: PathBuf,
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based, optional)
    pub column: Option<usize>,
    /// End line number (optional)
    pub end_line: Option<usize>,
    /// End column number (optional)
    pub end_column: Option<usize>,
    /// Code snippet showing the problematic code
    pub code_snippet: Option<String>,
    /// Suggested fix or remediation
    pub fix_suggestion: Option<String>,
    /// Category (e.g., "injection", "crypto", "auth")
    pub category: String,
    /// CWE identifiers (e.g., ["CWE-89", "CWE-564"])
    pub cwe_ids: Vec<String>,
    /// Source tool name (e.g., "opengrep", "bandit", "gosec")
    pub source: String,
    /// Programming language (e.g., "python", "go", "c")
    pub language: String,
}

impl SastFinding {
    /// Get a short summary string
    pub fn summary(&self) -> String {
        format!(
            "[{}] {} ({}:{})",
            self.severity,
            self.rule_id,
            self.file_path.display(),
            self.line,
        )
    }

    /// Check if this finding meets a severity threshold
    pub fn meets_severity_threshold(&self, threshold: &Severity) -> bool {
        self.severity.meets_threshold(threshold)
    }
}
