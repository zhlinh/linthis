// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Watch mode for file system monitoring and auto-linting.
//!
//! This module provides functionality for watching files for changes
//! and automatically running lint checks when modifications are detected.
//!
//! ## Features
//!
//! - **File system watching**: Monitor directories for file changes
//! - **Event debouncing**: Coalesce rapid changes to avoid excessive re-runs
//! - **State management**: Track lint results and file statuses
//! - **Desktop notifications**: Optional alerts when issues are found
//!
//! ## Usage
//!
//! ```rust,no_run
//! use linthis::watch::{WatchConfig, FileWatcher, Debouncer};
//! use std::path::PathBuf;
//!
//! let config = WatchConfig {
//!     paths: vec![PathBuf::from("src/")],
//!     debounce_ms: 300,
//!     ..Default::default()
//! };
//!
//! // Create a file watcher
//! let watcher = FileWatcher::new(config.paths.clone()).unwrap();
//!
//! // Create a debouncer for event coalescing
//! let mut debouncer = Debouncer::new(config.debounce_ms);
//! ```

mod debounce;
mod state;
mod watcher;

#[cfg(feature = "notifications")]
mod notifications;

pub use debounce::Debouncer;
pub use state::{FileStatus, WatchState};
pub use watcher::{FileWatcher, WatchEvent, WatchEventKind};

#[cfg(feature = "notifications")]
pub use notifications::notify_issues;

use std::path::PathBuf;

/// Configuration for watch mode.
#[derive(Debug, Clone)]
pub struct WatchConfig {
    /// Paths to watch (files or directories)
    pub paths: Vec<PathBuf>,
    /// Only check, don't format
    pub check_only: bool,
    /// Only format, don't check
    pub format_only: bool,
    /// Debounce delay in milliseconds
    pub debounce_ms: u64,
    /// Enable desktop notifications
    pub notify: bool,
    /// Disable TUI (use simple stdout output)
    pub no_tui: bool,
    /// Clear screen before each run
    pub clear: bool,
    /// Verbose output
    pub verbose: bool,
    /// Languages to check (empty = auto-detect)
    pub languages: Vec<crate::Language>,
    /// Exclusion patterns
    pub exclude_patterns: Vec<String>,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            paths: vec![PathBuf::from(".")],
            check_only: false,
            format_only: false,
            debounce_ms: 300,
            notify: false,
            no_tui: false,
            clear: false,
            verbose: false,
            languages: Vec::new(),
            exclude_patterns: Vec::new(),
        }
    }
}

/// Result of a watch lint run
#[derive(Debug, Clone, Default)]
pub struct WatchResult {
    /// Total files checked
    pub files_checked: usize,
    /// Number of errors found
    pub error_count: usize,
    /// Number of warnings found
    pub warning_count: usize,
    /// Number of info messages
    pub info_count: usize,
    /// Files that were formatted
    pub files_formatted: usize,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Whether this was a clean run (no issues)
    pub is_clean: bool,
}

impl WatchResult {
    /// Create a new WatchResult from a RunResult
    pub fn from_run_result(result: &crate::utils::types::RunResult) -> Self {
        let mut error_count = 0;
        let mut warning_count = 0;
        let mut info_count = 0;

        for issue in &result.issues {
            match issue.severity {
                crate::Severity::Error => error_count += 1,
                crate::Severity::Warning => warning_count += 1,
                crate::Severity::Info => info_count += 1,
            }
        }

        Self {
            files_checked: result.total_files,
            error_count,
            warning_count,
            info_count,
            files_formatted: result.format_results.len(),
            duration_ms: result.duration_ms,
            is_clean: result.issues.is_empty(),
        }
    }

    /// Get total issue count
    pub fn total_issues(&self) -> usize {
        self.error_count + self.warning_count + self.info_count
    }
}
