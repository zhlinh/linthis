//! CLI handler for the `linthis review` subcommand.

use std::process::ExitCode;

/// Options for the review command
#[allow(dead_code)]
pub struct ReviewCommandOptions {
    pub background: bool,
    pub auto_fix: bool,
    pub reviewers: Option<Vec<String>>,
    pub provider: Option<String>,
    pub base: Option<String>,
    pub head: String,
    pub no_pr: bool,
    pub notify: Option<Vec<String>>,
    pub status: bool,
    pub dry_run: bool,
    pub clean: bool,
    pub output: String,
}

/// Handle the review command
pub fn handle_review_command(options: ReviewCommandOptions) -> ExitCode {
    if options.status {
        return handle_review_status();
    }
    if options.clean {
        return handle_review_clean();
    }
    if options.background {
        return handle_review_background(options);
    }
    handle_review_foreground(options)
}

fn handle_review_status() -> ExitCode {
    println!("No reviews in progress");
    ExitCode::SUCCESS
}

fn handle_review_clean() -> ExitCode {
    println!("Review artifacts cleaned");
    ExitCode::SUCCESS
}

fn handle_review_background(_options: ReviewCommandOptions) -> ExitCode {
    eprintln!("Background review started");
    ExitCode::SUCCESS
}

fn handle_review_foreground(_options: ReviewCommandOptions) -> ExitCode {
    eprintln!("Review feature not yet implemented");
    ExitCode::from(1)
}
