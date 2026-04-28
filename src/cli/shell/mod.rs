// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

//! Shell integration commands: completion + built-in aliases.
//!
//! Adds tab-completion and a small set of aliases (`lt`, `lts`, `ltm`, `ltr`)
//! to the user's bash / zsh / fish / PowerShell. State lives in
//! `~/.linthis/shell-state.toml`; the per-shell source files
//! `~/.linthis/shell.{bash,zsh,fish,ps1}` are fully regenerated from that
//! state on every `add`/`remove`. The user's rc file gets a marker block
//! that sources the per-shell file.

mod detect;
mod rc;
mod render;
mod state;

use std::process::ExitCode;

use super::commands::ShellCommands;

/// Handle shell subcommands.
pub fn handle_shell_command(action: ShellCommands) -> ExitCode {
    // Filled in by later tasks. The skeleton just keeps the build green.
    let _ = action;
    eprintln!("[linthis shell] not yet implemented");
    ExitCode::from(1)
}
