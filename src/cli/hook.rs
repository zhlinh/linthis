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

use super::commands::{AgentProvider, HookCommands, HookEvent, HookTool};

/// Handle hook subcommands
pub fn handle_hook_command(action: HookCommands) -> ExitCode {
    match action {
        HookCommands::Install { hook_type, hook_event, force, yes, provider, args } => {
            handle_hook_install(hook_type, hook_event, force, yes, provider, args)
        }
        HookCommands::Uninstall { hook_type, hook_event, all, yes } => {
            // Agent type has its own uninstall flow
            if matches!(hook_type, Some(HookTool::Agent)) {
                return handle_agent_hook_uninstall(yes);
            }
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
    force: bool,
    yes: bool,
    provider: Option<String>,
    args: Option<String>,
) -> ExitCode {
    use std::io::{self, Write};

    // Agent type has its own installation flow
    if matches!(hook_type, Some(HookTool::Agent)) {
        // Parse provider as AgentProvider if given
        let agent_provider = provider.as_deref().and_then(|p| {
            match p.to_lowercase().as_str() {
                "claude" => Some(AgentProvider::Claude),
                "cursor" => Some(AgentProvider::Cursor),
                "windsurf" => Some(AgentProvider::Windsurf),
                "copilot" => Some(AgentProvider::Copilot),
                "cline" => Some(AgentProvider::Cline),
                "codebuddy" => Some(AgentProvider::Codebuddy),
                _ => {
                    eprintln!("{}: Unknown agent provider '{}'. Valid options: claude, cursor, windsurf, copilot, cline, codebuddy", "Error".red(), p);
                    None
                }
            }
        });
        // If provider was given but invalid, exit
        if provider.is_some() && agent_provider.is_none() {
            return ExitCode::from(1);
        }
        return handle_agent_hook_install(agent_provider, force, yes);
    }

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
                        return handle_hook_install_impl(hook_type, &hook_event, true, false, args.clone());
                    }
                    "2" => {
                        // Append
                        return handle_hook_install_impl(hook_type, &hook_event, false, true, args.clone());
                    }
                    "3" => {
                        // Backup and replace
                        let backup_path = hook_path.with_extension(format!("{}.backup", hook_filename));
                        if let Err(e) = std::fs::copy(&hook_path, &backup_path) {
                            eprintln!("{}: Failed to create backup: {}", "Error".red(), e);
                            return ExitCode::from(2);
                        }
                        println!("{} Created backup at {}", "✓".green(), backup_path.display());
                        return handle_hook_install_impl(hook_type, &hook_event, true, false, args.clone());
                    }
                    _ => {
                        println!("Installation cancelled");
                        return ExitCode::SUCCESS;
                    }
                }
            } else {
                // Non-interactive mode: append by default
                return handle_hook_install_impl(hook_type, &hook_event, false, true, args.clone());
            }
        }

        println!("  Use {} to overwrite, or {} to append", "--force".yellow(), "choose option 2".cyan());
        return ExitCode::from(1);
    }

    // No existing hook or force mode - create new hook
    handle_hook_install_impl(hook_type, &hook_event, force, false, args)
}

