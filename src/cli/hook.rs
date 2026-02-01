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

use super::commands::{HookCommands, HookEvent, HookTool};

/// Handle hook subcommands
pub fn handle_hook_command(action: HookCommands) -> ExitCode {
    match action {
        HookCommands::Install { hook_type, hook_event, check_only, format_only, force, yes, ai, provider, accept_all } => {
            handle_hook_install(hook_type, hook_event, check_only, format_only, force, yes, ai, provider, accept_all)
        }
        HookCommands::Uninstall { hook_event, all, yes } => {
            handle_hook_uninstall(hook_event, all, yes)
        }
        HookCommands::Status => {
            handle_hook_status()
        }
        HookCommands::Check => {
            handle_hook_check()
        }
        HookCommands::CommitMsgCheck { msg_file } => {
            handle_commit_msg_check(&msg_file)
        }
    }
}

/// Install git hook (pre-commit, pre-push, or commit-msg)
fn handle_hook_install(
    hook_type: Option<HookTool>,
    hook_event: HookEvent,
    check_only: bool,
    format_only: bool,
    force: bool,
    yes: bool,
    ai: bool,
    provider: Option<String>,
    accept_all: bool,
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

    let hook_filename = hook_event.hook_filename();
    let hook_path = git_root.join(".git/hooks").join(hook_filename);

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
                        return handle_hook_install_impl(hook_type, &hook_event, check_only, format_only, true, false, ai, provider.clone(), accept_all);
                    }
                    "2" => {
                        // Append
                        return handle_hook_install_impl(hook_type, &hook_event, check_only, format_only, false, true, ai, provider.clone(), accept_all);
                    }
                    "3" => {
                        // Backup and replace
                        let backup_path = hook_path.with_extension(format!("{}.backup", hook_filename));
                        if let Err(e) = std::fs::copy(&hook_path, &backup_path) {
                            eprintln!("{}: Failed to create backup: {}", "Error".red(), e);
                            return ExitCode::from(2);
                        }
                        println!("{} Created backup at {}", "✓".green(), backup_path.display());
                        return handle_hook_install_impl(hook_type, &hook_event, check_only, format_only, true, false, ai, provider.clone(), accept_all);
                    }
                    _ => {
                        println!("Installation cancelled");
                        return ExitCode::SUCCESS;
                    }
                }
            } else {
                // Non-interactive mode: append by default
                return handle_hook_install_impl(hook_type, &hook_event, check_only, format_only, false, true, ai, provider.clone(), accept_all);
            }
        }

        println!("  Use {} to overwrite, or {} to append", "--force".yellow(), "choose option 2".cyan());
        return ExitCode::from(1);
    }

    // No existing hook or force mode - create new hook
    handle_hook_install_impl(hook_type, &hook_event, check_only, format_only, force, false, ai, provider, accept_all)
}

