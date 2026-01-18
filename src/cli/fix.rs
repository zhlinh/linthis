// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Fix mode handler for loading and fixing issues from saved result files.
//!
//! This module handles the `--fix` flag when used without `-c`,
//! loading results from a file and entering interactive fix mode.

use colored::Colorize;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::cli::helpers::find_latest_result_file;
use crate::cli::recheck::{
    print_recheck_footer, print_recheck_header, print_recheck_summary, recheck_modified_files,
};
use linthis::interactive::run_interactive;

/// Handle `--fix` mode when loading results from file.
///
/// This is used when `--fix` is specified without `-c` or `-f`,
/// loading previous results from a file and entering interactive mode.
///
/// # Arguments
/// * `source` - The source to load from: "last" for the latest result file,
///   or a path to a specific result file
/// * `quiet` - Whether to suppress output
/// * `verbose` - Whether to show verbose output
///
/// # Returns
/// Exit code based on the original result's exit code
pub fn handle_fix_from_file(source: &str, quiet: bool, verbose: bool) -> ExitCode {
    let path = if source == "last" {
        match find_latest_result_file() {
            Some(p) => p,
            None => {
                eprintln!(
                    "{}: No result files found in .linthis/result/",
                    "Error".red()
                );
                eprintln!(
                    "  Run {} first to generate a result file.",
                    "linthis -c".cyan()
                );
                return ExitCode::from(1);
            }
        }
    } else {
        PathBuf::from(source)
    };

    if !path.exists() {
        eprintln!(
            "{}: Result file not found: {}",
            "Error".red(),
            path.display()
        );
        return ExitCode::from(1);
    }

    println!("{} Loading results from: {}", "→".cyan(), path.display());

    match std::fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<linthis::utils::types::RunResult>(&content) {
            Ok(result) => {
                if result.issues.is_empty() {
                    println!("{}", "No issues in the saved result.".green());
                    return ExitCode::SUCCESS;
                }
                println!(
                    "  Found {} issue{} from previous run\n",
                    result.issues.len(),
                    if result.issues.len() == 1 { "" } else { "s" }
                );

                let interactive_result = run_interactive(&result);

                // Recheck modified files if any changes were made
                if !interactive_result.modified_files.is_empty() {
                    print_recheck_header();

                    let recheck_result = recheck_modified_files(
                        &interactive_result.modified_files,
                        &result.issues,
                        quiet,
                        verbose,
                    );

                    let fixed_count = interactive_result.edited + interactive_result.ignored;
                    print_recheck_summary(&recheck_result, fixed_count);
                    print_recheck_footer();
                }

                ExitCode::from(result.exit_code as u8)
            }
            Err(e) => {
                eprintln!(
                    "{}: Failed to parse result file as JSON: {}",
                    "Error".red(),
                    e
                );
                eprintln!("  Result files are saved in JSON format by default.");
                eprintln!("  Make sure the file is a valid JSON result file.");
                ExitCode::from(2)
            }
        },
        Err(e) => {
            eprintln!("{}: Failed to read result file: {}", "Error".red(), e);
            ExitCode::from(2)
        }
    }
}
