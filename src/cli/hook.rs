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

use super::commands::{AgentFixProvider, AgentProvider, HookCommands, HookEvent, HookTool};

/// Helper module for getting home directory (cross-platform, no external crate)
mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var("HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
    }
}

/// Handle hook subcommands
pub fn handle_hook_command(action: HookCommands) -> ExitCode {
    match action {
        HookCommands::Install { hook_type, hook_event, force, yes, global, provider, args } => {
            handle_hook_install(hook_type, hook_event, force, yes, global, provider, args)
        }
        HookCommands::Uninstall { hook_type, hook_event, all, yes, global } => {
            // Agent type has its own uninstall flow
            if matches!(hook_type, Some(HookTool::Agent)) {
                return handle_agent_hook_uninstall(yes, global);
            }
            handle_hook_uninstall(hook_event, all, yes, global)
        }
        HookCommands::Status => {
            handle_hook_status()
        }
        HookCommands::Check => {
            handle_hook_check()
        }
        HookCommands::CommitMsgCheck { msg_or_file } => {
            handle_commit_msg_check(&msg_or_file)
        }
    }
}

/// Install git hook (pre-commit, pre-push, or commit-msg)
fn handle_hook_install(
    hook_type: Option<HookTool>,
    hook_event: HookEvent,
    force: bool,
    yes: bool,
    global: bool,
    provider: Option<String>,
    args: Option<String>,
) -> ExitCode {
    use std::io::{self, Write};

    // *-with-agent types: install base hook + agent fix fallback
    if hook_type.as_ref().map(|t| t.has_agent_fix()).unwrap_or(false) {
        let fix_provider = match resolve_agent_fix_provider(provider.as_deref(), yes) {
            Ok(p)  => p,
            Err(e) => return e,
        };
        let base = hook_type.as_ref().unwrap().base_tool().clone();
        return match &base {
            HookTool::Git => handle_git_with_agent_install(&hook_event, force, &fix_provider, &args),
            HookTool::Prek | HookTool::PreCommit => {
                handle_precommit_with_agent_install(&base, &hook_event, force, &fix_provider, &args)
            }
            _ => ExitCode::from(1),
        };
    }

    // Agent type has its own installation flow
    if matches!(hook_type, Some(HookTool::Agent)) {
        // Parse provider as AgentProvider if given
        let agent_provider = provider.as_deref().and_then(|p| {
            match p.to_lowercase().as_str() {
                "claude"    => Some(AgentProvider::Claude),
                "codex"     => Some(AgentProvider::Codex),
                "gemini"    => Some(AgentProvider::Gemini),
                "cursor"    => Some(AgentProvider::Cursor),
                "droid"     => Some(AgentProvider::Droid),
                "auggie" | "aug" | "augment" => Some(AgentProvider::Auggie),
                "codebuddy" => Some(AgentProvider::Codebuddy),
                _ => {
                    eprintln!("{}: Unknown agent provider '{}'. Valid options: claude, codex, gemini, cursor, droid, auggie, codebuddy", "Error".red(), p);
                    None
                }
            }
        });
        // If provider was given but invalid, exit
        if provider.is_some() && agent_provider.is_none() {
            return ExitCode::from(1);
        }
        return handle_agent_hook_install(agent_provider, force, yes, global);
    }

    // Global non-agent hook: install into ~/.config/git/hooks
    if global {
        return handle_global_hook_install(hook_type, &hook_event, force, yes, &args);
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

// =============================================================================
// Global hook installation (git config --global core.hooksPath)
// =============================================================================

/// Global hooks directory (XDG standard).
fn global_hooks_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".config/git/hooks"))
}

/// Build the global hook script with the hook event name substituted.
fn build_global_hook_script_for_event(
    hook_event: &HookEvent,
    args: &Option<String>,
    fix_provider: Option<&AgentFixProvider>,
) -> String {
    let linthis_cmd = build_hook_command(hook_event, args);

    // For commit-msg, git passes the message file as $1.  Strip the literal
    // "$1" from the command string so it is not embedded inside the variable
    // assignment (which would break with paths containing spaces).  We instead
    // call `$LINTHIS_CMD "$@"` at every invocation site so the argument is
    // forwarded correctly.  For pre-commit / pre-push, "$@" expands to nothing,
    // so the change is a no-op for those events.
    let linthis_cmd_var = match hook_event {
        HookEvent::CommitMsg => linthis_cmd
            .trim_end_matches(" \"$1\"")
            .to_string(),
        _ => linthis_cmd.clone(),
    };

    let error_msg = agent_fix_error_msg(hook_event);
    let fix_block = match fix_provider {
        None => String::new(),
        Some(p) => {
            let prompt = agent_fix_prompt_for_event(hook_event);
            let agent_cmd = agent_fix_headless_cmd(p, &prompt);
            format!(
                "  if [ $LINTHIS_EXIT -ne 0 ]; then\n\
                 \x20\x20\x20 echo \"[linthis] {error_msg}. Invoking {provider} to fix...\"\n\
                 \x20\x20\x20 {agent}\n\
                 \x20\x20\x20 $LINTHIS_CMD \"$@\"\n\
                 \x20\x20\x20 LINTHIS_EXIT=$?\n\
                 \x20 fi\n",
                provider = p,
                agent = agent_cmd,
                error_msg = error_msg,
            )
        }
    };
    let fix_block_direct = match fix_provider {
        None => String::new(),
        Some(p) => {
            let prompt = agent_fix_prompt_for_event(hook_event);
            let agent_cmd = agent_fix_headless_cmd(p, &prompt);
            format!(
                "  if [ $LINTHIS_EXIT -ne 0 ]; then\n\
                 \x20\x20\x20 echo \"[linthis] {error_msg}. Invoking {provider} to fix...\"\n\
                 \x20\x20\x20 {agent}\n\
                 \x20\x20\x20 $LINTHIS_CMD \"$@\"\n\
                 \x20\x20\x20 LINTHIS_EXIT=$?\n\
                 \x20 fi\n",
                provider = p,
                agent = agent_cmd,
                error_msg = error_msg,
            )
        }
    };

    let event_name = hook_event.hook_filename();
    format!(
        "#!/bin/sh\n\
         # linthis-hook\n\
         \n\
         LINTHIS_CMD=\"{linthis}\"\n\
         \n\
         # Locate the local project hook (git-dir aware)\n\
         GIT_DIR=\"$(git rev-parse --git-dir 2>/dev/null)\"\n\
         LOCAL_HOOK=\"\"\n\
         if [ -n \"$GIT_DIR\" ]; then\n\
         \x20 LOCAL_HOOK=\"$GIT_DIR/hooks/{event}\"\n\
         fi\n\
         \n\
         if [ -f \"$LOCAL_HOOK\" ] && [ -x \"$LOCAL_HOOK\" ]; then\n\
         \x20 if grep -qE '^[^#]*linthis' \"$LOCAL_HOOK\" 2>/dev/null; then\n\
         \x20\x20\x20 # Local hook already calls linthis — delegate entirely\n\
         \x20\x20\x20 exec \"$LOCAL_HOOK\" \"$@\"\n\
         \x20 else\n\
         \x20\x20\x20 # Local hook exists but has no linthis — run linthis first, then delegate\n\
         \x20\x20\x20 $LINTHIS_CMD \"$@\"\n\
         \x20\x20\x20 LINTHIS_EXIT=$?\n\
         {fix_local}\
         \x20\x20\x20 \"$LOCAL_HOOK\" \"$@\"\n\
         \x20\x20\x20 LOCAL_EXIT=$?\n\
         \x20\x20\x20 [ $LINTHIS_EXIT -ne 0 ] && exit $LINTHIS_EXIT\n\
         \x20\x20\x20 exit $LOCAL_EXIT\n\
         \x20 fi\n\
         else\n\
         \x20 # No local hook — run linthis directly\n\
         \x20 $LINTHIS_CMD \"$@\"\n\
         \x20 LINTHIS_EXIT=$?\n\
         {fix_direct}\
         \x20 exit $LINTHIS_EXIT\n\
         fi\n",
        linthis = linthis_cmd_var,
        event = event_name,
        fix_local = fix_block,
        fix_direct = fix_block_direct,
    )
}

