// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Formatter trait definition for language-specific formatters.

use crate::utils::types::FormatResult;
use crate::{Language, Result};
use std::path::Path;

/// Trait for implementing language-specific formatters.
///
/// Each formatter implementation should shell out to an external
/// formatting tool and return whether the file was modified.
pub trait Formatter: Send + Sync {
    /// Returns the name of this formatter (e.g., "rustfmt", "black").
    fn name(&self) -> &str;

    /// Returns the languages this formatter supports.
    fn supported_languages(&self) -> &[Language];

    /// Format a single file in place.
    ///
    /// # Arguments
    /// * `path` - Path to the file to format
    ///
    /// # Returns
    /// A FormatResult indicating whether the file was changed.
    fn format(&self, path: &Path) -> Result<FormatResult>;

    /// Check if formatting would change the file (dry run).
    ///
    /// # Arguments
    /// * `path` - Path to the file to check
    ///
    /// # Returns
    /// true if the file would be modified, false otherwise.
    fn check(&self, path: &Path) -> Result<bool>;

    /// Check if this formatter supports the given language.
    fn supports(&self, lang: Language) -> bool {
        self.supported_languages().contains(&lang)
    }

    /// Check if the external formatter tool is available.
    fn is_available(&self) -> bool;
}
