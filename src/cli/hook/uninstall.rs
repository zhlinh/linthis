// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Hook uninstallation: handle_hook_uninstall, uninstall_single_hook,
//! handle_global_hook_uninstall, and related helpers.

use colored::Colorize;
use std::process::ExitCode;

use super::agent::handle_agent_hook_uninstall;
use super::metadata::remove_installed_hook;
use super::{find_git_root, global_hooks_dir, is_linthis_hook_file};
use crate::cli::commands::{HookEvent, HookTool};

/// Uninstall a global git hook from ~/.config/git/hooks/<event>.
///
/// If no linthis hooks remain in that directory, also unsets `core.hooksPath`.
fn handle_global_hook_uninstall(hook_event: Option<HookEvent>, all: bool, yes: bool) -> ExitCode {
    use std::io::{self, Write};

    let hooks_dir = match global_hooks_dir() {
        Some(d) => d,
        None => {
            eprintln!("{}: Could not determine home directory", "Error".red());
            return ExitCode::from(1);
        }
    };

    let events_to_remove: Vec<HookEvent> = if all {
        vec![
            HookEvent::PreCommit,
            HookEvent::PrePush,
            HookEvent::CommitMsg,
        ]
    } else {
        vec![hook_event.unwrap_or(HookEvent::PreCommit)]
    };

    let mut any_removed = false;

    for event in &events_to_remove {
        let hook_path = hooks_dir.join(event.hook_filename());
        if !hook_path.exists() || !is_linthis_hook_file(&hook_path) {
            continue;
        }

        if !yes {
            print!(
                "Remove global {} hook at {}? [y/N]: ",
                event.hook_filename().cyan(),
                hook_path.display()
            );
            io::stdout().flush().ok();
            let mut input = String::new();
            io::stdin().read_line(&mut input).ok();
            if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                println!("Skipped {}", event.hook_filename());
                continue;
            }
        }

        // Deleting the file would take another tool's hook with it — Git LFS
        // shares these names — so only our block goes.
        match super::remove_hook_block(&hook_path) {
            Ok(_) => {
                println!(
                    "{} Removed global {} hook",
                    "✓".green(),
                    event.hook_filename()
                );
                remove_installed_hook("global", "", event);
                any_removed = true;
            }
            Err(e) => {
                eprintln!(
                    "{}: Failed to remove {}: {}",
                    "Error".red(),
                    hook_path.display(),
                    e
                );
            }
        }
    }

    if !any_removed {
        println!("{}: No global linthis hooks found", "Info".cyan());
        return ExitCode::SUCCESS;
    }

    // Check if any linthis hooks remain; if not, unset core.hooksPath
    let remaining = [
        HookEvent::PreCommit,
        HookEvent::PrePush,
        HookEvent::CommitMsg,
    ]
    .iter()
    .any(|e| is_linthis_hook_file(&hooks_dir.join(e.hook_filename())));

    if !remaining {
        let _ = std::process::Command::new("git")
            .args(["config", "--global", "--unset", "core.hooksPath"])
            .status();
        println!("{} Unset global {}", "✓".green(), "core.hooksPath".cyan());
    }

    ExitCode::SUCCESS
}

/// Remove linthis's block from a hook file, keeping other content.
///
/// Dropping every line that mentions linthis would leave the block's `if` and
/// `fi` behind and break the hook for whoever else is in the file.
fn remove_linthis_lines_from_hook(
    hook_path: &std::path::Path,
    _existing_content: &str,
) -> Result<(), ExitCode> {
    if let Err(e) = super::remove_hook_block(hook_path) {
        eprintln!("{}: Failed to update hook: {}", "Error".red(), e);
        return Err(ExitCode::from(2));
    }
    Ok(())
}

/// Delete a hook file entirely.
fn delete_hook_file(hook_path: &std::path::Path) -> Result<(), ExitCode> {
    if let Err(e) = std::fs::remove_file(hook_path) {
        eprintln!("{}: Failed to delete hook: {}", "Error".red(), e);
        return Err(ExitCode::from(2));
    }
    Ok(())
}

/// Check if hook content has non-linthis executable lines.
fn hook_has_other_content(content: &str) -> bool {
    content.lines().any(|line| {
        let trimmed = line.trim();
        !trimmed.is_empty()
            && !trimmed.starts_with('#')
            && !trimmed.starts_with("#!/")
            && !trimmed.contains("linthis")
    })
}

/// Uninstall a single hook
pub(crate) fn uninstall_single_hook(
    git_root: &std::path::Path,
    hook_event: &HookEvent,
    yes: bool,
) -> ExitCode {
    let hook_path = git_root.join(".git/hooks").join(hook_event.hook_filename());

    if !hook_path.exists() {
        return ExitCode::from(1);
    }

    let existing_content = match std::fs::read_to_string(&hook_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("{}: Failed to read hook file: {}", "Error".red(), e);
            return ExitCode::from(2);
        }
    };

    if !existing_content.contains("linthis") {
        return ExitCode::from(1);
    }

    let has_other = hook_has_other_content(&existing_content);

    let result = if yes {
        uninstall_hook_auto(&hook_path, &existing_content, has_other, hook_event)
    } else {
        uninstall_hook_interactive(&hook_path, &existing_content, has_other)
    };

    if let Err(code) = result {
        return code;
    }

    let project_str = git_root.to_str().unwrap_or("").to_string();
    remove_installed_hook("local", &project_str, hook_event);
    ExitCode::SUCCESS
}