/// Install a global git hook into ~/.config/git/hooks/<event>.
///
/// After writing the script, configures `git config --global core.hooksPath`
/// to point at that directory.
fn handle_global_hook_install(
    hook_type: Option<HookTool>,
    hook_event: &HookEvent,
    force: bool,
    yes: bool,
    args: &Option<String>,
) -> ExitCode {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::io::{self, Write};

    // Resolve agent fix provider for *-with-agent types
    let fix_provider: Option<AgentFixProvider> =
        if hook_type.as_ref().map(|t| t.has_agent_fix()).unwrap_or(false) {
            match resolve_agent_fix_provider(None, yes) {
                Ok(p) => Some(p),
                Err(e) => return e,
            }
        } else {
            None
        };

    let hooks_dir = match global_hooks_dir() {
        Some(d) => d,
        None => {
            eprintln!("{}: Could not determine home directory", "Error".red());
            return ExitCode::from(1);
        }
    };

    let hook_filename = hook_event.hook_filename();
    let hook_path = hooks_dir.join(hook_filename);

    // Confirm with user unless --yes
    if !yes {
        println!(
            "This will install a global {} hook at {}",
            hook_filename.cyan(),
            hook_path.display()
        );
        println!(
            "and set {} in your global git config.",
            "core.hooksPath".cyan()
        );
        print!("Continue? [y/N]: ");
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Installation cancelled");
            return ExitCode::SUCCESS;
        }
    }

    // Check if already exists
    if hook_path.exists() && !force {
        if let Ok(existing) = fs::read_to_string(&hook_path) {
            if existing.contains("# linthis-hook") {
                println!(
                    "{}: Global {} hook already installed at {}",
                    "Info".cyan(),
                    hook_filename,
                    hook_path.display()
                );
                return ExitCode::SUCCESS;
            }
        }
        eprintln!(
            "{}: {} already exists (not by linthis). Use --force to overwrite.",
            "Warning".yellow(),
            hook_path.display()
        );
        return ExitCode::from(1);
    }

    // Create directory
    if let Err(e) = fs::create_dir_all(&hooks_dir) {
        eprintln!("{}: Failed to create {}: {}", "Error".red(), hooks_dir.display(), e);
        return ExitCode::from(2);
    }

    // Generate script
    let content = build_global_hook_script_for_event(hook_event, args, fix_provider.as_ref());

    // Write hook file
    match fs::write(&hook_path, &content) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{}: Failed to write {}: {}", "Error".red(), hook_path.display(), e);
            return ExitCode::from(2);
        }
    }

    // Make executable on Unix
    #[cfg(unix)]
    {
        if let Ok(meta) = fs::metadata(&hook_path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&hook_path, perms);
        }
    }

    // Set git config --global core.hooksPath
    let hooks_dir_str = hooks_dir.to_string_lossy().to_string();
    let git_config_result = std::process::Command::new("git")
        .args(["config", "--global", "core.hooksPath", &hooks_dir_str])
        .status();

    match git_config_result {
        Ok(status) if status.success() => {
            println!("{} Installed global {} hook → {}", "✓".green(), hook_filename, hook_path.display());
            println!("{} Set {} = {}", "✓".green(), "core.hooksPath".cyan(), hooks_dir_str);
            println!();
            println!("  {}", "How it works (Strategy B — local takes priority):".dimmed());
            println!("  {} If local hook has linthis → global delegates entirely", "·".dimmed());
            println!("  {} If local hook has no linthis → global runs linthis first, then delegates", "·".dimmed());
            println!("  {} No local hook → global runs linthis directly", "·".dimmed());
        }
        Ok(_) | Err(_) => {
            println!("{} Installed global {} hook → {}", "✓".green(), hook_filename, hook_path.display());
            eprintln!(
                "{}: Failed to set core.hooksPath automatically. Run manually:\n  git config --global core.hooksPath {}",
                "Warning".yellow(),
                hooks_dir_str
            );
        }
    }

    ExitCode::SUCCESS
}