/// Internal implementation of hook installation
fn handle_hook_install_impl(
    hook_type: Option<HookTool>,
    hook_event: &HookEvent,
    force: bool,
    append: bool,
    args: Option<String>,
) -> ExitCode {
    let tool = hook_type.unwrap_or(HookTool::Git);

    // For append mode, we need to modify create_hook_config to support appending
    if append {
        // For now, use create_hook_config which already handles appending for git hooks
        if let Err(exit_code) = create_hook_config(&tool, hook_event, false, &args) {
            return exit_code;
        }
    } else if let Err(exit_code) = create_hook_config(&tool, hook_event, force, &args) {
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

    // Check agent integration status
    println!("\n{}", "Agent Integration".bold());
    let mut any_agent_installed = false;
    for p in ALL_AGENT_PROVIDERS {
        let installed = agent_is_installed(&git_root, p);
        if installed {
            any_agent_installed = true;
            let path = agent_rules_path(&git_root, p);
            println!("{} {} ({})", "✓".green(), p, path.display());
            // Show extra info for Claude (Stop Hook)
            if matches!(p, AgentProvider::Claude) {
                let settings_path = git_root.join(".claude/settings.local.json");
                let has_stop_hook = settings_path.exists()
                    && std::fs::read_to_string(&settings_path)
                        .map(|c| c.contains("linthis"))
                        .unwrap_or(false);
                if has_stop_hook {
                    println!("  {} Stop Hook ({})", "✓".green(), settings_path.display());
                }
            }
        } else {
            println!("{} {} (not installed)", "✗".red(), p);
        }
    }

    println!("\n{}", "Commands:".bold());
    if !any_hook_installed {
        println!("  Install pre-commit:  {}", "linthis hook install".cyan());
        println!("  Install pre-push:    {}", "linthis hook install --event pre-push".cyan());
        println!("  Install commit-msg:  {}", "linthis hook install --event commit-msg".cyan());
    } else {
        println!("  Install hook:   {}", "linthis hook install --event <event>".cyan());
        println!("  Uninstall hook: {}", "linthis hook uninstall --event <event>".cyan());
        println!("  Uninstall all:  {}", "linthis hook uninstall --all".cyan());
    }
    if !any_agent_installed {
        println!("  Install agent:  {}", "linthis hook install --type agent".cyan());
    } else {
        println!("  Install agent:  {}", "linthis hook install --type agent --provider <name>".cyan());
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
        // Uninstall all hooks (including agent hooks)
        let hook_events = [HookEvent::PreCommit, HookEvent::PrePush, HookEvent::CommitMsg];
        let mut any_uninstalled = false;

        for event in &hook_events {
            let result = uninstall_single_hook(&git_root, event, yes);
            if result == ExitCode::SUCCESS {
                any_uninstalled = true;
            }
        }

        // Also uninstall agent hooks
        let agent_result = handle_agent_hook_uninstall(yes);
        if agent_result == ExitCode::SUCCESS {
            any_uninstalled = true;
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
        HookTool::Git | HookTool::Agent => return Ok(()), // Git/Agent hooks don't need install step
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
fn create_hook_config(tool: &HookTool, hook_event: &HookEvent, force: bool, args: &Option<String>) -> Result<(), ExitCode> {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    let hook_filename = hook_event.hook_filename();

    match tool {
        HookTool::Agent => {
            // Agent hooks are handled separately in handle_agent_hook_install
            return Ok(());
        }
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
            let hook_cmd = build_hook_command(hook_event, args);

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
            let linthis_hook_line = build_hook_command(hook_event, args);

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

/// Build the linthis command for a hook based on event type and extra args
fn build_hook_command(
    hook_event: &HookEvent,
    args: &Option<String>,
) -> String {
    match hook_event {
        HookEvent::PreCommit => {
            // For pre-commit: check staged files with hook event output
            // Default args: "-c -f" (check + format)
            let extra = args.as_deref().unwrap_or("-c -f");
            format!("linthis -s {} --hook-event=pre-commit", extra)
        }
        HookEvent::PrePush => {
            // For pre-push: check all files (more comprehensive) with hook event output
            // Default args: "-c -f" (check + format)
            let extra = args.as_deref().unwrap_or("-c -f");
            format!("linthis {} --hook-event=pre-push", extra)
        }
        HookEvent::CommitMsg => {
            // For commit-msg: validate commit message using the msg file passed as $1
            "linthis hook commit-msg-check \"$1\"".to_string()
        }
    }
}

/// Get the git action for a hook event
fn hook_action(hook_event: &HookEvent) -> &'static str {
    match hook_event {
        HookEvent::PreCommit => "commit",
        HookEvent::PrePush => "push",
        HookEvent::CommitMsg => "commit",
    }
}

// =============================================================================
// Agent (AI Coding Agent) Multi-Provider Integration
// =============================================================================

/// All supported agent providers (in display order)
const ALL_AGENT_PROVIDERS: &[AgentProvider] = &[
    AgentProvider::Claude,
    AgentProvider::Cursor,
    AgentProvider::Windsurf,
    AgentProvider::Copilot,
    AgentProvider::Cline,
    AgentProvider::Codebuddy,
];

/// The section marker used in append-style files (CLAUDE.md, copilot-instructions.md)
const AGENT_SECTION_MARKER: &str = "## Linthis Agent Rules";

/// Shared lint rules body (provider-agnostic)
fn agent_lint_rules_body() -> String {
    r#"### After modifying code files

After editing code files, run linthis to check for issues:

```bash
linthis -i <file1> -i <file2> -c
```

- Use separate `-i` flags for each modified file
- Use `-c` (check-only) — do NOT use `--fix` or `linthis fix`
- If lint issues are found, **fix them yourself by editing the code directly**, then re-run linthis to confirm

### Before committing

Always run linthis on staged files before any `git commit`:

```bash
linthis -s -c
```

If issues are found, fix them by editing the code, re-stage, and re-check until clean.

### Key principle

Never rely on `linthis --fix` or `linthis fix` for automated fixing. Always read the lint errors, understand them, and apply fixes manually through code edits. This ensures higher quality fixes with proper context awareness."#
        .to_string()
}

/// Content for CLAUDE.md (append section)
fn agent_content_claude_md() -> String {
    format!("\n{}\n\n{}\n", AGENT_SECTION_MARKER, agent_lint_rules_body())
}

/// Content for Cursor .cursor/rules/linthis.mdc (dedicated file with YAML frontmatter)
fn agent_content_cursor_mdc() -> String {
    format!(
        r#"---
description: Linthis lint rules for code quality
alwaysApply: true
---

# Linthis Agent Rules

{}
"#,
        agent_lint_rules_body()
    )
}

/// Content for Windsurf .windsurf/rules/linthis.md (dedicated file)
fn agent_content_windsurf_md() -> String {
    format!("# Linthis Agent Rules\n\n{}\n", agent_lint_rules_body())
}

/// Content for GitHub Copilot .github/copilot-instructions.md (append section)
fn agent_content_copilot_md() -> String {
    format!("\n{}\n\n{}\n", AGENT_SECTION_MARKER, agent_lint_rules_body())
}

/// Content for Cline .clinerules/linthis.md (dedicated file)
fn agent_content_cline_md() -> String {
    format!("# Linthis Agent Rules\n\n{}\n", agent_lint_rules_body())
}

/// Content for CodeBuddy .codebuddy/rules/linthis.md (dedicated file)
fn agent_content_codebuddy_md() -> String {
    format!("# Linthis Agent Rules\n\n{}\n", agent_lint_rules_body())
}

/// Generate the Stop hook JSON content for .claude/settings.local.json
fn agent_stop_hook_json() -> String {
    r#"{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "prompt",
            "prompt": "Before finishing, check if any code files were modified during this session (Write/Edit/Bash tools). If code was modified:\n1. Run `linthis -i <file1> -i <file2> -c` on all modified files to check for lint issues\n2. If issues are found, fix them yourself by editing the code directly (do NOT use `linthis --fix` or `linthis fix`)\n3. Re-run `linthis -i <files> -c` to confirm all issues are resolved\n4. Only approve stopping once lint passes with no errors\n\nIf no code files were modified, approve stopping immediately."
          }
        ]
      }
    ]
  }
}"#
    .to_string()
}

