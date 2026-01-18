// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Git hook management commands.
//!
//! This module provides functions for installing, uninstalling, and managing
//! git pre-commit hooks for linthis integration.

use colored::Colorize;
use std::path::PathBuf;
use std::process::ExitCode;

use super::commands::{HookCommands, HookTool};

/// Handle hook subcommands
pub fn handle_hook_command(action: HookCommands) -> ExitCode {
    match action {
        HookCommands::Install { hook_type, check_only, format_only, force, yes } => {
            handle_hook_install(hook_type, check_only, format_only, force, yes)
        }
        HookCommands::Uninstall { yes } => {
            handle_hook_uninstall(yes)
        }
        HookCommands::Status => {
            handle_hook_status()
        }
        HookCommands::Check => {
            handle_hook_check()
        }
    }
}

/// Install git pre-commit hook
fn handle_hook_install(
    hook_type: Option<HookTool>,
    check_only: bool,
    format_only: bool,
    force: bool,
    yes: bool,
) -> ExitCode {
    use std::io::{self, Write};

    // Find git root
    let git_root = match find_git_root() {
        Some(root) => root,
        None => {
            eprintln!("{}: Not in a git repository", "Error".red());
            eprintln!("  Run this command from within a git repository");
            return ExitCode::from(1);
        }
    };

    let hook_path = git_root.join(".git/hooks/pre-commit");

    // Check for existing hook
    if hook_path.exists() && !force {
        println!("{}: {} already exists", "Warning".yellow(), hook_path.display());

        // Read and analyze existing hook
        if let Ok(existing_content) = std::fs::read_to_string(&hook_path) {
            let has_linthis = existing_content.contains("linthis");
            let has_prek = existing_content.contains("prek") || std::path::Path::new(".pre-commit-config.yaml").exists();
            let has_precommit = existing_content.contains("pre-commit");
            let has_husky = existing_content.contains("husky");

            println!("\nDetected hook content:");
            if has_linthis {
                println!("  {} linthis", "✓".green());
            }
            if has_prek {
                println!("  {} prek/pre-commit framework", "⚠".yellow());
            }
            if has_precommit && !has_prek {
                println!("  {} pre-commit hooks", "⚠".yellow());
            }
            if has_husky {
                println!("  {} husky", "⚠".yellow());
            }

            if !yes {
                println!("\nOptions:");
                println!("  1. {} - Replace existing hook with linthis", "Replace".cyan());
                println!("  2. {} - Append linthis to existing hook", "Append".cyan());
                println!("  3. {} - Create backup and replace", "Backup".cyan());
                println!("  4. {} - Cancel", "Cancel".cyan());

                print!("\nChoose an option [1-4]: ");
                io::stdout().flush().unwrap();

                let mut choice = String::new();
                io::stdin().read_line(&mut choice).ok();

                match choice.trim() {
                    "1" => {
                        // Replace: use force flag internally
                        return handle_hook_install_impl(hook_type, check_only, format_only, true, false);
                    }
                    "2" => {
                        // Append
                        return handle_hook_install_impl(hook_type, check_only, format_only, false, true);
                    }
                    "3" => {
                        // Backup and replace
                        let backup_path = hook_path.with_extension("pre-commit.backup");
                        if let Err(e) = std::fs::copy(&hook_path, &backup_path) {
                            eprintln!("{}: Failed to create backup: {}", "Error".red(), e);
                            return ExitCode::from(2);
                        }
                        println!("{} Created backup at {}", "✓".green(), backup_path.display());
                        return handle_hook_install_impl(hook_type, check_only, format_only, true, false);
                    }
                    "4" | _ => {
                        println!("Installation cancelled");
                        return ExitCode::SUCCESS;
                    }
                }
            } else {
                // Non-interactive mode: append by default
                return handle_hook_install_impl(hook_type, check_only, format_only, false, true);
            }
        }

        println!("  Use {} to overwrite, or {} to append", "--force".yellow(), "choose option 2".cyan());
        return ExitCode::from(1);
    }

    // No existing hook or force mode - create new hook
    handle_hook_install_impl(hook_type, check_only, format_only, force, false)
}