/// Uninstall a global git hook from ~/.config/git/hooks/<event>.
///
/// If no linthis hooks remain in that directory, also unsets `core.hooksPath`.
fn handle_global_hook_uninstall(hook_event: Option<HookEvent>, all: bool, yes: bool) -> ExitCode {
    use std::fs;
    use std::io::{self, Write};

    let hooks_dir = match global_hooks_dir() {
        Some(d) => d,
        None => {
            eprintln!("{}: Could not determine home directory", "Error".red());
            return ExitCode::from(1);
        }
    };

    let events_to_remove: Vec<HookEvent> = if all {
        vec![HookEvent::PreCommit, HookEvent::PrePush, HookEvent::CommitMsg]
    } else {
        vec![hook_event.unwrap_or(HookEvent::PreCommit)]
    };

    let mut any_removed = false;

    for event in &events_to_remove {
        let hook_path = hooks_dir.join(event.hook_filename());
        if !hook_path.exists() {
            continue;
        }

        let has_linthis = fs::read_to_string(&hook_path)
            .map(|c| c.contains("# linthis-hook"))
            .unwrap_or(false);

        if !has_linthis {
            continue;
        }

        if !yes {
            print!("Remove global {} hook at {}? [y/N]: ", event.hook_filename().cyan(), hook_path.display());
            io::stdout().flush().ok();
            let mut input = String::new();
            io::stdin().read_line(&mut input).ok();
            if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                println!("Skipped {}", event.hook_filename());
                continue;
            }
        }

        match fs::remove_file(&hook_path) {
            Ok(_) => {
                println!("{} Removed global {} hook", "✓".green(), event.hook_filename());
                any_removed = true;
            }
            Err(e) => {
                eprintln!("{}: Failed to remove {}: {}", "Error".red(), hook_path.display(), e);
            }
        }
    }

    if !any_removed {
        println!("{}: No global linthis hooks found", "Info".cyan());
        return ExitCode::SUCCESS;
    }

    // Check if any linthis hooks remain; if not, unset core.hooksPath
    let remaining = [HookEvent::PreCommit, HookEvent::PrePush, HookEvent::CommitMsg]
        .iter()
        .any(|e| {
            let p = hooks_dir.join(e.hook_filename());
            p.exists() && fs::read_to_string(&p).map(|c| c.contains("# linthis-hook")).unwrap_or(false)
        });

    if !remaining {
        let _ = std::process::Command::new("git")
            .args(["config", "--global", "--unset", "core.hooksPath"])
            .status();
        println!("{} Unset global {}", "✓".green(), "core.hooksPath".cyan());
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

    // --- Project-level hooks ---
    println!("{}", "Project Hooks (.git/hooks/):".bold());
    for event in &hook_events {
        let hook_path = git_root.join(".git/hooks").join(event.hook_filename());

        if hook_path.exists() {
            any_hook_installed = true;
            println!("{} {} [project]", "✓".green(), hook_path.display());
            println!("    {}", event.description().dimmed());

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

    // --- Global hooks ---
    println!();
    println!("{}", "Global Hooks (~/.config/git/hooks/):".bold());
    let global_hooks_path = global_hooks_dir();
    // Check if core.hooksPath is configured
    let core_hooks_path = std::process::Command::new("git")
        .args(["config", "--global", "core.hooksPath"])
        .output()
        .ok()
        .and_then(|o| if o.status.success() { Some(String::from_utf8_lossy(&o.stdout).trim().to_string()) } else { None });

    if let Some(ref path_str) = core_hooks_path {
        println!("  {} = {}", "core.hooksPath".cyan(), path_str);
    } else {
        println!("  {} (core.hooksPath not set)", "ℹ".cyan());
    }

    let mut any_global_hook = false;
    if let Some(ref ghooks_dir) = global_hooks_path {
        for event in &hook_events {
            let hook_path = ghooks_dir.join(event.hook_filename());
            if hook_path.exists() {
                any_global_hook = true;
                if let Ok(content) = std::fs::read_to_string(&hook_path) {
                    let has_linthis = content.contains("# linthis-hook");
                    if has_linthis {
                        println!("{} {} [global]", "✓".green(), hook_path.display());
                        println!("    {} Strategy B: local hook takes priority", "ℹ".dimmed());
                    } else {
                        println!("{} {} [global, not by linthis]", "⚠".yellow(), hook_path.display());
                    }
                }
            }
        }
    }
    if !any_global_hook {
        println!("  {} No global linthis hooks installed", "ℹ".cyan());
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

    // Check agent integration status (project-level only for status display)
    println!("\n{}", "Agent Integration".bold());
    let mut any_agent_installed = false;
    for p in ALL_AGENT_PROVIDERS {
        let installed = agent_is_installed(&git_root, p, false);
        if installed {
            any_agent_installed = true;
            let path = agent_rules_path(&git_root, p, false);
            println!("{} {} ({})", "✓".green(), p, path.display());
            // Show extra info for Claude/CodeBuddy (Stop Hook)
            if let Some(settings_path) = agent_stop_hook_settings_path(&git_root, p) {
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
fn handle_hook_uninstall(hook_event: Option<HookEvent>, all: bool, yes: bool, global: bool) -> ExitCode {
    if global {
        return handle_global_hook_uninstall(hook_event, all, yes);
    }

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
        let agent_result = handle_agent_hook_uninstall(yes, false);
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
                        .filter(|line| !line.contains("linthis") && !line.contains("# linthis-hook"))
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
                .filter(|line| !line.contains("linthis") && !line.contains("# linthis-hook"))
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
        HookTool::Git | HookTool::Agent
        | HookTool::GitWithAgent | HookTool::PrekWithAgent | HookTool::PreCommitWithAgent => {
            return Ok(()) // handled separately
        }
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
        HookTool::Agent
        | HookTool::GitWithAgent | HookTool::PrekWithAgent | HookTool::PreCommitWithAgent => {
            // Handled separately before create_hook_config is called
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
                new_content.push_str("\n# linthis-hook\n");
                new_content.push_str(&linthis_hook_line);
                new_content.push('\n');

                match fs::write(&hook_path, new_content) {
                    Ok(_) => {
                        println!(
                            "{} Added linthis to existing {} {} [project]",
                            "✓".green(),
                            hook_path.display(),
                            "(appended)".dimmed()
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
                // Create new hook file (include # linthis-hook marker for global hook detection)
                let content = format!("#!/bin/sh\n# linthis-hook\n{}\n", linthis_hook_line);

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

                        println!("{} Created {} [project]", "✓".green(), hook_path.display());
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
            // For pre-commit: check + format staged files
            // Default "-c -f" = RunMode::Both (check AND format)
            let extra = args.as_deref().unwrap_or("-c -f");
            format!("linthis -s {} --hook-event=pre-commit", extra)
        }
        HookEvent::PrePush => {
            // For pre-push: check + format all files
            // Default "-c -f" = RunMode::Both (check AND format)
            let extra = args.as_deref().unwrap_or("-c -f");
            format!("linthis {} --hook-event=pre-push", extra)
        }
        HookEvent::CommitMsg => {
            // For commit-msg: validate commit message using the msg file passed as $1
            "linthis cmsg \"$1\"".to_string()
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
// Agent Fix Provider (--type *-with-agent) helpers
// =============================================================================

/// All AgentFixProvider variants in detection-priority order
const ALL_AGENT_FIX_PROVIDERS: &[AgentFixProvider] = &[
    AgentFixProvider::Claude,
    AgentFixProvider::Codex,
    AgentFixProvider::Gemini,
    AgentFixProvider::Cursor,
    AgentFixProvider::Droid,
    AgentFixProvider::Auggie,
    AgentFixProvider::Codebuddy,
];

/// Return the binary name used to invoke the agent CLI headlessly.
/// Used for PATH detection via `which`.
fn agent_fix_bin(provider: &AgentFixProvider) -> &'static str {
    match provider {
        AgentFixProvider::Claude    => "claude",
        AgentFixProvider::Codex     => "codex",
        AgentFixProvider::Gemini    => "gemini",
        AgentFixProvider::Cursor    => "cursor-agent",
        AgentFixProvider::Droid     => "droid",
        AgentFixProvider::Auggie    => "auggie",
        AgentFixProvider::Codebuddy => "codebuddy",
    }
}

/// Build the headless shell command that invokes the agent with a prompt.
///
/// Commands confirmed from official docs:
/// - Claude:    `claude -p '...'`             (claude -p / --print)
/// - Codex:     `codex exec '...'`            (codex exec subcommand for non-interactive)
/// - Gemini:    `gemini -p '...'`             (gemini -p / --prompt)
/// - Cursor:    `cursor-agent chat '...'`     (cursor-agent chat subcommand)
/// - Droid:     `droid exec --auto low '...'` (droid exec with --auto for edits)
/// - Auggie:    `auggie --print '...'`        (auggie --print for headless/non-interactive)
/// - Codebuddy: `codebuddy -p '...'`         (codebuddy -p / --prompt)
fn agent_fix_headless_cmd(provider: &AgentFixProvider, prompt: &str) -> String {
    // Escape single quotes in prompt for shell safety
    let escaped = prompt.replace('\'', "'\\''");
    match provider {
        AgentFixProvider::Claude    => format!("claude -p --dangerously-skip-permissions '{}'", escaped),
        AgentFixProvider::Codex     => format!("codex exec --ask-for-approval never '{}'", escaped),
        AgentFixProvider::Gemini    => format!("gemini -p --approval-mode=auto_edit '{}'", escaped),
        AgentFixProvider::Cursor    => format!("cursor-agent chat --force '{}'", escaped),
        AgentFixProvider::Droid     => format!("droid exec --auto high '{}'", escaped),
        AgentFixProvider::Auggie    => format!("auggie --print '{}'", escaped),
        AgentFixProvider::Codebuddy => format!("codebuddy -p --dangerously-skip-permissions '{}'", escaped),
    }
}

/// Detect which AgentFixProvider CLIs are available in PATH
fn detect_agent_fix_providers() -> Vec<AgentFixProvider> {
    ALL_AGENT_FIX_PROVIDERS
        .iter()
        .filter(|p| is_command_available(agent_fix_bin(p)))
        .cloned()
        .collect()
}

/// Resolve AgentFixProvider from an optional --provider string.
/// - If specified: parse and validate.
/// - If not specified + yes: auto-detect first available CLI.
/// - If not specified + interactive: show selection menu.
fn resolve_agent_fix_provider(
    provider: Option<&str>,
    yes: bool,
) -> Result<AgentFixProvider, ExitCode> {
    if let Some(p) = provider {
        let parsed = match p.to_lowercase().as_str() {
            "claude"             => Some(AgentFixProvider::Claude),
            "codex"              => Some(AgentFixProvider::Codex),
            "gemini"             => Some(AgentFixProvider::Gemini),
            "cursor"             => Some(AgentFixProvider::Cursor),
            "droid"              => Some(AgentFixProvider::Droid),
            "auggie" | "aug" | "augment" => Some(AgentFixProvider::Auggie),
            "codebuddy"          => Some(AgentFixProvider::Codebuddy),
            _ => None,
        };
        return parsed.ok_or_else(|| {
            eprintln!(
                "{}: Unknown agent fix provider '{}'. Valid: claude, codex, gemini, cursor, droid, auggie",
                "Error".red(), p
            );
            ExitCode::from(1)
        });
    }

    let detected = detect_agent_fix_providers();

    if yes {
        // Auto-detect: use first available, default to claude
        return Ok(detected.into_iter().next().unwrap_or(AgentFixProvider::Claude));
    }

    // Interactive menu
    use std::io::{self, Write};

    println!("{}", "Select AI agent for automatic fix:".bold());
    println!();

    for (i, p) in ALL_AGENT_FIX_PROVIDERS.iter().enumerate() {
        let available = is_command_available(agent_fix_bin(p));
        let tag = if available {
            format!(" {}", "(detected)".cyan())
        } else {
            String::new()
        };
        println!("  {}. {}{}", i + 1, p, tag);
    }
    println!();
    print!("Choose [1-{}]: ", ALL_AGENT_FIX_PROVIDERS.len());
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let n: usize = input.trim().parse().unwrap_or(0);

    if n >= 1 && n <= ALL_AGENT_FIX_PROVIDERS.len() {
        Ok(ALL_AGENT_FIX_PROVIDERS[n - 1].clone())
    } else {
        println!("Installation cancelled");
        Err(ExitCode::SUCCESS)
    }
}

/// Build the agent fix prompt based on the hook event type.
fn agent_fix_prompt_for_event(hook_event: &HookEvent) -> String {
    match hook_event {
        HookEvent::CommitMsg => format!(
            "The commit message failed validation (not in Conventional Commits format). \
             Read the commit message file passed as argument, \
             rewrite it to follow the format: type(scope)?: description \
             where type is one of: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert. \
             Keep the original intent of the message. Write the corrected message back to the same file. \
             Verify with 'linthis cmsg <file>' until it passes."
        ),
        _ => format!(
            "Staged files have linthis lint errors. \
             Run 'linthis -s -c' to inspect them. \
             Fix all issues by editing the files directly (do NOT use linthis --fix). \
             Verify with 'linthis -s -c' until it passes cleanly."
        ),
    }
}

/// Error message for agent fix echo based on hook event type.
fn agent_fix_error_msg(hook_event: &HookEvent) -> &'static str {
    match hook_event {
        HookEvent::CommitMsg => "Commit message validation failed",
        _ => "Lint errors detected",
    }
}

/// Build the full git hook shell script with agent fix fallback.
fn build_git_with_agent_hook_script(linthis_cmd: &str, fix_provider: &AgentFixProvider, hook_event: &HookEvent) -> String {
    let prompt = agent_fix_prompt_for_event(hook_event);
    let agent_cmd = agent_fix_headless_cmd(fix_provider, &prompt);
    let error_msg = agent_fix_error_msg(hook_event);
    format!(
        "#!/bin/sh\n\
         \n\
         LINTHIS_CMD=\"{linthis}\"\n\
         \n\
         $LINTHIS_CMD\n\
         LINTHIS_EXIT=$?\n\
         \n\
         if [ $LINTHIS_EXIT -ne 0 ]; then\n\
         \x20 echo \"[linthis] {error_msg}. Invoking {provider} to fix...\"\n\
         \x20 {agent}\n\
         \x20 # Re-verify after agent fix\n\
         \x20 $LINTHIS_CMD\n\
         \x20 LINTHIS_EXIT=$?\n\
         fi\n\
         \n\
         exit $LINTHIS_EXIT\n",
        linthis = linthis_cmd,
        provider = fix_provider,
        agent = agent_cmd,
        error_msg = error_msg,
    )
}

/// Install a git hook with agent fix fallback
fn handle_git_with_agent_install(
    hook_event: &HookEvent,
    force: bool,
    fix_provider: &AgentFixProvider,
    args: &Option<String>,
) -> ExitCode {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    let git_root = match find_git_root() {
        Some(root) => root,
        None => {
            eprintln!("{}: Not in a git repository", "Error".red());
            return ExitCode::from(1);
        }
    };

    let hook_filename = hook_event.hook_filename();
    let hook_path = git_root.join(".git/hooks").join(hook_filename);
    let linthis_cmd = build_hook_command(hook_event, args);
    let content = build_git_with_agent_hook_script(&linthis_cmd, fix_provider, hook_event);

    if hook_path.exists() && !force {
        eprintln!(
            "{}: {} already exists. Use --force to overwrite.",
            "Warning".yellow(),
            hook_path.display()
        );
        return ExitCode::from(1);
    }

    if let Some(parent) = hook_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("{}: Failed to create hooks directory: {}", "Error".red(), e);
            return ExitCode::from(2);
        }
    }

    match fs::write(&hook_path, &content) {
        Ok(_) => {
            #[cfg(unix)]
            {
                if let Ok(meta) = fs::metadata(&hook_path) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o755);
                    let _ = fs::set_permissions(&hook_path, perms);
                }
            }
            println!("{} Created {} (git-with-agent, {})", "✓".green(), hook_path.display(), fix_provider);
            println!("  {} On lint failure: {}", "→".dimmed(), agent_fix_bin(fix_provider).cyan());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}: Failed to create {}: {}", "Error".red(), hook_path.display(), e);
            ExitCode::from(2)
        }
    }
}

/// Install prek/pre-commit config + a wrapper git hook with agent fix fallback
fn handle_precommit_with_agent_install(
    base_tool: &HookTool,
    hook_event: &HookEvent,
    force: bool,
    fix_provider: &AgentFixProvider,
    args: &Option<String>,
) -> ExitCode {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    // 1. Install the base prek/pre-commit config
    if let Err(exit) = create_hook_config(base_tool, hook_event, force, args) {
        return exit;
    }

    // 2. Wrap with a git hook script that calls prek/pre-commit and
    //    invokes the agent on failure
    let git_root = match find_git_root() {
        Some(root) => root,
        None => return ExitCode::from(1),
    };

    let tool_cmd = match base_tool {
        HookTool::Prek       => "prek run",
        HookTool::PreCommit  => "pre-commit run --all-files",
        _ => return ExitCode::from(1),
    };

    let prompt = format!(
        "The {tool} pre-commit check failed with lint errors. \
         Run '{tool_cmd}' to see them. Fix all issues by editing the files directly. \
         Verify by running '{tool_cmd}' again until it passes.",
        tool = fix_provider,
        tool_cmd = tool_cmd,
    );
    let agent_cmd = agent_fix_headless_cmd(fix_provider, &prompt);
    let wrapper = format!(
        "#!/bin/sh\n\
         \n\
         {tool_cmd}\n\
         EXIT=$?\n\
         \n\
         if [ $EXIT -ne 0 ]; then\n\
         \x20 echo \"[linthis] Errors detected. Invoking {provider} to fix...\"\n\
         \x20 {agent}\n\
         \x20 {tool_cmd}\n\
         \x20 EXIT=$?\n\
         fi\n\
         \n\
         exit $EXIT\n",
        tool_cmd = tool_cmd,
        provider = fix_provider,
        agent = agent_cmd,
    );

    let hook_filename = hook_event.hook_filename();
    let hook_path = git_root.join(".git/hooks").join(hook_filename);

    match fs::write(&hook_path, &wrapper) {
        Ok(_) => {
            #[cfg(unix)]
            {
                if let Ok(meta) = fs::metadata(&hook_path) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o755);
                    let _ = fs::set_permissions(&hook_path, perms);
                }
            }
            println!("{} Created wrapper {} ({}-with-agent, {})", "✓".green(), hook_path.display(), match base_tool { HookTool::Prek => "prek", _ => "pre-commit" }, fix_provider);
            println!("  {} On failure: {}", "→".dimmed(), agent_fix_bin(fix_provider).cyan());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}: Failed to create wrapper hook: {}", "Error".red(), e);
            ExitCode::from(2)
        }
    }
}

// =============================================================================
// Agent (AI Coding Agent) Multi-Provider Integration
// =============================================================================

/// All supported agent providers (in display order)
const ALL_AGENT_PROVIDERS: &[AgentProvider] = &[
    AgentProvider::Claude,
    AgentProvider::Codex,
    AgentProvider::Gemini,
    AgentProvider::Cursor,
    AgentProvider::Droid,
    AgentProvider::Auggie,
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

### Commit message format

All commit messages MUST follow Conventional Commits format:

```
type(scope)?: description
```

Valid types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert

Examples:
- `feat: add user authentication`
- `fix(api): handle null response`
- `docs: update README`

Validate with: `linthis cmsg "your message"`

### Key principle

Never rely on `linthis --fix` or `linthis fix` for automated fixing. Always read the lint errors, understand them, and apply fixes manually through code edits. This ensures higher quality fixes with proper context awareness."#
        .to_string()
}

/// Content for CLAUDE.md (append section)
fn agent_content_claude_md() -> String {
    format!("\n{}\n\n{}\n", AGENT_SECTION_MARKER, agent_lint_rules_body())
}

/// Content for Codex AGENTS.md (append section)
fn agent_content_codex_md() -> String {
    format!("\n{}\n\n{}\n", AGENT_SECTION_MARKER, agent_lint_rules_body())
}

/// Content for Gemini .gemini/instructions.md (dedicated file)
fn agent_content_gemini_md() -> String {
    format!("# Linthis Agent Rules\n\n{}\n", agent_lint_rules_body())
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

/// Content for Droid .droid/rules/linthis.md (dedicated file)
fn agent_content_droid_md() -> String {
    format!("# Linthis Agent Rules\n\n{}\n", agent_lint_rules_body())
}

/// Content for Auggie .augment/rules/linthis.md (dedicated file)
fn agent_content_auggie_md() -> String {
    format!("# Linthis Agent Rules\n\n{}\n", agent_lint_rules_body())
}

/// Content for CodeBuddy .codebuddy/rules/linthis.md (dedicated file)
fn agent_content_codebuddy_md() -> String {
    format!("# Linthis Agent Rules\n\n{}\n", agent_lint_rules_body())
}

/// Generate the Stop hook JSON content for .claude/settings.json
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

/// Get the rules file path for a given agent provider.
///
/// When `global` is true, `base` is the user home directory; otherwise it is
/// the project git root.  Claude's project-level file is `CLAUDE.md` at the
/// repo root, while the user-level file lives in `~/.claude/CLAUDE.md`.
fn agent_rules_path(base: &std::path::Path, provider: &AgentProvider, global: bool) -> PathBuf {
    match provider {
        AgentProvider::Claude => {
            if global {
                base.join(".claude/CLAUDE.md")
            } else {
                base.join("CLAUDE.md")
            }
        }
        AgentProvider::Codex => {
            if global {
                base.join(".codex/AGENTS.md")
            } else {
                base.join("AGENTS.md")
            }
        }
        AgentProvider::Gemini   => base.join(".gemini/instructions.md"),
        AgentProvider::Cursor   => base.join(".cursor/rules/linthis.mdc"),
        AgentProvider::Droid    => base.join(".droid/rules/linthis.md"),
        AgentProvider::Auggie   => base.join(".augment/rules/linthis.md"),
        AgentProvider::Codebuddy => base.join(".codebuddy/rules/linthis.md"),
    }
}

/// Get the Stop Hook settings file path for providers that support it.
///
/// `base` is either the project git root (local) or the user home directory
/// (global).
fn agent_stop_hook_settings_path(base: &std::path::Path, provider: &AgentProvider) -> Option<PathBuf> {
    match provider {
        AgentProvider::Claude => Some(base.join(".claude/settings.json")),
        AgentProvider::Codebuddy => Some(base.join(".codebuddy/settings.json")),
        _ => None,
    }
}

/// Print "Installed Stop Hook" message if the provider supports it
fn print_stop_hook_installed(base: &std::path::Path, provider: &AgentProvider) {
    if let Some(settings_path) = agent_stop_hook_settings_path(base, provider) {
        println!(
            "{} Installed Stop Hook → {}",
            "✓".green(),
            settings_path.display()
        );
    }
}

/// Print info about an already-installed agent provider (file path + content)
fn print_agent_installed_info(base: &std::path::Path, provider: &AgentProvider, global: bool) {
    let path = agent_rules_path(base, provider, global);
    println!(
        "       {} {}",
        "File:".dimmed(),
        path.display()
    );

    // For Claude/CodeBuddy, also show settings file (Stop Hook)
    if let Some(settings_path) = agent_stop_hook_settings_path(base, provider) {
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
            AgentProvider::Claude | AgentProvider::Codex => {
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
fn agent_is_installed(base: &std::path::Path, provider: &AgentProvider, global: bool) -> bool {
    let path = agent_rules_path(base, provider, global);
    match provider {
        // Append-style: check for section marker in file
        AgentProvider::Claude | AgentProvider::Codex => {
            path.exists()
                && std::fs::read_to_string(&path)
                    .map(|c| c.contains(AGENT_SECTION_MARKER))
                    .unwrap_or(false)
        }
        // Dedicated file: check if file exists and contains linthis
        AgentProvider::Gemini
        | AgentProvider::Cursor
        | AgentProvider::Droid
        | AgentProvider::Auggie
        | AgentProvider::Codebuddy => {
            path.exists()
                && std::fs::read_to_string(&path)
                    .map(|c| c.contains("linthis") || c.contains("Linthis"))
                    .unwrap_or(false)
        }
    }
}

/// Detect which agent providers are likely in use (by checking for their directories).
///
/// `base` is either the project git root (local) or the user home directory
/// (global).
fn detect_agent_providers(base: &std::path::Path) -> Vec<AgentProvider> {
    let mut detected = Vec::new();
    if base.join(".claude").exists() {
        detected.push(AgentProvider::Claude);
    }
    if base.join("AGENTS.md").exists() || base.join(".codex").exists() {
        detected.push(AgentProvider::Codex);
    }
    if base.join(".gemini").exists() {
        detected.push(AgentProvider::Gemini);
    }
    if base.join(".cursor").exists() {
        detected.push(AgentProvider::Cursor);
    }
    if base.join(".droid").exists() {
        detected.push(AgentProvider::Droid);
    }
    if base.join(".augment").exists() {
        detected.push(AgentProvider::Auggie);
    }
    if base.join(".codebuddy").exists() {
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
                AgentProvider::Claude    => "Claude Code",
                AgentProvider::Codex     => "Codex",
                AgentProvider::Gemini    => "Gemini",
                AgentProvider::Cursor    => "Cursor",
                AgentProvider::Droid     => "Droid",
                AgentProvider::Auggie    => "Auggie",
                AgentProvider::Codebuddy => "CodeBuddy",
            };
            let dir = match p {
                AgentProvider::Claude    => ".claude",
                AgentProvider::Codex     => "AGENTS.md",
                AgentProvider::Gemini    => ".gemini",
                AgentProvider::Cursor    => ".cursor",
                AgentProvider::Droid     => ".droid",
                AgentProvider::Auggie    => ".augment",
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
fn install_agent_provider(base: &std::path::Path, provider: &AgentProvider, global: bool) -> Result<(), String> {
    let rules_path = agent_rules_path(base, provider, global);

    match provider {
        AgentProvider::Claude => {
            // Install CLAUDE.md rules (append)
            install_agent_append_rules(&rules_path, &agent_content_claude_md(), "# Project Instructions\n")?;
            // Also install Stop Hook
            if let Some(settings_path) = agent_stop_hook_settings_path(base, provider) {
                install_agent_stop_hook(base, &settings_path)?;
            }
        }
        AgentProvider::Codex => {
            // Install AGENTS.md rules (append)
            install_agent_append_rules(&rules_path, &agent_content_codex_md(), "# Agent Instructions\n")?;
        }
        AgentProvider::Gemini => {
            install_agent_dedicated_file(&rules_path, &agent_content_gemini_md())?;
        }
        AgentProvider::Cursor => {
            install_agent_dedicated_file(&rules_path, &agent_content_cursor_mdc())?;
        }
        AgentProvider::Droid => {
            install_agent_dedicated_file(&rules_path, &agent_content_droid_md())?;
        }
        AgentProvider::Auggie => {
            install_agent_dedicated_file(&rules_path, &agent_content_auggie_md())?;
        }
        AgentProvider::Codebuddy => {
            install_agent_dedicated_file(&rules_path, &agent_content_codebuddy_md())?;
            // Also install Stop Hook
            if let Some(settings_path) = agent_stop_hook_settings_path(base, provider) {
                install_agent_stop_hook(base, &settings_path)?;
            }
        }
    }

    Ok(())
}

/// Uninstall agent integration for a specific provider
fn uninstall_agent_provider(base: &std::path::Path, provider: &AgentProvider, global: bool) -> Result<(), String> {
    match provider {
        AgentProvider::Claude => {
            let rules_md = agent_rules_path(base, provider, global);
            if rules_md.exists() {
                remove_agent_section_from_file(&rules_md)?;
            }
            if let Some(settings_path) = agent_stop_hook_settings_path(base, provider) {
                if settings_path.exists() {
                    remove_agent_stop_hook(&settings_path)?;
                }
            }
        }
        AgentProvider::Codex => {
            let agents_md = agent_rules_path(base, provider, global);
            if agents_md.exists() {
                remove_agent_section_from_file(&agents_md)?;
            }
        }
        AgentProvider::Gemini
        | AgentProvider::Cursor
        | AgentProvider::Droid
        | AgentProvider::Auggie => {
            let path = agent_rules_path(base, provider, global);
            remove_agent_dedicated_file(&path)?;
        }
        AgentProvider::Codebuddy => {
            let path = agent_rules_path(base, provider, global);
            remove_agent_dedicated_file(&path)?;
            if let Some(settings_path) = agent_stop_hook_settings_path(base, provider) {
                if settings_path.exists() {
                    remove_agent_stop_hook(&settings_path)?;
                }
            }
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

/// Install the Stop Hook into a settings JSON file (e.g. .claude/settings.json, .codebuddy/settings.json)
fn install_agent_stop_hook(
    _git_root: &std::path::Path,
    settings_path: &std::path::Path,
) -> Result<(), String> {
    use std::fs;

    if let Some(parent) = settings_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
        }
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
            .ok_or("settings.json root is not an object")?
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

/// Remove the Stop Hook from .claude/settings.json
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

/// Install agent hooks with multi-provider support.
///
/// When `global` is true, rules are installed into the user home directory
/// (`~/.claude/CLAUDE.md`, `~/.cursor/rules/linthis.mdc`, etc.) without
/// requiring a git repository.  When false, rules are installed in the
/// project git root (project-level).
fn handle_agent_hook_install(
    provider: Option<AgentProvider>,
    force: bool,
    yes: bool,
    global: bool,
) -> ExitCode {
    use std::io::{self, Write};

    // Resolve the base directory: home dir for global, git root for project-level.
    let base = if global {
        match dirs::home_dir() {
            Some(home) => home,
            None => {
                eprintln!("{}: Could not determine home directory", "Error".red());
                return ExitCode::from(1);
            }
        }
    } else {
        match find_git_root() {
            Some(root) => root,
            None => {
                eprintln!("{}: Not in a git repository", "Error".red());
                eprintln!("  Run this command from within a git repository, or use --global / -g to install user-level rules");
                return ExitCode::from(1);
            }
        }
    };

    println!("{}", "🤖 AI Coding Agent Integration".bold());
    if global {
        println!("  {} Installing user-level rules in {}", "→".dimmed(), base.display());
    }
    println!();

    // If a specific provider was given, install just that one
    if let Some(ref p) = provider {
        let installed = agent_is_installed(&base, p, global);
        if installed && !force {
            println!(
                "{}: {} is already installed",
                "Info".cyan(),
                p
            );
            print_agent_installed_info(&base, p, global);
            return ExitCode::SUCCESS;
        }

        match install_agent_provider(&base, p, global) {
            Ok(_) => {
                let path = agent_rules_path(&base, p, global);
                println!("{} Installed {} → {}", "✓".green(), p, path.display());
                print_stop_hook_installed(&base, p);
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
        let detected = detect_agent_providers(&base);
        let targets: Vec<AgentProvider> = if detected.is_empty() {
            ALL_AGENT_PROVIDERS.to_vec()
        } else {
            detected
        };

        let mut any_installed = false;
        for p in &targets {
            if agent_is_installed(&base, p, global) && !force {
                println!("{}: {} already installed", "Info".cyan(), p);
                print_agent_installed_info(&base, p, global);
                continue;
            }
            match install_agent_provider(&base, p, global) {
                Ok(_) => {
                    let path = agent_rules_path(&base, p, global);
                    println!("{} Installed {} → {}", "✓".green(), p, path.display());
                    print_stop_hook_installed(&base, p);
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
    let detected = detect_agent_providers(&base);

    // Build ordered list: detected/installed first, then others
    let mut ordered: Vec<&AgentProvider> = Vec::new();
    for p in ALL_AGENT_PROVIDERS {
        if detected.iter().any(|d| std::mem::discriminant(d) == std::mem::discriminant(p))
            || agent_is_installed(&base, p, global)
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
        let is_installed = agent_is_installed(&base, p, global);
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
        if agent_is_installed(&base, p, global) && !force {
            println!("{}: {} already installed", "Info".cyan(), p);
            print_agent_installed_info(&base, p, global);
            continue;
        }
        match install_agent_provider(&base, p, global) {
            Ok(_) => {
                let path = agent_rules_path(&base, p, global);
                println!("{} Installed {} → {}", "✓".green(), p, path.display());
                print_stop_hook_installed(&base, p);
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

/// Uninstall agent hooks for all installed providers.
///
/// When `global` is true, removes rules from the user home directory;
/// otherwise removes from the project git root.
fn handle_agent_hook_uninstall(yes: bool, global: bool) -> ExitCode {
    use std::io::{self, Write};

    let base = if global {
        match dirs::home_dir() {
            Some(home) => home,
            None => {
                eprintln!("{}: Could not determine home directory", "Error".red());
                return ExitCode::from(1);
            }
        }
    } else {
        match find_git_root() {
            Some(root) => root,
            None => {
                return ExitCode::from(1);
            }
        }
    };

    // Find all installed providers in the target scope
    let installed: Vec<&AgentProvider> = ALL_AGENT_PROVIDERS
        .iter()
        .filter(|p| agent_is_installed(&base, p, global))
        .collect();

    if installed.is_empty() {
        return ExitCode::from(1); // Nothing to uninstall
    }

    if !yes {
        println!("{}", "Agent Integration:".bold());
        for p in &installed {
            let path = agent_rules_path(&base, p, global);
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
        match uninstall_agent_provider(&base, p, global) {
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
///
/// `msg_or_file` may be either a path to the commit message file (as passed by
/// git's commit-msg hook) or the commit message string itself.  When the value
/// resolves to an existing file it is read from disk; otherwise the value is
/// used as the message content directly, which is convenient for CI/testing:
///
///   linthis hook commit-msg-check .git/COMMIT_EDITMSG
///   linthis hook commit-msg-check "feat: add new feature"
pub fn handle_commit_msg_check(msg_or_file: &str) -> ExitCode {
    use linthis::config::Config;
    use regex::Regex;
    use std::fs;

    // Load config to get hooks settings
    let project_root = linthis::utils::get_project_root();
    let config = Config::load_merged(&project_root);

    // Accept either a file path or a raw message string
    let path = std::path::Path::new(msg_or_file);
    let commit_msg = if path.exists() {
        match fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("{}: Failed to read commit message file: {}", "Error".red(), e);
                return ExitCode::from(1);
            }
        }
    } else {
        // Treat as a direct commit message string
        msg_or_file.to_string()
    };

    // Skip if empty (allows empty commits with --allow-empty-message)
    let first_line = commit_msg.lines().next().unwrap_or("").trim();
    if first_line.is_empty() || first_line.starts_with('#') {
        return ExitCode::SUCCESS;
    }

    // Use pattern from config
    let pattern = &config.cmsg.commit_msg_pattern;

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
    if config.cmsg.require_ticket {
        let ticket_pattern = config.cmsg.ticket_pattern.as_deref()
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