/// Get the rules file path for a given agent provider
fn agent_rules_path(git_root: &std::path::Path, provider: &AgentProvider) -> PathBuf {
    match provider {
        AgentProvider::Claude => git_root.join("CLAUDE.md"),
        AgentProvider::Cursor => git_root.join(".cursor/rules/linthis.mdc"),
        AgentProvider::Windsurf => git_root.join(".windsurf/rules/linthis.md"),
        AgentProvider::Copilot => git_root.join(".github/copilot-instructions.md"),
        AgentProvider::Cline => git_root.join(".clinerules/linthis.md"),
        AgentProvider::Codebuddy => git_root.join(".codebuddy/rules/linthis.md"),
    }
}

/// Print info about an already-installed agent provider (file path + content)
fn print_agent_installed_info(git_root: &std::path::Path, provider: &AgentProvider) {
    let path = agent_rules_path(git_root, provider);
    println!(
        "       {} {}",
        "File:".dimmed(),
        path.display()
    );

    // For Claude, also show settings file
    if matches!(provider, AgentProvider::Claude) {
        let settings_path = git_root.join(".claude/settings.local.json");
        if settings_path.exists() {
            println!(
                "       {} {}",
                "File:".dimmed(),
                settings_path.display()
            );
        }
    }

    // Show the installed linthis content section
    if let Ok(content) = std::fs::read_to_string(&path) {
        match provider {
            // Append-style: extract the linthis section
            AgentProvider::Claude | AgentProvider::Copilot => {
                if let Some(start) = content.find(AGENT_SECTION_MARKER) {
                    let section = &content[start..];
                    println!("       {}:", "Content".dimmed());
                    for line in section.lines() {
                        println!("       {}", line.dimmed());
                    }
                }
            }
            // Dedicated file: show full content
            _ => {
                println!("       {}:", "Content".dimmed());
                for line in content.lines() {
                    println!("       {}", line.dimmed());
                }
            }
        }
    }
}

