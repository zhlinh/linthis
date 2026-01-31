// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! LSP (Language Server Protocol) server for linthis.
//!
//! This module implements an LSP server that provides:
//! - Real-time diagnostics (lint issues) on file save
//! - Code formatting support
//! - Integration with VS Code, Neovim, and other LSP clients

mod diagnostics;
mod document;
mod server;

pub use server::{run_lsp_server, run_lsp_server_with_config, LspMode};
