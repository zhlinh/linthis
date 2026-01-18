// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! CLI module for linthis command-line interface.
//!
//! This module contains the command definitions and handlers for the
//! linthis CLI application.

mod commands;
mod helpers;
mod hook;
mod init;
mod plugin;
mod runner;

pub use commands::{Cli, Commands};
pub use helpers::{find_latest_result_file, print_fix_hint, run_benchmark, strip_ansi_codes};
pub use hook::handle_hook_command;
pub use init::{handle_config_command, handle_init_command, init_linter_configs};
pub use plugin::handle_plugin_command;
pub use runner::{perform_auto_sync, perform_self_update};
