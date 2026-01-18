// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Init and config command handlers.
//!
//! This module provides handlers for initializing linthis configuration
//! and managing configuration values.

use colored::Colorize;
use std::process::ExitCode;

use super::commands::{ConfigCommands, HookCommands};
use super::handle_hook_command;

/// Handle config subcommands
pub fn handle_config_command(action: ConfigCommands) -> ExitCode {
    use linthis::config::cli;

    match action {
        ConfigCommands::Add {
            field,
            value,
            global,
        } => cli::handle_config_add(field.as_str(), &value, global),
        ConfigCommands::Remove {
            field,
            value,
            global,
        } => cli::handle_config_remove(field.as_str(), &value, global),
        ConfigCommands::Clear { field, global } => cli::handle_config_clear(field.as_str(), global),
        ConfigCommands::Set {
            field,
            value,
            global,
        } => cli::handle_config_set(&field, &value, global),
        ConfigCommands::Unset { field, global } => cli::handle_config_unset(&field, global),
        ConfigCommands::Get { field, global } => cli::handle_config_get(&field, global),
        ConfigCommands::List { verbose, global } => cli::handle_config_list(verbose, global),
    }
}

/// Handle init subcommand
pub fn handle_init_command(
    global: bool,
    with_hook: bool,
    force: bool
) -> ExitCode {
    use linthis::config::Config;

    let config_path = if global {
        // Global config path: ~/.linthis/config.toml
        let home = match home_dir() {
            Some(h) => h,
            None => {
                eprintln!("{}: Cannot determine home directory", "Error".red());
                return ExitCode::from(1);
            }
        };
        home.join(".linthis").join("config.toml")
    } else {
        // Project config path: .linthis/config.toml in current directory
        Config::project_config_path(&std::env::current_dir().unwrap_or_default())
    };

    // Create or skip config file based on existence
    if config_path.exists() && !force {
        // Config already exists, skip creation but continue with hook setup
        println!(
            "{}: {} already exists, skipping config creation",
            "Info".cyan(),
            config_path.display()
        );
    } else {
        // Create parent directory if needed
        if let Some(parent) = config_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!(
                    "{}: Failed to create directory {}: {}",
                    "Error".red(),
                    parent.display(),
                    e
                );
                return ExitCode::from(2);
            }
        }

        // Create config file
        let content = Config::generate_default_toml();
        match std::fs::write(&config_path, content) {
            Ok(_) => {
                println!("{} Created {}", "✓".green(), config_path.display());
            }
            Err(e) => {
                eprintln!("{}: Failed to create config: {}", "Error".red(), e);
                return ExitCode::from(2);
            }
        }
    }

    // Handle hook installation if requested
    if with_hook {
        if global {
            eprintln!(
                "{}: Global config does not support --with-hook",
                "Warning".yellow()
            );
            eprintln!("  Global hook template feature has been removed");
            eprintln!("  Use {} in each project instead",
                "linthis hook install".cyan()
            );
        } else {
            // Install hook for project
            println!();
            let exit_code = handle_hook_command(HookCommands::Install {
                hook_type: None,   // Use default hook type (Git)
                check_only: false, // Not check-only
                format_only: false, // Not format-only
                force,             // Use force flag from init
                yes: true,         // Non-interactive mode
            });
            if exit_code != ExitCode::SUCCESS {
                return exit_code;
            }
        }
    }

    ExitCode::SUCCESS
}

/// Get the user's home directory
fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var("USERPROFILE").ok().map(std::path::PathBuf::from))
}

/// Initialize default config files for all linters/formatters
pub fn init_linter_configs() -> ExitCode {
    use linthis::templates::get_default_configs;
    use std::fs;
    use std::path::Path;

    let configs = get_default_configs();
    let mut created = 0;
    let mut skipped = 0;

    println!(
        "{}",
        "Generating default linter/formatter configs...".cyan()
    );

    for (filename, content) in configs {
        let path = Path::new(filename);
        if path.exists() {
            println!("  {} {} (already exists)", "⊘".yellow(), filename);
            skipped += 1;
        } else {
            match fs::write(path, content) {
                Ok(_) => {
                    println!("  {} {}", "✓".green(), filename);
                    created += 1;
                }
                Err(e) => {
                    eprintln!("  {} {} ({})", "✗".red(), filename, e);
                }
            }
        }
    }

    println!();
    println!(
        "Created {} config file{}, skipped {} existing",
        created,
        if created == 1 { "" } else { "s" },
        skipped
    );

    ExitCode::SUCCESS
}
