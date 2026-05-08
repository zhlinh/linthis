// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Hook installation: handle_hook_install, handle_hook_install_single,
//! handle_global_hook_install, and related prompts.

use colored::Colorize;
use std::path::PathBuf;
use std::process::ExitCode;

use super::agent::handle_agent_hook_install;
use super::config::create_hook_config;
use super::metadata::{
    apply_yes_fallback, deduplicate_hook_events, deduplicate_hook_types, save_installed_hook,
};
use super::script::{
    agent_fix_bin, build_thin_wrapper_script, merge_model_into_provider_args,
    parse_provider_with_model, resolve_agent_fix_provider, shell_agent_availability_check,
    shell_timer_functions,
};
use super::{find_git_root, global_hooks_dir, write_hook_script};
use crate::cli::commands::{AgentFixProvider, AgentProvider, HookEvent, HookTool};

/// Type alias for a named constructor table used in interactive prompts.
type NamedConstructorList<T> = [(&'static str, fn() -> T)];

/// Interactive menu for selecting hook types. Returns selected types (never empty unless cancelled).
pub(crate) fn prompt_hook_types(show_all: bool) -> Option<Vec<HookTool>> {
    use std::io::{self, Write};
    // Use bare function pointers (fn() -> HookTool) — const-safe, no dyn trait needed.
    const TYPES: &NamedConstructorList<HookTool> = &[
        ("git", || HookTool::Git),
        ("git-with-agent", || HookTool::GitWithAgent),
        ("prek", || HookTool::Prek),
        ("prek-with-agent", || HookTool::PrekWithAgent),
        ("agent", || HookTool::Agent),
    ];
    let all_idx = TYPES.len() + 1;
    let cancel_idx = if show_all {
        TYPES.len() + 2
    } else {
        TYPES.len() + 1
    };
    println!("\nSelect hook type(s) [comma-separated, e.g. 1,2]:");
    for (i, (name, _)) in TYPES.iter().enumerate() {
        println!("  {}. {}", i + 1, name);
    }
    if show_all {
        println!("  {}. all", all_idx);
    }
    println!("  {}. Cancel", cancel_idx);
    print!("\n> ");
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let input = input.trim();
    if input.is_empty() || input == cancel_idx.to_string() {
        return None;
    }
    if show_all && (input == all_idx.to_string() || input.eq_ignore_ascii_case("all")) {
        return Some(TYPES.iter().map(|(_, f)| f()).collect());
    }
    let selected: Vec<HookTool> = input
        .split(',')
        .filter_map(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| n >= 1 && n <= TYPES.len())
        .map(|n| (TYPES[n - 1].1)())
        .collect();
    if selected.is_empty() {
        None
    } else {
        Some(selected)
    }
}

/// Interactive menu for selecting hook events. Returns selected events (never empty unless cancelled).
pub(crate) fn prompt_hook_events(show_all: bool) -> Option<Vec<HookEvent>> {
    use std::io::{self, Write};
    const EVENTS: &NamedConstructorList<HookEvent> = &[
        ("pre-commit", || HookEvent::PreCommit),
        ("commit-msg", || HookEvent::CommitMsg),
        ("pre-push", || HookEvent::PrePush),
    ];
    let all_idx = EVENTS.len() + 1;
    let cancel_idx = if show_all {
        EVENTS.len() + 2
    } else {
        EVENTS.len() + 1
    };
    println!("\nSelect event(s) [comma-separated, e.g. 1,2]:");
    for (i, (name, _)) in EVENTS.iter().enumerate() {
        println!("  {}. {}", i + 1, name);
    }
    if show_all {
        println!("  {}. all", all_idx);
    }
    println!("  {}. Cancel", cancel_idx);
    print!("\n> ");
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let input = input.trim();
    if input.is_empty() || input == cancel_idx.to_string() {
        return None;
    }
    if show_all && (input == all_idx.to_string() || input.eq_ignore_ascii_case("all")) {
        return Some(EVENTS.iter().map(|(_, f)| f()).collect());
    }
    let selected: Vec<HookEvent> = input
        .split(',')
        .filter_map(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| n >= 1 && n <= EVENTS.len())
        .map(|n| (EVENTS[n - 1].1)())
        .collect();
    if selected.is_empty() {
        None
    } else {
        Some(selected)
    }
}

/// Resolve types and events for install: dedup + interactive prompt or -y fallback.
/// Returns `Ok((types, events))` or `Err(ExitCode)` if the user cancels.
pub(crate) fn resolve_install_types_events(
    hook_types: Vec<HookTool>,
    hook_events: Vec<HookEvent>,
    yes: bool,
) -> Result<(Vec<HookTool>, Vec<HookEvent>), ExitCode> {
    if yes {
        return Ok(apply_yes_fallback(
            deduplicate_hook_types(hook_types),
            deduplicate_hook_events(hook_events),
        ));
    }
    let types = resolve_or_prompt_types(hook_types, true, "Installation cancelled")?;
    let events = resolve_or_prompt_events(hook_events, true, "Installation cancelled")?;
    Ok((types, events))
}

/// Expand an event list to include all three hook events (pre-commit,
/// commit-msg, pre-push) when `--all-events` or `--all` is passed.
/// If the user also provided specific events, those are merged in.
pub(crate) fn expand_all_events(existing: Vec<HookEvent>) -> Vec<HookEvent> {
    let mut all = vec![
        HookEvent::PreCommit,
        HookEvent::CommitMsg,
        HookEvent::PrePush,
    ];
    all.extend(existing);
    deduplicate_hook_events(all)
}

/// Expand the type list to include `agent` plus the previously-installed
/// shell hook type (git-with-agent if that was last used, otherwise git).
/// Merges any explicitly-provided types.
pub(crate) fn expand_all_types(existing: Vec<HookTool>, global: bool) -> Vec<HookTool> {
    let scope = if global { "global" } else { "local" };
    let project = if global {
        String::new()
    } else {
        find_git_root()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    };

    let has_git_with_agent = super::metadata::load_installed_hooks()
        .hooks
        .iter()
        .any(|h| h.scope == scope && h.project == project && h.hook_type == "git-with-agent");

    let default_shell = if has_git_with_agent {
        HookTool::GitWithAgent
    } else {
        HookTool::Git
    };

    let mut types = vec![default_shell, HookTool::Agent];
    types.extend(existing);
    deduplicate_hook_types(types)
}

/// Dedup types and prompt interactively if empty.
fn resolve_or_prompt_types(
    hook_types: Vec<HookTool>,
    show_all: bool,
    cancel_msg: &str,
) -> Result<Vec<HookTool>, ExitCode> {
    let types = deduplicate_hook_types(hook_types);
    if types.is_empty() {
        match prompt_hook_types(show_all) {
            Some(t) => Ok(t),
            None => {
                println!("{}", cancel_msg);
                Err(ExitCode::SUCCESS)
            }
        }
    } else {
        Ok(types)
    }
}

/// Dedup events and prompt interactively if empty.
fn resolve_or_prompt_events(
    hook_events: Vec<HookEvent>,
    show_all: bool,
    cancel_msg: &str,
) -> Result<Vec<HookEvent>, ExitCode> {
    let events = deduplicate_hook_events(hook_events);
    if events.is_empty() {
        match prompt_hook_events(show_all) {
            Some(e) => Ok(e),
            None => {
                println!("{}", cancel_msg);
                Err(ExitCode::SUCCESS)
            }
        }
    } else {
        Ok(events)
    }
}

/// Resolve types and events for uninstall: dedup + interactive prompt or flags.
/// Returns `Ok((types, events))` or `Err(ExitCode)` if the user cancels.
pub(crate) fn resolve_uninstall_types_events(
    hook_types: Vec<HookTool>,
    hook_events: Vec<HookEvent>,
    skip_prompt: bool,
) -> Result<(Vec<HookTool>, Vec<HookEvent>), ExitCode> {
    if skip_prompt {
        return Ok((
            deduplicate_hook_types(hook_types),
            deduplicate_hook_events(hook_events),
        ));
    }
    let types = resolve_or_prompt_types(hook_types, true, "Uninstall cancelled")?;
    let events = resolve_or_prompt_events(hook_events, true, "Uninstall cancelled")?;
    Ok((types, events))
}

/// Parse an agent provider string into an `AgentProvider` enum.
/// Returns `None` for unknown provider names (with error printed).
fn parse_agent_provider(name: &str) -> Option<AgentProvider> {
    match name.to_lowercase().as_str() {
        "claude" => Some(AgentProvider::Claude),
        "codex" => Some(AgentProvider::Codex),
        "gemini" => Some(AgentProvider::Gemini),
        "cursor" => Some(AgentProvider::Cursor),
        "droid" => Some(AgentProvider::Droid),
        "auggie" | "aug" | "augment" => Some(AgentProvider::Auggie),
        "codebuddy" => Some(AgentProvider::Codebuddy),
        "openclaw" => Some(AgentProvider::Openclaw),
        _ => {
            eprintln!(
                "{}: Unknown agent provider '{}'. Valid options: claude, codex, gemini, cursor, droid, auggie, codebuddy, openclaw",
                "Error".red(), name
            );
            None
        }
    }
}

/// Parameters for hook installation.
pub(crate) struct HookInstallParams {
    pub hook_types: Vec<HookTool>,
    pub hook_events: Vec<HookEvent>,
    pub force: bool,
    pub yes: bool,
    pub global: bool,
    pub provider: Option<String>,
    pub args: Option<String>,
    pub provider_args: Option<String>,
    pub model: Option<String>,
}

/// Install git hooks for all combinations of types x events (cartesian product).
pub(crate) fn handle_hook_install(params: HookInstallParams) -> ExitCode {
    let mut overall = ExitCode::SUCCESS;
    let HookInstallParams {
        hook_types,
        hook_events,
        force,
        yes,
        global,
        provider,
        args,
        provider_args,
        model,
    } = params;

    // Support provider/model syntax (e.g. "claude/opus" -> provider="claude", model="opus")
    let (provider, provider_args) = if let Some(ref raw) = provider {
        let (name, model_from_provider) = parse_provider_with_model(raw);
        let merged = merge_model_into_provider_args(model_from_provider, provider_args.as_deref());
        (Some(name.to_string()), merged)
    } else {
        (provider, provider_args)
    };
    // Merge --model flag into provider_args (takes effect after provider/model syntax above)
    let provider_args = merge_model_into_provider_args(model.as_deref(), provider_args.as_deref());

    // If any selected type needs an agent-fix provider, resolve it once upfront
    let preresolved_fix_provider: Option<AgentFixProvider> =
        if hook_types.iter().any(|t| t.has_agent_fix()) {
            match resolve_agent_fix_provider(provider.as_deref(), yes) {
                Ok(p) => Some(p),
                Err(e) => return e,
            }
        } else {
            None
        };

    for hook_type in &hook_types {
        for hook_event in &hook_events {
            let code = handle_hook_install_single(&HookInstallSingleParams {
                hook_type: Some(hook_type.clone()),
                hook_event: hook_event.clone(),
                force,
                yes,
                global,
                provider: provider.clone(),
                preresolved_fix_provider: preresolved_fix_provider.clone(),
                args: args.clone(),
                provider_args: provider_args.clone(),
            });
            if code != ExitCode::SUCCESS {
                overall = code;
            }
        }

        // Auto-install post-commit hook alongside pre-commit for git/git-with-agent
        // types to support the fixup fix_commit_mode. The post-commit script self-guards
        // on the pre-commit fixup sentinel (.git/linthis/pending-fixup.json) and exits
        // immediately when it is absent — so non-fixup modes and `git commit --no-verify`
        // (which bypasses pre-commit, leaving no sentinel) both produce a clean no-op.
        let is_git_type = matches!(hook_type, HookTool::Git | HookTool::GitWithAgent);
        let has_pre_commit = hook_events
            .iter()
            .any(|e| matches!(e, HookEvent::PreCommit));
        if is_git_type && has_pre_commit {
            let code = handle_hook_install_single(&HookInstallSingleParams {
                hook_type: Some(hook_type.clone()),
                hook_event: HookEvent::PostCommit,
                force,
                yes,
                global,
                provider: provider.clone(),
                preresolved_fix_provider: preresolved_fix_provider.clone(),
                args: args.clone(),
                provider_args: provider_args.clone(),
            });
            if code != ExitCode::SUCCESS {
                overall = code;
            }
        }
    }
    overall
}

/// Print detected hook content analysis for an existing hook file.
fn print_existing_hook_analysis(content: &str) {
    let has_linthis = content.contains("linthis");
    let has_prek =
        content.contains("prek") || std::path::Path::new(".pre-commit-config.yaml").exists();
    let has_precommit = content.contains("pre-commit");
    let has_husky = content.contains("husky");

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
}

/// Prompt the user for how to handle an existing hook conflict.
fn prompt_existing_hook_action(
    hook_path: &std::path::Path,
    hook_filename: &str,
    hook_type: &Option<HookTool>,
    hook_event: &HookEvent,
    args: &Option<String>,
) -> ExitCode {
    use std::io::{self, Write};

    println!("\nOptions:");
    println!(
        "  1. {} - Replace existing hook with linthis",
        "Replace".cyan()
    );
    println!("  2. {} - Append linthis to existing hook", "Append".cyan());
    println!("  3. {} - Create backup and replace", "Backup".cyan());
    println!("  4. {} - Cancel", "Cancel".cyan());

    print!("\nChoose an option [1-4]: ");
    io::stdout().flush().unwrap();

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).ok();

    match choice.trim() {
        "1" => handle_hook_install_impl(hook_type.clone(), hook_event, true, false, args.clone()),
        "2" => handle_hook_install_impl(hook_type.clone(), hook_event, false, true, args.clone()),
        "3" => {
            let backup_path = hook_path.with_extension(format!("{}.backup", hook_filename));
            if let Err(e) = std::fs::copy(hook_path, &backup_path) {
                eprintln!("{}: Failed to create backup: {}", "Error".red(), e);
                return ExitCode::from(2);
            }
            println!(
                "{} Created backup at {}",
                "✓".green(),
                backup_path.display()
            );
            handle_hook_install_impl(hook_type.clone(), hook_event, true, false, args.clone())
        }
        _ => {
            println!("Installation cancelled");
            ExitCode::SUCCESS
        }
    }
}

/// Parameters for installing a single hook type x event pair.
struct HookInstallSingleParams {
    hook_type: Option<HookTool>,
    hook_event: HookEvent,
    force: bool,
    yes: bool,
    global: bool,
    provider: Option<String>,
    preresolved_fix_provider: Option<AgentFixProvider>,
    args: Option<String>,
    provider_args: Option<String>,
}

/// Handle installation of *-with-agent hook types.
fn install_with_agent_hook(params: &HookInstallSingleParams) -> ExitCode {
    let hook_type = params.hook_type.as_ref().unwrap();
    let fix_provider = if let Some(p) = params.preresolved_fix_provider.clone() {
        p
    } else {
        match resolve_agent_fix_provider(params.provider.as_deref(), params.yes) {
            Ok(p) => p,
            Err(e) => return e,
        }
    };
    let base = hook_type.base_tool().clone();
    match &base {
        HookTool::Git => handle_git_with_agent_install(
            &params.hook_event,
            params.force,
            params.global,
            params.yes,
            &fix_provider,
            &params.args,
            params.provider_args.as_deref(),
        ),
        HookTool::Prek => handle_precommit_with_agent_install(
            &base,
            &params.hook_event,
            params.force,
            &fix_provider,
            &params.args,
        ),
        _ => ExitCode::from(1),
    }
}

/// Install git hook (pre-commit, pre-push, or commit-msg) for a single type x event pair.
fn handle_hook_install_single(params: &HookInstallSingleParams) -> ExitCode {
    if params
        .hook_type
        .as_ref()
        .map(|t| t.has_agent_fix())
        .unwrap_or(false)
    {
        return install_with_agent_hook(params);
    }

    if matches!(params.hook_type, Some(HookTool::Agent)) {
        let agent_provider = params.provider.as_deref().and_then(parse_agent_provider);
        if params.provider.is_some() && agent_provider.is_none() {
            return ExitCode::from(1);
        }
        return handle_agent_hook_install(
            agent_provider,
            std::slice::from_ref(&params.hook_event),
            params.force,
            params.yes,
            params.global,
        );
    }

    if params.global {
        return handle_global_hook_install(
            params.hook_type.clone(),
            &params.hook_event,
            params.force,
            params.yes,
            &params.args,
            params.provider_args.as_deref(),
        );
    }

    let git_root = match find_git_root() {
        Some(root) => root,
        None => {
            eprintln!("{}: Not in a git repository", "Error".red());
            eprintln!("  Run this command from within a git repository");
            return ExitCode::from(1);
        }
    };

    let hook_filename = params.hook_event.hook_filename();
    let hook_path = git_root.join(".git/hooks").join(hook_filename);

    let is_empty_hook = hook_path.exists()
        && std::fs::read_to_string(&hook_path)
            .map(|s| s.trim().is_empty())
            .unwrap_or(false);
    if hook_path.exists() && !params.force && !is_empty_hook {
        return handle_existing_hook_conflict(
            &hook_path,
            hook_filename,
            &params.hook_type,
            &params.hook_event,
            params.yes,
            &params.args,
        );
    }

    handle_hook_install_impl(
        params.hook_type.clone(),
        &params.hook_event,
        params.force,
        false,
        params.args.clone(),
    )
}

/// Handle the case where a hook file already exists and --force was not specified.
fn handle_existing_hook_conflict(
    hook_path: &std::path::Path,
    hook_filename: &str,
    hook_type: &Option<HookTool>,
    hook_event: &HookEvent,
    yes: bool,
    args: &Option<String>,
) -> ExitCode {
    println!(
        "{}: {} already exists",
        "Warning".yellow(),
        hook_path.display()
    );

    if let Ok(existing_content) = std::fs::read_to_string(hook_path) {
        print_existing_hook_analysis(&existing_content);

        if !yes {
            return prompt_existing_hook_action(
                hook_path,
                hook_filename,
                hook_type,
                hook_event,
                args,
            );
        }
        // Non-interactive mode: append by default
        return handle_hook_install_impl(hook_type.clone(), hook_event, false, true, args.clone());
    }

    println!(
        "  Use {} to overwrite, or {} to append",
        "--force".yellow(),
        "choose option 2".cyan()
    );
    ExitCode::from(1)
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

    if append {
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

/// Prompt user to confirm a global hook install. Returns true if user confirms.
pub(crate) fn confirm_global_install(hook_filename: &str, hook_path: &std::path::Path) -> bool {
    use std::io::{self, Write};
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
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Check if an existing hook blocks installation.
/// Returns Ok(()) to proceed, Err(ExitCode) to stop.
pub(crate) fn check_existing_global_hook(
    hook_path: &std::path::Path,
    hook_filename: &str,
    force: bool,
) -> Result<(), ExitCode> {
    if !hook_path.exists() || force {
        return Ok(());
    }
    if let Ok(existing) = std::fs::read_to_string(hook_path) {
        if existing.trim().is_empty() {
            return Ok(()); // Empty file — treat as not installed
        }
        if existing.contains("# linthis-hook") || existing.contains("linthis hook run") {
            println!(
                "{}: Global {} hook already installed at {}",
                "Info".cyan(),
                hook_filename,
                hook_path.display()
            );
            return Err(ExitCode::SUCCESS);
        }
        eprintln!(
            "{}: {} already exists (not by linthis). Use --force to overwrite.",
            "Warning".yellow(),
            hook_path.display()
        );
        return Err(ExitCode::from(1));
    }
    Ok(())
}

/// Set git config --global core.hooksPath and print result messages.
fn set_global_hooks_path_config(
    hook_filename: &str,
    hook_path: &std::path::Path,
    hooks_dir_str: &str,
) {
    let git_config_result = std::process::Command::new("git")
        .args(["config", "--global", "core.hooksPath", hooks_dir_str])
        .status();

    match git_config_result {
        Ok(status) if status.success() => {
            println!(
                "{} Installed global {} hook → {}",
                "✓".green(),
                hook_filename,
                hook_path.display()
            );
            println!(
                "{} Set {} = {}",
                "✓".green(),
                "core.hooksPath".cyan(),
                hooks_dir_str
            );
            println!(
                "  {} Thin wrapper: hook logic auto-updates with linthis",
                "→".dimmed()
            );
            println!();
            println!("  {}", "How it works (local takes priority):".dimmed());
            println!(
                "  {} If local hook has linthis → global delegates entirely",
                "·".dimmed()
            );
            println!(
                "  {} If local hook has no linthis → global runs linthis first, then delegates",
                "·".dimmed()
            );
            println!(
                "  {} No local hook → global runs linthis directly",
                "·".dimmed()
            );
        }
        Ok(_) | Err(_) => {
            println!(
                "{} Installed global {} hook → {}",
                "✓".green(),
                hook_filename,
                hook_path.display()
            );
            eprintln!(
                "{}: Failed to set core.hooksPath automatically. Run manually:\n  git config --global core.hooksPath {}",
                "Warning".yellow(),
                hooks_dir_str
            );
        }
    }
}

/// Install a global git hook into ~/.config/git/hooks/<event>.
pub(crate) fn handle_global_hook_install(
    hook_type: Option<HookTool>,
    hook_event: &HookEvent,
    force: bool,
    yes: bool,
    args: &Option<String>,
    provider_args: Option<&str>,
) -> ExitCode {
    use std::fs;

    // Resolve agent fix provider for *-with-agent types
    let fix_provider: Option<AgentFixProvider> = if hook_type
        .as_ref()
        .map(|t| t.has_agent_fix())
        .unwrap_or(false)
    {
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

    if !yes && !confirm_global_install(hook_filename, &hook_path) {
        println!("Installation cancelled");
        return ExitCode::SUCCESS;
    }

    if let Err(code) = check_existing_global_hook(&hook_path, hook_filename, force) {
        return code;
    }

    if let Err(e) = fs::create_dir_all(&hooks_dir) {
        eprintln!(
            "{}: Failed to create {}: {}",
            "Error".red(),
            hooks_dir.display(),
            e
        );
        return ExitCode::from(2);
    }

    let effective_hook_type = hook_type.clone().unwrap_or(HookTool::Git);
    let provider_cow = fix_provider.as_ref().map(|p| p.as_str());
    let provider_str = provider_cow.as_deref();
    let content = build_thin_wrapper_script(
        hook_event,
        &effective_hook_type,
        provider_str,
        true,
        provider_args,
    );
    let _ = args; // args are embedded in binary logic at run time

    if let Err(code) = write_hook_script(&hook_path, &content) {
        return code;
    }

    let hooks_dir_str = hooks_dir.to_string_lossy().to_string();
    set_global_hooks_path_config(hook_filename, &hook_path, &hooks_dir_str);

    save_installed_hook(
        "global",
        "",
        hook_event,
        &effective_hook_type,
        provider_str,
        provider_args,
    );

    ExitCode::SUCCESS
}

/// Resolve the hook path, scope, and project string for a hook install (local or global).
fn resolve_hook_install_target(
    hook_filename: &str,
    global: bool,
) -> Result<(PathBuf, &'static str, String), ExitCode> {
    if global {
        let hooks_dir = match global_hooks_dir() {
            Some(d) => d,
            None => {
                eprintln!(
                    "{}: Could not determine global hooks directory",
                    "Error".red()
                );
                return Err(ExitCode::from(1));
            }
        };
        Ok((hooks_dir.join(hook_filename), "global", String::new()))
    } else {
        let git_root = match find_git_root() {
            Some(root) => root,
            None => {
                eprintln!("{}: Not in a git repository", "Error".red());
                return Err(ExitCode::from(1));
            }
        };
        let project_str = git_root.to_str().unwrap_or("").to_string();
        Ok((
            git_root.join(".git/hooks").join(hook_filename),
            "local",
            project_str,
        ))
    }
}

/// Install a git hook with agent fix fallback
fn handle_git_with_agent_install(
    hook_event: &HookEvent,
    force: bool,
    global: bool,
    yes: bool,
    fix_provider: &AgentFixProvider,
    args: &Option<String>,
    provider_args: Option<&str>,
) -> ExitCode {
    use std::fs;

    let hook_filename = hook_event.hook_filename();
    let _ = args;

    let (hook_path, scope, project) = match resolve_hook_install_target(hook_filename, global) {
        Ok(t) => t,
        Err(code) => return code,
    };

    if global && !yes && !confirm_global_install(hook_filename, &hook_path) {
        println!("Installation cancelled");
        return ExitCode::SUCCESS;
    }

    let provider_name = fix_provider.as_str();
    let content = build_thin_wrapper_script(
        hook_event,
        &HookTool::GitWithAgent,
        Some(provider_name.as_ref()),
        global,
        provider_args,
    );

    if let Err(code) = check_existing_global_hook(&hook_path, hook_filename, force) {
        return code;
    }

    if let Some(parent) = hook_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("{}: Failed to create hooks directory: {}", "Error".red(), e);
            return ExitCode::from(2);
        }
    }

    if let Err(code) = write_hook_script(&hook_path, &content) {
        return code;
    }

    println!(
        "{} Created {} (git-with-agent, {})",
        "✓".green(),
        hook_path.display(),
        fix_provider
    );
    println!(
        "  {} On lint failure: {}",
        "→".dimmed(),
        agent_fix_bin(fix_provider).cyan()
    );
    println!(
        "  {} Thin wrapper: hook logic auto-updates with linthis",
        "→".dimmed()
    );

    if global {
        if let Some(hooks_dir) = global_hooks_dir() {
            let hooks_dir_str = hooks_dir.to_string_lossy().to_string();
            let _ = std::process::Command::new("git")
                .args(["config", "--global", "core.hooksPath", &hooks_dir_str])
                .status();
            println!(
                "{} Set {} = {}",
                "✓".green(),
                "core.hooksPath".cyan(),
                hooks_dir_str
            );
        }
    }

    let provider_name = fix_provider.as_str();
    save_installed_hook(
        scope,
        &project,
        hook_event,
        &HookTool::GitWithAgent,
        Some(provider_name.as_ref()),
        provider_args,
    );
    ExitCode::SUCCESS
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
        HookTool::Prek => "prek run",
        _ => return ExitCode::from(1),
    };

    let prompt = format!(
        "The {tool} pre-commit check failed with lint errors. \
         Run '{tool_cmd}' to see them. Fix all issues by editing the files directly. \
         Verify by running '{tool_cmd}' again until it passes.",
        tool = fix_provider,
        tool_cmd = tool_cmd,
    );
    let agent_cmd = super::script::agent_fix_headless_cmd(fix_provider, &prompt, None);
    let timer_fns = shell_timer_functions();
    let agent_check = shell_agent_availability_check(fix_provider);
    let wrapper = format!(
        "#!/bin/sh\n\
         {timer}\
         {tool_cmd}\n\
         EXIT=$?\n\
         \n\
         if [ $EXIT -ne 0 ]; then\n\
         \x20 # Check if agent provider is available before attempting fix\n\
         \x20 {agent_check}\
         \x20 if [ \"$_LINTHIS_AGENT_OK\" = \"1\" ]; then\n\
         \x20\x20\x20 echo \"[linthis] Errors detected. Invoking {provider} to fix...\" >&2\n\
         \x20\x20\x20 start_timer \"Fixing with {provider}\"\n\
         \x20\x20\x20 {agent}\n\
         \x20\x20\x20 stop_timer\n\
         \x20\x20\x20 echo \"[linthis] Re-verifying...\" >&2\n\
         \x20\x20\x20 {tool_cmd}\n\
         \x20\x20\x20 EXIT=$?\n\
         \x20 fi\n\
         fi\n\
         \n\
         exit $EXIT\n",
        timer = timer_fns,
        tool_cmd = tool_cmd,
        provider = fix_provider,
        agent = agent_cmd,
        agent_check = agent_check,
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
            println!(
                "{} Created wrapper {} ({}-with-agent, {})",
                "✓".green(),
                hook_path.display(),
                match base_tool {
                    HookTool::Prek => "prek",
                    _ => "pre-commit",
                },
                fix_provider
            );
            println!(
                "  {} On failure: {}",
                "→".dimmed(),
                agent_fix_bin(fix_provider).cyan()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}: Failed to create wrapper hook: {}", "Error".red(), e);
            ExitCode::from(2)
        }
    }
}
