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

mod cache;
mod commands;
mod doctor;
mod fix;
mod helpers;
mod hook;
mod init;
mod paths;
mod plugin;
mod recheck;
mod report;
mod runner;

pub use cache::handle_cache_command;
pub use commands::{Cli, Commands};
pub use doctor::handle_doctor_command;
pub use fix::handle_fix_from_file;
pub use helpers::{print_fix_hint, run_benchmark, strip_ansi_codes};
pub use hook::handle_hook_command;
pub use init::{handle_config_command, handle_init_command, init_linter_configs};
pub use paths::{collect_paths, PathCollectionOptions, PathCollectionResult};
pub use plugin::handle_plugin_command;
pub use recheck::{
    print_recheck_footer, print_recheck_header, print_recheck_summary, recheck_modified_files,
};
pub use report::handle_report_command;
pub use runner::{perform_auto_sync, perform_self_update};