/// Check if agent integration is installed for a given provider
fn agent_is_installed(git_root: &std::path::Path, provider: &AgentProvider) -> bool {
    let path = agent_rules_path(git_root, provider);
    match provider {
        // Append-style: check for section marker in file
        AgentProvider::Claude | AgentProvider::Copilot => {
            path.exists()
                && std::fs::read_to_string(&path)
                    .map(|c| c.contains(AGENT_SECTION_MARKER))
                    .unwrap_or(false)
        }
        // Dedicated file: check if file exists and contains linthis
        AgentProvider::Cursor
        | AgentProvider::Windsurf
        | AgentProvider::Cline
        | AgentProvider::Codebuddy => {
            path.exists()
                && std::fs::read_to_string(&path)
                    .map(|c| c.contains("linthis") || c.contains("Linthis"))
                    .unwrap_or(false)
        }
    }
}

/// Detect which agent providers are likely in use (by checking for their directories)
fn detect_agent_providers(git_root: &std::path::Path) -> Vec<AgentProvider> {
    let mut detected = Vec::new();
    if git_root.join(".claude").exists() {
        detected.push(AgentProvider::Claude);
    }
    if git_root.join(".cursor").exists() {
        detected.push(AgentProvider::Cursor);
    }
    if git_root.join(".windsurf").exists() {
        detected.push(AgentProvider::Windsurf);
    }
    if git_root.join(".github").exists() {
        detected.push(AgentProvider::Copilot);
    }
    if git_root.join(".clinerules").exists() {
        detected.push(AgentProvider::Cline);
    }
    if git_root.join(".codebuddy").exists() {
        detected.push(AgentProvider::Codebuddy);
    }
    detected
}

/// Lightweight agent provider detection for dynamic help text.
///
/// Returns a list of (display_name, detected) tuples using the project root.
/// This avoids requiring a git root parameter.
pub fn detect_agent_providers_lightweight() -> Vec<(&'static str, bool)> {
    let root = linthis::utils::get_project_root();
    ALL_AGENT_PROVIDERS
        .iter()
        .map(|p| {
            let name = match p {
                AgentProvider::Claude => "Claude Code",
                AgentProvider::Cursor => "Cursor",
                AgentProvider::Windsurf => "Windsurf",
                AgentProvider::Copilot => "GitHub Copilot",
                AgentProvider::Cline => "Cline",
                AgentProvider::Codebuddy => "CodeBuddy",
            };
            let dir = match p {
                AgentProvider::Claude => ".claude",
                AgentProvider::Cursor => ".cursor",
                AgentProvider::Windsurf => ".windsurf",
                AgentProvider::Copilot => ".github",
                AgentProvider::Cline => ".clinerules",
                AgentProvider::Codebuddy => ".codebuddy",
            };
            (name, root.join(dir).exists())
        })
        .collect()
}

/// Install a dedicated rules file (Cursor, Windsurf, Cline, CodeBuddy)
fn install_agent_dedicated_file(path: &std::path::Path, content: &str) -> Result<(), String> {
    use std::fs;

    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
        }
    }

    fs::write(path, content)
        .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;

    Ok(())
}

