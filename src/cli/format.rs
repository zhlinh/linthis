// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Format subcommand handler for running formatters with backup support.
//!
//! This module handles the `format` subcommand, supporting:
//! - Running formatters on files with automatic backup
//! - Undo (restore from backup)
//! - Listing available backups

use colored::Colorize;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::cli::backup::{create_backup, handle_list_backups, handle_undo};
use crate::cli::paths::{collect_paths, PathCollectionOptions, PathCollectionResult};
use linthis::{RunMode, RunOptions};

/// Options for the format subcommand
pub struct FormatCommandOptions {
    /// Files or directories to include
    pub paths: Vec<PathBuf>,
    /// Format only staged files
    pub staged: bool,
    /// Format only modified files (staged + unstaged)
    pub modified: bool,
    /// Exclude patterns
    pub exclude: Option<Vec<String>>,
    /// Undo last format
    pub undo: bool,
    /// Backup source for undo
    pub source: String,
    /// List available backups
    pub list_backups: bool,
    /// Verbose output
    pub verbose: bool,
    /// Quiet mode
    pub quiet: bool,
}

/// Handle the format subcommand
pub fn handle_format_command(options: FormatCommandOptions) -> ExitCode {
    // Handle --list-backups
    if options.list_backups {
        return handle_list_backups("linthis format");
    }

    // Handle --undo
    if options.undo {
        return handle_undo(&options.source, "linthis format --list-backups");
    }

    // Collect files to format
    let path_options = PathCollectionOptions {
        paths: options.paths,
        staged: options.staged,
        since: None,
        modified: options.modified,
        exclude: options.exclude.unwrap_or_default(),
        no_default_excludes: false,
        no_gitignore: false,
        verbose: options.verbose,
    };

    let files = match collect_paths(&path_options) {
        PathCollectionResult::Success(files, _excludes) => files,
        PathCollectionResult::Empty(msg) => {
            if !options.quiet {
                println!("{} {}", "→".cyan(), msg);
            }
            return ExitCode::SUCCESS;
        }
        PathCollectionResult::Error(msg, code) => {
            eprintln!("{}: {}", "Error".red(), msg);
            return ExitCode::from(code as u8);
        }
    };

    if files.is_empty() {
        if !options.quiet {
            println!("{} No files to format.", "→".cyan());
        }
        return ExitCode::SUCCESS;
    }

    if !options.quiet {
        println!(
            "{} Formatting {} file{}...",
            "→".cyan(),
            files.len(),
            if files.len() == 1 { "" } else { "s" }
        );
    }

    // Create backup before formatting
    create_backup(&files, "format (linthis format)", options.quiet);

    // Run format-only mode
    let run_options = RunOptions {
        paths: files,
        mode: RunMode::FormatOnly,
        verbose: options.verbose,
        quiet: options.quiet,
        ..Default::default()
    };

    match linthis::run(&run_options) {
        Ok(result) => {
            let formatted_count = result.format_results.iter().filter(|r| r.changed).count();
            if !options.quiet {
                if formatted_count > 0 {
                    println!(
                        "{} Formatted {} file{}",
                        "✓".green().bold(),
                        formatted_count,
                        if formatted_count == 1 { "" } else { "s" }
                    );
                    println!("  Use {} to undo", "linthis format --undo".cyan());
                } else {
                    println!("{} All files already formatted", "✓".green().bold());
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}: Format failed: {}", "Error".red(), e);
            ExitCode::from(1)
        }
    }
}
