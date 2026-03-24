// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! SAST scanner trait and options.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::finding::SastFinding;
use crate::security::vulnerability::Severity;

/// Trait for SAST tools that scan source code for security issues.
pub trait SastScanner: Send + Sync {
    /// Tool name (e.g., "opengrep", "bandit")
    fn name(&self) -> &str;

    /// Languages this scanner supports
    fn supported_languages(&self) -> &[&str];

    /// Check if the tool is installed and available
    fn is_available(&self) -> bool;

    /// Run SAST scan on the given path/files
    ///
    /// - `path`: project root directory
    /// - `files`: specific files to scan (empty = scan entire project)
    /// - `options`: scan configuration
    fn scan(
        &self,
        path: &Path,
        files: &[PathBuf],
        options: &SastScanOptions,
    ) -> Result<Vec<SastFinding>, String>;

    /// Get installation hint for this tool
    fn install_hint(&self) -> String;
}

/// Options for SAST scanning
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SastScanOptions {
    /// Minimum severity to report
    pub severity_threshold: Option<Severity>,
    /// Custom config/rules file path
    pub config_path: Option<PathBuf>,
    /// Specific rule sets to enable
    pub rules: Vec<String>,
    /// Rules to exclude
    pub exclude: Vec<String>,
    /// Verbose output
    pub verbose: bool,
}
