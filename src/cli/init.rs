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

use super::commands::{ConfigCommands, HookCommands, HookEvent};
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
        ConfigCommands::Migrate {
            from_tool,
            dry_run,
            backup,
            verbose,
        } => handle_config_migrate(from_tool, dry_run, backup, verbose),
    }
}

/// Handle config migrate subcommand
fn handle_config_migrate(
    from_tool: Option<String>,
    dry_run: bool,
    backup: bool,
    verbose: bool,
) -> ExitCode {
    use linthis::config::migrate::{migrate_configs, MigrationOptions, Tool, WarningSeverity};

    let project_root = std::env::current_dir().unwrap_or_default();

    // Parse tool filter
    let tool_filter = match from_tool.as_ref() {
        Some(t) => match Tool::parse(t) {
            Some(tool) => Some(tool),
            None => {
                eprintln!(
                    "{}: Unknown tool '{}'. Supported: eslint, prettier, black, isort",
                    "Error".red(),
                    t
                );
                return ExitCode::from(1);
            }
        },
        None => None,
    };

    let options = MigrationOptions {
        dry_run,
        backup,
        tool_filter,
        verbose,
    };

    println!(
        "{}",
        if dry_run {
            "Analyzing configuration files (dry run)...".cyan()
        } else {
            "Migrating configuration files...".cyan()
        }
    );

    match migrate_configs(&project_root, &options) {
        Ok(result) => {
            let mut has_errors = false;

            // Print warnings
            for warning in &result.warnings {
                let prefix = match warning.severity {
                    WarningSeverity::Info => "Info".cyan(),
                    WarningSeverity::Warning => "Warning".yellow(),
                    WarningSeverity::Error => {
                        has_errors = true;
                        "Error".red()
                    }
                };
                println!("  {} [{}]: {}", prefix, warning.source, warning.message);
            }

            if dry_run {
                // Show preview of changes
                println!();
                println!("{}", "Changes that would be made:".bold());
                if result.config_changes.is_empty() {
                    println!("  {}", "(no changes)".dimmed());
                } else {
                    for change in &result.config_changes {
                        println!("  {} {}", "→".cyan(), change);
                    }
                }
            } else {
                // Show actual results
                if !result.backed_up_files.is_empty() {
                    println!();
                    println!("{}", "Backed up files:".bold());
                    for path in &result.backed_up_files {
                        println!("  {} {}", "✓".green(), path.display());
                    }
                }

                if !result.created_files.is_empty() {
                    println!();
                    println!("{}", "Created files:".bold());
                    for path in &result.created_files {
                        println!("  {} {}", "✓".green(), path.display());
                    }
                }
            }

            // Print suggestions
            if !result.suggestions.is_empty() {
                println!();
                println!("{}", "Suggestions:".bold());
                for suggestion in &result.suggestions {
                    println!("  💡 {}", suggestion);
                }
            }

            // Summary
            println!();
            if dry_run {
                let change_count = result.config_changes.len();
                if change_count > 0 {
                    println!(
                        "{} Dry run complete. {} change(s) would be made.",
                        "✓".green(),
                        change_count
                    );
                    println!("  Run without {} to apply changes.", "--dry-run".cyan());
                } else {
                    println!("{} No configuration files to migrate.", "ℹ".blue());
                }
            } else if result.created_files.is_empty() && !has_errors {
                println!("{} No configuration files to migrate.", "ℹ".blue());
            } else if !has_errors {
                println!(
                    "{} Migration complete! {} file(s) created.",
                    "✓".green(),
                    result.created_files.len()
                );
            }

            if has_errors {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            ExitCode::from(1)
        }
    }
}

/// Handle init subcommand
pub fn handle_init_command(global: bool, with_hook: bool, force: bool) -> ExitCode {
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
            eprintln!(
                "  Use {} in each project instead",
                "linthis hook install".cyan()
            );
        } else {
            // Install hook for project
            println!();
            let exit_code = handle_hook_command(HookCommands::Install {
                hook_types: vec![],                      // Use default hook type (Git)
                hook_events: vec![HookEvent::PreCommit], // Default to pre-commit hook
                force,                                   // Use force flag from init
                yes: true,                               // Non-interactive mode
                global: false,                           // Project-level install
                provider: None,                          // No provider specified
                args: None,                              // Use default args (-c -f)
                provider_args: None,                     // No extra agent CLI args
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
        .or_else(|| {
            std::env::var("USERPROFILE")
                .ok()
                .map(std::path::PathBuf::from)
        })
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

    for (filename, content) in &configs {
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

    // Ensure .linthis/ is in .gitignore (working directory for review results, caches, etc.)
    let gitignore_path = Path::new(".gitignore");
    let existing_gitignore = fs::read_to_string(gitignore_path).unwrap_or_default();
    if !existing_gitignore
        .lines()
        .any(|line| line.trim() == ".linthis/" || line.trim() == ".linthis")
    {
        let mut content = existing_gitignore;
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str("\n# Linthis working directory\n.linthis/\n");
        match fs::write(gitignore_path, &content) {
            Ok(_) => {
                println!("  {} Added .linthis/ to .gitignore", "✓".green());
            }
            Err(e) => {
                eprintln!("  {} Failed to update .gitignore: {}", "✗".red(), e);
                eprintln!("  Please add '.linthis/' to .gitignore manually");
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