/// Install rules by appending a section to an existing file (Claude CLAUDE.md, Copilot copilot-instructions.md)
fn install_agent_append_rules(
    path: &std::path::Path,
    content: &str,
    default_header: &str,
) -> Result<(), String> {
    use std::fs;

    if path.exists() {
        let existing = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        let new_content = if existing.contains(AGENT_SECTION_MARKER) {
            // Replace existing section
            if let Some(start) = existing.find(AGENT_SECTION_MARKER) {
                let after_marker = &existing[start + AGENT_SECTION_MARKER.len()..];
                let section_end = after_marker
                    .find("\n## ")
                    .map(|pos| start + AGENT_SECTION_MARKER.len() + pos)
                    .unwrap_or(existing.len());

                let mut result = existing[..start].trim_end().to_string();
                result.push_str(content);
                let remaining = existing[section_end..].trim_start();
                if !remaining.is_empty() {
                    result.push_str(remaining);
                    if !result.ends_with('\n') {
                        result.push('\n');
                    }
                }
                result
            } else {
                let mut result = existing.trim_end().to_string();
                result.push_str(content);
                result
            }
        } else {
            let mut result = existing.trim_end().to_string();
            result.push_str(content);
            result
        };

        fs::write(path, new_content)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
    } else {
        // Create new file with header
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
            }
        }
        let new_content = format!("{}{}", default_header, content);
        fs::write(path, new_content)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
    }

    Ok(())
}

/// Install agent integration for a specific provider
fn install_agent_provider(git_root: &std::path::Path, provider: &AgentProvider) -> Result<(), String> {
    let rules_path = agent_rules_path(git_root, provider);

    match provider {
        AgentProvider::Claude => {
            // Install CLAUDE.md rules (append)
            install_agent_append_rules(&rules_path, &agent_content_claude_md(), "# Project Instructions\n")?;
            // Also install Stop Hook
            let settings_path = git_root.join(".claude/settings.local.json");
            install_agent_stop_hook(git_root, &settings_path)?;
        }
        AgentProvider::Cursor => {
            install_agent_dedicated_file(&rules_path, &agent_content_cursor_mdc())?;
        }
        AgentProvider::Windsurf => {
            install_agent_dedicated_file(&rules_path, &agent_content_windsurf_md())?;
        }
        AgentProvider::Copilot => {
            install_agent_append_rules(&rules_path, &agent_content_copilot_md(), "# Copilot Instructions\n")?;
        }
        AgentProvider::Cline => {
            install_agent_dedicated_file(&rules_path, &agent_content_cline_md())?;
        }
        AgentProvider::Codebuddy => {
            install_agent_dedicated_file(&rules_path, &agent_content_codebuddy_md())?;
        }
    }

    Ok(())
}

/// Uninstall agent integration for a specific provider
fn uninstall_agent_provider(git_root: &std::path::Path, provider: &AgentProvider) -> Result<(), String> {
    match provider {
        AgentProvider::Claude => {
            let claude_md = agent_rules_path(git_root, provider);
            if claude_md.exists() {
                remove_agent_section_from_file(&claude_md)?;
            }
            let settings_path = git_root.join(".claude/settings.local.json");
            if settings_path.exists() {
                remove_agent_stop_hook(&settings_path)?;
            }
        }
        AgentProvider::Copilot => {
            let copilot_md = agent_rules_path(git_root, provider);
            if copilot_md.exists() {
                remove_agent_section_from_file(&copilot_md)?;
            }
        }
        AgentProvider::Cursor
        | AgentProvider::Windsurf
        | AgentProvider::Cline
        | AgentProvider::Codebuddy => {
            let path = agent_rules_path(git_root, provider);
            remove_agent_dedicated_file(&path)?;
        }
    }

    Ok(())
}

