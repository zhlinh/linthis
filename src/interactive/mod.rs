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

mod editor;
mod menu;
mod nolint;
mod quickfix;

pub use editor::open_in_editor;
pub use menu::{run_interactive, InteractiveAction};
pub use nolint::add_nolint_comment;
pub use quickfix::{generate_quickfix, generate_quickfix_from_result, write_quickfix_file};
