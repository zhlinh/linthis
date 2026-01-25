// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Interactive mode for reviewing lint issues one by one.
//!
//! Provides a TUI-like experience for:
//! - Reviewing issues interactively
//! - Opening files in editor at the issue location
//! - Adding NOLINT comments to suppress specific issues
//! - Generating vim quickfix format output
//! - AI-powered fix suggestions

pub mod ai_fix;
mod editor;
mod menu;
mod nolint;
mod quickfix;

use thiserror::Error;

/// Errors that can occur during interactive mode operations
#[derive(Error, Debug)]
pub enum InteractiveError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to write quickfix file: {0}")]
    QuickfixWrite(String),

    #[error("Failed to launch editor '{editor}': {message}")]
    EditorLaunch { editor: String, message: String },

    #[error("File operation failed: {0}")]
    FileOperation(String),

    #[error("Invalid line number {line} (file has {total} lines)")]
    InvalidLineNumber { line: usize, total: usize },

    #[error("File not found: {0}")]
    FileNotFound(String),
}

/// Result type for interactive operations
pub type InteractiveResult<T> = std::result::Result<T, InteractiveError>;

pub use ai_fix::{run_ai_fix_all, run_ai_fix_single, AiFixConfig, AiFixResult};
pub use editor::open_in_editor;
pub use menu::{run_interactive, InteractiveAction};
pub use nolint::add_nolint_comment;
pub use quickfix::{generate_quickfix, generate_quickfix_from_result, write_quickfix_file};