/// Remove a dedicated rules file and clean up empty parent directories
fn remove_agent_dedicated_file(path: &std::path::Path) -> Result<(), String> {
    use std::fs;

    if path.exists() {
        fs::remove_file(path)
            .map_err(|e| format!("Failed to remove {}: {}", path.display(), e))?;

        // Try to remove empty parent directories (e.g., .cursor/rules/)
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir(parent); // Ignore error if not empty
            if let Some(grandparent) = parent.parent() {
                // Only remove if it's a dotdir like .cursor, .windsurf etc.
                if grandparent
                    .file_name()
                    .map(|n| n.to_string_lossy().starts_with('.'))
                    .unwrap_or(false)
                {
                    let _ = fs::remove_dir(grandparent);
                }
            }
        }
    }

    Ok(())
}

/// Remove the linthis section from a file (CLAUDE.md, copilot-instructions.md)
fn remove_agent_section_from_file(path: &std::path::Path) -> Result<(), String> {
    use std::fs;

    let existing = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    if let Some(start) = existing.find(AGENT_SECTION_MARKER) {
        let after_marker = &existing[start + AGENT_SECTION_MARKER.len()..];
        let section_end = after_marker
            .find("\n## ")
            .map(|pos| start + AGENT_SECTION_MARKER.len() + pos)
            .unwrap_or(existing.len());

        let mut result = existing[..start].trim_end().to_string();
        let remaining = existing[section_end..].trim_start();
        if !remaining.is_empty() {
            result.push_str("\n\n");
            result.push_str(remaining);
        }
        if !result.ends_with('\n') {
            result.push('\n');
        }

        fs::write(path, result)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
    }

    Ok(())
}

/// Install the Stop Hook into .claude/settings.local.json
fn install_agent_stop_hook(
    git_root: &std::path::Path,
    settings_path: &std::path::Path,
) -> Result<(), String> {
    use std::fs;

    let claude_dir = git_root.join(".claude");
    if !claude_dir.exists() {
        fs::create_dir_all(&claude_dir)
            .map_err(|e| format!("Failed to create .claude directory: {}", e))?;
    }

    if settings_path.exists() {
        let existing = fs::read_to_string(settings_path)
            .map_err(|e| format!("Failed to read {}: {}", settings_path.display(), e))?;

        let mut json: serde_json::Value = serde_json::from_str(&existing)
            .map_err(|e| format!("Failed to parse {}: {}", settings_path.display(), e))?;

        let stop_hook_json: serde_json::Value = serde_json::from_str(&agent_stop_hook_json())
            .map_err(|e| format!("Failed to parse stop hook template: {}", e))?;

        let hooks = json
            .as_object_mut()
            .ok_or("settings.local.json root is not an object")?
            .entry("hooks")
            .or_insert_with(|| serde_json::json!({}));

        let hooks_obj = hooks
            .as_object_mut()
            .ok_or("hooks field is not an object")?;

        if let Some(stop_hooks) = stop_hook_json.get("hooks").and_then(|h| h.get("Stop")) {
            hooks_obj.insert("Stop".to_string(), stop_hooks.clone());
        }

        let output = serde_json::to_string_pretty(&json)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
        fs::write(settings_path, output + "\n")
            .map_err(|e| format!("Failed to write {}: {}", settings_path.display(), e))?;
    } else {
        fs::write(settings_path, agent_stop_hook_json() + "\n")
            .map_err(|e| format!("Failed to write {}: {}", settings_path.display(), e))?;
    }

    Ok(())
}

/// Remove the Stop Hook from .claude/settings.local.json
fn remove_agent_stop_hook(settings_path: &std::path::Path) -> Result<(), String> {
    use std::fs;

    let existing = fs::read_to_string(settings_path)
        .map_err(|e| format!("Failed to read {}: {}", settings_path.display(), e))?;

    let mut json: serde_json::Value = serde_json::from_str(&existing)
        .map_err(|e| format!("Failed to parse {}: {}", settings_path.display(), e))?;

    if let Some(hooks) = json.get_mut("hooks").and_then(|h| h.as_object_mut()) {
        hooks.remove("Stop");
        if hooks.is_empty() {
            json.as_object_mut().unwrap().remove("hooks");
        }
    }

    if json.as_object().map(|o| o.is_empty()).unwrap_or(false) {
        fs::remove_file(settings_path)
            .map_err(|e| format!("Failed to remove {}: {}", settings_path.display(), e))?;
        if let Some(parent) = settings_path.parent() {
            let _ = fs::remove_dir(parent);
        }
    } else {
        let output = serde_json::to_string_pretty(&json)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
        fs::write(settings_path, output + "\n")
            .map_err(|e| format!("Failed to write {}: {}", settings_path.display(), e))?;
    }

    Ok(())
}