/// Internal implementation of hook installation
fn handle_hook_install_impl(
    hook_type: Option<HookTool>,
    check_only: bool,
    format_only: bool,
    force: bool,
    append: bool,
) -> ExitCode {
    let tool = hook_type.unwrap_or(HookTool::Git);

    // For append mode, we need to modify create_hook_config to support appending
    if append {
        // For now, use create_hook_config which already handles appending for git hooks
        if let Err(exit_code) = create_hook_config(&tool, check_only, format_only, false) {
            return exit_code;
        }
    } else {
        if let Err(exit_code) = create_hook_config(&tool, check_only, format_only, force) {
            return exit_code;
        }
    }

    ExitCode::SUCCESS
}

/// Show git hook status
fn handle_hook_status() -> ExitCode {
    // Find git root
    let git_root = match find_git_root() {
        Some(root) => root,
        None => {
            eprintln!("{}: Not in a git repository", "Error".red());
            return ExitCode::from(1);
        }
    };

    let hook_path = git_root.join(".git/hooks/pre-commit");
    let prek_config = std::path::Path::new(".pre-commit-config.yaml");

    println!("{}", "Git Hook Status".bold());
    println!("Repository: {}", git_root.display());
    println!();

    // Check pre-commit hook
    if hook_path.exists() {
        println!("{} {}", "✓".green(), hook_path.display());

        if let Ok(content) = std::fs::read_to_string(&hook_path) {
            let has_linthis = content.contains("linthis");
            let has_prek = content.contains("prek");
            let has_precommit = content.contains("pre-commit");
            let has_husky = content.contains("husky");

            println!("\nHook contains:");
            if has_linthis {
                println!("  {} linthis", "✓".green());
            }
            if has_prek {
                println!("  {} prek", "ℹ".cyan());
            }
            if has_precommit {
                println!("  {} pre-commit", "ℹ".cyan());
            }
            if has_husky {
                println!("  {} husky", "ℹ".cyan());
            }

            if !has_linthis && !has_prek && !has_precommit && !has_husky {
                println!("  {} Custom hook", "ℹ".cyan());
            }
        }
    } else {
        println!("{} No pre-commit hook installed", "✗".red());
    }

    // Check for prek/pre-commit config
    if prek_config.exists() {
        println!("\n{} {}", "✓".green(), prek_config.display());

        if let Ok(content) = std::fs::read_to_string(prek_config) {
            if content.contains("linthis") {
                println!("  {} Contains linthis configuration", "✓".green());
            } else {
                println!("  {} No linthis configuration found", "⚠".yellow());
            }
        }
    }

    println!("\n{}", "Next steps:".bold());
    if !hook_path.exists() {
        println!("  Run {} to install hook", "linthis hook install".cyan());
    } else if let Ok(content) = std::fs::read_to_string(&hook_path) {
        if !content.contains("linthis") {
            println!("  Run {} to add linthis to existing hook", "linthis hook install".cyan());
        }
    }

    ExitCode::SUCCESS
}

