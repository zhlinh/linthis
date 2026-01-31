// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Checker trait definition for language-specific linters.

use crate::utils::types::LintIssue;
use crate::{Language, Result};
use std::path::Path;

/// Trait for implementing language-specific checkers (linters).
///
/// Each checker implementation should shell out to an external linter
/// and parse its output into standardized LintIssue structs.
pub trait Checker: Send + Sync {
    /// Returns the name of this checker (e.g., "clippy", "pylint").
    fn name(&self) -> &str;

    /// Returns the languages this checker supports.
    fn supported_languages(&self) -> &[Language];

    /// Check a single file and return any lint issues found.
    ///
    /// # Arguments
    /// * `path` - Path to the file to check
    ///
    /// # Returns
    /// A vector of lint issues, or an error if the check failed.
    fn check(&self, path: &Path) -> Result<Vec<LintIssue>>;

    /// Check a single file with an optional external config path.
    ///
    /// This method allows the ConfigResolver to pass a plugin config when
    /// no local config exists. The default implementation ignores the config
    /// and calls `check()`.
    ///
    /// # Arguments
    /// * `path` - Path to the file to check
    /// * `config` - Optional path to a config file to use
    ///
    /// # Returns
    /// A vector of lint issues, or an error if the check failed.
    fn check_with_config(&self, path: &Path, config: Option<&Path>) -> Result<Vec<LintIssue>> {
        // Default implementation: ignore config and use check()
        let _ = config;
        self.check(path)
    }

    /// Check if this checker supports the given language.
    fn supports(&self, lang: Language) -> bool {
        self.supported_languages().contains(&lang)
    }

    /// Check if the external linter tool is available.
    fn is_available(&self) -> bool;
}