/// Install agent hooks with multi-provider support
fn handle_agent_hook_install(
    provider: Option<AgentProvider>,
    force: bool,
    yes: bool,
) -> ExitCode {
    use std::io::{self, Write};

    let git_root = match find_git_root() {
        Some(root) => root,
        None => {
            eprintln!("{}: Not in a git repository", "Error".red());
            return ExitCode::from(1);
        }
    };

    println!("{}", "🤖 AI Coding Agent Integration".bold());
    println!();

    // If a specific provider was given, install just that one
    if let Some(ref p) = provider {
        let installed = agent_is_installed(&git_root, p);
        if installed && !force {
            println!(
                "{}: {} is already installed",
                "Info".cyan(),
                p
            );
            print_agent_installed_info(&git_root, p);
            return ExitCode::SUCCESS;
        }

        match install_agent_provider(&git_root, p) {
            Ok(_) => {
                let path = agent_rules_path(&git_root, p);
                println!("{} Installed {} → {}", "✓".green(), p, path.display());
                if matches!(p, AgentProvider::Claude) {
                    let settings_path = git_root.join(".claude/settings.local.json");
                    println!(
                        "{} Installed Stop Hook → {}",
                        "✓".green(),
                        settings_path.display()
                    );
                }
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("{}: Failed to install {}: {}", "Error".red(), p, e);
                return ExitCode::from(2);
            }
        }
    }

    // Auto-detect and install all if -y
    if yes {
        let detected = detect_agent_providers(&git_root);
        let targets: Vec<AgentProvider> = if detected.is_empty() {
            ALL_AGENT_PROVIDERS.to_vec()
        } else {
            detected
        };

        let mut any_installed = false;
        for p in &targets {
            if agent_is_installed(&git_root, p) && !force {
                println!("{}: {} already installed", "Info".cyan(), p);
                print_agent_installed_info(&git_root, p);
                continue;
            }
            match install_agent_provider(&git_root, p) {
                Ok(_) => {
                    let path = agent_rules_path(&git_root, p);
                    println!("{} Installed {} → {}", "✓".green(), p, path.display());
                    if matches!(p, AgentProvider::Claude) {
                        let settings_path = git_root.join(".claude/settings.local.json");
                        println!(
                            "{} Installed Stop Hook → {}",
                            "✓".green(),
                            settings_path.display()
                        );
                    }
                    any_installed = true;
                }
                Err(e) => {
                    eprintln!("{}: Failed to install {}: {}", "Error".red(), p, e);
                }
            }
        }

        if any_installed {
            println!();
            println!("{}", "Agents will auto-check code quality after edits.".bold());
        }
        return ExitCode::SUCCESS;
    }

    // Interactive menu
    let detected = detect_agent_providers(&git_root);

    // Build ordered list: detected/installed first, then others
    let mut ordered: Vec<&AgentProvider> = Vec::new();
    for p in ALL_AGENT_PROVIDERS {
        if detected.iter().any(|d| std::mem::discriminant(d) == std::mem::discriminant(p))
            || agent_is_installed(&git_root, p)
        {
            ordered.push(p);
        }
    }
    for p in ALL_AGENT_PROVIDERS {
        if !ordered.iter().any(|o| std::mem::discriminant(*o) == std::mem::discriminant(p)) {
            ordered.push(p);
        }
    }
    let provider_count = ordered.len();

    println!("Select agent(s) to integrate with linthis:");
    println!();

    for (i, p) in ordered.iter().enumerate() {
        let is_installed = agent_is_installed(&git_root, p);
        let is_detected = detected.iter().any(|d| std::mem::discriminant(d) == std::mem::discriminant(p));
        let mut status_parts = Vec::new();
        if is_installed {
            status_parts.push("installed".to_string());
        }
        if is_detected && !is_installed {
            status_parts.push("detected".to_string());
        }
        let status = if status_parts.is_empty() {
            String::new()
        } else {
            format!(" ({})", status_parts.join(", "))
        };
        println!("  {}. {}{}", i + 1, p, if is_installed {
            format!(" {}", status.yellow())
        } else if is_detected {
            format!(" {}", status.cyan())
        } else {
            status
        });
    }

    println!();
    println!("  {}. All detected agents", provider_count + 1);
    println!("  {}. All agents", provider_count + 2);
    println!("  {}. Cancel", provider_count + 3);
    println!();
    print!("Choose (comma-separated for multiple, e.g. 1,2): ");
    io::stdout().flush().unwrap();

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).ok();
    let choice = choice.trim();

    let cancel_num = provider_count + 3;
    if choice == cancel_num.to_string() || choice.is_empty() {
        println!("Installation cancelled");
        return ExitCode::SUCCESS;
    }

    let all_detected_num = provider_count + 1;
    let all_agents_num = provider_count + 2;

    let selected: Vec<&AgentProvider> = if choice == all_detected_num.to_string() {
        if detected.is_empty() {
            println!("{}: No agents detected, installing all", "Info".cyan());
            ordered.clone()
        } else {
            detected.iter().collect()
        }
    } else if choice == all_agents_num.to_string() {
        ordered.clone()
    } else {
        let mut selected = Vec::new();
        for part in choice.split(',') {
            if let Ok(num) = part.trim().parse::<usize>() {
                if num >= 1 && num <= provider_count {
                    selected.push(ordered[num - 1]);
                }
            }
        }
        if selected.is_empty() {
            println!("Installation cancelled");
            return ExitCode::SUCCESS;
        }
        selected
    };

    println!();
    let mut any_installed = false;
    for p in &selected {
        if agent_is_installed(&git_root, p) && !force {
            println!("{}: {} already installed", "Info".cyan(), p);
            print_agent_installed_info(&git_root, p);
            continue;
        }
        match install_agent_provider(&git_root, p) {
            Ok(_) => {
                let path = agent_rules_path(&git_root, p);
                println!("{} Installed {} → {}", "✓".green(), p, path.display());
                if matches!(p, AgentProvider::Claude) {
                    let settings_path = git_root.join(".claude/settings.local.json");
                    println!(
                        "{} Installed Stop Hook → {}",
                        "✓".green(),
                        settings_path.display()
                    );
                }
                any_installed = true;
            }
            Err(e) => {
                eprintln!("{}: Failed to install {}: {}", "Error".red(), p, e);
            }
        }
    }

    if any_installed {
        println!();
        println!("{}", "Agents will auto-check code quality after edits.".bold());
    }

    ExitCode::SUCCESS
}

