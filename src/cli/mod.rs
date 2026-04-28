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

mod backup;
mod cache;
pub mod commands;
mod complexity;
mod doctor;
mod fix;
mod format;
mod helpers;
pub mod hook;
mod init;
mod license;
mod paths;
mod plugin;
mod recheck;
mod report;
mod review;
mod runner;
mod security;
pub mod shell;
mod watch;

pub use backup::{create_backup, handle_backup_command};
pub use cache::handle_cache_command;
pub use commands::{Cli, Commands};
pub use complexity::{
    handle_complexity_command, run_complexity_analysis, ComplexityCommandOptions,
};
pub use doctor::handle_doctor_command;
pub use fix::{handle_fix_command, FixCommandOptions};
pub use format::{handle_format_command, FormatCommandOptions};
pub use helpers::{
    print_fix_hint, resolve_ai_provider, run_benchmark, select_ai_provider_interactive,
    strip_ansi_codes,
};
pub use hook::{handle_commit_msg_check, handle_hook_command};
pub use init::{handle_config_command, handle_init_command, init_linter_configs};
pub use license::{handle_license_command, LicenseCommandParams};
pub use paths::{collect_paths, PathCollectionOptions, PathCollectionResult};
pub use plugin::handle_plugin_command;
pub use report::handle_report_command;
pub use review::{handle_review_command, ReviewCommandOptions};
pub use runner::{handle_self_update_command, perform_auto_sync, perform_self_update};
pub use security::{handle_security_command, run_sast_scan, SecurityCommandParams};
pub use shell::handle_shell_command;
pub use watch::run_watch;