/// Non-interactive hook uninstall: remove linthis lines or delete file.
fn uninstall_hook_auto(
    hook_path: &std::path::Path,
    existing_content: &str,
    has_other: bool,
    hook_event: &HookEvent,
) -> Result<(), ExitCode> {
    if has_other {
        remove_linthis_lines_from_hook(hook_path, existing_content)?;
        println!(
            "{} Removed linthis from {} hook",
            "✓".green(),
            hook_event.hook_filename()
        );
    } else {
        delete_hook_file(hook_path)?;
        println!(
            "{} Deleted {} hook",
            "✓".green(),
            hook_event.hook_filename()
        );
    }
    Ok(())
}

/// Interactive hook uninstall: prompt user for action.
fn uninstall_hook_interactive(
    hook_path: &std::path::Path,
    existing_content: &str,
    has_other: bool,
) -> Result<(), ExitCode> {
    use std::io::{self, Write};

    println!("{}: {} contains:", "Warning".yellow(), hook_path.display());
    println!("  {} linthis", "✓".green());
    if has_other {
        println!("  {} Other hooks/commands", "⚠".yellow());
    }

    println!("\nOptions:");
    if has_other {
        println!(
            "  1. {} - Remove only linthis lines",
            "Remove linthis".cyan()
        );
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
            if has_other {
                remove_linthis_lines_from_hook(hook_path, existing_content)?;
                println!(
                    "{} Removed linthis from {}",
                    "✓".green(),
                    hook_path.display()
                );
            } else {
                delete_hook_file(hook_path)?;
                println!("{} Deleted {}", "✓".green(), hook_path.display());
            }
        }
        "2" if has_other => {
            delete_hook_file(hook_path)?;
            println!("{} Deleted {}", "✓".green(), hook_path.display());
        }
        _ => {
            println!("Uninstall cancelled");
            // Return success but don't remove TOML record (cancelled)
        }
    }
    Ok(())
}

/// Uninstall global hooks (git + agent) for the given events.
fn uninstall_global_hooks(
    effective_events: &[HookEvent],
    include_git: bool,
    include_agent: bool,
    all_events: bool,
    yes: bool,
) -> ExitCode {
    let mut result = ExitCode::SUCCESS;
    if include_git {
        if all_events {
            result = handle_global_hook_uninstall(None, true, yes);
        } else {
            for event in effective_events {
                result = handle_global_hook_uninstall(Some(event.clone()), false, yes);
            }
        }
    }
    if include_agent {
        handle_agent_hook_uninstall(yes, true, effective_events);
    }
    result
}

/// Uninstall local (project-level) hooks for the given events.
fn uninstall_local_hooks(
    effective_events: &[HookEvent],
    include_git: bool,
    include_agent: bool,
    yes: bool,
) -> ExitCode {
    let git_root = match find_git_root() {
        Some(root) => root,
        None => {
            eprintln!("{}: Not in a git repository", "Error".red());
            return ExitCode::from(1);
        }
    };

    let mut any_uninstalled = false;
    if include_git {
        for event in effective_events {
            if uninstall_single_hook(&git_root, event, yes) == ExitCode::SUCCESS {
                any_uninstalled = true;
            }
        }
    }
    if include_agent
        && handle_agent_hook_uninstall(yes, false, effective_events) == ExitCode::SUCCESS
    {
        any_uninstalled = true;
    }
    if !any_uninstalled {
        println!("{}: No hooks with linthis found", "Info".cyan());
    }
    ExitCode::SUCCESS
}

pub(crate) fn handle_hook_uninstall(
    hook_types: Vec<HookTool>,
    hook_events: Vec<HookEvent>,
    all: bool,
    all_types: bool,
    all_events: bool,
    yes: bool,
    global: bool,
) -> ExitCode {
    const ALL_EVENTS_LIST: [HookEvent; 3] = [
        HookEvent::PreCommit,
        HookEvent::PrePush,
        HookEvent::CommitMsg,
    ];

    let effective_events: Vec<HookEvent> = if all || all_events {
        ALL_EVENTS_LIST.to_vec()
    } else {
        hook_events
    };

    let include_agent = all || all_types || hook_types.iter().any(|t| matches!(t, HookTool::Agent));
    let include_git = all || all_types || hook_types.iter().any(|t| !matches!(t, HookTool::Agent));

    if global {
        uninstall_global_hooks(
            &effective_events,
            include_git,
            include_agent,
            all || all_events,
            yes,
        )
    } else {
        uninstall_local_hooks(&effective_events, include_git, include_agent, yes)
    }
}