/// Uninstall agent hooks for all installed providers
fn handle_agent_hook_uninstall(yes: bool) -> ExitCode {
    use std::io::{self, Write};

    let git_root = match find_git_root() {
        Some(root) => root,
        None => {
            return ExitCode::from(1);
        }
    };

    // Find all installed providers
    let installed: Vec<&AgentProvider> = ALL_AGENT_PROVIDERS
        .iter()
        .filter(|p| agent_is_installed(&git_root, p))
        .collect();

    if installed.is_empty() {
        return ExitCode::from(1); // Nothing to uninstall
    }

    if !yes {
        println!("{}", "Agent Integration:".bold());
        for p in &installed {
            let path = agent_rules_path(&git_root, p);
            println!("  {} {} ({})", "✓".green(), p, path.display());
        }
        println!();
        print!("Remove agent integration? [y/N]: ");
        io::stdout().flush().unwrap();

        let mut answer = String::new();
        io::stdin().read_line(&mut answer).ok();
        if !answer.trim().eq_ignore_ascii_case("y") {
            println!("Uninstall cancelled");
            return ExitCode::SUCCESS;
        }
    }

    let mut any_removed = false;
    for p in &installed {
        match uninstall_agent_provider(&git_root, p) {
            Ok(_) => {
                println!("{} Removed {} integration", "✓".green(), p);
                any_removed = true;
            }
            Err(e) => {
                eprintln!("{}: Failed to remove {}: {}", "Error".red(), p, e);
            }
        }
    }

    if any_removed {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
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