/// Uninstall git pre-commit hook
fn handle_hook_uninstall(yes: bool) -> ExitCode {
    use std::io::{self, Write};

    // Find git root
    let git_root = match find_git_root() {
        Some(root) => root,
        None => {
            eprintln!("{}: Not in a git repository", "Error".red());
            return ExitCode::from(1);
        }
    };

    let hook_path = git_root.join(".git/hooks/pre-commit");

    if !hook_path.exists() {
        println!("{}: No pre-commit hook found", "Info".cyan());
        return ExitCode::SUCCESS;
    }

    // Read existing hook
    let existing_content = match std::fs::read_to_string(&hook_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("{}: Failed to read hook file: {}", "Error".red(), e);
            return ExitCode::from(2);
        }
    };

    let has_linthis = existing_content.contains("linthis");
    let has_other_content = existing_content.lines()
        .any(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with('#')
                && !trimmed.starts_with("#!/")
                && !trimmed.contains("linthis")
        });

    if !has_linthis {
        println!("{}: Hook does not contain linthis", "Info".cyan());
        println!("  Nothing to uninstall");
        return ExitCode::SUCCESS;
    }

    if !yes {
        println!("{}: {} contains:", "Warning".yellow(), hook_path.display());
        if has_linthis {
            println!("  {} linthis", "✓".green());
        }
        if has_other_content {
            println!("  {} Other hooks/commands", "⚠".yellow());
        }

        println!("\nOptions:");
        if has_other_content {
            println!("  1. {} - Remove only linthis lines", "Remove linthis".cyan());
            println!("  2. {} - Delete entire hook file", "Delete all".cyan());
        } else {
            println!("  1. {} - Delete hook file", "Delete".cyan());
        }
        println!("  3. {} - Cancel", "Cancel".cyan());

        print!("\nChoose an option: ");
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice).ok();

        match choice.trim() {
            "1" => {
                if has_other_content {
                    // Remove only linthis lines
                    let new_content: String = existing_content
                        .lines()
                        .filter(|line| !line.contains("linthis") && !line.contains("# linthis hook"))
                        .collect::<Vec<_>>()
                        .join("\n");

                    if let Err(e) = std::fs::write(&hook_path, new_content + "\n") {
                        eprintln!("{}: Failed to update hook: {}", "Error".red(), e);
                        return ExitCode::from(2);
                    }
                    println!("{} Removed linthis from {}", "✓".green(), hook_path.display());
                } else {
                    // Delete entire file
                    if let Err(e) = std::fs::remove_file(&hook_path) {
                        eprintln!("{}: Failed to delete hook: {}", "Error".red(), e);
                        return ExitCode::from(2);
                    }
                    println!("{} Deleted {}", "✓".green(), hook_path.display());
                }
            }
            "2" if has_other_content => {
                // Delete entire file
                if let Err(e) = std::fs::remove_file(&hook_path) {
                    eprintln!("{}: Failed to delete hook: {}", "Error".red(), e);
                    return ExitCode::from(2);
                }
                println!("{} Deleted {}", "✓".green(), hook_path.display());
            }
            "3" | _ => {
                println!("Uninstall cancelled");
                return ExitCode::SUCCESS;
            }
        }
    } else {
        // Non-interactive mode: remove only linthis if there's other content, otherwise delete file
        if has_other_content {
            let new_content: String = existing_content
                .lines()
                .filter(|line| !line.contains("linthis") && !line.contains("# linthis hook"))
                .collect::<Vec<_>>()
                .join("\n");

            if let Err(e) = std::fs::write(&hook_path, new_content + "\n") {
                eprintln!("{}: Failed to update hook: {}", "Error".red(), e);
                return ExitCode::from(2);
            }
            println!("{} Removed linthis from hook", "✓".green());
        } else {
            if let Err(e) = std::fs::remove_file(&hook_path) {
                eprintln!("{}: Failed to delete hook: {}", "Error".red(), e);
                return ExitCode::from(2);
            }
            println!("{} Deleted hook", "✓".green());
        }
    }

    ExitCode::SUCCESS
}

/// Check for hook conflicts
fn handle_hook_check() -> ExitCode {
    // Find git root
    let git_root = match find_git_root() {
        Some(root) => root,
        None => {
            eprintln!("{}: Not in a git repository", "Error".red());
            return ExitCode::from(1);
        }
    };

    let hook_path = git_root.join(".git/hooks/pre-commit");
    let prek_config = std::path::Path::new(".pre-commit-config.yaml");
    let husky_dir = std::path::Path::new(".husky");

    println!("{}", "Checking for hook conflicts...".bold());
    println!();

    let mut has_conflicts = false;
    let mut warnings = Vec::new();

    // Check pre-commit hook
    if hook_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&hook_path) {
            let has_linthis = content.contains("linthis");
            let has_prek = content.contains("prek");
            let has_precommit = content.contains("pre-commit");
            let has_husky = content.contains("husky");

            let tool_count = [has_prek, has_precommit, has_husky, has_linthis]
                .iter()
                .filter(|&&x| x)
                .count();

            if tool_count > 1 {
                has_conflicts = true;
                println!("{} Multiple hook tools detected in {}", "⚠".yellow(), hook_path.display());
                if has_linthis {
                    println!("  {} linthis", "✓".green());
                }
                if has_prek {
                    println!("  {} prek", "⚠".yellow());
                }
                if has_precommit {
                    println!("  {} pre-commit", "⚠".yellow());
                }
                if has_husky {
                    println!("  {} husky", "⚠".yellow());
                }
                warnings.push("Consider using only one hook management tool");
            }
        }
    }

    // Check for prek/pre-commit config without hook
    if prek_config.exists() {
        if let Ok(content) = std::fs::read_to_string(prek_config) {
            if content.contains("linthis") {
                if !hook_path.exists() {
                    has_conflicts = true;
                    println!("{} {} exists but no hook installed", "⚠".yellow(), prek_config.display());
                    warnings.push("Run 'prek install' or 'pre-commit install' to activate hooks");
                }
            }
        }
    }

    // Check for husky
    if husky_dir.exists() {
        let husky_pre_commit = husky_dir.join("pre-commit");
        if husky_pre_commit.exists() {
            println!("{} Husky detected: {}", "ℹ".cyan(), husky_pre_commit.display());
            warnings.push("Husky manages its own hooks in .husky/ directory");
            warnings.push("To use linthis with husky, add linthis command to .husky/pre-commit");
        }
    }

    println!();
    if has_conflicts {
        println!("{}", "Conflicts detected:".yellow().bold());
        for warning in warnings {
            println!("  • {}", warning);
        }
        println!();
        println!("{}", "Recommendations:".bold());
        println!("  • Use {} to see current hook setup", "linthis hook status".cyan());
        println!("  • Choose one hook tool and stick with it");
        println!("  • For teams, document hook setup in README");
    } else {
        println!("{} No conflicts detected", "✓".green().bold());
    }

    ExitCode::SUCCESS
}

/// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    std::process::Command::new("which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Install hooks using the specified tool
fn install_hooks(tool: &HookTool) -> Result<(), String> {
    use std::process::Command;

    let (cmd, tool_name) = match tool {
        HookTool::Prek => ("prek", "prek"),
        HookTool::PreCommit => ("pre-commit", "pre-commit"),
        HookTool::Git => return Ok(()), // Git hooks don't need install step
    };

    let output = Command::new(cmd)
        .arg("install")
        .output()
        .map_err(|e| format!("Failed to execute {} install: {}", tool_name, e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("{} install failed: {}", tool_name, stderr))
    }
}

/// Find the git repository root directory by searching upwards from current directory
pub fn find_git_root() -> Option<PathBuf> {
    use std::env;

    let mut current_dir = env::current_dir().ok()?;

    loop {
        let git_dir = current_dir.join(".git");
        if git_dir.exists() {
            return Some(current_dir);
        }

        // Try to go up one directory
        match current_dir.parent() {
            Some(parent) => current_dir = parent.to_path_buf(),
            None => return None, // Reached root directory without finding .git
        }
    }
}

/// Create hook configuration file based on the selected tool
fn create_hook_config(tool: &HookTool, hook_check_only: bool, hook_format_only: bool, force: bool) -> Result<(), ExitCode> {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    match tool {
        HookTool::Prek | HookTool::PreCommit => {
            let config_path = std::path::PathBuf::from(".pre-commit-config.yaml");

            if config_path.exists() && !force {
                eprintln!(
                    "{}: {} already exists, skipping",
                    "Warning".yellow(),
                    config_path.display()
                );
                return Ok(());
            }

            // Build hook command based on options
            let hook_cmd = if hook_check_only {
                "linthis -s -c -w"
            } else if hook_format_only {
                "linthis -s -f -w"
            } else {
                // Default: run both check and format, fail on warnings
                "linthis -s -c -f -w"
            };

            let content = format!(r#"repos:
  - repo: local
    hooks:
      - id: linthis
        name: linthis
        entry: {}
        language: system
        pass_filenames: false
"#, hook_cmd);

            match fs::write(&config_path, content) {
                Ok(_) => {
                    let tool_name = match tool {
                        HookTool::Prek => "prek",
                        HookTool::PreCommit => "pre-commit",
                        _ => unreachable!(),
                    };
                    println!(
                        "{} Created {} ({}/pre-commit compatible)",
                        "✓".green(),
                        config_path.display(),
                        tool_name
                    );

                    // Check if tool is installed and auto-install hooks
                    let cmd_name = tool_name;
                    if is_command_available(cmd_name) {
                        println!("\n{} Detected installed", tool_name.cyan());
                        print!("{} Installing hooks... ", "→".cyan());
                        std::io::Write::flush(&mut std::io::stdout()).ok();

                        match install_hooks(tool) {
                            Ok(_) => {
                                println!("{}", "✓".green());
                                println!("\n{} Pre-commit hooks are ready!", "✓".green().bold());
                                println!("  Hooks will run automatically on {}", "git commit".cyan());
                            }
                            Err(e) => {
                                println!("{}", "✗".red());
                                eprintln!("{}: {}", "Warning".yellow(), e);
                                println!("\nPlease run manually: {}", format!("{} install", tool_name).cyan());
                            }
                        }
                    } else {
                        // Tool not installed, show installation instructions
                        // Both prek and pre-commit can be installed via pip
                        println!("\nNext steps:");
                        if matches!(tool, HookTool::Prek) {
                            println!("  1. Install prek: {}", "pip install prek".cyan());
                            println!("  2. Set up hooks: {}", "prek install".cyan());
                        } else {
                            println!("  1. Install pre-commit: {}", "pip install pre-commit".cyan());
                            println!("  2. Set up hooks: {}", "pre-commit install".cyan());
                        }
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!(
                        "{}: Failed to create {}: {}",
                        "Error".red(),
                        config_path.display(),
                        e
                    );
                    Err(ExitCode::from(2))
                }
            }
        }
        HookTool::Git => {
            // Find git repository root directory
            let git_root = match find_git_root() {
                Some(root) => root,
                None => {
                    eprintln!(
                        "{}: Not in a git repository, cannot create .git/hooks/pre-commit",
                        "Error".red()
                    );
                    return Err(ExitCode::from(1));
                }
            };

            let git_hooks_dir = git_root.join(".git/hooks");
            let hook_path = git_hooks_dir.join("pre-commit");

            // Create hooks directory if it doesn't exist
            if !git_hooks_dir.exists() {
                if let Err(e) = fs::create_dir_all(&git_hooks_dir) {
                    eprintln!(
                        "{}: Failed to create hooks directory {}: {}",
                        "Error".red(),
                        git_hooks_dir.display(),
                        e
                    );
                    return Err(ExitCode::from(2));
                }
            }

            // Build hook command based on options
            let linthis_hook_line = if hook_check_only {
                "linthis -s -c -w"
            } else if hook_format_only {
                "linthis -s -f -w"
            } else {
                // Default: run both check and format, fail on warnings
                "linthis -s -c -f -w"
            };

            // Check if hook file already exists
            if hook_path.exists() {
                // Read existing content
                let existing_content = match fs::read_to_string(&hook_path) {
                    Ok(content) => content,
                    Err(e) => {
                        eprintln!(
                            "{}: Failed to read existing hook file: {}",
                            "Error".red(),
                            e
                        );
                        return Err(ExitCode::from(2));
                    }
                };

                // Check if linthis is already in the hook
                if existing_content.contains(linthis_hook_line) {
                    println!(
                        "{}: linthis hook already exists in {}",
                        "Info".cyan(),
                        hook_path.display()
                    );
                    return Ok(());
                }

                // Append linthis to the existing hook
                let mut new_content = existing_content.clone();
                if !new_content.ends_with('\n') {
                    new_content.push('\n');
                }
                new_content.push_str("\n# linthis hook\n");
                new_content.push_str(linthis_hook_line);
                new_content.push('\n');

                match fs::write(&hook_path, new_content) {
                    Ok(_) => {
                        println!(
                            "{} Added linthis to existing {}",
                            "✓".green(),
                            hook_path.display()
                        );
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!(
                            "{}: Failed to update {}: {}",
                            "Error".red(),
                            hook_path.display(),
                            e
                        );
                        Err(ExitCode::from(2))
                    }
                }
            } else {
                // Create new hook file
                let content = format!("#!/bin/sh\n{}\n", linthis_hook_line);

                match fs::write(&hook_path, content) {
                    Ok(_) => {
                        // Make the hook executable
                        #[cfg(unix)]
                        {
                            let mut perms = fs::metadata(&hook_path)
                                .map_err(|e| {
                                    eprintln!("{}: Failed to get file metadata: {}", "Error".red(), e);
                                    ExitCode::from(2)
                                })?
                                .permissions();
                            perms.set_mode(0o755);
                            fs::set_permissions(&hook_path, perms).map_err(|e| {
                                eprintln!("{}: Failed to set permissions: {}", "Error".red(), e);
                                ExitCode::from(2)
                            })?;
                        }

                        println!("{} Created {}", "✓".green(), hook_path.display());
                        #[cfg(not(unix))]
                        {
                            println!("\nNext steps:");
                            println!("  Make sure the hook is executable:");
                            println!("    {}", "chmod +x .git/hooks/pre-commit".cyan());
                        }
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!(
                            "{}: Failed to create {}: {}",
                            "Error".red(),
                            hook_path.display(),
                            e
                        );
                        Err(ExitCode::from(2))
                    }
                }
            }
        }
    }
}