/// Internal implementation of hook installation
fn handle_hook_install_impl(
    hook_type: Option<HookTool>,
    hook_event: &HookEvent,
    check_only: bool,
    format_only: bool,
    force: bool,
    append: bool,
    ai: bool,
    provider: Option<String>,
    accept_all: bool,
) -> ExitCode {
    let tool = hook_type.unwrap_or(HookTool::Git);

    // For append mode, we need to modify create_hook_config to support appending
    if append {
        // For now, use create_hook_config which already handles appending for git hooks
        if let Err(exit_code) = create_hook_config(&tool, hook_event, check_only, format_only, false, ai, &provider, accept_all) {
            return exit_code;
        }
    } else if let Err(exit_code) = create_hook_config(&tool, hook_event, check_only, format_only, force, ai, &provider, accept_all) {
        return exit_code;
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

    let prek_config = std::path::Path::new(".pre-commit-config.yaml");

    println!("{}", "Git Hook Status".bold());
    println!("Repository: {}", git_root.display());
    println!();

    // Check all hook types
    let hook_events = [HookEvent::PreCommit, HookEvent::PrePush, HookEvent::CommitMsg];
    let mut any_hook_installed = false;

    for event in &hook_events {
        let hook_path = git_root.join(".git/hooks").join(event.hook_filename());

        if hook_path.exists() {
            any_hook_installed = true;
            println!("{} {} ({})", "✓".green(), event.hook_filename(), event.description());

            if let Ok(content) = std::fs::read_to_string(&hook_path) {
                let has_linthis = content.contains("linthis");
                let has_prek = content.contains("prek");
                let has_precommit = content.contains("pre-commit");
                let has_husky = content.contains("husky");

                if has_linthis {
                    println!("    {} linthis", "✓".green());
                }
                if has_prek {
                    println!("    {} prek", "ℹ".cyan());
                }
                if has_precommit {
                    println!("    {} pre-commit", "ℹ".cyan());
                }
                if has_husky {
                    println!("    {} husky", "ℹ".cyan());
                }

                if !has_linthis && !has_prek && !has_precommit && !has_husky {
                    println!("    {} Custom hook", "ℹ".cyan());
                }
            }
        } else {
            println!("{} {} (not installed)", "✗".red(), event.hook_filename());
        }
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

    println!("\n{}", "Available hooks:".bold());
    println!("  {} - runs before each commit", "pre-commit".cyan());
    println!("  {} - runs before push to remote", "pre-push".cyan());
    println!("  {} - validates commit message format", "commit-msg".cyan());

    println!("\n{}", "Commands:".bold());
    if !any_hook_installed {
        println!("  Install pre-commit:  {}", "linthis hook install".cyan());
        println!("  Install pre-push:    {}", "linthis hook install --hook pre-push".cyan());
        println!("  Install commit-msg:  {}", "linthis hook install --hook commit-msg".cyan());
    } else {
        println!("  Install hook:   {}", "linthis hook install --hook <hook-type>".cyan());
        println!("  Uninstall hook: {}", "linthis hook uninstall --hook <hook-type>".cyan());
        println!("  Uninstall all:  {}", "linthis hook uninstall --all".cyan());
    }

    ExitCode::SUCCESS
}

/// Uninstall git hook (specific event or all)
fn handle_hook_uninstall(hook_event: Option<HookEvent>, all: bool, yes: bool) -> ExitCode {
    // Find git root
    let git_root = match find_git_root() {
        Some(root) => root,
        None => {
            eprintln!("{}: Not in a git repository", "Error".red());
            return ExitCode::from(1);
        }
    };

    if all {
        // Uninstall all hooks
        let hook_events = [HookEvent::PreCommit, HookEvent::PrePush, HookEvent::CommitMsg];
        let mut any_uninstalled = false;

        for event in &hook_events {
            let result = uninstall_single_hook(&git_root, event, yes);
            if result == ExitCode::SUCCESS {
                any_uninstalled = true;
            }
        }

        if !any_uninstalled {
            println!("{}: No hooks with linthis found", "Info".cyan());
        }

        return ExitCode::SUCCESS;
    }

    // Uninstall specific hook (default to pre-commit)
    let event = hook_event.unwrap_or(HookEvent::PreCommit);
    uninstall_single_hook(&git_root, &event, yes)
}

/// Uninstall a single hook
fn uninstall_single_hook(git_root: &std::path::Path, hook_event: &HookEvent, yes: bool) -> ExitCode {
    use std::io::{self, Write};

    let hook_path = git_root.join(".git/hooks").join(hook_event.hook_filename());

    if !hook_path.exists() {
        return ExitCode::from(1); // Not an error, just not installed
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
        return ExitCode::from(1); // Not an error, just no linthis
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
            _ => {
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
            println!("{} Removed linthis from {} hook", "✓".green(), hook_event.hook_filename());
        } else {
            if let Err(e) = std::fs::remove_file(&hook_path) {
                eprintln!("{}: Failed to delete hook: {}", "Error".red(), e);
                return ExitCode::from(2);
            }
            println!("{} Deleted {} hook", "✓".green(), hook_event.hook_filename());
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
            if content.contains("linthis") && !hook_path.exists() {
                has_conflicts = true;
                println!("{} {} exists but no hook installed", "⚠".yellow(), prek_config.display());
                warnings.push("Run 'prek install' or 'pre-commit install' to activate hooks");
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
fn install_hooks(tool: &HookTool, hook_event: &HookEvent) -> Result<(), String> {
    use std::process::Command;

    let (cmd, tool_name) = match tool {
        HookTool::Prek => ("prek", "prek"),
        HookTool::PreCommit => ("pre-commit", "pre-commit"),
        HookTool::Git => return Ok(()), // Git hooks don't need install step
    };

    let hook_type_arg = hook_event.hook_filename();

    let output = Command::new(cmd)
        .arg("install")
        .arg("--hook-type")
        .arg(hook_type_arg)
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

/// Create hook configuration file based on the selected tool and event
fn create_hook_config(tool: &HookTool, hook_event: &HookEvent, hook_check_only: bool, hook_format_only: bool, force: bool, ai: bool, provider: &Option<String>, accept_all: bool) -> Result<(), ExitCode> {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    let hook_filename = hook_event.hook_filename();

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

            // Build hook command based on options and event type
            let hook_cmd = build_hook_command(hook_event, hook_check_only, hook_format_only, ai, provider, accept_all);

            // For prek/pre-commit, we need to specify the stage for different hook types
            let stage = match hook_event {
                HookEvent::PreCommit => "pre-commit",
                HookEvent::PrePush => "pre-push",
                HookEvent::CommitMsg => "commit-msg",
            };

            let content = format!(r#"repos:
  - repo: local
    hooks:
      - id: linthis-{}
        name: linthis ({})
        entry: {}
        language: system
        stages: [{}]
        pass_filenames: false
"#, hook_filename, hook_event.description(), hook_cmd, stage);

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

                        match install_hooks(tool, hook_event) {
                            Ok(_) => {
                                println!("{}", "✓".green());
                                println!("\n{} {} hooks are ready!", "✓".green().bold(), hook_filename);
                                println!("  Hooks will run automatically on {}", format!("git {}", hook_action(hook_event)).cyan());
                            }
                            Err(e) => {
                                println!("{}", "✗".red());
                                eprintln!("{}: {}", "Warning".yellow(), e);
                                println!("\nPlease run manually: {}", format!("{} install --hook-type {}", tool_name, hook_filename).cyan());
                            }
                        }
                    } else {
                        // Tool not installed, show installation instructions
                        // Both prek and pre-commit can be installed via pip
                        println!("\nNext steps:");
                        if matches!(tool, HookTool::Prek) {
                            println!("  1. Install prek: {}", "pip install prek".cyan());
                            println!("  2. Set up hooks: {}", format!("prek install --hook-type {}", hook_filename).cyan());
                        } else {
                            println!("  1. Install pre-commit: {}", "pip install pre-commit".cyan());
                            println!("  2. Set up hooks: {}", format!("pre-commit install --hook-type {}", hook_filename).cyan());
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
                        "{}: Not in a git repository, cannot create .git/hooks/{}",
                        "Error".red(),
                        hook_filename
                    );
                    return Err(ExitCode::from(1));
                }
            };

            let git_hooks_dir = git_root.join(".git/hooks");
            let hook_path = git_hooks_dir.join(hook_filename);

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

            // Build hook command based on options and event type
            let linthis_hook_line = build_hook_command(hook_event, hook_check_only, hook_format_only, ai, provider, accept_all);

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
                if existing_content.contains(&linthis_hook_line) {
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
                new_content.push_str(&linthis_hook_line);
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
                            println!("    {}", format!("chmod +x .git/hooks/{}", hook_filename).cyan());
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

/// Build the linthis command for a hook based on event type and options
fn build_hook_command(
    hook_event: &HookEvent,
    hook_check_only: bool,
    hook_format_only: bool,
    ai: bool,
    provider: &Option<String>,
    accept_all: bool,
) -> String {
    let mut cmd = match hook_event {
        HookEvent::PreCommit => {
            // For pre-commit: check staged files with hook mode output
            if hook_check_only {
                "linthis -s -c --hook-mode=pre-commit".to_string()
            } else if hook_format_only {
                "linthis -s -f --hook-mode=pre-commit".to_string()
            } else {
                "linthis -s -c -f --hook-mode=pre-commit".to_string()
            }
        }
        HookEvent::PrePush => {
            // For pre-push: check all files (more comprehensive) with hook mode output
            if hook_check_only {
                "linthis -c --hook-mode=pre-push".to_string()
            } else if hook_format_only {
                "linthis -f --hook-mode=pre-push".to_string()
            } else {
                "linthis -c -f --hook-mode=pre-push".to_string()
            }
        }
        HookEvent::CommitMsg => {
            // For commit-msg: validate commit message using the msg file passed as $1
            "linthis hook commit-msg-check \"$1\"".to_string()
        }
    };

    // Add AI-related flags for auto-fix during hook execution
    // Note: commit-msg hook doesn't support AI fix (it validates message format, not code)
    if ai && !matches!(hook_event, HookEvent::CommitMsg) {
        cmd.push_str(" --fix --ai");
        if let Some(ref p) = provider {
            cmd.push_str(&format!(" --provider {}", p));
        }
        if accept_all {
            cmd.push_str(" --accept-all");
        }
    }

    cmd
}

/// Get the git action for a hook event
fn hook_action(hook_event: &HookEvent) -> &'static str {
    match hook_event {
        HookEvent::PreCommit => "commit",
        HookEvent::PrePush => "push",
        HookEvent::CommitMsg => "commit",
    }
}

/// Handle commit message validation
pub fn handle_commit_msg_check(msg_file: &std::path::Path) -> ExitCode {
    use linthis::config::Config;
    use regex::Regex;
    use std::fs;

    // Load config to get hooks settings
    let project_root = linthis::utils::get_project_root();
    let config = Config::load_merged(&project_root);

    // Read the commit message from file
    let commit_msg = match fs::read_to_string(msg_file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("{}: Failed to read commit message file: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    // Skip if empty (allows empty commits with --allow-empty-message)
    let first_line = commit_msg.lines().next().unwrap_or("").trim();
    if first_line.is_empty() || first_line.starts_with('#') {
        return ExitCode::SUCCESS;
    }

    // Use pattern from config
    let pattern = &config.hooks.commit_msg_pattern;

    let regex = match Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}: Invalid commit message pattern in config: {}", "Error".red(), e);
            return ExitCode::from(2);
        }
    };

    // Check main pattern
    if !regex.is_match(first_line) {
        print_commit_msg_error(first_line);
        return ExitCode::from(1);
    }

    // Check for ticket reference if required
    if config.hooks.require_ticket {
        let ticket_pattern = config.hooks.ticket_pattern.as_deref()
            .unwrap_or(r"\[\w+-\d+\]");
        let ticket_regex = match Regex::new(ticket_pattern) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{}: Invalid ticket pattern in config: {}", "Error".red(), e);
                return ExitCode::from(2);
            }
        };

        if !ticket_regex.is_match(first_line) {
            eprintln!("{}", "╭────────────────────────────────────────╮".red());
            eprintln!("{}", "│ 🔴 Ticket Reference Required          │".red());
            eprintln!("{}", "├────────────────────────────────────────┤".red());
            eprintln!("│ Your message:                          │");
            eprintln!("│   {}", first_line);
            eprintln!("│                                        │");
            eprintln!("│ Ticket reference is required.          │");
            eprintln!("│ Pattern: {}                            │", ticket_pattern);
            eprintln!("│                                        │");
            eprintln!("│ Example:                               │");
            eprintln!("│   feat: [PROJ-123] add feature         │");
            eprintln!("{}", "├────────────────────────────────────────┤".red());
            eprintln!("│ To skip this check:                    │");
            eprintln!("│   git commit --no-verify               │");
            eprintln!("{}", "╰────────────────────────────────────────╯".red());
            return ExitCode::from(1);
        }
    }

    println!("{} Commit message format is valid", "✓".green());
    ExitCode::SUCCESS
}

/// Print commit message validation error
fn print_commit_msg_error(first_line: &str) {
    eprintln!("{}", "╭────────────────────────────────────────╮".red());
    eprintln!("{}", "│ 🔴 Commit Message Validation Failed   │".red());
    eprintln!("{}", "├────────────────────────────────────────┤".red());
    eprintln!("│ Your message:                          │");
    eprintln!("│   {}", first_line);
    eprintln!("│                                        │");
    eprintln!("│ Expected format (Conventional Commits):│");
    eprintln!("│   type(scope)?: description            │");
    eprintln!("│                                        │");
    eprintln!("│ Valid types:                           │");
    eprintln!("│   feat, fix, docs, style, refactor,   │");
    eprintln!("│   perf, test, build, ci, chore, revert │");
    eprintln!("│                                        │");
    eprintln!("│ Examples:                              │");
    eprintln!("│   feat: add user authentication        │");
    eprintln!("│   fix(api): handle null response       │");
    eprintln!("│   docs: update README                  │");
    eprintln!("{}", "├────────────────────────────────────────┤".red());
    eprintln!("│ To skip this check:                    │");
    eprintln!("│   git commit --no-verify               │");
    eprintln!("{}", "╰────────────────────────────────────────╯".red());
}
