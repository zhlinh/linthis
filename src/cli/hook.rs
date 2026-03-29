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
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::ExitCode;

use super::commands::{AgentFixProvider, AgentProvider, HookCommands, HookEvent, HookTool};

// =============================================================================
// Installed hooks metadata (persisted to ~/.linthis/installed-hooks.toml)
// =============================================================================

/// Record of a single installed hook (stored in installed-hooks.toml).
#[derive(Serialize, Deserialize, Clone, Debug)]
struct InstalledHook {
    /// "local" or "global"
    scope: String,
    /// Absolute path to git repo root (empty for global scope)
    project: String,
    /// Hook event name (e.g. "pre-commit", "commit-msg")
    event: String,
    /// Hook tool name (e.g. "git", "git-with-agent")
    hook_type: String,
    /// AI provider name for the fix fallback (empty string if none)
    provider: String,
    /// All agent providers that have skills installed for this hook.
    /// Used by `hook sync` to know which providers to refresh.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    skill_providers: Vec<String>,
    /// Extra arguments passed verbatim to the AI agent CLI (e.g. "--model opus").
    #[serde(default, skip_serializing_if = "String::is_empty")]
    provider_args: String,
}

/// Top-level structure of ~/.linthis/installed-hooks.toml.
#[derive(Serialize, Deserialize, Default, Debug)]
struct InstalledHooksFile {
    #[serde(default)]
    hooks: Vec<InstalledHook>,
}

/// Returns the path to the installed-hooks.toml file.
fn installed_hooks_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".linthis").join("installed-hooks.toml"))
}

/// Load the installed-hooks.toml file (returns empty struct if missing or unreadable).
fn load_installed_hooks() -> InstalledHooksFile {
    let path = match installed_hooks_path() {
        Some(p) => p,
        None => return InstalledHooksFile::default(),
    };
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return InstalledHooksFile::default(),
    };
    toml::from_str(&raw).unwrap_or_default()
}

/// Upsert a hook entry in ~/.linthis/installed-hooks.toml.
///
/// Deduplicates by (scope, project, event, hook_type): if an entry with the
/// same key already exists its provider field is updated; otherwise a new
/// entry is appended.
fn save_installed_hook(
    scope: &str,
    project: &str,
    event: &HookEvent,
    hook_type: &HookTool,
    provider: Option<&str>,
    provider_args: Option<&str>,
) {
    save_installed_hook_inner(scope, project, event, hook_type, provider, &[], provider_args);
}

/// Save hook metadata with optional skill_providers list.
fn save_installed_hook_inner(
    scope: &str,
    project: &str,
    event: &HookEvent,
    hook_type: &HookTool,
    provider: Option<&str>,
    skill_providers: &[&str],
    provider_args: Option<&str>,
) {
    let path = match installed_hooks_path() {
        Some(p) => p,
        None => return,
    };

    let mut file = load_installed_hooks();
    let event_str = event.as_str().to_string();
    let hook_type_str = hook_type.as_str().to_string();
    let provider_str = provider.unwrap_or("").to_string();

    // Upsert by (scope, project, event, hook_type).
    let existing = file.hooks.iter_mut().find(|h| {
        h.scope == scope
            && h.project == project
            && h.event == event_str
            && h.hook_type == hook_type_str
    });
    let provider_args_str = provider_args.unwrap_or("").to_string();
    if let Some(entry) = existing {
        entry.provider = provider_str;
        if !skill_providers.is_empty() {
            entry.skill_providers = skill_providers.iter().map(|s| s.to_string()).collect();
        }
        entry.provider_args = provider_args_str;
    } else {
        file.hooks.push(InstalledHook {
            scope: scope.to_string(),
            project: project.to_string(),
            event: event_str,
            hook_type: hook_type_str,
            provider: provider_str,
            skill_providers: skill_providers.iter().map(|s| s.to_string()).collect(),
            provider_args: provider_args_str,
        });
    }

    // Write back
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(raw) = toml::to_string_pretty(&file) {
        let _ = std::fs::write(&path, raw);
    }
}

/// Add a skill provider to an existing hook entry without changing the fix provider.
fn add_skill_provider_to_hook(
    scope: &str,
    project: &str,
    event: &HookEvent,
    skill_provider: &str,
) {
    let path = match installed_hooks_path() {
        Some(p) => p,
        None => return,
    };

    let mut file = load_installed_hooks();
    let event_str = event.as_str();

    // Only match the "agent" entry — skill_providers belong on the agent record,
    // not on git-with-agent or other hook type records.
    let existing = file.hooks.iter_mut().find(|h| {
        h.scope == scope && h.project == project && h.event == event_str && h.hook_type == "agent"
    });
    if let Some(entry) = existing {
        let sp = skill_provider.to_string();
        if !entry.skill_providers.contains(&sp) {
            entry.skill_providers.push(sp);
        }
    } else {
        // Create a new "agent" entry for skill tracking
        file.hooks.push(InstalledHook {
            scope: scope.to_string(),
            project: project.to_string(),
            event: event_str.to_string(),
            hook_type: "agent".to_string(),
            provider: String::new(),
            skill_providers: vec![skill_provider.to_string()],
            provider_args: String::new(),
        });
    }

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(raw) = toml::to_string_pretty(&file) {
        let _ = std::fs::write(&path, raw);
    }
}

/// Remove a hook entry from ~/.linthis/installed-hooks.toml.
///
/// Matches by (scope, project, event). Removes the first matching entry.
fn remove_installed_hook(scope: &str, project: &str, event: &HookEvent) {
    let path = match installed_hooks_path() {
        Some(p) => p,
        None => return,
    };

    let mut file = load_installed_hooks();
    let event_str = event.as_str();

    file.hooks.retain(|h| {
        !(h.scope == scope && h.project == project && h.event == event_str)
    });

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(raw) = toml::to_string_pretty(&file) {
        let _ = std::fs::write(&path, raw);
    }
}

/// Remove a specific skill provider from a hook entry.
///
/// If the skill_providers list becomes empty, the entry is kept (the hook itself
/// may still be installed — only the skill provider list is trimmed).
fn remove_skill_provider_from_hook(
    scope: &str,
    project: &str,
    event: &HookEvent,
    skill_provider: &str,
) {
    let path = match installed_hooks_path() {
        Some(p) => p,
        None => return,
    };

    let mut file = load_installed_hooks();
    let event_str = event.as_str();

    let existing = file.hooks.iter_mut().find(|h| {
        h.scope == scope && h.project == project && h.event == event_str && h.hook_type == "agent"
    });
    if let Some(entry) = existing {
        entry.skill_providers.retain(|sp| sp != skill_provider);
    }

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(raw) = toml::to_string_pretty(&file) {
        let _ = std::fs::write(&path, raw);
    }
}

/// Deduplicate hook types: remove exact dups; for base/with-agent pairs, keep with-agent.
fn deduplicate_hook_types(types: Vec<HookTool>) -> Vec<HookTool> {
    let mut result: Vec<HookTool> = Vec::new();
    for t in types {
        // Skip exact duplicates
        if result.iter().any(|r| std::mem::discriminant(r) == std::mem::discriminant(&t)) {
            continue;
        }
        // If the with-agent variant of this type is already present, skip the base
        let base_already_upgraded = match &t {
            HookTool::Git => result.iter().any(|r| matches!(r, HookTool::GitWithAgent)),
            HookTool::Prek => result.iter().any(|r| matches!(r, HookTool::PrekWithAgent)),
            _ => false,
        };
        if base_already_upgraded {
            continue;
        }
        // If we're adding a with-agent, remove the base if already present
        match &t {
            HookTool::GitWithAgent => result.retain(|r| !matches!(r, HookTool::Git)),
            HookTool::PrekWithAgent => result.retain(|r| !matches!(r, HookTool::Prek)),
            _ => {}
        }
        result.push(t);
    }
    result
}

/// Deduplicate hook events: remove exact duplicates (preserve order).
fn deduplicate_hook_events(events: Vec<HookEvent>) -> Vec<HookEvent> {
    let mut seen = std::collections::HashSet::new();
    events
        .into_iter()
        .filter(|e| seen.insert(std::mem::discriminant(e)))
        .collect()
}

/// Apply -y/--yes fallback when types/events vecs are empty.
/// ONLY call this when the --yes flag is set; when -y is absent, empty vecs
/// should trigger the interactive prompt instead.
/// Returns (types, events) with fallbacks applied.
fn apply_yes_fallback(
    types: Vec<HookTool>,
    events: Vec<HookEvent>,
) -> (Vec<HookTool>, Vec<HookEvent>) {
    let resolved_types = if types.is_empty() {
        vec![HookTool::Git]
    } else {
        types
    };
    let resolved_events = if events.is_empty() {
        let agent_only =
            resolved_types.len() == 1 && matches!(resolved_types[0], HookTool::Agent);
        if agent_only {
            vec![HookEvent::PreCommit, HookEvent::CommitMsg, HookEvent::PrePush]
        } else {
            vec![HookEvent::PreCommit]
        }
    } else {
        events
    };
    (resolved_types, resolved_events)
}

/// Interactive menu for selecting hook types. Returns selected types (never empty unless cancelled).
fn prompt_hook_types(show_all: bool) -> Option<Vec<HookTool>> {
    use std::io::{self, Write};
    // Use bare function pointers (fn() -> HookTool) — const-safe, no dyn trait needed.
    // Non-capturing closures over fieldless enum variants coerce to fn() pointers.
    const TYPES: &[(&str, fn() -> HookTool)] = &[
        ("git", || HookTool::Git),
        ("git-with-agent", || HookTool::GitWithAgent),
        ("prek", || HookTool::Prek),
        ("prek-with-agent", || HookTool::PrekWithAgent),
        ("agent", || HookTool::Agent),
    ];
    let all_idx = TYPES.len() + 1;
    let cancel_idx = if show_all { TYPES.len() + 2 } else { TYPES.len() + 1 };
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
fn prompt_hook_events(show_all: bool) -> Option<Vec<HookEvent>> {
    use std::io::{self, Write};
    // Use bare function pointers for const-safety (matches prompt_hook_types pattern).
    // Non-capturing closures over fieldless enum variants coerce to fn() pointers.
    const EVENTS: &[(&str, fn() -> HookEvent)] = &[
        ("pre-commit", || HookEvent::PreCommit),
        ("commit-msg", || HookEvent::CommitMsg),
        ("pre-push", || HookEvent::PrePush),
    ];
    let all_idx = EVENTS.len() + 1;
    let cancel_idx = if show_all { EVENTS.len() + 2 } else { EVENTS.len() + 1 };
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

/// Resolve types and events for install: dedup + interactive prompt or -y fallback.
/// Returns `Ok((types, events))` or `Err(ExitCode)` if the user cancels.
fn resolve_install_types_events(
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
    let types = resolve_or_prompt_types(hook_types, false, "Installation cancelled")?;
    let events = resolve_or_prompt_events(hook_events, false, "Installation cancelled")?;
    Ok((types, events))
}

/// Resolve types and events for uninstall: dedup + interactive prompt or flags.
/// Returns `Ok((types, events))` or `Err(ExitCode)` if the user cancels.
fn resolve_uninstall_types_events(
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

/// Handle hook subcommands
pub fn handle_hook_command(action: HookCommands) -> ExitCode {
    match action {
        HookCommands::Install { hook_types, hook_events, force, yes, global, provider, args, provider_args } => {
            let (hook_types, hook_events) = match resolve_install_types_events(hook_types, hook_events, yes) {
                Ok(r) => r,
                Err(code) => return code,
            };
            handle_hook_install(hook_types, hook_events, force, yes, global, provider, args, provider_args)
        }
        HookCommands::Uninstall { hook_types, hook_events, all, all_types, all_events, yes, global } => {
            let skip_prompt = all || all_types || all_events || yes;
            let (types, events) = match resolve_uninstall_types_events(hook_types, hook_events, skip_prompt) {
                Ok(r) => r,
                Err(code) => return code,
            };
            handle_hook_uninstall(types, events, all, all_types, all_events, yes, global)
        }
        HookCommands::Status => handle_hook_status(),
        HookCommands::List { global } => handle_hook_list(global),
        HookCommands::Check => handle_hook_check(),
        HookCommands::CommitMsgCheck { msg_or_file } => handle_commit_msg_check(&msg_or_file, false, None),
        HookCommands::Run { event, hook_type, provider, provider_args, global, hook_args } => {
            let code = handle_hook_run(&event, &hook_type, provider.as_deref(), provider_args.as_deref(), global, &hook_args);
            ExitCode::from(code as u8)
        }
        HookCommands::Sync { global, yes } => {
            let code = handle_hook_sync(global, yes);
            ExitCode::from(code as u8)
        }
    }
}

/// Install git hooks for all combinations of types × events (cartesian product).
fn handle_hook_install(
    hook_types: Vec<HookTool>,
    hook_events: Vec<HookEvent>,
    force: bool,
    yes: bool,
    global: bool,
    provider: Option<String>,
    args: Option<String>,
    provider_args: Option<String>,
) -> ExitCode {
    let mut overall = ExitCode::SUCCESS;

    // Support provider/model syntax (e.g. "claude/opus" → provider="claude", model="opus")
    let (provider, provider_args) = if let Some(ref raw) = provider {
        let (name, model) = parse_provider_with_model(raw);
        let merged = merge_model_into_provider_args(model, provider_args.as_deref());
        (Some(name.to_string()), merged)
    } else {
        (provider, provider_args)
    };

    // If any selected type needs an agent-fix provider, resolve it once upfront
    // so the interactive prompt is shown only once regardless of how many events
    // are being installed.
    let preresolved_fix_provider: Option<AgentFixProvider> =
        if hook_types.iter().any(|t| t.has_agent_fix()) {
            match resolve_agent_fix_provider(provider.as_deref(), yes) {
                Ok(p)  => Some(p),
                Err(e) => return e,
            }
        } else {
            None
        };

    for hook_type in &hook_types {
        for hook_event in &hook_events {
            let code = handle_hook_install_single(
                Some(hook_type.clone()),
                hook_event.clone(),
                force,
                yes,
                global,
                provider.clone(),
                preresolved_fix_provider.clone(),
                args.clone(),
                provider_args.clone(),
            );
            if code != ExitCode::SUCCESS {
                overall = code;
            }
        }
    }
    overall
}

/// Parse an agent provider string into an `AgentProvider` enum.
/// Returns `None` for unknown provider names (with error printed).
fn parse_agent_provider(name: &str) -> Option<AgentProvider> {
    match name.to_lowercase().as_str() {
        "claude"    => Some(AgentProvider::Claude),
        "codex"     => Some(AgentProvider::Codex),
        "gemini"    => Some(AgentProvider::Gemini),
        "cursor"    => Some(AgentProvider::Cursor),
        "droid"     => Some(AgentProvider::Droid),
        "auggie" | "aug" | "augment" => Some(AgentProvider::Auggie),
        "codebuddy" => Some(AgentProvider::Codebuddy),
        "openclaw"  => Some(AgentProvider::Openclaw),
        _ => {
            eprintln!(
                "{}: Unknown agent provider '{}'. Valid options: claude, codex, gemini, cursor, droid, auggie, codebuddy, openclaw",
                "Error".red(), name
            );
            None
        }
    }
}

/// Print detected hook content analysis for an existing hook file.
fn print_existing_hook_analysis(content: &str) {
    let has_linthis = content.contains("linthis");
    let has_prek = content.contains("prek") || std::path::Path::new(".pre-commit-config.yaml").exists();
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
/// Returns the ExitCode for the chosen action.
fn prompt_existing_hook_action(
    hook_path: &std::path::Path,
    hook_filename: &str,
    hook_type: &Option<HookTool>,
    hook_event: &HookEvent,
    args: &Option<String>,
) -> ExitCode {
    use std::io::{self, Write};

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
        "1" => handle_hook_install_impl(hook_type.clone(), hook_event, true, false, args.clone()),
        "2" => handle_hook_install_impl(hook_type.clone(), hook_event, false, true, args.clone()),
        "3" => {
            let backup_path = hook_path.with_extension(format!("{}.backup", hook_filename));
            if let Err(e) = std::fs::copy(hook_path, &backup_path) {
                eprintln!("{}: Failed to create backup: {}", "Error".red(), e);
                return ExitCode::from(2);
            }
            println!("{} Created backup at {}", "✓".green(), backup_path.display());
            handle_hook_install_impl(hook_type.clone(), hook_event, true, false, args.clone())
        }
        _ => {
            println!("Installation cancelled");
            ExitCode::SUCCESS
        }
    }
}

/// Handle installation of *-with-agent hook types.
fn install_with_agent_hook(
    hook_type: &HookTool,
    hook_event: &HookEvent,
    force: bool,
    yes: bool,
    global: bool,
    provider: Option<&str>,
    preresolved_fix_provider: Option<AgentFixProvider>,
    args: &Option<String>,
    provider_args: Option<&str>,
) -> ExitCode {
    let fix_provider = if let Some(p) = preresolved_fix_provider {
        p
    } else {
        match resolve_agent_fix_provider(provider, yes) {
            Ok(p)  => p,
            Err(e) => return e,
        }
    };
    let base = hook_type.base_tool().clone();
    match &base {
        HookTool::Git => handle_git_with_agent_install(hook_event, force, global, yes, &fix_provider, args, provider_args),
        HookTool::Prek => handle_precommit_with_agent_install(&base, hook_event, force, &fix_provider, args),
        _ => ExitCode::from(1),
    }
}

/// Install git hook (pre-commit, pre-push, or commit-msg) for a single type x event pair.
fn handle_hook_install_single(
    hook_type: Option<HookTool>,
    hook_event: HookEvent,
    force: bool,
    yes: bool,
    global: bool,
    provider: Option<String>,
    preresolved_fix_provider: Option<AgentFixProvider>,
    args: Option<String>,
    provider_args: Option<String>,
) -> ExitCode {
    if hook_type.as_ref().map(|t| t.has_agent_fix()).unwrap_or(false) {
        return install_with_agent_hook(
            hook_type.as_ref().unwrap(), &hook_event, force, yes, global,
            provider.as_deref(), preresolved_fix_provider, &args, provider_args.as_deref(),
        );
    }

    if matches!(hook_type, Some(HookTool::Agent)) {
        let agent_provider = provider.as_deref().and_then(parse_agent_provider);
        if provider.is_some() && agent_provider.is_none() {
            return ExitCode::from(1);
        }
        return handle_agent_hook_install(agent_provider, &[hook_event.clone()], force, yes, global);
    }

    if global {
        return handle_global_hook_install(hook_type, &hook_event, force, yes, &args, provider_args.as_deref());
    }

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

    let is_empty_hook = hook_path.exists()
        && std::fs::read_to_string(&hook_path).map(|s| s.trim().is_empty()).unwrap_or(false);
    if hook_path.exists() && !force && !is_empty_hook {
        return handle_existing_hook_conflict(&hook_path, hook_filename, &hook_type, &hook_event, yes, &args);
    }

    handle_hook_install_impl(hook_type, &hook_event, force, false, args)
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
    println!("{}: {} already exists", "Warning".yellow(), hook_path.display());

    if let Ok(existing_content) = std::fs::read_to_string(hook_path) {
        print_existing_hook_analysis(&existing_content);

        if !yes {
            return prompt_existing_hook_action(hook_path, hook_filename, hook_type, hook_event, args);
        }
        // Non-interactive mode: append by default
        return handle_hook_install_impl(hook_type.clone(), hook_event, false, true, args.clone());
    }

    println!("  Use {} to overwrite, or {} to append", "--force".yellow(), "choose option 2".cyan());
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

/// Build a thin wrapper script that delegates to `linthis hook run` at runtime.
///
/// The wrapper is 3 lines:
/// ```sh
/// #!/bin/sh
/// exec linthis hook run --event <event> --type <type> [--provider <p>] [--global] "$@"
/// ```
/// This means hook logic always comes from the installed linthis binary,
/// so upgrading linthis automatically updates hook behaviour without reinstallation.
fn build_thin_wrapper_script(
    event: &HookEvent,
    hook_type: &HookTool,
    provider: Option<&str>,
    global: bool,
    provider_args: Option<&str>,
) -> String {
    let provider_arg = provider
        .filter(|p| !p.is_empty())
        .map(|p| format!(" --provider {p}"))
        .unwrap_or_default();
    let provider_args_arg = provider_args
        .filter(|a| !a.is_empty())
        .map(|a| format!(" --provider-args '{}'", a.replace('\'', "'\\''")))
        .unwrap_or_default();
    let global_arg = if global { " --global" } else { "" };
    format!(
        "#!/bin/sh\nexec linthis hook run --event {} --type {}{}{}{} \"$@\"\n",
        event.as_str(),
        hook_type.as_str(),
        provider_arg,
        provider_args_arg,
        global_arg,
    )
}

/// Build the shell preamble and local-hook argument style for pre-push events.
/// Returns (preamble_script, local_hook_args_expression).
fn build_pre_push_preamble() -> (String, &'static str) {
    let preamble = "# For pre-push: save remote args, read stdin for push info\n\
         _REMOTE_NAME=\"$1\"\n\
         _REMOTE_URL=\"$2\"\n\
         # Read push info from stdin: <local_ref> <local_sha> <remote_ref> <remote_sha>\n\
         _IS_TAG=0\n\
         _LOCAL_SHA=\"\"\n\
         _REMOTE_SHA=\"\"\n\
         while read -r _LREF _LSHA _RREF _RSHA; do\n\
         \x20 # Skip tag pushes — no source code to check\n\
         \x20 case \"$_LREF\" in refs/tags/*) _IS_TAG=1 ;; esac\n\
         \x20 _LOCAL_SHA=\"$_LSHA\"\n\
         \x20 _REMOTE_SHA=\"$_RSHA\"\n\
         done\n\
         if [ \"$_IS_TAG\" = \"1\" ]; then\n\
         \x20 exit 0\n\
         fi\n\
         # Compute changed files between remote and local\n\
         _ZERO_SHA=\"0000000000000000000000000000000000000000\"\n\
         if [ \"$_REMOTE_SHA\" = \"$_ZERO_SHA\" ]; then\n\
         \x20 # New branch: diff against default branch\n\
         \x20 _BASE=$(git rev-parse 'HEAD~1' 2>/dev/null || echo \"$_LOCAL_SHA\")\n\
         else\n\
         \x20 _BASE=\"$_REMOTE_SHA\"\n\
         fi\n\
         _PUSHED_FILES=$(git diff --name-only \"$_BASE\"..\"$_LOCAL_SHA\" 2>/dev/null | grep -v '^$')\n\
         # No files to push = nothing to check\n\
         if [ -z \"$_PUSHED_FILES\" ]; then\n\
         \x20 exit 0\n\
         fi\n\
         set --\n\
         while IFS= read -r _F; do set -- \"$@\" -i \"$_F\"; done <<_EOF_\n\
         $_PUSHED_FILES\n\
         _EOF_\n\
         \n"
        .to_string();
    (preamble, "\"$_REMOTE_NAME\" \"$_REMOTE_URL\"")
}

/// Build the agent fix command for a given hook event.
fn agent_fix_cmd_for_event(provider: &AgentFixProvider, hook_event: &HookEvent) -> String {
    if matches!(hook_event, HookEvent::CommitMsg) {
        agent_fix_headless_cmd_commit_msg(provider, None)
    } else {
        let prompt = agent_fix_prompt_for_event(hook_event);
        agent_fix_headless_cmd(provider, &prompt, None)
    }
}

/// Build the shell fix block that invokes an agent on lint failure.
fn build_agent_fix_block(
    provider: &AgentFixProvider,
    hook_event: &HookEvent,
) -> String {
    let agent_cmd = agent_fix_cmd_for_event(provider, hook_event);
    let agent_check = shell_agent_availability_check(provider);
    let error_msg = agent_fix_error_msg(hook_event);
    let new_msg_print = if matches!(hook_event, HookEvent::CommitMsg) {
        agent_fix_show_fixed_cmsg("   ")
    } else {
        String::new()
    };
    format!(
        "  if [ $LINTHIS_EXIT -ne 0 ]; then\n\
         \x20\x20\x20 {agent_check}\
         \x20\x20\x20 if [ \"$_LINTHIS_AGENT_OK\" = \"1\" ]; then\n\
         \x20\x20\x20\x20\x20 echo \"[linthis] {error_msg}. Invoking {provider} to fix...\" >&2\n\
         \x20\x20\x20\x20\x20 start_timer \"Fixing with {provider}\"\n\
         \x20\x20\x20\x20\x20 {agent}\n\
         \x20\x20\x20\x20\x20 stop_timer\n\
         \x20\x20\x20\x20\x20 echo \"[linthis] Re-verifying...\" >&2\n\
         \x20\x20\x20\x20\x20 $LINTHIS_CMD \"$@\"\n\
         \x20\x20\x20\x20\x20 LINTHIS_EXIT=$?\n\
         \x20\x20\x20 fi\n\
         {new_msg_print}\
         \x20 fi\n",
        provider = provider,
        agent = agent_cmd,
        agent_check = agent_check,
        error_msg = error_msg,
        new_msg_print = new_msg_print,
    )
}

/// Build the linthis command variable for the global hook script.
/// For commit-msg, strips "$1" so it can be forwarded via "$@".
fn build_linthis_cmd_var(hook_event: &HookEvent, args: &Option<String>) -> String {
    let cmd = build_hook_command(hook_event, args);
    match hook_event {
        HookEvent::CommitMsg => cmd.trim_end_matches(" \"$1\"").to_string(),
        _ => cmd,
    }
}

/// Resolve preamble and local-hook argument style for a given event.
fn resolve_event_preamble(hook_event: &HookEvent) -> (String, &'static str) {
    if matches!(hook_event, HookEvent::PrePush) {
        build_pre_push_preamble()
    } else {
        (String::new(), "\"$@\"")
    }
}

/// Resolve the fix block, review block, and timer block for the global hook script.
fn resolve_global_hook_blocks(
    hook_event: &HookEvent,
    fix_provider: Option<&AgentFixProvider>,
) -> (String, String, &'static str, &'static str) {
    let fix_block = fix_provider.map(|p| build_agent_fix_block(p, hook_event)).unwrap_or_default();
    let fix_block_direct = fix_provider.map(|p| build_agent_fix_block(p, hook_event)).unwrap_or_default();
    let review_block = if matches!(hook_event, HookEvent::PrePush) {
        "\n# Trigger background AI code review (non-blocking)\n\
         linthis review --background 2>/dev/null &\n"
    } else {
        ""
    };
    let timer_block = if fix_provider.is_some() { shell_timer_functions() } else { "" };
    (fix_block, fix_block_direct, review_block, timer_block)
}

/// Build the global hook script with the hook event name substituted.
fn build_global_hook_script_for_event(
    hook_event: &HookEvent,
    args: &Option<String>,
    fix_provider: Option<&AgentFixProvider>,
) -> String {
    let linthis_cmd_var = build_linthis_cmd_var(hook_event, args);
    let (pre_push_preamble, local_hook_orig_args) = resolve_event_preamble(hook_event);
    let (fix_block, fix_block_direct, review_block, timer_block) = resolve_global_hook_blocks(hook_event, fix_provider);
    let event_name = hook_event.hook_filename();

    format!(
        "#!/bin/sh\n\
         # linthis-hook\n\
         {timer}\
         LINTHIS_CMD=\"{linthis}\"\n\
         {pre_push_preamble}\
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
         \x20\x20\x20 exec \"$LOCAL_HOOK\" {local_hook_orig_args}\n\
         \x20 else\n\
         \x20\x20\x20 # Local hook exists but has no linthis — run linthis first, then delegate\n\
         \x20\x20\x20 $LINTHIS_CMD \"$@\"\n\
         \x20\x20\x20 LINTHIS_EXIT=$?\n\
         {fix_local}\
         \x20\x20\x20 \"$LOCAL_HOOK\" {local_hook_orig_args}\n\
         \x20\x20\x20 LOCAL_EXIT=$?\n\
         {review}\
         \x20\x20\x20 [ $LINTHIS_EXIT -ne 0 ] && exit $LINTHIS_EXIT\n\
         \x20\x20\x20 exit $LOCAL_EXIT\n\
         \x20 fi\n\
         else\n\
         \x20 # No local hook — run linthis directly\n\
         \x20 $LINTHIS_CMD \"$@\"\n\
         \x20 LINTHIS_EXIT=$?\n\
         {fix_direct}\
         {review}\
         \x20 exit $LINTHIS_EXIT\n\
         fi\n",
        timer = timer_block,
        linthis = linthis_cmd_var,
        pre_push_preamble = pre_push_preamble,
        event = event_name,
        local_hook_orig_args = local_hook_orig_args,
        fix_local = fix_block,
        fix_direct = fix_block_direct,
        review = review_block,
    )
}

/// Prompt user to confirm a global hook install. Returns true if user confirms.
fn confirm_global_install(hook_filename: &str, hook_path: &std::path::Path) -> bool {
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
fn check_existing_global_hook(hook_path: &std::path::Path, hook_filename: &str, force: bool) -> Result<(), ExitCode> {
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
                "Info".cyan(), hook_filename, hook_path.display()
            );
            return Err(ExitCode::SUCCESS);
        }
        eprintln!(
            "{}: {} already exists (not by linthis). Use --force to overwrite.",
            "Warning".yellow(), hook_path.display()
        );
        return Err(ExitCode::from(1));
    }
    Ok(())
}

/// Write a hook script to disk and make it executable.
fn write_hook_script(hook_path: &std::path::Path, content: &str) -> Result<(), ExitCode> {
    use std::fs;
    if let Err(e) = fs::write(hook_path, content) {
        eprintln!("{}: Failed to write {}: {}", "Error".red(), hook_path.display(), e);
        return Err(ExitCode::from(2));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(hook_path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(hook_path, perms);
        }
    }
    Ok(())
}

/// Set git config --global core.hooksPath and print result messages.
fn set_global_hooks_path_config(hook_filename: &str, hook_path: &std::path::Path, hooks_dir_str: &str) {
    let git_config_result = std::process::Command::new("git")
        .args(["config", "--global", "core.hooksPath", hooks_dir_str])
        .status();

    match git_config_result {
        Ok(status) if status.success() => {
            println!("{} Installed global {} hook → {}", "✓".green(), hook_filename, hook_path.display());
            println!("{} Set {} = {}", "✓".green(), "core.hooksPath".cyan(), hooks_dir_str);
            println!("  {} Thin wrapper: hook logic auto-updates with linthis", "→".dimmed());
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
    provider_args: Option<&str>,
) -> ExitCode {
    use std::fs;

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

    if !yes && !confirm_global_install(hook_filename, &hook_path) {
        println!("Installation cancelled");
        return ExitCode::SUCCESS;
    }

    if let Err(code) = check_existing_global_hook(&hook_path, hook_filename, force) {
        return code;
    }

    if let Err(e) = fs::create_dir_all(&hooks_dir) {
        eprintln!("{}: Failed to create {}: {}", "Error".red(), hooks_dir.display(), e);
        return ExitCode::from(2);
    }

    let effective_hook_type = hook_type.clone().unwrap_or(HookTool::Git);
    let provider_str = fix_provider.as_ref().map(|p| p.as_str());
    let content = build_thin_wrapper_script(hook_event, &effective_hook_type, provider_str, true, provider_args);
    let _ = args; // args are embedded in binary logic at run time

    if let Err(code) = write_hook_script(&hook_path, &content) {
        return code;
    }

    let hooks_dir_str = hooks_dir.to_string_lossy().to_string();
    set_global_hooks_path_config(hook_filename, &hook_path, &hooks_dir_str);

    save_installed_hook("global", "", hook_event, &effective_hook_type, provider_str, provider_args);

    ExitCode::SUCCESS
}

/// Check if a hook file is a linthis-managed hook.
fn is_linthis_hook_file(hook_path: &std::path::Path) -> bool {
    std::fs::read_to_string(hook_path)
        .map(|c| c.contains("# linthis-hook") || c.contains("linthis hook run"))
        .unwrap_or(false)
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
        if !hook_path.exists() || !is_linthis_hook_file(&hook_path) {
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
                remove_installed_hook("global", "", event);
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
        .any(|e| is_linthis_hook_file(&hooks_dir.join(e.hook_filename())));

    if !remaining {
        let _ = std::process::Command::new("git")
            .args(["config", "--global", "--unset", "core.hooksPath"])
            .status();
        println!("{} Unset global {}", "✓".green(), "core.hooksPath".cyan());
    }

    ExitCode::SUCCESS
}

/// Print project-level hook status for each event. Returns true if any hook is installed.
fn print_project_hook_status(git_root: &std::path::Path, hook_events: &[HookEvent]) -> bool {
    let mut any_installed = false;
    println!("{}", "Project Hooks (.git/hooks/):".bold());
    for event in hook_events {
        let hook_path = git_root.join(".git/hooks").join(event.hook_filename());
        if hook_path.exists() {
            any_installed = true;
            println!("{} {} [project]", "✓".green(), hook_path.display());
            println!("    {}", event.description().dimmed());
            print_hook_content_analysis(&hook_path);
        } else {
            println!("{} {} (not installed)", "✗".red(), event.hook_filename());
        }
    }
    any_installed
}

/// Print analysis of detected tools in a hook file (linthis, prek, pre-commit, husky).
fn print_hook_content_analysis(hook_path: &std::path::Path) {
    if let Ok(content) = std::fs::read_to_string(hook_path) {
        let has_linthis = content.contains("linthis");
        let has_prek = content.contains("prek");
        let has_precommit = content.contains("pre-commit");
        let has_husky = content.contains("husky");

        if has_linthis { println!("    {} linthis", "✓".green()); }
        if has_prek { println!("    {} prek", "ℹ".cyan()); }
        if has_precommit { println!("    {} pre-commit", "ℹ".cyan()); }
        if has_husky { println!("    {} husky", "ℹ".cyan()); }
        if !has_linthis && !has_prek && !has_precommit && !has_husky {
            println!("    {} Custom hook", "ℹ".cyan());
        }
    }
}

/// Print global hook status section.
fn print_global_hook_status(hook_events: &[HookEvent]) {
    println!();
    println!("{}", "Global Hooks (~/.config/git/hooks/):".bold());
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
    if let Some(ref ghooks_dir) = global_hooks_dir() {
        for event in hook_events {
            let hook_path = ghooks_dir.join(event.hook_filename());
            if hook_path.exists() {
                any_global_hook = true;
                let has_linthis = is_linthis_hook_file(&hook_path);
                if has_linthis {
                    println!("{} {} [global]", "✓".green(), hook_path.display());
                    println!("    {} Strategy B: local hook takes priority", "ℹ".dimmed());
                } else {
                    println!("{} {} [global, not by linthis]", "⚠".yellow(), hook_path.display());
                }
            }
        }
    }
    if !any_global_hook {
        println!("  {} No global linthis hooks installed", "ℹ".cyan());
    }
}

/// Print agent integration status for the project. Returns true if any agent is installed.
fn print_agent_status(
    git_root: &std::path::Path,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> bool {
    println!("\n{}", "Agent Integration".bold());
    let events = [HookEvent::PreCommit, HookEvent::CommitMsg, HookEvent::PrePush];
    let mut any_installed = false;
    for p in ALL_AGENT_PROVIDERS {
        if agent_is_installed(git_root, p, false, skill_names) {
            any_installed = true;
            println!("{} {}", "✓".green(), p);
            for event in &events {
                let path = agent_skill_path(git_root, p, false, event, skill_names);
                if path.exists() {
                    println!("  {} {} ({})", "✓".green().dimmed(), path.display(), event.hook_filename());
                }
            }
            if let Some(settings_path) = agent_stop_hook_settings_path(git_root, p) {
                let has_stop_hook = settings_path.exists()
                    && std::fs::read_to_string(&settings_path).map(|c| c.contains("linthis")).unwrap_or(false);
                if has_stop_hook {
                    println!("  {} Stop Hook ({})", "✓".green().dimmed(), settings_path.display());
                }
            }
        } else {
            println!("{} {} (not installed)", "✗".red(), p);
        }
    }
    any_installed
}

/// Show git hook status
fn handle_hook_status() -> ExitCode {
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

    let hook_events = [HookEvent::PreCommit, HookEvent::PrePush, HookEvent::CommitMsg];

    let any_hook_installed = print_project_hook_status(&git_root, &hook_events);
    print_global_hook_status(&hook_events);

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

    let skill_names_cfg = linthis::config::Config::load_merged(&git_root).hook.agent.skill_names;
    let any_agent_installed = print_agent_status(&git_root, Some(&skill_names_cfg));

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

/// List shell hooks in a given hooks directory. Returns the count of hooks found.
fn list_shell_hooks(
    hooks_dir: &std::path::Path,
    scope: &str,
    project: &std::path::Path,
    hook_events: &[HookEvent],
    toml: &InstalledHooksFile,
) -> usize {
    let mut count = 0;
    let mut any = false;
    for event in hook_events {
        let hook_path = hooks_dir.join(event.hook_filename());
        if !hook_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&hook_path).unwrap_or_default();
        if !content.contains("linthis") {
            continue;
        }
        any = true;
        count += 1;

        let hook_type = detect_hook_type_from_content(&content, toml, scope, project, event);
        let provider = detect_provider_from_content(&content, toml, scope, project, event);

        println!(
            "  {} {} {} {}",
            "✓".green(),
            event.hook_filename(),
            format!("[{}]", hook_type).dimmed(),
            if provider.is_empty() { String::new() } else { format!("(provider: {})", provider) }
        );
    }
    if !any {
        println!("  {} No linthis shell hooks installed", "—".dimmed());
    }
    count
}

/// List agent skills for a given base directory. Returns the count of skill entries found.
fn list_agent_skills(
    base: &std::path::Path,
    global: bool,
    hook_events: &[HookEvent],
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> usize {
    let mut count = 0;
    let mut any = false;
    for p in ALL_AGENT_PROVIDERS {
        if !agent_is_installed(base, p, global, skill_names) {
            continue;
        }
        any = true;
        let mut event_tags: Vec<&str> = Vec::new();
        for event in hook_events {
            let path = agent_skill_path(base, p, global, event, skill_names);
            if path.exists() {
                count += 1;
                event_tags.push(event.hook_filename());
            }
        }
        let stop_hook = agent_stop_hook_settings_path(base, p)
            .map(|sp| sp.exists() && std::fs::read_to_string(&sp).map(|c| c.contains("linthis")).unwrap_or(false))
            .unwrap_or(false);
        println!(
            "  {} {} [{}]{}",
            "✓".green(),
            p,
            event_tags.join(", "),
            if stop_hook { format!(" + {}", "stop-hook".dimmed()) } else { String::new() }
        );
    }
    if !any {
        let label = if global { "No global agent skills installed" } else { "No agent skills installed" };
        println!("  {} {}", "—".dimmed(), label);
    }
    count
}

/// Print the summary footer for `hook list`.
fn print_list_footer(count: usize, global: bool) {
    println!();
    if count == 0 {
        if global {
            println!("No global hooks installed.");
            println!("  Use {} to view project hooks.", "linthis hook list".cyan());
        } else {
            println!("No project hooks installed.");
            println!("  Use {} to view global hooks.", "linthis hook list -g".cyan());
        }
    } else {
        let hint = if global {
            format!(" (use {} for project hooks)", "linthis hook list".cyan())
        } else {
            format!(" (use {} for global hooks)", "linthis hook list -g".cyan())
        };
        println!("{} {} hook entries found{}", "Total:".bold(), count, hint);
    }
}

/// List all installed linthis hooks.
fn handle_hook_list(global: bool) -> ExitCode {
    let toml = load_installed_hooks();

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let skill_names_cfg = linthis::config::Config::load_merged(&cwd).hook.agent.skill_names;
    let skill_names = Some(&skill_names_cfg);

    let hook_events = [HookEvent::PreCommit, HookEvent::PrePush, HookEvent::CommitMsg];
    let scope_label = if global { "Global" } else { "Project" };

    println!("{}", format!("Installed Hooks ({})", scope_label).bold());
    println!();

    let mut count: usize = 0;

    if global {
        println!("{}", "Shell Hooks (~/.config/git/hooks/)".bold());
        if let Some(ref ghooks) = global_hooks_dir() {
            count += list_shell_hooks(ghooks, "global", &PathBuf::new(), &hook_events, &toml);
        } else {
            println!("  {} No global linthis shell hooks installed", "—".dimmed());
        }

        println!();
        println!("{}", "Agent Skills (~/)".bold());
        if let Some(ref home_dir) = dirs::home_dir() {
            count += list_agent_skills(home_dir, true, &hook_events, skill_names);
        }
    } else {
        let git_root = find_git_root();
        let project_root_display = git_root.as_ref().map(|r| r.display().to_string())
            .unwrap_or_else(|| "(not in a git repository)".to_string());

        println!("{}", "Shell Hooks (.git/hooks/)".bold());
        if let Some(ref root) = git_root {
            count += list_shell_hooks(&root.join(".git/hooks"), "local", root, &hook_events, &toml);
        } else {
            println!("  {} {}", "—".dimmed(), project_root_display);
        }

        println!();
        println!("{}", "Agent Skills".bold());
        if let Some(ref root) = git_root {
            count += list_agent_skills(root, false, &hook_events, skill_names);
        } else {
            println!("  {} {}", "—".dimmed(), project_root_display);
        }
    }

    print_list_footer(count, global);
    ExitCode::SUCCESS
}

/// Detect the hook type (git, git-with-agent, prek, prek-with-agent) from script content.
///
/// Falls back to the TOML registry if content analysis is inconclusive.
fn detect_hook_type_from_content(
    content: &str,
    toml: &InstalledHooksFile,
    scope: &str,
    project: &std::path::Path,
    event: &HookEvent,
) -> String {
    // Content-based detection
    let has_agent = content.contains("_LINTHIS_AGENT_OK") || content.contains("agent");
    let has_prek = content.contains("prek");

    if has_prek && has_agent {
        return "prek-with-agent".to_string();
    }
    if has_prek {
        return "prek".to_string();
    }
    if has_agent && (content.contains("claude") || content.contains("codex") || content.contains("openclaw")
        || content.contains("gemini") || content.contains("codebuddy") || content.contains("droid")
        || content.contains("auggie") || content.contains("cursor-agent"))
    {
        return "git-with-agent".to_string();
    }

    // Fall back to TOML registry
    let project_str = project.to_string_lossy();
    let event_str = event.hook_filename();
    for hook in &toml.hooks {
        if hook.scope == scope && hook.event == event_str
            && (scope == "global" || hook.project == project_str.as_ref())
        {
            return hook.hook_type.clone();
        }
    }

    "git".to_string()
}

/// Detect the provider name from script content or TOML registry.
fn detect_provider_from_content(
    content: &str,
    toml: &InstalledHooksFile,
    scope: &str,
    project: &std::path::Path,
    event: &HookEvent,
) -> String {
    // Content-based detection
    let providers = [
        ("claude", "claude"),
        ("codex", "codex"),
        ("gemini", "gemini"),
        ("cursor-agent", "cursor"),
        ("droid", "droid"),
        ("auggie", "auggie"),
        ("codebuddy", "codebuddy"),
        ("openclaw", "openclaw"),
    ];
    for (pattern, name) in &providers {
        if content.contains(pattern) && (content.contains("_LINTHIS_AGENT_OK") || content.contains("agent")) {
            return name.to_string();
        }
    }

    // Fall back to TOML registry
    let project_str = project.to_string_lossy();
    let event_str = event.hook_filename();
    for hook in &toml.hooks {
        if hook.scope == scope && hook.event == event_str
            && (scope == "global" || hook.project == project_str.as_ref())
        {
            return hook.provider.clone();
        }
    }

    String::new()
}

/// Uninstall git hooks for the given types × events combinations.
///
/// Flag semantics:
/// - `--all`        : uninstall every type × every event (both git files and agent hooks)
/// - `--all-types`  : uninstall every type for the specified `--event`(s)
/// - `--all-events` : uninstall every event for the specified `--type`(s)
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
    if include_agent {
        if handle_agent_hook_uninstall(yes, false, effective_events) == ExitCode::SUCCESS {
            any_uninstalled = true;
        }
    }
    if !any_uninstalled {
        println!("{}: No hooks with linthis found", "Info".cyan());
    }
    ExitCode::SUCCESS
}

fn handle_hook_uninstall(
    hook_types: Vec<HookTool>,
    hook_events: Vec<HookEvent>,
    all: bool,
    all_types: bool,
    all_events: bool,
    yes: bool,
    global: bool,
) -> ExitCode {
    const ALL_EVENTS_LIST: [HookEvent; 3] = [HookEvent::PreCommit, HookEvent::PrePush, HookEvent::CommitMsg];

    let effective_events: Vec<HookEvent> = if all || all_events {
        ALL_EVENTS_LIST.to_vec()
    } else {
        hook_events
    };

    let include_agent = all || all_types || hook_types.iter().any(|t| matches!(t, HookTool::Agent));
    let include_git = all || all_types || hook_types.iter().any(|t| !matches!(t, HookTool::Agent));

    if global {
        uninstall_global_hooks(&effective_events, include_git, include_agent, all || all_events, yes)
    } else {
        uninstall_local_hooks(&effective_events, include_git, include_agent, yes)
    }
}

/// Remove only linthis lines from a hook file, keeping other content.
fn remove_linthis_lines_from_hook(hook_path: &std::path::Path, existing_content: &str) -> Result<(), ExitCode> {
    let new_content: String = existing_content
        .lines()
        .filter(|line| !line.contains("linthis") && !line.contains("# linthis-hook"))
        .collect::<Vec<_>>()
        .join("\n");

    if let Err(e) = std::fs::write(hook_path, new_content + "\n") {
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
fn uninstall_single_hook(git_root: &std::path::Path, hook_event: &HookEvent, yes: bool) -> ExitCode {
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
        println!("{} Removed linthis from {} hook", "✓".green(), hook_event.hook_filename());
    } else {
        delete_hook_file(hook_path)?;
        println!("{} Deleted {} hook", "✓".green(), hook_event.hook_filename());
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
            if has_other {
                remove_linthis_lines_from_hook(hook_path, existing_content)?;
                println!("{} Removed linthis from {}", "✓".green(), hook_path.display());
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

/// Check hook file for multiple tool conflicts. Returns (has_conflicts, warnings).
fn check_hook_tool_conflicts(hook_path: &std::path::Path) -> (bool, Vec<&'static str>) {
    let mut warnings = Vec::new();
    if !hook_path.exists() {
        return (false, warnings);
    }
    if let Ok(content) = std::fs::read_to_string(hook_path) {
        let tools = [
            content.contains("prek"),
            content.contains("pre-commit"),
            content.contains("husky"),
            content.contains("linthis"),
        ];
        let tool_count = tools.iter().filter(|&&x| x).count();
        if tool_count > 1 {
            println!("{} Multiple hook tools detected in {}", "⚠".yellow(), hook_path.display());
            if content.contains("linthis") { println!("  {} linthis", "✓".green()); }
            if content.contains("prek") { println!("  {} prek", "⚠".yellow()); }
            if content.contains("pre-commit") { println!("  {} pre-commit", "⚠".yellow()); }
            if content.contains("husky") { println!("  {} husky", "⚠".yellow()); }
            warnings.push("Consider using only one hook management tool");
            return (true, warnings);
        }
    }
    (false, warnings)
}

/// Check for hook conflicts
fn handle_hook_check() -> ExitCode {
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

    let (mut has_conflicts, mut warnings) = check_hook_tool_conflicts(&hook_path);

    if prek_config.exists() {
        if let Ok(content) = std::fs::read_to_string(prek_config) {
            if content.contains("linthis") && !hook_path.exists() {
                has_conflicts = true;
                println!("{} {} exists but no hook installed", "⚠".yellow(), prek_config.display());
                warnings.push("Run 'prek install' or 'pre-commit install' to activate hooks");
            }
        }
    }

    if husky_dir.exists() && husky_dir.join("pre-commit").exists() {
        println!("{} Husky detected: {}", "ℹ".cyan(), husky_dir.join("pre-commit").display());
        warnings.push("Husky manages its own hooks in .husky/ directory");
        warnings.push("To use linthis with husky, add linthis command to .husky/pre-commit");
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
        HookTool::Git | HookTool::Agent
        | HookTool::GitWithAgent | HookTool::PrekWithAgent => {
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

/// Format a `HookSource` as a TOML inline-table string, e.g. `{ plugin = "lt", file = "..." }`.
fn format_hook_source(source: &linthis::config::HookSource) -> String {
    use linthis::config::HookSource;
    match source {
        HookSource::Plugin { plugin, file } => {
            format!("{{ plugin = \"{}\", file = \"{}\" }}", plugin, file)
        }
        HookSource::File { file } => {
            format!("{{ file = \"{}\" }}", file)
        }
        HookSource::Url { url } => {
            format!("{{ url = \"{}\" }}", url)
        }
        HookSource::Git { git, git_ref, path } => {
            if let Some(r) = git_ref {
                format!("{{ git = \"{}\", ref = \"{}\", path = \"{}\" }}", git, r, path)
            } else {
                format!("{{ git = \"{}\", path = \"{}\" }}", git, path)
            }
        }
        HookSource::Marketplace { marketplace, plugin, file } => {
            format!(
                "{{ marketplace = \"{}\", plugin = \"{}\", file = \"{}\" }}",
                marketplace, plugin, file
            )
        }
    }
}

/// Describe where the hook behavior comes from (Tier 1/2/3) as a human-readable string.
///
/// - Tier 1 (fixed path): `"hooks/git-with-agent/pre-push (fixed path)"`
/// - Tier 2 (TOML entry): `"[hook.git-with-agent]\npre-push = { source = { ... } }"`
/// - Tier 3 (built-in):   `"built-in → linthis --hook-event=pre-push"`
fn describe_hook_source(tool: &HookTool, hook_event: &HookEvent) -> String {
    use linthis::config::Config;
    use linthis::hooks::resolver;

    let dir = match tool_type_dir(tool) {
        Some(d) => d,
        None => return "built-in (agent)".to_string(),
    };

    let project_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    if resolver::fixed_git_hook_path(&project_root, dir, hook_event.hook_filename()).is_some() {
        return format!("hooks/{}/{} (fixed path)", dir, hook_event.hook_filename());
    }

    let config = Config::load_merged(&project_root);
    let event_key = hook_event.hook_filename();

    if let Some(entry) = lookup_hook_config_entry(&config.hook, tool, event_key) {
        let section = format!("[hook.{}]", dir);
        let source_str = format_hook_source(&entry.source);
        return format!("{}\n{} = {{ source = {} }}", section, event_key, source_str);
    }

    let cmd = build_hook_command(hook_event, &None);
    format!("built-in → {}", cmd)
}

/// Map a HookTool to its directory name for hook override lookups.
/// Returns None for Agent (handled separately).
fn tool_type_dir(tool: &HookTool) -> Option<&'static str> {
    match tool {
        HookTool::Git => Some("git"),
        HookTool::GitWithAgent => Some("git-with-agent"),
        HookTool::Prek => Some("prek"),
        HookTool::PrekWithAgent => Some("prek-with-agent"),
        HookTool::Agent => None,
    }
}

/// Look up the TOML hook config entry for a given tool and event key.
fn lookup_hook_config_entry<'a>(
    hook_cfg: &'a linthis::config::HookConfig,
    tool: &HookTool,
    event_key: &str,
) -> Option<&'a linthis::config::HookSourceEntry> {
    match tool {
        HookTool::Git => hook_cfg.git.get(event_key),
        HookTool::GitWithAgent => hook_cfg.git_with_agent.get(event_key),
        HookTool::Prek => hook_cfg.prek.get(event_key),
        HookTool::PrekWithAgent => hook_cfg.prek_with_agent.get(event_key),
        HookTool::Agent => None,
    }
}

/// Resolve hook script content from Tier-1 (fixed path) or Tier-2 (TOML config),
/// returning `Ok(Some(content))` if an override is found, `Ok(None)` to fall through
/// to the built-in generator, or `Err(ExitCode)` for a hard resolution error.
fn resolve_hook_override(tool: &HookTool, hook_event: &HookEvent) -> Result<Option<String>, ExitCode> {
    use linthis::config::Config;
    use linthis::hooks::resolver;

    let dir = match tool_type_dir(tool) {
        Some(d) => d,
        None => return Ok(None),
    };

    let project_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    // Tier 1: fixed-path auto-discovery
    if let Some(fixed) = resolver::fixed_git_hook_path(&project_root, dir, hook_event.hook_filename()) {
        match std::fs::read_to_string(fixed.as_path()) {
            Ok(content) => return Ok(Some(content)),
            Err(e) => {
                eprintln!("{}: Failed to read fixed-path override '{}': {}", "Error".red(), fixed.display(), e);
                return Err(ExitCode::from(2));
            }
        }
    }

    // Tier 2: TOML source mapping
    let config = Config::load_merged(&project_root);
    let event_key = hook_event.hook_filename();
    if let Some(entry) = lookup_hook_config_entry(&config.hook, tool, event_key) {
        match resolver::resolve_to_string(&entry.source, &project_root, &config.hook.marketplaces) {
            Ok(content) => return Ok(Some(content)),
            Err(e) => {
                eprintln!("{}: Failed to resolve hook override for '{}/{}': {}", "Error".red(), dir, event_key, e);
                return Err(ExitCode::from(2));
            }
        }
    }

    Ok(None)
}

/// Create prek/pre-commit config file for a hook event.
fn create_prek_config(
    tool: &HookTool,
    hook_event: &HookEvent,
    force: bool,
    args: &Option<String>,
) -> Result<(), ExitCode> {
    let config_path = std::path::PathBuf::from(".pre-commit-config.yaml");
    let hook_filename = hook_event.hook_filename();

    if config_path.exists() && !force {
        eprintln!("{}: {} already exists, skipping", "Warning".yellow(), config_path.display());
        return Ok(());
    }

    if let Some(override_content) = resolve_hook_override(tool, hook_event)? {
        std::fs::write(&config_path, override_content)
            .map_err(|e| { eprintln!("{}: Failed to write '{}': {}", "Error".red(), config_path.display(), e); ExitCode::from(2) })?;
        println!("{} Created {} [override]", "✓".green(), config_path.display());
        return Ok(());
    }

    let hook_cmd = build_hook_command(hook_event, args);
    let stage = hook_event.hook_filename();
    let content = format!(
        "repos:\n  - repo: local\n    hooks:\n      - id: linthis-{}\n        name: linthis ({})\n        entry: {}\n        language: system\n        stages: [{}]\n        pass_filenames: false\n",
        hook_filename, hook_event.description(), hook_cmd, stage
    );

    std::fs::write(&config_path, content)
        .map_err(|e| { eprintln!("{}: Failed to create {}: {}", "Error".red(), config_path.display(), e); ExitCode::from(2) })?;

    let tool_name = "prek";
    println!("{} Created {} ({}/pre-commit compatible)", "✓".green(), config_path.display(), tool_name);
    print_prek_next_steps(tool, hook_event, hook_filename);
    Ok(())
}

/// Print next steps after prek config creation (auto-install or manual instructions).
fn print_prek_next_steps(tool: &HookTool, hook_event: &HookEvent, hook_filename: &str) {
    let tool_name = "prek";
    if is_command_available(tool_name) {
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
        println!("\nNext steps:");
        if matches!(tool, HookTool::Prek) {
            println!("  1. Install prek: {}", "pip install prek".cyan());
            println!("  2. Set up hooks: {}", format!("prek install --hook-type {}", hook_filename).cyan());
        } else {
            println!("  1. Install pre-commit: {}", "pip install pre-commit".cyan());
            println!("  2. Set up hooks: {}", format!("pre-commit install --hook-type {}", hook_filename).cyan());
        }
    }
}

/// Create a git hook file (thin wrapper or append to existing).
fn create_git_hook_config(
    tool: &HookTool,
    hook_event: &HookEvent,
    force: bool,
    args: &Option<String>,
) -> Result<(), ExitCode> {
    use std::fs;

    let hook_filename = hook_event.hook_filename();
    let git_root = match find_git_root() {
        Some(root) => root,
        None => {
            eprintln!("{}: Not in a git repository, cannot create .git/hooks/{}", "Error".red(), hook_filename);
            return Err(ExitCode::from(1));
        }
    };

    let git_hooks_dir = git_root.join(".git/hooks");
    let hook_path = git_hooks_dir.join(hook_filename);

    if !git_hooks_dir.exists() {
        fs::create_dir_all(&git_hooks_dir)
            .map_err(|e| { eprintln!("{}: Failed to create hooks directory {}: {}", "Error".red(), git_hooks_dir.display(), e); ExitCode::from(2) })?;
    }

    // Tier-1/2 override check
    if let Some(override_content) = resolve_hook_override(tool, hook_event)? {
        return write_git_hook_override(&hook_path, &override_content, force);
    }

    let linthis_hook_line = build_hook_command(hook_event, args);

    if hook_path.exists() {
        return append_linthis_to_existing_hook(&hook_path, &linthis_hook_line);
    }

    // Create new hook file as thin wrapper
    let content = build_thin_wrapper_script(hook_event, &HookTool::Git, None, false, None);
    write_hook_script(&hook_path, &content)?;

    println!("{} Created {} [project]", "✓".green(), hook_path.display());
    println!("  {} Thin wrapper: hook logic auto-updates with linthis", "→".dimmed());
    #[cfg(not(unix))]
    {
        println!("\nNext steps:");
        println!("  Make sure the hook is executable:");
        println!("    {}", format!("chmod +x .git/hooks/{}", hook_filename).cyan());
    }
    let project = git_root.to_str().unwrap_or("").to_string();
    save_installed_hook("local", &project, hook_event, &HookTool::Git, None, None);
    Ok(())
}

/// Write an override hook script, optionally appending to existing content.
fn write_git_hook_override(
    hook_path: &std::path::Path,
    override_content: &str,
    force: bool,
) -> Result<(), ExitCode> {
    let content = if hook_path.exists() && !force {
        let mut existing = std::fs::read_to_string(hook_path).unwrap_or_default();
        if !existing.ends_with('\n') { existing.push('\n'); }
        existing.push_str("\n# linthis-hook (override)\n");
        existing.push_str(override_content);
        existing
    } else {
        override_content.to_string()
    };
    write_hook_script(hook_path, &content)?;
    println!("{} Created {} [project, override]", "✓".green(), hook_path.display());
    Ok(())
}

/// Append linthis to an existing hook file, or report if already present.
fn append_linthis_to_existing_hook(
    hook_path: &std::path::Path,
    linthis_hook_line: &str,
) -> Result<(), ExitCode> {
    let existing_content = std::fs::read_to_string(hook_path)
        .map_err(|e| { eprintln!("{}: Failed to read existing hook file: {}", "Error".red(), e); ExitCode::from(2) })?;

    if existing_content.contains(linthis_hook_line) || existing_content.contains("linthis hook run") {
        println!("{}: linthis hook already exists in {}", "Info".cyan(), hook_path.display());
        return Ok(());
    }

    let mut new_content = existing_content;
    if !new_content.ends_with('\n') { new_content.push('\n'); }
    new_content.push_str("\n# linthis-hook\n");
    new_content.push_str(linthis_hook_line);
    new_content.push('\n');

    std::fs::write(hook_path, new_content)
        .map_err(|e| { eprintln!("{}: Failed to update {}: {}", "Error".red(), hook_path.display(), e); ExitCode::from(2) })?;
    println!("{} Added linthis to existing {} {} [project]", "✓".green(), hook_path.display(), "(appended)".dimmed());
    Ok(())
}

fn create_hook_config(tool: &HookTool, hook_event: &HookEvent, force: bool, args: &Option<String>) -> Result<(), ExitCode> {
    match tool {
        HookTool::Agent | HookTool::GitWithAgent | HookTool::PrekWithAgent => Ok(()),
        HookTool::Prek => create_prek_config(tool, hook_event, force, args),
        HookTool::Git => create_git_hook_config(tool, hook_event, force, args),
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
            // For pre-push: check only (formatting should happen at pre-commit stage)
            // Default "-c" = RunMode::CheckOnly
            let extra = args.as_deref().unwrap_or("-c");
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
    AgentFixProvider::Openclaw,
];

/// Split a `provider[/model]` string into (provider_name, Option<model>).
///
/// Examples:
///   "claude"       → ("claude", None)
///   "claude/opus"  → ("claude", Some("opus"))
///   "gemini/flash" → ("gemini", Some("flash"))
fn parse_provider_with_model(raw: &str) -> (&str, Option<&str>) {
    match raw.split_once('/') {
        Some((provider, model)) if !model.is_empty() => (provider, Some(model)),
        _ => (raw, None),
    }
}

/// Merge a model extracted from `provider/model` syntax into existing provider_args.
///
/// If `--provider-args` already contains `--model`, the `/model` part is ignored
/// and a warning is printed (explicit `--provider-args` takes precedence).
fn merge_model_into_provider_args(model: Option<&str>, existing: Option<&str>) -> Option<String> {
    // Check if existing provider_args already specifies --model
    if let (Some(m), Some(pa)) = (model, existing) {
        if pa.contains("--model") {
            eprintln!(
                "{}: --provider-args already contains --model, ignoring '{}' from provider/model syntax",
                "Warning".yellow(), m
            );
            return Some(pa.to_string());
        }
    }
    match (model, existing) {
        (Some(m), Some(pa)) => Some(format!("--model {} {}", m, pa)),
        (Some(m), None)     => Some(format!("--model {}", m)),
        (None, Some(pa))    => Some(pa.to_string()),
        (None, None)        => None,
    }
}

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
        AgentFixProvider::Openclaw => "openclaw",
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
fn agent_fix_headless_cmd(provider: &AgentFixProvider, prompt: &str, provider_args: Option<&str>) -> String {
    // Escape single quotes in prompt for shell safety
    let escaped = prompt.replace('\'', "'\\''");
    let extra = provider_args
        .filter(|a| !a.is_empty())
        .map(|a| format!(" {a}"))
        .unwrap_or_default();
    match provider {
        AgentFixProvider::Claude    => format!("claude -p{extra} --dangerously-skip-permissions '{}'", escaped),
        AgentFixProvider::Codex     => format!("codex exec{extra} --ask-for-approval never '{}'", escaped),
        AgentFixProvider::Gemini    => format!("gemini -p{extra} --approval-mode=auto_edit '{}'", escaped),
        AgentFixProvider::Cursor    => format!("cursor-agent chat{extra} --force '{}'", escaped),
        AgentFixProvider::Droid     => format!("droid exec{extra} --auto high '{}'", escaped),
        AgentFixProvider::Auggie    => format!("auggie{extra} --print '{}'", escaped),
        AgentFixProvider::Codebuddy => format!("codebuddy -p{extra} --dangerously-skip-permissions '{}'", escaped),
        AgentFixProvider::Openclaw => format!("openclaw agent{extra} --message '{}'", escaped),
    }
}

/// Generate a shell snippet that checks whether the provider binary exists in PATH.
///
/// If the binary is not found, prints a friendly message suggesting installation
/// or provider change, then gracefully degrades (skips the agent invocation).
/// The snippet sets `_LINTHIS_AGENT_OK=1` if available, `_LINTHIS_AGENT_OK=0` otherwise.
fn shell_agent_availability_check(provider: &AgentFixProvider) -> String {
    let bin = agent_fix_bin(provider);
    format!(
        "if command -v {bin} >/dev/null 2>&1; then\n\
         \x20 _LINTHIS_AGENT_OK=1\n\
         else\n\
         \x20 _LINTHIS_AGENT_OK=0\n\
         \x20 echo \"[linthis] ⚠ '{bin}' not found in PATH — skipping AI auto-fix\" >&2\n\
         \x20 echo \"[linthis]   To install: https://docs.anthropic.com/en/docs/claude-code\" >&2\n\
         \x20 echo \"[linthis]   To change provider: linthis hook install -g --type git-with-agent --provider <name> --event <event> --force\" >&2\n\
         \x20 echo \"[linthis]   Please fix the issues manually and retry.\" >&2\n\
         fi\n",
        bin = bin,
    )
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
            "openclaw"           => Some(AgentFixProvider::Openclaw),
            _ => None,
        };
        return parsed.ok_or_else(|| {
            eprintln!(
                "{}: Unknown agent fix provider '{}'. Valid: claude, codex, gemini, cursor, droid, auggie, codebuddy, openclaw",
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
/// Note: CommitMsg uses agent_fix_headless_cmd_commit_msg() instead (needs $1 expansion).
fn agent_fix_prompt_for_event(_hook_event: &HookEvent) -> String {
    "Staged files have linthis lint errors. \
     Run 'linthis -s' to inspect them. \
     Fix all issues by editing the files directly (do NOT use linthis --fix). \
     Verify with 'linthis -s' until it passes cleanly."
        .to_string()
}

/// Shell snippet printed after a successful agent commit-msg fix.
/// Shows the fixed message in green so it's visible in the terminal.
/// `indent` is the per-line prefix (spaces) matching the surrounding if-block depth.
fn agent_fix_show_fixed_cmsg(indent: &str) -> String {
    format!(
        "{i}if [ $LINTHIS_EXIT -eq 0 ] && [ -n \"$_MSG_FILE\" ]; then\n\
         {i}  printf '\\033[0;32m[linthis] ✓ New message: %s\\033[0m\\n' \"$(cat \"$_MSG_FILE\")\" >&2\n\
         {i}fi\n",
        i = indent,
    )
}

/// Build the agent command for commit-msg hook: captures $1 in _MSG_FILE then invokes agent.
/// Uses double-quoted prompt string so $_MSG_FILE expands at shell runtime.
fn agent_fix_headless_cmd_commit_msg(provider: &AgentFixProvider, provider_args: Option<&str>) -> String {
    let prompt = "Commit message validation failed (not in Conventional Commits format). \
        Fix the commit message file at $_MSG_FILE: \
        (1) run 'git diff --cached --stat' to understand what actually changed, \
        (2) run 'git log -n 5 --oneline' to check recent commit style AND the language used \
        (Chinese or English) — match that language for the description, \
        (3) choose the correct type (feat/fix/refactor/perf/docs/style/test/build/ci/chore/revert) \
        based on the diff, \
        (4) rewrite to: type(scope)?: description — lowercase type, ≤72 chars, no trailing period. \
        Overwrite $_MSG_FILE directly without asking. \
        Verify with 'linthis cmsg $_MSG_FILE' until it passes.";
    // Escape backslashes and double quotes for use in double-quoted shell string
    let escaped = prompt.replace('\\', "\\\\").replace('"', "\\\"");
    let extra = provider_args
        .filter(|a| !a.is_empty())
        .map(|a| format!(" {a}"))
        .unwrap_or_default();
    let bin_cmd = match provider {
        AgentFixProvider::Claude    => format!("claude -p{extra} --dangerously-skip-permissions \"{}\"", escaped),
        AgentFixProvider::Codex     => format!("codex exec{extra} --ask-for-approval never \"{}\"", escaped),
        AgentFixProvider::Gemini    => format!("gemini -p{extra} --approval-mode=auto_edit \"{}\"", escaped),
        AgentFixProvider::Cursor    => format!("cursor-agent chat{extra} --force \"{}\"", escaped),
        AgentFixProvider::Droid     => format!("droid exec{extra} --auto high \"{}\"", escaped),
        AgentFixProvider::Auggie    => format!("auggie{extra} --print \"{}\"", escaped),
        AgentFixProvider::Codebuddy => format!("codebuddy -p{extra} --dangerously-skip-permissions \"{}\"", escaped),
        AgentFixProvider::Openclaw => format!("openclaw agent{extra} --message \"{}\"", escaped),
    };
    // Prepend variable capture so $_MSG_FILE is available in the double-quoted prompt
    format!("_MSG_FILE=\"$1\"; {}", bin_cmd)
}

/// Error message for agent fix echo based on hook event type.
fn agent_fix_error_msg(hook_event: &HookEvent) -> &'static str {
    match hook_event {
        HookEvent::CommitMsg => "Commit message validation failed",
        _ => "Lint errors detected",
    }
}

/// Shell function to print a colored review summary box.
///
/// Usage: _print_review_box "passed"|"blocked" "message"
///
/// Emits a colored Unicode box to stderr similar to the linthis Rust output:
///   ╭────────────────────────────────────────────────╮
///   │ ✓ Linthis 📤 [Pre-push] Review Passed         │
///   ├────────────────────────────────────────────────┤
///   │ No critical issues found                       │
///   ╰────────────────────────────────────────────────╯
fn shell_review_box_fn() -> &'static str {
    // Raw string: \033 is literal backslash-0-3-3 (ANSI ESC via printf)
    // Box width = 52 (50 inner dashes + 2 border chars)
    // Inner content width = 48 (box_width - 4 for "│ " + " │")
    // Header visual widths (📤 emoji = 2 columns):
    //   "✓ Linthis 📤 [Pre-push] Review Passed"  = 36 chars, visual 37 → pad 11
    //   "✗ Linthis 📤 [Pre-push] Review Blocked" = 37 chars, visual 38 → pad 10
    r#"
_print_review_box() {
  if [ "$1" = "passed" ]; then
    _RH="✓ Linthis 📤 [Pre-push] Review Passed"
    _RC="\033[32m"
    _RHP="           "
  else
    _RH="✗ Linthis 📤 [Pre-push] Review Blocked"
    _RC="\033[31m"
    _RHP="          "
  fi
  _RN="\033[0m"
  _RM=$(printf "%-48s" "$2")
  printf "${_RC}╭──────────────────────────────────────────────────╮${_RN}\n" >&2
  printf "${_RC}│ ${_RH}${_RHP}│${_RN}\n" >&2
  printf "${_RC}├──────────────────────────────────────────────────┤${_RN}\n" >&2
  printf "${_RC}│ ${_RM} │${_RN}\n" >&2
  if [ "$1" != "passed" ]; then
    printf "${_RC}├──────────────────────────────────────────────────┤${_RN}\n" >&2
    printf "${_RC}│ To skip this check:                              │${_RN}\n" >&2
    printf "${_RC}│   git push --no-verify                           │${_RN}\n" >&2
  fi
  printf "${_RC}╰──────────────────────────────────────────────────╯${_RN}\n" >&2
}
"#
}

/// Shell snippet: a background elapsed-time spinner.
///
/// Usage in hook scripts:
///   start_timer "Fixing with claude"
///   <long-running command>
///   stop_timer
///
/// Prints: `[linthis] Fixing with claude (5s)` updating every second.
/// The cursor stays on the blank line below the timer so it does not
/// visually merge with the timer text.  stop_timer clears the timer line.
fn shell_timer_functions() -> &'static str {
    r#"
_linthis_timer_pid=""
start_timer() {
  _linthis_label="$1"
  printf "[linthis] ⠋ %s (0s)\n" "$_linthis_label" >&2
  (
    _i=0
    _s=0
    while true; do
      sleep 0.1
      _i=$((_i + 1))
      case $((_i % 10)) in
        0) _spin="⠋" ;;
        1) _spin="⠙" ;;
        2) _spin="⠹" ;;
        3) _spin="⠸" ;;
        4) _spin="⠼" ;;
        5) _spin="⠴" ;;
        6) _spin="⠦" ;;
        7) _spin="⠧" ;;
        8) _spin="⠇" ;;
        9) _spin="⠏" ;;
      esac
      if [ $((_i % 10)) -eq 0 ]; then
        _s=$((_s + 1))
      fi
      printf "\033[1A\r[linthis] %s %s (%ds)\033[K\n" "$_spin" "$_linthis_label" "$_s" >&2
    done
  ) &
  _linthis_timer_pid=$!
}
stop_timer() {
  if [ -n "$_linthis_timer_pid" ]; then
    kill "$_linthis_timer_pid" 2>/dev/null
    wait "$_linthis_timer_pid" 2>/dev/null
    _linthis_timer_pid=""
    printf "\r\033[K" >&2
  fi
}
"#
}

/// The review prompt for the pre-push agent code review.
fn prepush_review_prompt() -> &'static str {
    "Perform a structured pre-push code review using the lt.review skill. \
     Steps: \
     (1) Run: BASE_SHA=$(git merge-base HEAD origin/main 2>/dev/null || git rev-parse HEAD~1); \
     git diff $BASE_SHA..HEAD --stat; git diff $BASE_SHA..HEAD --name-status; git diff $BASE_SHA..HEAD. \
     (2) Review for Critical (security, data loss, broken API, logic errors), \
     Important (missing error handling, performance), and Minor issues. \
     (3) Write the review to .linthis/review/result/review-$(date +%Y%m%d-%H%M%S).md \
     (create the directory if needed). \
     (4) If Critical issues found: print '❌ Push blocked — fix Critical issues first' and exit 1. \
     If Important issues only: print '⚠️ Push with caution'. \
     If Minor or none: print '✅ Review passed'. \
     Exit 0 unless Critical issues were found."
}

/// Build the pre-push hook script that ALWAYS triggers an agent code review.
fn build_git_with_agent_prepush_script(linthis_cmd: &str, fix_provider: &AgentFixProvider, provider_args: Option<&str>) -> String {
    let agent_cmd = agent_fix_headless_cmd(fix_provider, prepush_review_prompt(), provider_args);
    let timer_fns = shell_timer_functions();
    let review_box = shell_review_box_fn();
    format!(
        "#!/bin/sh\n\
         {timer}\
         {review_box}\
         \n\
         # Compute files changed in commits being pushed vs upstream (or HEAD~1 as fallback)\n\
         _BASE=$(git rev-parse '@{{u}}' 2>/dev/null || \\\n\
         \x20       git merge-base HEAD origin/main 2>/dev/null || \\\n\
         \x20       git rev-parse 'HEAD~1' 2>/dev/null)\n\
         _PUSHED_FILES=$(git diff --name-only \"$_BASE\"..HEAD 2>/dev/null | grep -v '^$')\n\
         \n\
         # Run lint check on pushed files only (skip if no file changes, e.g. empty commits)\n\
         # Build -i <file> args for each pushed file (linthis uses -i, not positional paths)\n\
         _LINTHIS_CHECKED=0\n\
         if [ -n \"$_PUSHED_FILES\" ]; then\n\
         \x20 set --\n\
         \x20 while IFS= read -r _F; do set -- \"$@\" -i \"$_F\"; done <<_EOF_\n\
         $_PUSHED_FILES\n\
         _EOF_\n\
         \x20 _LINT_OUT=$({linthis} \"$@\" 2>&1)\n\
         \x20 LINTHIS_EXIT=$?\n\
         \x20 printf \"%s\\n\" \"$_LINT_OUT\" >&2\n\
         \x20 # Extract actual number of files checked from linthis output\n\
         \x20 _LINTHIS_CHECKED=$(printf \"%s\" \"$_LINT_OUT\" | sed -n 's/.*Files checked:[[:space:]]*\\([0-9]*\\).*/\\1/p' | tail -1)\n\
         \x20 _LINTHIS_CHECKED=${{_LINTHIS_CHECKED:-0}}\n\
         fi\n\
         \n\
         # Skip agent review if no files were actually checked\n\
         if [ -z \"$_PUSHED_FILES\" ] || [ \"$_LINTHIS_CHECKED\" = \"0\" ]; then\n\
         \x20 echo \"[linthis] No files to review — skipping code review\" >&2\n\
         \x20 exit 0\n\
         fi\n\
         \n\
         # Check if agent provider is available before review\n\
         {agent_check}\
         if [ \"$_LINTHIS_AGENT_OK\" = \"0\" ]; then\n\
         \x20 exit 0\n\
         fi\n\
         \n\
         # Invoke agent code review before push\n\
         echo \"[linthis] Invoking {provider} code review...\" >&2\n\
         start_timer \"Reviewing with {provider}\"\n\
         {agent}\n\
         REVIEW_EXIT=$?\n\
         stop_timer\n\
         \n\
         # Find the latest review report and check for critical issues\n\
         REVIEW_REPORT=$(ls -t .linthis/review/result/review-*.md 2>/dev/null | head -1)\n\
         if [ -n \"$REVIEW_REPORT\" ]; then\n\
         \x20 # Check for actual critical issues (agent exit code is unreliable)\n\
         \x20 _CRITICAL=$(awk '/^## Critical Issues/{{found=1;next}} found && /^## /{{found=0}} found && /^- \\[/{{print}}' \"$REVIEW_REPORT\")\n\
         \x20 if [ -n \"$_CRITICAL\" ]; then\n\
         \x20\x20\x20 _print_review_box \"blocked\" \"Critical issues found — fix before pushing\"\n\
         \x20\x20\x20 echo \"[linthis] Review saved: $REVIEW_REPORT\" >&2\n\
         \x20\x20\x20 exit 1\n\
         \x20 else\n\
         \x20\x20\x20 _print_review_box \"passed\" \"No critical issues found\"\n\
         \x20\x20\x20 echo \"[linthis] Review saved: $REVIEW_REPORT\" >&2\n\
         \x20 fi\n\
         fi\n\
         \n\
         exit $REVIEW_EXIT\n",
        timer = timer_fns,
        review_box = review_box,
        linthis = linthis_cmd,
        provider = fix_provider,
        agent = agent_cmd,
        agent_check = shell_agent_availability_check(fix_provider),
    )
}

/// Build the full git hook shell script with agent fix fallback.
fn build_git_with_agent_hook_script(linthis_cmd: &str, fix_provider: &AgentFixProvider, hook_event: &HookEvent, provider_args: Option<&str>) -> String {
    // Pre-push uses a dedicated review flow (always triggers agent, not only on failure)
    if matches!(hook_event, HookEvent::PrePush) {
        return build_git_with_agent_prepush_script(linthis_cmd, fix_provider, provider_args);
    }

    let agent_cmd = if matches!(hook_event, HookEvent::CommitMsg) {
        agent_fix_headless_cmd_commit_msg(fix_provider, provider_args)
    } else {
        let prompt = agent_fix_prompt_for_event(hook_event);
        agent_fix_headless_cmd(fix_provider, &prompt, provider_args)
    };
    let error_msg = agent_fix_error_msg(hook_event);
    let timer_fns = shell_timer_functions();
    let new_msg_print = if matches!(hook_event, HookEvent::CommitMsg) {
        agent_fix_show_fixed_cmsg("  ")
    } else {
        String::new()
    };
    let agent_check = shell_agent_availability_check(fix_provider);
    format!(
        "#!/bin/sh\n\
         {timer}\
         LINTHIS_CMD=\"{linthis}\"\n\
         _STAGED_FILES=$(git diff --cached --name-only)\n\
         \n\
         # Skip entirely if no staged files (empty commit)\n\
         if [ -z \"$_STAGED_FILES\" ]; then\n\
         \x20 exit 0\n\
         fi\n\
         \n\
         $LINTHIS_CMD\n\
         LINTHIS_EXIT=$?\n\
         # Re-stage files modified by linthis -f (auto-format), regardless of exit code\n\
         if [ -n \"$_STAGED_FILES\" ]; then\n\
         \x20 echo \"$_STAGED_FILES\" | xargs git add\n\
         fi\n\
         \n\
         if [ $LINTHIS_EXIT -ne 0 ]; then\n\
         \x20 # Check if agent provider is available before attempting fix\n\
         \x20 {agent_check}\
         \x20 if [ \"$_LINTHIS_AGENT_OK\" = \"1\" ]; then\n\
         \x20\x20\x20 echo \"[linthis] {error_msg}. Invoking {provider} to fix...\" >&2\n\
         \x20\x20\x20 start_timer \"Fixing with {provider}\"\n\
         \x20\x20\x20 {agent}\n\
         \x20\x20\x20 stop_timer\n\
         \x20\x20\x20 # Re-stage files modified by agent fix\n\
         \x20\x20\x20 if [ -n \"$_STAGED_FILES\" ]; then\n\
         \x20\x20\x20\x20\x20 echo \"$_STAGED_FILES\" | xargs git add\n\
         \x20\x20\x20 fi\n\
         \x20\x20\x20 # Re-verify after agent fix\n\
         \x20\x20\x20 echo \"[linthis] Re-verifying...\" >&2\n\
         \x20\x20\x20 $LINTHIS_CMD\n\
         \x20\x20\x20 LINTHIS_EXIT=$?\n\
         \x20 fi\n\
         {new_msg_print}\
         fi\n\
         \n\
         exit $LINTHIS_EXIT\n",
        timer = timer_fns,
        linthis = linthis_cmd,
        agent_check = agent_check,
        provider = fix_provider,
        agent = agent_cmd,
        error_msg = error_msg,
        new_msg_print = new_msg_print,
    )
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
                eprintln!("{}: Could not determine global hooks directory", "Error".red());
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
        Ok((git_root.join(".git/hooks").join(hook_filename), "local", project_str))
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

    let content = build_thin_wrapper_script(
        hook_event, &HookTool::GitWithAgent, Some(fix_provider.as_str()), global, provider_args,
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

    println!("{} Created {} (git-with-agent, {})", "✓".green(), hook_path.display(), fix_provider);
    println!("  {} On lint failure: {}", "→".dimmed(), agent_fix_bin(fix_provider).cyan());
    println!("  {} Thin wrapper: hook logic auto-updates with linthis", "→".dimmed());

    if global {
        if let Some(hooks_dir) = global_hooks_dir() {
            let hooks_dir_str = hooks_dir.to_string_lossy().to_string();
            let _ = std::process::Command::new("git")
                .args(["config", "--global", "core.hooksPath", &hooks_dir_str])
                .status();
            println!("{} Set {} = {}", "✓".green(), "core.hooksPath".cyan(), hooks_dir_str);
        }
    }

    save_installed_hook(scope, &project, hook_event, &HookTool::GitWithAgent, Some(fix_provider.as_str()), provider_args);
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
        HookTool::Prek       => "prek run",
        _ => return ExitCode::from(1),
    };

    let prompt = format!(
        "The {tool} pre-commit check failed with lint errors. \
         Run '{tool_cmd}' to see them. Fix all issues by editing the files directly. \
         Verify by running '{tool_cmd}' again until it passes.",
        tool = fix_provider,
        tool_cmd = tool_cmd,
    );
    let agent_cmd = agent_fix_headless_cmd(fix_provider, &prompt, None);
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
    AgentProvider::Openclaw,
];

/// The section marker used in append-style files (CLAUDE.md, AGENTS.md)
const AGENT_SECTION_MARKER: &str = "## Linthis Lint Rule";
/// Legacy section marker for backward compatibility (uninstall/detection)
const AGENT_SECTION_MARKER_LEGACY: &str = "## Linthis Agent Rules";


/// Generic skill body for a given event (used by all providers).
fn agent_event_content_generic(event: &HookEvent) -> String {
    match event {
        HookEvent::PreCommit => agent_lint_body(),
        HookEvent::CommitMsg => agent_cmsg_body(),
        HookEvent::PrePush => agent_review_body(),
    }
}

fn agent_lint_body() -> String {
    r#"## Companion Skills

When the user asks to commit, if the **lt-cmsg** skill is also available, both lt-lint and lt-cmsg should be invoked together. Run lt-lint first (to fix code issues), then lt-cmsg (to validate the commit message).

## Goal

Catch and fix code quality issues before they enter the repository. Running lint at pre-commit ensures every committed file meets project coding standards, preventing style debt from accumulating over time.

## When to Skip

If no code files were modified in this session, approve immediately.

## Key Commands

| Scope | Command | Description |
|-------|---------|-------------|
| Staged files | `linthis -s` | Check & format all files in the git staging area (`git add`ed) |
| Modified files | `linthis -m` | Check & format all locally modified files (staged + unstaged) |
| Specific files | `linthis -i <f1> -i <f2>` | Check & format listed files — one `-i` per file |
| Check only | append `-c` | Lint only, no formatting (e.g. `linthis -s -c`) |

## Steps

1. Identify modified code files in this session (files written or edited via Write/Edit tools, or via Bash)
2. Run lint + format on those files:
   - `linthis -m` to cover all modified files at once, or
   - `linthis -i <file1> -i <file2>` to target specific files
   - **Note**: linthis may auto-format files (whitespace, trailing newlines, etc.) in addition to reporting lint errors
3. Read the lint output carefully — each issue includes file path, line number, and rule name
4. If issues are found, fix them by editing the code directly
   - Do **NOT** use `linthis --fix` or `linthis fix` — fixing manually ensures you understand the issue and don't introduce regressions from blind automated transforms
5. Re-run linthis to confirm all issues are resolved
6. **Re-stage**: if any files were already staged before step 2, linting/formatting may have changed them on disk. You must re-stage those files so the index matches the working tree:
   ```
   git add <formatted or fixed files>
   ```
7. Final check: run `linthis -s -c` (check-only on staged files) to verify the staging area is clean
8. Only approve the commit once lint passes with zero errors

## Example

```
$ linthis -i src/handler.go

src/handler.go:15:1: exported function HandleRequest should have comment (golint)
src/handler.go:23:4: error return value not checked (errcheck)

2 issues found
```

Fix line 15 by adding a doc comment, and line 23 by handling the error return value. Then re-run to confirm zero errors. If files were staged, re-stage: `git add src/handler.go`."#
        .to_string()
}

fn agent_cmsg_body() -> String {
    r#"## Companion Skills

When the user asks to commit, if the **lt-lint** skill is also available, both lt-lint and lt-cmsg should be invoked together. Run lt-lint first (to fix code issues), then lt-cmsg (to validate the commit message).

## Goal

Ensure every commit message follows Conventional Commits format and accurately reflects the actual code changes. A well-structured commit history makes code review, changelog generation, and git bisect much easier.

## When to Skip

If `linthis cmsg .git/COMMIT_EDITMSG` passes on the first run, approve immediately with `✅ Commit message OK`.

## Configuration

The validation pattern can be configured in `.linthis/config.toml` (project-level) or the global linthis config. If no config is present, `linthis cmsg` defaults to Conventional Commits format:

```toml
[cmsg]
commit_msg_pattern = "^(feat|fix|docs|...)\\(\\S+\\)?: .{1,72}"
require_ticket = false          # require ticket reference e.g. [JIRA-123]
ticket_pattern = "\\[\\w+-\\d+\\]"  # custom ticket regex
```

`linthis cmsg` resolves config automatically (project → global → built-in default). To check the effective pattern quickly:

```bash
linthis config get cmsg.commit_msg_pattern      # project-level
linthis config get cmsg.commit_msg_pattern -g   # global
# "not found" means built-in default (Conventional Commits) applies
```

## Steps

1. Run `linthis cmsg .git/COMMIT_EDITMSG` — this is the **authoritative validator** and reads `.linthis/config.toml` automatically
2. If linthis cmsg **passes** → output `✅ Commit message OK` and approve immediately
3. If linthis cmsg **fails** → read the error output to understand what rule was violated, then:
   - Run `git diff --cached --stat` to understand what files actually changed — the type prefix must match the actual diff
   - Run `git log -n 5 --oneline` to check the recent commit style **and language** (Chinese or English) — match that language for consistency
   - **Automatically rewrite** `.git/COMMIT_EDITMSG` based on the linthis error hints + diff analysis — do NOT ask for confirmation
4. Re-run `linthis cmsg .git/COMMIT_EDITMSG` to confirm the rewrite passes

## Type Selection Guide

Select the type by examining the staged diff, not by guessing from the message:

| Type | When to use |
|------|-------------|
| **feat** | New feature or functionality |
| **fix** | Bug fix |
| **refactor** | Code restructured, no behavior change |
| **docs** | Documentation only |
| **style** | Formatting, whitespace, lint fixes |
| **test** | Adding or updating tests |
| **build** | Build scripts, deps, CI config |
| **chore** | Maintenance, tooling |

## Examples

**Good:**
```
feat: add user authentication module
fix(parser): handle nil pointer when input is empty
docs: update README with setup instructions
refactor(core): extract common utility functions
```

**Bad → Fixed:**
```
# Bad: wrong type (diff shows bug fix)
feat: fix login crash on empty password
# Fixed:
fix(auth): handle empty password input gracefully

# Bad: vague, no type
update code
# Fixed (based on diff):
refactor(utils): extract shared validation logic
```"#
        .to_string()
}

fn agent_review_body() -> String {
    r#"## Goal

Catch issues that lint can't — logic errors, security vulnerabilities, architectural problems, and missing test coverage. This is the last automated quality gate before code reaches the remote, so focus on issues that would be costly to fix after pushing.

## When to Skip

If there are no outgoing commits (local is up-to-date with remote), approve immediately.

## Steps

### Step 1 — Gather diff

```bash
BASE_SHA=$(git merge-base HEAD origin/main 2>/dev/null || git rev-parse HEAD~1)
HEAD_SHA=$(git rev-parse HEAD)
git diff "$BASE_SHA".."$HEAD_SHA" --stat
git diff "$BASE_SHA".."$HEAD_SHA" --name-status
git diff "$BASE_SHA".."$HEAD_SHA"
```

### Step 2 — Print diff stats

```
📊 Diff Stats
  Base:  <BASE_SHA>
  Head:  <HEAD_SHA>
  Files: N changed, +X insertions, -Y deletions

📁 Changed Files
  ✅ M  src/foo.rs
  ⚠️ A  src/bar.rs   (new file — review carefully)
  ⏭️ D  src/old.rs   (deleted)
```

### Step 3 — Review by category

| Category | What to look for | Severity |
|---|---|---|
| **Critical** | Security vulnerabilities (injection, hardcoded secrets), data loss risk, logic errors, broken API | Blocking |
| **Important** | Missing error handling, untested edge cases, performance issues, missing test coverage | Should fix |
| **Minor** | Style inconsistencies, redundant code, missing comments | Optional |

Focus on the diff, not the whole file — only review what changed. Explain **why** something is a problem and suggest concrete fixes.

### Step 4 — Write structured review

Output to terminal AND write to `.linthis/review/result/review-<YYYYMMDD-HHMMSS>.md`:

```markdown
# Code Review — <HEAD_SHA>
Date: <timestamp>
Base: <BASE_SHA> → Head: <HEAD_SHA>
Files: N changed, +X -Y

## Summary
<1-3 sentence overall assessment>

## Critical Issues
- [ ] <file>:<line> — <description>

## Important Issues
- [ ] <file>:<line> — <description>

## Minor Issues
- [ ] <file>:<line> — <description>

## Assessment
BLOCK / PROCEED WITH FIXES / APPROVED
```

Create `.linthis/review/result/` directory if it doesn't exist.

### Step 5 — Gate the push

- **Critical issues** → output `❌ Push blocked — fix Critical issues first`; do not proceed
- **Important issues only** → output `⚠️ Push with caution`; ask user to confirm
- **Minor or none** → output `✅ Review passed`; proceed

## Review Principles

- **Don't nitpick style** — that's what the lint skill handles. Focus on logic, security, and architecture
- **Explain why** — "SQL injection lets attackers execute arbitrary queries" is more actionable than just "SQL injection found"
- **Suggest concrete fixes** — show corrected code when possible, not just "fix this""#
        .to_string()
}

/// Generate the Stop hook JSON content for .claude/settings.json
const AGENT_STOP_HOOK_JSON: &str = r#"{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "prompt",
            "prompt": "Before finishing, check if any code files were modified during this session (Write/Edit/Bash tools). If code was modified:\n1. Run `linthis -i <file1> -i <file2>` on all modified files to check for lint issues\n2. If issues are found, fix them yourself by editing the code directly (do NOT use `linthis --fix` or `linthis fix`)\n3. Re-run `linthis -i <files>` to confirm all issues are resolved\n4. Only approve stopping once lint passes with no errors\n\nIf no code files were modified, approve stopping immediately.\n\nYou MUST respond with valid JSON: {\"ok\": true} to approve stopping, or {\"ok\": false, \"reason\": \"description of remaining lint issues\"} to block."
          }
        ]
      }
    ]
  }
}"#;

fn agent_stop_hook_json_ref() -> &'static str {
    AGENT_STOP_HOOK_JSON
}

/// Get the short event name used for skill file naming.
fn event_short_name(event: &HookEvent) -> &'static str {
    match event {
        HookEvent::PreCommit => "lint",
        HookEvent::CommitMsg => "cmsg",
        HookEvent::PrePush   => "review",
    }
}

/// Resolve the custom or default skill name for an event.
fn resolve_skill_name(
    event: &HookEvent,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
    prefix: &str,
) -> String {
    let custom: Option<&str> = skill_names.and_then(|sn| match event {
        HookEvent::PreCommit => sn.pre_commit.as_deref(),
        HookEvent::CommitMsg => sn.commit_msg.as_deref(),
        HookEvent::PrePush   => sn.pre_push.as_deref(),
    });
    custom.map_or_else(|| format!("{}{}", prefix, event_short_name(event)), |n| n.to_string())
}

/// Get the skill file path for a given agent provider and hook event.
fn agent_skill_path(
    base: &std::path::Path,
    provider: &AgentProvider,
    global: bool,
    event: &HookEvent,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> PathBuf {
    match provider {
        AgentProvider::Claude => {
            let dir_name = resolve_skill_name(event, skill_names, "lt-");
            base.join(".claude/skills").join(dir_name).join("SKILL.md")
        }
        AgentProvider::Codex => {
            if global { base.join(".codex/AGENTS.md") } else { base.join("AGENTS.md") }
        }
        AgentProvider::Gemini => {
            let name = resolve_skill_name(event, skill_names, "linthis-");
            base.join(".gemini").join(format!("{}.md", name))
        }
        AgentProvider::Cursor => {
            let name = resolve_skill_name(event, skill_names, "linthis-");
            base.join(".cursor/rules").join(format!("{}.mdc", name))
        }
        AgentProvider::Droid => {
            let name = resolve_skill_name(event, skill_names, "linthis-");
            base.join(".droid/rules").join(format!("{}.md", name))
        }
        AgentProvider::Auggie => {
            let name = resolve_skill_name(event, skill_names, "linthis-");
            base.join(".augment/rules").join(format!("{}.md", name))
        }
        AgentProvider::Codebuddy => {
            let dir_name = resolve_skill_name(event, skill_names, "lt-");
            base.join(".codebuddy/skills").join(dir_name).join("SKILL.md")
        }
        AgentProvider::Openclaw => {
            let dir_name = resolve_skill_name(event, skill_names, "lt-");
            base.join(".openclaw/skills").join(dir_name).join("SKILL.md")
        }
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
        // OpenClaw has its own hooks system; no settings.json needed
        _ => None,
    }
}

/// Print extra installed file messages (Stop Hook)
fn print_extra_installed(base: &std::path::Path, provider: &AgentProvider) {
    // Stop Hook
    if let Some(settings_path) = agent_stop_hook_settings_path(base, provider) {
        println!(
            "{} Installed Stop Hook → {}",
            "✓".green(),
            settings_path.display()
        );
    }
}

/// Print info about an already-installed agent provider (file path + content)
fn print_agent_installed_info(
    base: &std::path::Path,
    provider: &AgentProvider,
    global: bool,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) {
    let events = [HookEvent::PreCommit, HookEvent::CommitMsg, HookEvent::PrePush];
    for event in &events {
        let path = agent_skill_path(base, provider, global, event, skill_names);
        if path.exists() {
            let event_name = match event {
                HookEvent::PreCommit => "pre-commit",
                HookEvent::CommitMsg => "commit-msg",
                HookEvent::PrePush => "pre-push",
            };
            println!("       {} {} ({})", "File:".dimmed(), path.display(), event_name);
        }
    }
    if let Some(settings_path) = agent_stop_hook_settings_path(base, provider) {
        if settings_path.exists() {
            println!("       {} {}", "File:".dimmed(), settings_path.display());
        }
    }
}

/// Check if agent integration is installed for a given provider
fn agent_is_installed(
    base: &std::path::Path,
    provider: &AgentProvider,
    global: bool,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> bool {
    let events = [HookEvent::PreCommit, HookEvent::CommitMsg, HookEvent::PrePush];
    match provider {
        // Section-based: check for any event section marker in AGENTS.md
        AgentProvider::Codex => {
            let path = agent_skill_path(base, provider, global, &HookEvent::PreCommit, skill_names);
            path.exists()
                && std::fs::read_to_string(&path)
                    .map(|c| {
                        c.contains(AGENT_SECTION_MARKER)
                            || c.contains("## Linthis Commit Message Rule")
                            || c.contains("## Linthis Review Rule")
                            || c.contains(AGENT_SECTION_MARKER_LEGACY)
                    })
                    .unwrap_or(false)
        }
        // Skill-dir-based: check if any per-event skill file exists
        AgentProvider::Claude | AgentProvider::Codebuddy | AgentProvider::Openclaw => {
            events.iter().any(|e| agent_skill_path(base, provider, global, e, skill_names).exists())
        }
        // File-based: check if any per-event file exists
        _ => events.iter().any(|e| agent_skill_path(base, provider, global, e, skill_names).exists()),
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
    if base.join("CODEBUDDY.md").exists() || base.join(".codebuddy").exists() {
        detected.push(AgentProvider::Codebuddy);
    }
    if base.join(".openclaw").exists() {
        detected.push(AgentProvider::Openclaw);
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
                AgentProvider::Openclaw  => "OpenClaw",
            };
            let detected = match p {
                AgentProvider::Claude    => root.join(".claude").exists(),
                AgentProvider::Codex     => root.join("AGENTS.md").exists(),
                AgentProvider::Gemini    => root.join(".gemini").exists(),
                AgentProvider::Cursor    => root.join(".cursor").exists(),
                AgentProvider::Droid     => root.join(".droid").exists(),
                AgentProvider::Auggie    => root.join(".augment").exists(),
                AgentProvider::Codebuddy => root.join("CODEBUDDY.md").exists() || root.join(".codebuddy").exists(),
                AgentProvider::Openclaw  => root.join(".openclaw").exists(),
            };
            (name, detected)
        })
        .collect()
}

/// Install a dedicated skill file (Cursor, Windsurf, Cline, CodeBuddy)
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

/// Recursively copy a directory tree from `src` to `dst`.
///
/// Creates `dst` if it does not exist.  Overwrites existing files.
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    use std::fs;

    if !dst.exists() {
        fs::create_dir_all(dst)
            .map_err(|e| format!("Failed to create directory {}: {}", dst.display(), e))?;
    }

    for entry in fs::read_dir(src)
        .map_err(|e| format!("Failed to read directory {}: {}", src.display(), e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy {} → {}: {}", src_path.display(), dst_path.display(), e))?;
        }
    }

    Ok(())
}

/// Map an event to its built-in agent plugin ID.
///
/// All events share a single plugin `"lt"` — the event determines which
/// skill *within* that plugin is used, not which plugin is resolved.
fn agent_plugin_id(_event: &HookEvent) -> &'static str {
    "lt"
}

/// Target directory for agent slash commands per provider.
fn agent_command_dir(base: &std::path::Path, provider: &AgentProvider) -> Option<std::path::PathBuf> {
    match provider {
        AgentProvider::Claude    => Some(base.join(".claude/commands/linthis")),
        AgentProvider::Codebuddy => Some(base.join(".codebuddy/commands/linthis")),
        AgentProvider::Gemini    => Some(base.join(".gemini/commands")),
        AgentProvider::Cursor    => Some(base.join(".cursor/commands")),
        AgentProvider::Droid     => Some(base.join(".droid/commands")),
        AgentProvider::Auggie    => Some(base.join(".augment/commands")),
        AgentProvider::Codex     => None, // Codex uses section-based AGENTS.md; no command dir
        AgentProvider::Openclaw  => Some(base.join(".openclaw/commands")),
    }
}

/// Install skill component from a plugin directory.
fn install_plugin_skill(
    plugin_dir: &std::path::Path,
    base: &std::path::Path,
    provider: &AgentProvider,
    event: &HookEvent,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
    target: Option<&linthis::config::AgentTargetConfig>,
) -> Result<(), String> {
    use std::fs;
    let (skill_name, _) = agent_event_skill_metadata(event, skill_names);
    let skill_src_dir = plugin_dir.join("skills").join(&skill_name);
    let skill_src = skill_src_dir.join("SKILL.md");
    if !skill_src.is_file() {
        return Ok(());
    }

    if let Some(target_skills) = target.and_then(|t| t.skills.as_deref()) {
        let custom_skill_dir = base.join(target_skills).join(&skill_name);
        return copy_dir_recursive(&skill_src_dir, &custom_skill_dir);
    }

    let skill_path = agent_skill_path(base, provider, false, event, skill_names);
    match provider {
        AgentProvider::Codex => {
            let content = fs::read_to_string(&skill_src)
                .map_err(|e| format!("Failed to read skill file '{}': {}", skill_src.display(), e))?;
            install_agent_append_section(&skill_path, &content, agent_event_section_marker(event), "# Agent Instructions\n")
        }
        AgentProvider::Claude | AgentProvider::Codebuddy | AgentProvider::Openclaw => {
            let target_dir = skill_path.parent().unwrap();
            copy_dir_recursive(&skill_src_dir, target_dir)?;
            if matches!(provider, AgentProvider::Openclaw) {
                openclaw_post_install_skill(target_dir);
            }
            Ok(())
        }
        _ => {
            let content = fs::read_to_string(&skill_src)
                .map_err(|e| format!("Failed to read skill file '{}': {}", skill_src.display(), e))?;
            install_agent_dedicated_file(&skill_path, &content)
        }
    }
}

/// Install command files from a plugin directory.
fn install_plugin_commands(
    plugin_dir: &std::path::Path,
    base: &std::path::Path,
    provider: &AgentProvider,
    target: Option<&linthis::config::AgentTargetConfig>,
) -> Result<(), String> {
    use std::fs;
    let cmd_src_dir = plugin_dir.join("commands");
    if !cmd_src_dir.is_dir() {
        return Ok(());
    }
    let cmd_dir = if let Some(target_commands) = target.and_then(|t| t.commands.as_deref()) {
        Some(base.join(target_commands))
    } else {
        agent_command_dir(base, provider)
    };
    if let Some(cmd_dir) = cmd_dir {
        if let Ok(entries) = fs::read_dir(&cmd_src_dir) {
            for entry in entries.flatten() {
                if entry.path().is_file() {
                    let cmd_target = cmd_dir.join(entry.file_name());
                    let content = fs::read_to_string(entry.path())
                        .map_err(|e| format!("Failed to read command file '{}': {}", entry.path().display(), e))?;
                    install_agent_dedicated_file(&cmd_target, &content)?;
                }
            }
        }
    }
    Ok(())
}

/// Get the default memory file path for a provider.
fn agent_memory_path(base: &std::path::Path, provider: &AgentProvider) -> Option<PathBuf> {
    match provider {
        AgentProvider::Claude    => Some(base.join("CLAUDE.md")),
        AgentProvider::Codebuddy => Some(base.join("CODEBUDDY.md")),
        AgentProvider::Gemini    => Some(base.join(".gemini/GEMINI.md")),
        AgentProvider::Cursor    => Some(base.join(".cursor/CURSOR.md")),
        AgentProvider::Droid     => Some(base.join(".droid/DROID.md")),
        AgentProvider::Auggie    => Some(base.join(".augment/AUGMENT.md")),
        AgentProvider::Codex     => None,
        AgentProvider::Openclaw  => Some(base.join("AGENTS.md")),
    }
}

/// Install memory component from a plugin directory.
fn install_plugin_memory(
    plugin_dir: &std::path::Path,
    base: &std::path::Path,
    provider: &AgentProvider,
    event: &HookEvent,
    target: Option<&linthis::config::AgentTargetConfig>,
) -> Result<(), String> {
    let mem_src = plugin_dir.join("memories").join("TOPLEVEL.md");
    if !mem_src.is_file() {
        return Ok(());
    }
    let memory_target = if let Some(target_memory) = target.and_then(|t| t.memory.as_deref()) {
        Some(base.join(target_memory))
    } else {
        agent_memory_path(base, provider)
    };
    if let Some(mem_target) = memory_target {
        let content = std::fs::read_to_string(&mem_src)
            .map_err(|e| format!("Failed to read memory file '{}': {}", mem_src.display(), e))?;
        let plugin_id = agent_plugin_id(event);
        let section_marker = &format!("linthis-memory-{}", plugin_id);
        install_agent_append_section(&mem_target, &content, section_marker, "")?;
    }
    Ok(())
}

/// Install stop hook from a plugin's hooks/hooks.json.
fn install_plugin_stop_hook(
    plugin_dir: &std::path::Path,
    base: &std::path::Path,
    provider: &AgentProvider,
    target: Option<&linthis::config::AgentTargetConfig>,
) -> Result<(), String> {
    let hooks_json_src = plugin_dir.join("hooks").join("hooks.json");
    if !hooks_json_src.is_file() || matches!(provider, AgentProvider::Openclaw) {
        return Ok(());
    }
    let settings_path = if let Some(target_settings) = target.and_then(|t| t.settings.as_deref()) {
        Some(base.join(target_settings))
    } else {
        agent_stop_hook_settings_path(base, provider)
    };
    if let Some(settings_path) = settings_path {
        let override_json = std::fs::read_to_string(&hooks_json_src)
            .map_err(|e| format!("Failed to read hooks.json '{}': {}", hooks_json_src.display(), e))?;
        install_agent_stop_hook_from_json(base, &settings_path, &override_json)?;
    }
    Ok(())
}

/// Resolve and install agent plugin components (skill, command, memory, hooks) from a plugin directory.
fn install_agent_plugin_from_dir(
    plugin_dir: &std::path::Path,
    base: &std::path::Path,
    provider: &AgentProvider,
    event: &HookEvent,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
    target: Option<&linthis::config::AgentTargetConfig>,
) -> Result<(), String> {
    install_plugin_skill(plugin_dir, base, provider, event, skill_names, target)?;
    install_plugin_commands(plugin_dir, base, provider, target)?;
    install_plugin_memory(plugin_dir, base, provider, event, target)?;
    install_plugin_stop_hook(plugin_dir, base, provider, target)?;
    Ok(())
}

/// Resolve an agent plugin entry from the nested `[hook.agent.plugins]` config.
///
/// Looks up the provider-specific map first, then falls back to `_default`.
fn resolve_agent_plugin<'a>(
    hook_config: &'a linthis::config::HookConfig,
    plugin_id: &str,
    provider: &str,
) -> Option<&'a linthis::config::HookSourceEntry> {
    hook_config.agent.plugins.get(provider)
        .and_then(|m| m.get(plugin_id))
        .or_else(|| hook_config.agent.plugins.get("_default")
            .and_then(|m| m.get(plugin_id)))
}

/// Tier-1/2 override check for an agent plugin (skill + command + memory bundle).
///
/// Returns `Ok(true)` if an override was found and installed, `Ok(false)` to fall through
/// to the built-in generator, or `Err` on a hard resolution failure.
///
/// When `global` is true, tier-1 (project-local fixed paths) is skipped but tier-2
/// (TOML agent-plugins config) still applies so globally installed plugins are synced.
fn resolve_and_install_agent_plugin_override(
    base: &std::path::Path,
    provider: &AgentProvider,
    event: &HookEvent,
    global: bool,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> Result<bool, String> {
    use linthis::config::Config;
    use linthis::hooks::resolver;

    let plugin_id = agent_plugin_id(event);
    let provider_name = format!("{:?}", provider).to_lowercase();
    let project_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    // Tier 1: fixed-path plugin directory (project-local only, skip for global installs)
    //   1a: hooks/agent/plugins/<provider>/<plugin>/  (provider override)
    //   1b: hooks/agent/plugins/<plugin>/              (default fallback)
    if !global {
        if let Some(plugin_dir) = resolver::fixed_agent_plugin_dir(&project_root, &provider_name, plugin_id) {
            install_agent_plugin_from_dir(&plugin_dir, base, provider, event, skill_names, None)?;
            return Ok(true);
        }
    }

    // Tier 2: TOML agent plugin entry with provider fallback
    let config = Config::load_merged(&project_root);
    if let Some(entry) = resolve_agent_plugin(&config.hook, plugin_id, &provider_name) {
        let resolved = resolver::resolve_to_dir(&entry.source, &project_root, &config.hook.marketplaces)
            .map_err(|e| format!("Failed to resolve agent plugin '{}': {}", plugin_id, e))?;
        install_agent_plugin_from_dir(resolved.path(), base, provider, event, skill_names, entry.target.as_ref())?;
        return Ok(true);
    }

    // Tier 2.5: Scan cached plugin directories for agent plugin overrides.
    // This covers global installs where plugin sources are configured but
    // [hook.agent.plugins] is not explicitly set in the TOML.
    {
        use linthis::plugin::{PluginCache, PluginConfigManager};

        let managers: Vec<_> = if global {
            [PluginConfigManager::global()].into_iter().filter_map(|r| r.ok()).collect()
        } else {
            [PluginConfigManager::project()].into_iter().filter_map(|r| r.ok()).collect()
        };

        if let Ok(cache) = PluginCache::new() {
            for mgr in &managers {
                if let Ok(plugins) = mgr.list_plugins() {
                    for (_name, url, _ref) in &plugins {
                        let cache_path = cache.url_to_cache_path(url);
                        // Try provider-specific override first, then _default fallback
                        let provider_dir = cache_path
                            .join("hooks")
                            .join("agent")
                            .join("plugins")
                            .join(&provider_name)
                            .join(plugin_id);
                        if provider_dir.is_dir() {
                            install_agent_plugin_from_dir(&provider_dir, base, provider, event, skill_names, None)?;
                            return Ok(true);
                        }
                        let default_dir = cache_path
                            .join("hooks")
                            .join("agent")
                            .join("plugins")
                            .join("_default")
                            .join(plugin_id);
                        if default_dir.is_dir() {
                            install_agent_plugin_from_dir(&default_dir, base, provider, event, skill_names, None)?;
                            return Ok(true);
                        }
                    }
                }
            }
        }
    }

    // Tier 3: no override — fall through to built-in generator
    Ok(false)
}

/// Resolve the OpenClaw global skills directory.
///
/// Probes well-known locations in order:
/// 1. `~/.openclaw/skills/`  (macOS / Linux)
/// 2. `C:\openclaw\openclaw\source\node_modules\openclaw\skills\`  (Windows)
///
/// Returns the first directory that exists, or `None`.
fn resolve_openclaw_global_skills_dir() -> Option<PathBuf> {
    // ~/.openclaw/skills/
    if let Some(home) = dirs::home_dir() {
        let candidate = home.join(".openclaw/skills");
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    // Windows fallback
    let win_candidate = PathBuf::from(r"C:\openclaw\openclaw\source\node_modules\openclaw\skills");
    if win_candidate.is_dir() {
        return Some(win_candidate);
    }
    None
}

/// Register an OpenClaw skill via CLI after files are written.
///
/// Strategy:
/// 1. Try `openclaw skills install <dir>` if the CLI is available.
/// 2. If the CLI is not in PATH, fallback to copying the skill directory into
///    OpenClaw's well-known global skills directories
///    (`~/.openclaw/skills/` or `C:\openclaw\...\skills\`).
fn openclaw_post_install_skill(skill_dir: &std::path::Path) {
    use std::process::Command;

    // ── Try CLI first ────────────────────────────────────────────────────
    if is_command_available("openclaw") {
        match Command::new("openclaw")
            .args(["skills", "install", &skill_dir.to_string_lossy()])
            .output()
        {
            Ok(output) if output.status.success() => {
                println!("  {} Registered skill via 'openclaw skills install'", "✓".green());
                println!("  {} Verify with 'openclaw skills list'", "→".dimmed());
                return;
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!(
                    "  {} 'openclaw skills install' exited with {}: {}",
                    "Warning".yellow(),
                    output.status,
                    stderr.trim()
                );
                // Fall through to direct-copy fallback
            }
            Err(e) => {
                println!("  {} Failed to run 'openclaw skills install': {}", "Warning".yellow(), e);
                // Fall through to direct-copy fallback
            }
        }
    }

    // ── Fallback: copy skill dir into global skills directory ─────────────
    let skill_name = match skill_dir.file_name() {
        Some(name) => name,
        None => {
            println!(
                "  {} Could not determine skill name from path '{}'",
                "Warning".yellow(), skill_dir.display()
            );
            return;
        }
    };

    if let Some(global_skills) = resolve_openclaw_global_skills_dir() {
        let target = global_skills.join(skill_name);
        match copy_dir_recursive(skill_dir, &target) {
            Ok(()) => {
                println!(
                    "  {} Copied skill to {} (CLI unavailable, direct copy fallback)",
                    "✓".green(), target.display()
                );
                println!("  {} When openclaw CLI is available, verify with 'openclaw skills list'", "→".dimmed());
            }
            Err(e) => {
                println!(
                    "  {} Failed to copy skill to {}: {}",
                    "Warning".yellow(), target.display(), e
                );
            }
        }
    } else {
        println!(
            "  {} 'openclaw' CLI not found and no known skills directory (~/.openclaw/skills/) detected.",
            "Notice".cyan()
        );
        println!("  {} Run 'openclaw skills install {}' manually after installing OpenClaw.", "→".dimmed(), skill_dir.display());
    }
}

/// Unregister an OpenClaw skill via CLI before files are removed.
///
/// Strategy mirrors `openclaw_post_install_skill`:
/// 1. Try `openclaw skills uninstall` if CLI is available.
/// 2. Fallback: remove skill directory from well-known global locations.
fn openclaw_post_uninstall_skill(skill_dir: &std::path::Path) {
    use std::process::Command;

    if is_command_available("openclaw") {
        match Command::new("openclaw")
            .args(["skills", "uninstall", &skill_dir.to_string_lossy()])
            .output()
        {
            Ok(output) if output.status.success() => {
                println!("  {} Unregistered skill via 'openclaw skills uninstall'", "✓".green());
                return;
            }
            _ => {
                // Fall through to direct-remove fallback
            }
        }
    }

    // Fallback: remove from global skills directory
    let skill_name = match skill_dir.file_name() {
        Some(name) => name,
        None => return,
    };
    if let Some(global_skills) = resolve_openclaw_global_skills_dir() {
        let target = global_skills.join(skill_name);
        if target.is_dir() {
            if let Err(e) = std::fs::remove_dir_all(&target) {
                println!(
                    "  {} Failed to remove skill dir {}: {}",
                    "Warning".yellow(), target.display(), e
                );
            } else {
                println!(
                    "  {} Removed skill from {} (direct removal fallback)",
                    "✓".green(), target.display()
                );
            }
        }
    }
}

/// Install a single agent skill for a given provider and event.
fn install_agent_skill(
    base: &std::path::Path,
    provider: &AgentProvider,
    global: bool,
    event: &HookEvent,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> Result<(), String> {
    // ── Tier-1/2 override check ──────────────────────────────────────────
    // Agent plugins bundle skill + command + memory; check before built-in generation.
    // Tier-1 (project-local fixed paths) is skipped for global installs; tier-2
    // (TOML agent-plugins config) applies to both local and global installs.
    match resolve_and_install_agent_plugin_override(base, provider, event, global, skill_names) {
        Ok(true) => {
            // Override installed skill/command/memory/hooks; stop hook is handled
            // inside install_agent_plugin_from_dir if hooks/hooks.json exists.
            return Ok(());
        }
        Ok(false) => {}            // fall through to built-in
        Err(e) => return Err(e),   // hard error; abort
    }
    // ── End override check ───────────────────────────────────────────────

    let skill_path = agent_skill_path(base, provider, global, event, skill_names);
    let content = agent_event_content_for_provider(provider, event, skill_names);

    match provider {
        AgentProvider::Codex => {
            // Section-based: append/update section keyed by event
            let section_marker = agent_event_section_marker(event);
            install_agent_append_section(&skill_path, &content, section_marker, "# Agent Instructions\n")?;
        }
        AgentProvider::Claude | AgentProvider::Codebuddy | AgentProvider::Openclaw => {
            // Skills subdirectory: write dedicated file
            install_agent_dedicated_file(&skill_path, &content)?;
            // Stop hook: install if pre-commit event
            if matches!(event, HookEvent::PreCommit) {
                if let Some(settings_path) = agent_stop_hook_settings_path(base, provider) {
                    install_agent_stop_hook(base, provider, &settings_path)?;
                }
            }
            // OpenClaw post-install: register skill via CLI
            if matches!(provider, AgentProvider::Openclaw) {
                if let Some(skill_dir) = skill_path.parent() {
                    openclaw_post_install_skill(skill_dir);
                }
            }
        }
        _ => {
            install_agent_dedicated_file(&skill_path, &content)?;
        }
    }
    Ok(())
}

/// Uninstall a single agent skill for a given provider and event.
fn uninstall_agent_skill(
    base: &std::path::Path,
    provider: &AgentProvider,
    global: bool,
    event: &HookEvent,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> Result<(), String> {
    let skill_path = agent_skill_path(base, provider, global, event, skill_names);

    match provider {
        AgentProvider::Codex => {
            if skill_path.exists() {
                let section_marker = agent_event_section_marker(event);
                remove_agent_section_by_marker(&skill_path, section_marker)?;
            }
        }
        AgentProvider::Claude | AgentProvider::Codebuddy | AgentProvider::Openclaw => {
            if skill_path.exists() {
                // OpenClaw: attempt CLI unregister before removing files
                if matches!(provider, AgentProvider::Openclaw) {
                    if let Some(skill_dir) = skill_path.parent() {
                        openclaw_post_uninstall_skill(skill_dir);
                    }
                }
                remove_agent_dedicated_file(&skill_path)?;
            }
            // Stop hook: remove if pre-commit event uninstalled
            if matches!(event, HookEvent::PreCommit) {
                if let Some(settings_path) = agent_stop_hook_settings_path(base, provider) {
                    if settings_path.exists() {
                        remove_agent_stop_hook(&settings_path)?;
                    }
                }
            }
        }
        _ => {
            if skill_path.exists() {
                remove_agent_dedicated_file(&skill_path)?;
            }
        }
    }
    Ok(())
}

/// Format skill content for a provider + event, wrapping in provider-specific frontmatter.
fn agent_event_content_for_provider(
    provider: &AgentProvider,
    event: &HookEvent,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> String {
    let body = agent_event_content_generic(event);
    let (name, desc) = agent_event_skill_metadata(event, skill_names);
    match provider {
        AgentProvider::Codex => body, // section body only; marker handled separately
        AgentProvider::Claude | AgentProvider::Codebuddy | AgentProvider::Openclaw => {
            // Skills file with frontmatter (required for skill discovery)
            format!("---\nname: {}\ndescription: {}\n---\n\n# {}\n\n{}\n", name, desc, name, body)
        }
        AgentProvider::Gemini | AgentProvider::Droid | AgentProvider::Auggie => {
            format!("---\nname: {}\ndescription: {}\n---\n\n# {}\n\n{}\n", name, desc, name, body)
        }
        AgentProvider::Cursor => {
            format!("---\ndescription: {}\nalwaysApply: true\n---\n\n# {}\n\n{}\n", desc, name, body)
        }
    }
}

fn agent_event_skill_metadata(
    event: &HookEvent,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> (String, &'static str) {
    // Check for a configured skill name override for this event
    let custom_name: Option<&str> = skill_names.and_then(|sn| match event {
        HookEvent::PreCommit => sn.pre_commit.as_deref(),
        HookEvent::CommitMsg => sn.commit_msg.as_deref(),
        HookEvent::PrePush   => sn.pre_push.as_deref(),
    });
    match event {
        HookEvent::PreCommit => (
            custom_name.unwrap_or("lt-lint").to_string(),
            "对暂存/修改的代码文件运行 linthis 代码检查，提交前修复所有问题。使用 `linthis -i <file>` 按项目编码规范检查，必须手动编辑修复（不能用 linthis --fix）。提交时若 lt-cmsg skill 也存在应一起触发。Run linthis lint checks on staged/modified code files and fix all issues before committing. Uses `linthis -i <file>`. Issues must be fixed by editing code directly. If the lt-cmsg skill also exists, both should be invoked together when committing.",
        ),
        HookEvent::CommitMsg => (
            custom_name.unwrap_or("lt-cmsg").to_string(),
            "验证并自动修复 git 提交信息，使其符合 Conventional Commits 规范。分析暂存区 diff 选择正确的 type 前缀（feat/fix/refactor 等），检查标题格式，自动改写不合规的提交信息。提交时若 lt-lint skill 也存在应一起触发。Validate and auto-fix git commit messages to comply with Conventional Commits. Analyzes staged diff to select correct type prefix, checks format, auto-rewrites malformed messages. If the lt-lint skill also exists, both should be invoked together when committing.",
        ),
        HookEvent::PrePush => (
            custom_name.unwrap_or("lt-review").to_string(),
            "推送前审查待推送的提交，检查代码质量、安全性和正确性问题。检查完整 diff 发现逻辑错误、安全漏洞（注入、硬编码密钥）、代码质量问题及测试覆盖缺失。由 pre-push hook 触发。Review outgoing commits for quality, security, and correctness before pushing. Catches logic errors, security vulnerabilities, code quality issues. Triggered by pre-push hook.",
        ),
    }
}

fn agent_event_section_marker(event: &HookEvent) -> &'static str {
    match event {
        HookEvent::PreCommit => "## Linthis Lint Rule",
        HookEvent::CommitMsg => "## Linthis Commit Message Rule",
        HookEvent::PrePush   => "## Linthis Review Rule",
    }
}

/// Append or replace a section (identified by marker) in a file.
fn install_agent_append_section(
    path: &std::path::Path,
    content: &str,
    section_marker: &str,
    file_header: &str,
) -> Result<(), String> {
    use std::fs;
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("create dir: {}", e))?;
        }
    }
    let section = format!("\n{}\n\n{}\n", section_marker, content);
    if path.exists() {
        let existing = fs::read_to_string(path)
            .map_err(|e| format!("read {}: {}", path.display(), e))?;
        if existing.contains(section_marker) {
            // Replace existing section
            let start = existing.find(section_marker).unwrap();
            // Find next ## heading after this section, or end of file
            let after = &existing[start + section_marker.len()..];
            let end = after.find("\n## ")
                .map(|i| start + section_marker.len() + i)
                .unwrap_or(existing.len());
            let updated = format!("{}{}{}", &existing[..start], &section[1..], &existing[end..]);
            fs::write(path, updated).map_err(|e| format!("write {}: {}", path.display(), e))?;
        } else {
            // Append
            let mut f = std::fs::OpenOptions::new().append(true).open(path)
                .map_err(|e| format!("open {}: {}", path.display(), e))?;
            use std::io::Write;
            f.write_all(section.as_bytes())
                .map_err(|e| format!("write {}: {}", path.display(), e))?;
        }
    } else {
        fs::write(path, format!("{}{}", file_header, section))
            .map_err(|e| format!("write {}: {}", path.display(), e))?;
    }
    Ok(())
}

/// Remove a specific section (by marker) from a file.
fn remove_agent_section_by_marker(path: &std::path::Path, marker: &str) -> Result<(), String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("read {}: {}", path.display(), e))?;
    if !content.contains(marker) {
        return Ok(());
    }
    let start = content.find(marker).unwrap();
    let after = &content[start + marker.len()..];
    let end = after.find("\n## ")
        .map(|i| start + marker.len() + i)
        .unwrap_or(content.len());
    // Also trim leading newline before section marker
    let trim_start = if start > 0 && content.as_bytes()[start - 1] == b'\n' {
        start - 1
    } else {
        start
    };
    let updated = format!("{}{}", &content[..trim_start], &content[end..]);
    if updated.trim().is_empty() {
        std::fs::remove_file(path).map_err(|e| format!("remove {}: {}", path.display(), e))?;
    } else {
        std::fs::write(path, updated).map_err(|e| format!("write {}: {}", path.display(), e))?;
    }
    Ok(())
}

/// Remove a dedicated skill file and clean up empty parent directories
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

/// Remove the linthis section from a file (CLAUDE.md, AGENTS.md)
/// Handles both current and legacy section markers.
fn remove_agent_section_from_file(path: &std::path::Path) -> Result<(), String> {
    use std::fs;

    let existing = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    // Try current marker first, then legacy
    let (start, marker_len) = if let Some(s) = existing.find(AGENT_SECTION_MARKER) {
        (s, AGENT_SECTION_MARKER.len())
    } else if let Some(s) = existing.find(AGENT_SECTION_MARKER_LEGACY) {
        (s, AGENT_SECTION_MARKER_LEGACY.len())
    } else {
        return Ok(());
    };

    let after_marker = &existing[start + marker_len..];
    let section_end = after_marker
        .find("\n## ")
        .map(|pos| start + marker_len + pos)
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

    Ok(())
}

/// Shallow-merge a stop hook JSON string into a settings file.
///
/// Shared logic for both override-from-plugin and built-in stop hook installation.
fn install_agent_stop_hook_from_json(
    _git_root: &std::path::Path,
    settings_path: &std::path::Path,
    override_json: &str,
) -> Result<(), String> {
    use std::fs;

    if let Some(parent) = settings_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
        }
    }

    // ── Shallow-merge override JSON into existing settings file ───────────
    if settings_path.exists() {
        let existing = fs::read_to_string(settings_path)
            .map_err(|e| format!("Failed to read {}: {}", settings_path.display(), e))?;

        let mut json: serde_json::Value = serde_json::from_str(&existing)
            .map_err(|e| format!("Failed to parse {}: {}", settings_path.display(), e))?;

        let override_val: serde_json::Value = serde_json::from_str(override_json)
            .map_err(|e| format!("Failed to parse stop hook JSON: {}", e))?;

        // Shallow merge: each top-level key from the override replaces the existing key entirely.
        if let (Some(root), Some(override_obj)) = (json.as_object_mut(), override_val.as_object()) {
            for (k, v) in override_obj {
                root.insert(k.clone(), v.clone());
            }
        }

        let output = serde_json::to_string_pretty(&json)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
        fs::write(settings_path, output + "\n")
            .map_err(|e| format!("Failed to write {}: {}", settings_path.display(), e))?;
    } else {
        fs::write(settings_path, override_json.to_string() + "\n")
            .map_err(|e| format!("Failed to write {}: {}", settings_path.display(), e))?;
    }

    Ok(())
}

/// Install the Stop Hook into a settings JSON file using TOML config or built-in fallback.
///
/// Called from the built-in Tier-3 code path (when no plugin override provided hooks.json).
/// Plugin overrides install stop hooks directly via `install_agent_plugin_from_dir`.
fn install_agent_stop_hook(
    git_root: &std::path::Path,
    _provider: &AgentProvider,
    settings_path: &std::path::Path,
) -> Result<(), String> {
    // Stop hook is now handled via plugin packages; use built-in only.
    let override_json_str: Option<String> = None;

    let override_json = override_json_str.as_deref().unwrap_or_else(|| agent_stop_hook_json_ref());
    install_agent_stop_hook_from_json(git_root, settings_path, override_json)
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
/// When `global` is true, skills are installed into the user home directory
/// (`~/.claude/CLAUDE.md`, `~/.cursor/rules/linthis.mdc`, etc.) without
/// requiring a git repository.  When false, skills are installed in the
/// project git root (project-level).
fn warn_legacy_if_present(base: &std::path::Path, provider: &AgentProvider) {
    match provider {
        AgentProvider::Claude => {
            let legacy = base.join("CLAUDE.md");
            if legacy.exists()
                && std::fs::read_to_string(&legacy)
                    .map(|c| c.contains("## Linthis"))
                    .unwrap_or(false)
            {
                println!(
                    "{}: Legacy linthis section detected in {} — you may remove it manually.",
                    "Notice".cyan(),
                    legacy.display()
                );
            }
        }
        AgentProvider::Codebuddy => {
            let legacy_md = base.join("CODEBUDDY.md");
            let legacy_skill = base.join(".codebuddy/skills/linthis/SKILL.md");
            if (legacy_md.exists()
                && std::fs::read_to_string(&legacy_md)
                    .map(|c| c.contains("## Linthis"))
                    .unwrap_or(false))
                || legacy_skill.exists()
            {
                println!(
                    "{}: Legacy linthis files detected (CODEBUDDY.md section / SKILL.md) — you may remove them manually.",
                    "Notice".cyan()
                );
            }
        }
        _ => {}
    }
}

fn uninstall_agent_legacy(base: &std::path::Path, provider: &AgentProvider) {
    match provider {
        AgentProvider::Claude => {
            let legacy = base.join("CLAUDE.md");
            if legacy.exists() {
                let _ = remove_agent_section_from_file(&legacy);
            }
        }
        AgentProvider::Codebuddy => {
            let legacy_md = base.join("CODEBUDDY.md");
            if legacy_md.exists() {
                let _ = remove_agent_section_from_file(&legacy_md);
            }
            let legacy_skill = base.join(".codebuddy/skills/linthis/SKILL.md");
            if legacy_skill.exists() {
                let _ = remove_agent_dedicated_file(&legacy_skill);
            }
        }
        _ => {}
    }
}

/// Resolve the base directory for agent hook installation (home for global, git root for local).
fn resolve_agent_base(global: bool) -> Result<PathBuf, ExitCode> {
    if global {
        dirs::home_dir().ok_or_else(|| {
            eprintln!("{}: Could not determine home directory", "Error".red());
            ExitCode::from(1)
        })
    } else {
        find_git_root().ok_or_else(|| {
            eprintln!("{}: Not in a git repository", "Error".red());
            eprintln!("  Run this command from within a git repository, or use --global / -g to install user-level skills");
            ExitCode::from(1)
        })
    }
}

/// Install agent skills for a list of providers across all events. Returns true if any succeeded.
fn install_agent_providers_batch(
    providers: &[&AgentProvider],
    base: &std::path::Path,
    events: &[HookEvent],
    force: bool,
    global: bool,
    scope: &str,
    project_str: &str,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> bool {
    let mut any_installed = false;
    for p in providers {
        if agent_is_installed(base, p, global, skill_names) && !force {
            println!("{}: {} already installed", "Info".cyan(), p);
            print_agent_installed_info(base, p, global, skill_names);
            continue;
        }
        warn_legacy_if_present(base, p);
        let provider_name = format!("{}", p).to_lowercase();
        let mut provider_ok = true;
        for event in events {
            match install_agent_skill(base, p, global, event, skill_names) {
                Ok(_) => {
                    let path = agent_skill_path(base, p, global, event, skill_names);
                    println!("{} Installed {} ({}) → {}", "✓".green(), p, event.hook_filename(), path.display());
                    add_skill_provider_to_hook(scope, project_str, event, &provider_name);
                }
                Err(e) => {
                    eprintln!("{}: Failed to install {} ({}): {}", "Error".red(), p, event.hook_filename(), e);
                    provider_ok = false;
                }
            }
        }
        if provider_ok {
            print_extra_installed(base, p);
            any_installed = true;
        }
    }
    any_installed
}

/// Build an ordered list of providers: detected/installed first, then others.
fn build_ordered_provider_list<'a>(
    base: &std::path::Path,
    detected: &[AgentProvider],
    global: bool,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> Vec<&'a AgentProvider> {
    let mut ordered: Vec<&AgentProvider> = Vec::new();
    for p in ALL_AGENT_PROVIDERS {
        if detected.iter().any(|d| std::mem::discriminant(d) == std::mem::discriminant(p))
            || agent_is_installed(base, p, global, skill_names)
        {
            ordered.push(p);
        }
    }
    for p in ALL_AGENT_PROVIDERS {
        if !ordered.iter().any(|o| std::mem::discriminant(*o) == std::mem::discriminant(p)) {
            ordered.push(p);
        }
    }
    ordered
}

/// Prompt user to select agents from an interactive menu.
/// Returns None if cancelled.
fn prompt_agent_selection<'a>(
    ordered: &[&'a AgentProvider],
    detected: &'a [AgentProvider],
    base: &std::path::Path,
    global: bool,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> Option<Vec<&'a AgentProvider>> {
    use std::io::{self, Write};

    let provider_count = ordered.len();
    println!("Select agent(s) to integrate with linthis:");
    println!();

    for (i, p) in ordered.iter().enumerate() {
        let is_installed = agent_is_installed(base, p, global, skill_names);
        let is_detected = detected.iter().any(|d| std::mem::discriminant(d) == std::mem::discriminant(p));
        let status = match (is_installed, is_detected) {
            (true, _) => format!(" {}", "(installed)".yellow()),
            (false, true) => format!(" {}", "(detected)".cyan()),
            _ => String::new(),
        };
        println!("  {}. {}{}", i + 1, p, status);
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

    if choice == (provider_count + 3).to_string() || choice.is_empty() {
        return None;
    }

    if choice == (provider_count + 1).to_string() {
        if detected.is_empty() {
            println!("{}: No agents detected, installing all", "Info".cyan());
            return Some(ordered.to_vec());
        }
        return Some(detected.iter().collect());
    }
    if choice == (provider_count + 2).to_string() {
        return Some(ordered.to_vec());
    }

    let selected: Vec<&AgentProvider> = choice.split(',')
        .filter_map(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| n >= 1 && n <= provider_count)
        .map(|n| ordered[n - 1])
        .collect();

    if selected.is_empty() { None } else { Some(selected) }
}

fn handle_agent_hook_install(
    provider: Option<AgentProvider>,
    events: &[HookEvent],
    force: bool,
    yes: bool,
    global: bool,
) -> ExitCode {
    let base = match resolve_agent_base(global) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let scope = if global { "global" } else { "local" };
    let project_str = if global { String::new() } else { base.to_str().unwrap_or("").to_string() };

    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let skill_names_cfg = linthis::config::Config::load_merged(&project_root).hook.agent.skill_names;
    let skill_names = Some(&skill_names_cfg);

    println!("{}", "🤖 AI Coding Agent Integration".bold());
    if global {
        println!("  {} Installing user-level skills in {}", "→".dimmed(), base.display());
    }
    println!();

    // Single provider specified
    if let Some(ref p) = provider {
        if agent_is_installed(&base, p, global, skill_names) && !force {
            println!("{}: {} is already installed", "Info".cyan(), p);
            print_agent_installed_info(&base, p, global, skill_names);
            return ExitCode::SUCCESS;
        }
        let providers = vec![p];
        install_agent_providers_batch(&providers, &base, events, force, global, scope, &project_str, skill_names);
        return ExitCode::SUCCESS;
    }

    // Auto-detect with -y
    if yes {
        let detected = detect_agent_providers(&base);
        let targets: Vec<&AgentProvider> = if detected.is_empty() {
            ALL_AGENT_PROVIDERS.iter().collect()
        } else {
            detected.iter().collect()
        };
        let any = install_agent_providers_batch(&targets, &base, events, force, global, scope, &project_str, skill_names);
        if any {
            println!();
            println!("{}", "Agents will auto-check code quality after edits.".bold());
        }
        return ExitCode::SUCCESS;
    }

    // Interactive menu
    let detected = detect_agent_providers(&base);
    let ordered = build_ordered_provider_list(&base, &detected, global, skill_names);

    let selected = match prompt_agent_selection(&ordered, &detected, &base, global, skill_names) {
        Some(s) => s,
        None => {
            println!("Installation cancelled");
            return ExitCode::SUCCESS;
        }
    };

    println!();
    let any = install_agent_providers_batch(&selected, &base, events, force, global, scope, &project_str, skill_names);
    if any {
        println!();
        println!("{}", "Agents will auto-check code quality after edits.".bold());
    }
    ExitCode::SUCCESS
}

/// Uninstall agent hooks for all installed providers.
fn handle_agent_hook_uninstall(yes: bool, global: bool, events: &[HookEvent]) -> ExitCode {
    use std::io::{self, Write};

    let base = match resolve_agent_base(global) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let skill_names_cfg = linthis::config::Config::load_merged(&project_root).hook.agent.skill_names;
    let skill_names = Some(&skill_names_cfg);

    let installed: Vec<&AgentProvider> = ALL_AGENT_PROVIDERS
        .iter()
        .filter(|p| agent_is_installed(&base, p, global, skill_names))
        .collect();

    if installed.is_empty() {
        return ExitCode::from(1);
    }

    if !yes {
        println!("{}", "Agent Integration:".bold());
        for p in &installed {
            let path = agent_skill_path(&base, p, global, &HookEvent::PreCommit, skill_names);
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

    let scope = if global { "global" } else { "local" };
    let project_str = if global { String::new() } else { base.to_str().unwrap_or("").to_string() };

    let mut any_removed = false;
    for p in &installed {
        let provider_name = format!("{}", p).to_lowercase();
        let ok = uninstall_provider_events(&base, p, events, global, skill_names, scope, &project_str, &provider_name);
        if ok {
            uninstall_agent_legacy(&base, p);
            any_removed = true;
        }
    }

    if any_removed { ExitCode::SUCCESS } else { ExitCode::from(1) }
}

/// Uninstall all event skills for a single provider. Returns true if all succeeded.
fn uninstall_provider_events(
    base: &std::path::Path,
    provider: &AgentProvider,
    events: &[HookEvent],
    global: bool,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
    scope: &str,
    project_str: &str,
    provider_name: &str,
) -> bool {
    let mut ok = true;
    for event in events {
        match uninstall_agent_skill(base, provider, global, event, skill_names) {
            Ok(_) => {
                println!("{} Uninstalled {} ({}) skill", "✓".green(), provider, event.hook_filename());
                remove_skill_provider_from_hook(scope, project_str, event, provider_name);
            }
            Err(e) => {
                eprintln!("{}: Failed to uninstall {} ({}): {}", "Error".red(), provider, event.hook_filename(), e);
                ok = false;
            }
        }
    }
    ok
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
/// Read the commit message from a file path or treat the string as the message itself.
fn read_commit_msg(msg_or_file: &str) -> Result<(String, bool), ExitCode> {
    let path = std::path::Path::new(msg_or_file);
    if path.exists() {
        match std::fs::read_to_string(path) {
            Ok(content) => Ok((content, true)),
            Err(e) => {
                eprintln!("{}: Failed to read commit message file: {}", "Error".red(), e);
                Err(ExitCode::from(1))
            }
        }
    } else {
        Ok((msg_or_file.to_string(), false))
    }
}

/// Validate a commit message against the configured pattern and ticket requirement.
/// Returns a list of validation errors (empty if valid).
fn validate_commit_msg(
    first_line: &str,
    config: &linthis::config::Config,
) -> Result<Vec<String>, ExitCode> {
    use regex::Regex;
    let mut errors = Vec::new();

    let regex = match Regex::new(&config.cmsg.commit_msg_pattern) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}: Invalid commit message pattern in config: {}", "Error".red(), e);
            return Err(ExitCode::from(2));
        }
    };

    if !regex.is_match(first_line) {
        errors.push(
            "Does not match Conventional Commits format (type(scope)?: description). Valid types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert".to_string()
        );
    }

    if config.cmsg.require_ticket {
        let ticket_pattern = config.cmsg.ticket_pattern.as_deref().unwrap_or(r"\[\w+-\d+\]");
        let ticket_regex = match Regex::new(ticket_pattern) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{}: Invalid ticket pattern in config: {}", "Error".red(), e);
                return Err(ExitCode::from(2));
            }
        };
        if !ticket_regex.is_match(first_line) {
            errors.push(format!(
                "Missing ticket reference (pattern: {}). Example: feat: [PROJ-123] add feature",
                ticket_pattern
            ));
        }
    }

    Ok(errors)
}

/// Print the ticket reference required error box.
fn print_ticket_error(first_line: &str, ticket_pattern: &str) {
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
}

pub fn handle_commit_msg_check(msg_or_file: &str, auto_fix: bool, provider: Option<&str>) -> ExitCode {
    use linthis::config::Config;

    let project_root = linthis::utils::get_project_root();
    let config = Config::load_merged(&project_root);

    let (commit_msg, is_file) = match read_commit_msg(msg_or_file) {
        Ok(r) => r,
        Err(code) => return code,
    };

    let first_line = commit_msg.lines().next().unwrap_or("").trim();
    if first_line.is_empty() || first_line.starts_with('#') {
        return ExitCode::SUCCESS;
    }

    let errors = match validate_commit_msg(first_line, &config) {
        Ok(e) => e,
        Err(code) => return code,
    };

    if errors.is_empty() {
        println!("{}", linthis::utils::output::format_cmsg_result(true, ""));
        let paths = linthis::utils::output::format_hook_paths_footer_pub(Some("commit-msg"));
        if !paths.is_empty() {
            println!("{}", paths);
        }
        return ExitCode::SUCCESS;
    }

    if auto_fix {
        let path = std::path::Path::new(msg_or_file);
        return handle_cmsg_auto_fix(&commit_msg, &errors, is_file, path, provider, config.ai.provider.as_deref());
    }

    if errors.iter().any(|e| e.contains("Conventional Commits")) {
        print_commit_msg_error(first_line);
    } else {
        let ticket_pattern = config.cmsg.ticket_pattern.as_deref().unwrap_or(r"\[\w+-\d+\]");
        print_ticket_error(first_line, ticket_pattern);
    }
    ExitCode::from(1)
}

/// Resolve an AiProviderConfig from its kind.
fn ai_provider_config_from_kind(kind: &linthis::ai::AiProviderKind) -> linthis::ai::AiProviderConfig {
    use linthis::ai::{AiProviderConfig, AiProviderKind};
    match kind {
        AiProviderKind::Claude => AiProviderConfig::claude(),
        AiProviderKind::ClaudeCli => AiProviderConfig::claude_cli(),
        AiProviderKind::CodeBuddy => AiProviderConfig::codebuddy(),
        AiProviderKind::CodeBuddyCli => AiProviderConfig::codebuddy_cli(),
        AiProviderKind::OpenAi => AiProviderConfig::openai(),
        AiProviderKind::CodexCli => AiProviderConfig::codex_cli(),
        AiProviderKind::Gemini => AiProviderConfig::gemini(),
        AiProviderKind::GeminiCli => AiProviderConfig::gemini_cli(),
        AiProviderKind::Local => AiProviderConfig::local(),
        AiProviderKind::Custom(name) => AiProviderConfig {
            kind: AiProviderKind::Custom(name.clone()),
            ..AiProviderConfig::default()
        },
        AiProviderKind::Mock => AiProviderConfig::mock(),
    }
}

/// Handle AI auto-fix for commit messages
fn handle_cmsg_auto_fix(
    original_msg: &str,
    errors: &[String],
    is_file: bool,
    file_path: &std::path::Path,
    cli_provider: Option<&str>,
    config_provider: Option<&str>,
) -> ExitCode {
    use crate::cli::helpers::resolve_ai_provider;
    use linthis::ai::{AiProvider, AiProviderKind, AiProviderTrait};

    let provider_name = resolve_ai_provider(cli_provider, config_provider);
    let kind: AiProviderKind = match provider_name.parse() {
        Ok(k) => k,
        Err(_) => {
            eprintln!("{}: Unknown AI provider: {}", "Error".red(), provider_name);
            return ExitCode::from(2);
        }
    };

    let provider = AiProvider::new(ai_provider_config_from_kind(&kind));

    eprintln!("{} Rewriting commit message with AI (provider: {})...", "→".cyan(), provider_name.cyan());

    let first_line = original_msg.lines().next().unwrap_or("").trim();
    let rest_of_msg: String = original_msg.lines().skip(1).collect::<Vec<_>>().join("\n");
    let error_desc = errors.join("; ");

    let prompt = format!(
        "Rewrite the following git commit message to conform to the Conventional Commits format.\n\n\
         Original message: {}\n\nValidation errors: {}\n\n\
         Rules:\n- Format: type(scope)?: description\n\
         - Valid types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert\n\
         - Keep the original intent and meaning\n\
         - Output ONLY the rewritten first line, nothing else (no quotes, no explanation)",
        first_line, error_desc
    );

    match provider.complete(&prompt, Some("You are a git commit message formatter. Output only the corrected commit message first line.")) {
        Ok(fixed_line) => {
            let fixed_line = fixed_line.trim().trim_matches('"').trim_matches('\'').trim();
            let fixed_msg = if rest_of_msg.is_empty() {
                format!("{}\n", fixed_line)
            } else {
                format!("{}\n{}", fixed_line, rest_of_msg)
            };

            if is_file {
                if let Err(e) = std::fs::write(file_path, &fixed_msg) {
                    eprintln!("{}: Failed to write fixed message: {}", "Error".red(), e);
                    return ExitCode::from(1);
                }
                eprintln!("{} Commit message rewritten: {} → {}", "✓".green(), first_line.dimmed(), fixed_line.green());
            } else {
                eprintln!("{} Suggested rewrite: {}", "✓".green(), fixed_line.green());
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}: AI auto-fix failed: {}", "Error".red(), e);
            print_commit_msg_error(first_line);
            ExitCode::from(1)
        }
    }
}

/// Print commit message validation error
fn print_commit_msg_error(first_line: &str) {
    eprintln!("{}", linthis::utils::output::format_cmsg_result(false, first_line));
    let paths = linthis::utils::output::format_hook_paths_footer_pub(Some("commit-msg"));
    if !paths.is_empty() {
        eprintln!("{}", paths);
    }
}

// =============================================================================
// `linthis hook run` — execute full hook logic at runtime from thin wrapper
// =============================================================================

/// Execute hook logic at runtime, called by thin wrapper scripts in .git/hooks/.
///
/// Builds the full hook script content from the current binary and runs it via
/// `sh`, forwarding any passthrough arguments from the original git hook
/// invocation (e.g. the commit message file path for commit-msg hooks).
/// Environment variable injected by `handle_hook_run` to detect re-entrant hook calls.
///
/// When the global hook script delegates to a local thin wrapper via
/// `exec "$LOCAL_HOOK"`, the local wrapper inherits this variable through the
/// exec chain.  A second `handle_hook_run` invocation that finds this variable
/// already set knows it is inside a delegation and skips local-hook checking,
/// preventing infinite recursion when both hooks are thin wrappers.
///
/// Each linthis process sets LINTHIS_HOOK_RUNNING_<pid>=1 so concurrent commits
/// (each with their own PID) don't interfere with each other's re-entrancy detection.
const LINTHIS_HOOK_RUNNING_PREFIX: &str = "LINTHIS_HOOK_RUNNING_";

/// Parse a provider string into an AgentFixProvider.
fn parse_agent_fix_provider_name(name: &str) -> Option<AgentFixProvider> {
    match name.to_lowercase().as_str() {
        "claude"    => Some(AgentFixProvider::Claude),
        "codex"     => Some(AgentFixProvider::Codex),
        "gemini"    => Some(AgentFixProvider::Gemini),
        "cursor"    => Some(AgentFixProvider::Cursor),
        "droid"     => Some(AgentFixProvider::Droid),
        "auggie" | "aug" | "augment" => Some(AgentFixProvider::Auggie),
        "codebuddy" => Some(AgentFixProvider::Codebuddy),
        "openclaw"  => Some(AgentFixProvider::Openclaw),
        _ => None,
    }
}

/// Build the re-entrant (direct) script for a git hook.
fn build_reentrant_git_script(event: &HookEvent) -> String {
    let linthis_cmd = build_hook_command(event, &None);
    if matches!(event, HookEvent::PrePush) {
        format!(
            "#!/bin/sh\n\
             _BASE=$(git rev-parse '@{{u}}' 2>/dev/null || \\\n\
             \x20       git rev-parse 'HEAD~1' 2>/dev/null)\n\
             _PUSHED_FILES=$(git diff --name-only \"$_BASE\"..HEAD 2>/dev/null | grep -v '^$')\n\
             if [ -n \"$_PUSHED_FILES\" ]; then\n\
             \x20 set --\n\
             \x20 while IFS= read -r _F; do set -- \"$@\" -i \"$_F\"; done <<_EOF_\n\
             $_PUSHED_FILES\n\
             _EOF_\n\
             \x20 {linthis} \"$@\"\n\
             fi\n",
            linthis = linthis_cmd
        )
    } else {
        format!("#!/bin/sh\n{linthis_cmd} \"$@\"\n")
    }
}

fn handle_hook_run(
    event: &HookEvent,
    hook_type: &HookTool,
    raw_provider: Option<&str>,
    raw_provider_args: Option<&str>,
    _global: bool,
    hook_args: &[String],
) -> i32 {
    let (provider_name, merged_pa) = if let Some(raw) = raw_provider {
        let (name, model) = parse_provider_with_model(raw);
        (Some(name), merge_model_into_provider_args(model, raw_provider_args))
    } else {
        (None, raw_provider_args.map(|s| s.to_string()))
    };
    let provider: Option<&str> = provider_name;
    let provider_args: Option<&str> = merged_pa.as_deref();

    let already_running = std::env::vars()
        .any(|(k, _)| k.starts_with(LINTHIS_HOOK_RUNNING_PREFIX));

    let script = match hook_type {
        HookTool::Git => {
            if already_running {
                build_reentrant_git_script(event)
            } else {
                build_global_hook_script_for_event(event, &None, None)
            }
        }
        HookTool::GitWithAgent => {
            let fix_provider = provider
                .and_then(parse_agent_fix_provider_name)
                .unwrap_or(AgentFixProvider::Claude);
            let linthis_cmd = build_hook_command(event, &None);
            build_git_with_agent_hook_script(&linthis_cmd, &fix_provider, event, provider_args)
        }
        _ => {
            eprintln!("{}: hook run: unsupported hook type '{}' (supported: git, git-with-agent)", "Error".red(), hook_type.as_str());
            return 1;
        }
    };

    {
        let description = describe_hook_source(hook_type, event);
        eprintln!("{}", format!("📄 Config: {}", description).dimmed());
    }

    let pid = std::process::id().to_string();
    let env_key = format!("{}{}", LINTHIS_HOOK_RUNNING_PREFIX, pid);

    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(&script)
        .arg("--")
        .args(hook_args)
        .env(&env_key, "1")
        .status();

    match status {
        Ok(s) => s.code().unwrap_or(1),
        Err(e) => {
            eprintln!("{}: hook run: failed to execute script: {}", "Error".red(), e);
            1
        }
    }
}

// =============================================================================
// `linthis hook sync` — re-sync installed hooks from persisted metadata
// =============================================================================

/// Detect hook type from thin wrapper content.
fn detect_hook_type_from_thin_wrapper(content: &str) -> HookTool {
    if content.contains("--type git-with-agent") {
        HookTool::GitWithAgent
    } else if content.contains("--type agent") {
        HookTool::Agent
    } else if content.contains("--type prek-with-agent") {
        HookTool::PrekWithAgent
    } else if content.contains("--type prek") {
        HookTool::Prek
    } else {
        HookTool::Git
    }
}

/// Detect provider from old-format hook content using heuristics.
fn detect_provider_from_old_hook(content: &str) -> Option<&'static str> {
    if content.contains("codebuddy") {
        Some("codebuddy")
    } else if content.contains("codex") {
        Some("codex")
    } else if content.contains("gemini") {
        Some("gemini")
    } else if content.contains("cursor") {
        Some("cursor")
    } else if content.contains("claude") {
        Some("claude")
    } else {
        None
    }
}

/// Detect whether an old-format hook has agent-fix patterns.
fn old_hook_has_agent(content: &str) -> bool {
    content.contains("start_timer")
        || content.contains("AGENT_PROVIDER")
        || content.contains("claude")
        || content.contains("codebuddy")
        || content.contains("codex")
}

/// Record metadata for a thin wrapper that already exists on disk.
fn record_thin_wrapper_metadata(
    content: &str,
    name: &str,
    event: &HookEvent,
    global: bool,
    project: &str,
) {
    let provider_opt = content
        .split("--provider ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .map(|s| s.trim_end_matches('"').to_string());
    let hook_type = detect_hook_type_from_thin_wrapper(content);
    let scope = if global { "global" } else { "local" };
    save_installed_hook(scope, project, event, &hook_type, provider_opt.as_deref(), None);
    println!("  {} recorded thin wrapper {} {} ({})", "✓".green(), name, hook_type.as_str(), scope);
}

/// Migrate an old-format linthis hook to a thin wrapper.
fn migrate_old_hook(
    path: &std::path::Path,
    content: &str,
    name: &str,
    event: &HookEvent,
    global: bool,
    project: &str,
) -> bool {
    let hook_type = if old_hook_has_agent(content) { HookTool::GitWithAgent } else { HookTool::Git };
    let provider_opt = detect_provider_from_old_hook(content);
    let thin = build_thin_wrapper_script(event, &hook_type, provider_opt, global, None);

    if let Err(e) = std::fs::write(path, &thin) {
        eprintln!("  {} Failed to migrate {}: {}", "✗".red(), name, e);
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(path, perms);
        }
    }

    let scope = if global { "global" } else { "local" };
    save_installed_hook(scope, project, event, &hook_type, provider_opt, None);
    println!("  {} migrated {} → thin wrapper {} ({})", "✓".green(), name, hook_type.as_str(), scope);
    eprintln!(
        "  {} Hook type inferred from old script content (heuristic). \
         If incorrect, re-install with the right type:\n  \
         linthis hook install{} --event {} --type <type> --force",
        "⚠".yellow(),
        if global { " -g" } else { "" },
        event.as_str(),
    );
    true
}

/// Scan `hook_dir` for old-format linthis hook scripts, migrate each to a thin
/// wrapper, save metadata, and return the number of hooks migrated.
fn detect_and_migrate_existing_hooks(hook_dir: &std::path::Path, global: bool, project: &str) -> usize {
    let event_map: &[(&str, HookEvent)] = &[
        ("pre-commit", HookEvent::PreCommit),
        ("pre-push", HookEvent::PrePush),
        ("commit-msg", HookEvent::CommitMsg),
    ];

    let mut migrated = 0_usize;
    let entries = match std::fs::read_dir(hook_dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };

    for entry in entries.flatten() {
        let filename = entry.file_name();
        let name = match filename.to_str() {
            Some(n) => n,
            None => continue,
        };

        let event = match event_map.iter().find(|(n, _)| *n == name) {
            Some((_, e)) => e,
            None => continue,
        };

        let path = entry.path();
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let is_old_format = content.contains("# linthis-hook");
        let is_thin_wrapper = content.contains("linthis hook run");
        if !is_old_format && !is_thin_wrapper {
            continue;
        }

        if is_thin_wrapper {
            record_thin_wrapper_metadata(&content, name, event, global, project);
            migrated += 1;
            continue;
        }

        if migrate_old_hook(&path, &content, name, event, global, project) {
            migrated += 1;
        }
    }

    migrated
}

/// Parse event string back to HookEvent enum.
fn parse_hook_event(s: &str) -> Option<HookEvent> {
    match s {
        "pre-commit" => Some(HookEvent::PreCommit),
        "pre-push"   => Some(HookEvent::PrePush),
        "commit-msg" => Some(HookEvent::CommitMsg),
        _ => None,
    }
}

/// Parse hook type string back to HookTool enum.
fn parse_hook_tool(s: &str) -> Option<HookTool> {
    match s {
        "git"             => Some(HookTool::Git),
        "git-with-agent"  => Some(HookTool::GitWithAgent),
        "agent"           => Some(HookTool::Agent),
        "prek"            => Some(HookTool::Prek),
        "prek-with-agent" => Some(HookTool::PrekWithAgent),
        _ => None,
    }
}

/// Parse a provider name string to AgentProvider. Used by sync to resolve skill_providers.
fn parse_sync_agent_provider(name: &str) -> Option<AgentProvider> {
    match name.to_lowercase().as_str() {
        "claude"    => Some(AgentProvider::Claude),
        "codex"     => Some(AgentProvider::Codex),
        "gemini"    => Some(AgentProvider::Gemini),
        "cursor"    => Some(AgentProvider::Cursor),
        "droid"     => Some(AgentProvider::Droid),
        "auggie" | "aug" | "augment" => Some(AgentProvider::Auggie),
        "codebuddy" => Some(AgentProvider::Codebuddy),
        "openclaw"  => Some(AgentProvider::Openclaw),
        _ => None,
    }
}

/// Group filtered hooks by hook_type for structured output.
fn group_hooks_by_type<'a>(filtered: &'a [&'a InstalledHook]) -> Vec<(&'a str, Vec<&'a &'a InstalledHook>)> {
    let type_order = ["agent", "git-with-agent", "prek-with-agent", "git", "prek"];
    let mut grouped: Vec<(&str, Vec<&&InstalledHook>)> = Vec::new();
    for ht in &type_order {
        let group: Vec<&&InstalledHook> = filtered.iter().filter(|h| h.hook_type == *ht).collect();
        if !group.is_empty() {
            grouped.push((ht, group));
        }
    }
    for hook in filtered {
        if !type_order.contains(&hook.hook_type.as_str()) {
            let existing = grouped.iter().any(|(ht, _)| *ht == hook.hook_type.as_str());
            if !existing {
                let group: Vec<&&InstalledHook> = filtered.iter().filter(|h| h.hook_type == hook.hook_type).collect();
                grouped.push((hook.hook_type.as_str(), group));
            }
        }
    }
    grouped
}

/// Re-write a thin wrapper hook script for a single hook entry. Returns Ok(()) or increments errors.
fn sync_thin_wrapper(
    hook: &InstalledHook,
    event: &HookEvent,
    hook_type: &HookTool,
    provider_opt: Option<&str>,
    global: bool,
    project_root: &std::path::Path,
) -> Result<(), ()> {
    if matches!(hook_type, HookTool::Agent | HookTool::Prek) {
        return Ok(());
    }

    let hook_dir = if global {
        match global_hooks_dir() {
            Some(d) => d,
            None => {
                eprintln!("  {} Could not determine global hooks directory", "✗".red());
                return Err(());
            }
        }
    } else {
        project_root.join(".git/hooks")
    };

    let hook_file = hook_dir.join(event.hook_filename());
    let pa_opt: Option<&str> = if hook.provider_args.is_empty() { None } else { Some(&hook.provider_args) };
    let thin_script = build_thin_wrapper_script(event, hook_type, provider_opt, global, pa_opt);
    if let Some(parent) = hook_file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(code) = write_hook_script(&hook_file, &thin_script) {
        let _ = code;
        eprintln!("  {} Failed to write {}", "✗".red(), hook_file.display());
        return Err(());
    }
    Ok(())
}

/// Re-sync agent skills for a single "agent" type hook entry.
fn sync_agent_skills(
    hook: &InstalledHook,
    event: &HookEvent,
    provider_opt: Option<&str>,
    global: bool,
    project_root: &std::path::Path,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> u32 {
    let base = if global { dirs::home_dir().unwrap_or_default() } else { project_root.to_path_buf() };

    let mut skill_targets: Vec<AgentProvider> = hook.skill_providers.iter()
        .filter_map(|name| parse_sync_agent_provider(name))
        .collect();

    // Backward compatibility: if no skill_providers, fall back to fix provider
    if skill_targets.is_empty() {
        if let Some(fb) = provider_opt.and_then(parse_sync_agent_provider) {
            skill_targets.push(fb);
        }
    }

    let mut errors = 0_u32;
    for provider in &skill_targets {
        let skill_path = agent_skill_path(&base, provider, global, event, skill_names);
        if let Err(e) = install_agent_skill(&base, provider, global, event, skill_names) {
            eprintln!("     {} agent sync error ({}): {}", "✗".red(), provider, e);
            errors += 1;
            continue;
        }
        println!("     {} {} skill → {}", "↳".dimmed(), provider, skill_path.display());
        if let Some(cmd_dir) = agent_command_dir(&base, provider) {
            if cmd_dir.exists() {
                println!("     {} {} command → {}", "↳".dimmed(), provider, cmd_dir.display());
            }
        }
        if matches!(event, HookEvent::PreCommit) {
            if let Some(settings_path) = agent_stop_hook_settings_path(&base, provider) {
                if settings_path.exists() {
                    println!("     {} {} stop hook → {}", "↳".dimmed(), provider, settings_path.display());
                }
            }
        }
    }
    errors
}

/// Disk-scan pass: refresh skills for providers that exist on disk but aren't in the TOML.
fn sync_disk_scan_pass(
    base: &std::path::Path,
    global: bool,
    filtered: &[&InstalledHook],
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) {
    let all_scan_providers = [
        AgentProvider::Claude, AgentProvider::Codebuddy, AgentProvider::Gemini,
        AgentProvider::Cursor, AgentProvider::Droid, AgentProvider::Auggie,
    ];
    let all_scan_events = [HookEvent::PreCommit, HookEvent::CommitMsg, HookEvent::PrePush];
    for scan_event in &all_scan_events {
        for scan_provider in &all_scan_providers {
            let skill_path = agent_skill_path(base, scan_provider, global, scan_event, skill_names);
            if !skill_path.exists() {
                continue;
            }
            let provider_name_lower = format!("{}", scan_provider).to_lowercase();
            let already_registered = filtered.iter().any(|h| {
                h.event == scan_event.as_str()
                    && matches!(h.hook_type.as_str(), "git-with-agent" | "agent" | "prek-with-agent")
                    && h.skill_providers.iter().any(|sp| sp.to_lowercase() == provider_name_lower)
            });
            if already_registered {
                continue;
            }
            if let Err(e) = install_agent_skill(base, scan_provider, global, scan_event, skill_names) {
                eprintln!("  {} skill refresh error ({:?}/{}): {}", "⚠".yellow(), scan_provider, scan_event.as_str(), e);
            }
        }
    }
}

/// Handle empty metadata case: auto-detect and migrate existing hooks.
fn handle_sync_no_metadata(global: bool, project_root: &std::path::Path) -> i32 {
    let hook_dir = if global {
        match global_hooks_dir() {
            Some(d) => d,
            None => {
                eprintln!("{}: Could not determine global hooks directory", "Error".red());
                return 1;
            }
        }
    } else {
        project_root.join(".git/hooks")
    };

    let detected = detect_and_migrate_existing_hooks(
        &hook_dir, global,
        if global { "" } else { project_root.to_str().unwrap_or("") },
    );

    if detected == 0 {
        if global {
            println!("No global linthis hooks found to sync.");
            println!("  Run {} to install global hooks", "linthis hook install -g".cyan());
        } else {
            println!("No local linthis hooks found for this project.");
            println!("  Run {} to install and record hooks", "linthis hook install".cyan());
            println!("  Use {} to sync global hooks.", "linthis hook sync -g".cyan());
        }
    }
    0
}

/// Sync a single hook entry: re-write thin wrapper and optionally re-sync agent skills.
/// Returns the number of errors encountered.
fn sync_single_hook(
    hook: &InstalledHook,
    hook_index: &mut usize,
    target_scope: &str,
    global: bool,
    project_root: &std::path::Path,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> u32 {
    let mut errors = 0_u32;

    let event = match parse_hook_event(&hook.event) {
        Some(e) => e,
        None => { eprintln!("  {} Unknown event '{}', skipping", "✗".red(), hook.event); return 1; }
    };
    let hook_type = match parse_hook_tool(&hook.hook_type) {
        Some(t) => t,
        None => { eprintln!("  {} Unknown hook type '{}', skipping", "✗".red(), hook.hook_type); return 1; }
    };
    let prov_str: &str = &hook.provider;
    let provider_opt: Option<&str> = if prov_str.is_empty() { None } else { Some(prov_str) };

    if sync_thin_wrapper(hook, &event, &hook_type, provider_opt, global, project_root).is_err() {
        return 1;
    }

    *hook_index += 1;
    let mut details = vec![target_scope.to_string()];
    if let Some(fp) = provider_opt {
        if !fp.is_empty() { details.push(format!("fix: {}", fp)); }
    }
    if !hook.skill_providers.is_empty() {
        details.push(format!("skills: {}", hook.skill_providers.join(",")));
    }
    println!("  {}. {} synced {} {} ({})", hook_index, "✓".green(), hook.event, hook.hook_type, details.join(", "));

    if matches!(hook_type, HookTool::Agent) {
        errors += sync_agent_skills(hook, &event, provider_opt, global, project_root, skill_names);
    }

    errors
}

/// Re-sync installed hooks for local project or global scope.
pub fn handle_hook_sync(global: bool, _yes: bool) -> i32 {
    let hooks_file = load_installed_hooks();
    let target_scope = if global { "global" } else { "local" };

    let project_root: PathBuf = if !global {
        match find_git_root() {
            Some(r) => r,
            None => {
                eprintln!("{}: Not in a git repository", "Error".red());
                return 1;
            }
        }
    } else {
        PathBuf::new()
    };

    let sync_project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let skill_names_cfg = linthis::config::Config::load_merged(&sync_project_root).hook.agent.skill_names;
    let skill_names = Some(&skill_names_cfg);

    let filtered: Vec<&InstalledHook> = hooks_file.hooks.iter()
        .filter(|h| h.scope == target_scope)
        .filter(|h| global || h.project.is_empty() || h.project == project_root.to_str().unwrap_or(""))
        .collect();

    if filtered.is_empty() {
        return handle_sync_no_metadata(global, &project_root);
    }

    let grouped = group_hooks_by_type(&filtered);

    println!("{} Syncing {} hook(s)...", "→".cyan(), filtered.len());
    let mut errors = 0_u32;
    let mut hook_index = 0_usize;

    for (group_type, group_hooks) in &grouped {
        println!();
        let gt: &str = group_type;
        let gh_len = group_hooks.len();
        println!("{} Type: {} ({} hook{})", "→".cyan(), gt.cyan(), gh_len, if gh_len == 1 { "" } else { "s" });

        for hook in group_hooks {
            errors += sync_single_hook(hook, &mut hook_index, target_scope, global, &project_root, skill_names);
        }
    }

    let base_for_scan = if global { dirs::home_dir().unwrap_or_default() } else { project_root.clone() };
    sync_disk_scan_pass(&base_for_scan, global, &filtered, skill_names);

    if errors > 0 {
        eprintln!("{} {} error(s) during sync", "⚠".yellow(), errors);
        1
    } else {
        println!("{} Hook sync complete", "✓".green());
        0
    }
}

/// Called automatically after `linthis plugin sync` to refresh agent skill
/// files for installed agent hooks.
///
/// Mirrors the `--global` flag from the `plugin sync` command so that
/// `plugin sync -g` re-syncs global hooks and `plugin sync` re-syncs local hooks.
/// Uses non-interactive defaults (equivalent to `linthis hook sync -y`).
pub fn handle_hook_sync_after_plugin_sync(global: bool) {
    let code = handle_hook_sync(global, true);
    if code != 0 {
        eprintln!("{}: Agent hook sync encountered errors (exit {})", "Warning".yellow(), code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::{HookEvent, HookTool};

    #[test]
    fn test_dedup_base_and_with_agent() {
        let input = vec![HookTool::Git, HookTool::GitWithAgent];
        let result = deduplicate_hook_types(input);
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], HookTool::GitWithAgent));
    }

    #[test]
    fn test_dedup_exact_duplicate() {
        let input = vec![HookTool::Git, HookTool::Git];
        let result = deduplicate_hook_types(input);
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], HookTool::Git));
    }

    #[test]
    fn test_dedup_agent_and_git_with_agent_coexist() {
        let input = vec![HookTool::Agent, HookTool::GitWithAgent];
        let result = deduplicate_hook_types(input);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_dedup_prek_pair() {
        let input = vec![HookTool::Prek, HookTool::PrekWithAgent, HookTool::Agent];
        let result = deduplicate_hook_types(input);
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|t| matches!(t, HookTool::PrekWithAgent)));
        assert!(result.iter().any(|t| matches!(t, HookTool::Agent)));
    }

    #[test]
    fn test_dedup_events_removes_exact_dups() {
        let input = vec![HookEvent::PreCommit, HookEvent::PreCommit, HookEvent::PrePush];
        let result = deduplicate_hook_events(input);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_fallback_empty_types_and_events() {
        let (types, events) = apply_yes_fallback(vec![], vec![]);
        assert_eq!(types, vec![HookTool::Git]);
        assert_eq!(events, vec![HookEvent::PreCommit]);
    }

    #[test]
    fn test_fallback_agent_only_yes() {
        let types = vec![HookTool::Agent];
        let events: Vec<HookEvent> = vec![];
        let (_, resolved_events) = apply_yes_fallback(types, events);
        assert_eq!(resolved_events.len(), 3); // all three events
    }

    #[test]
    fn test_fallback_mixed_yes() {
        // Mixed types: apply_yes_fallback passes types through unchanged; events → [pre-commit]
        // Note: dedup (git+agent coexist) is handled by deduplicate_hook_types separately
        let types = vec![HookTool::Git, HookTool::Agent];
        let events: Vec<HookEvent> = vec![];
        let (resolved_types, resolved_events) = apply_yes_fallback(types, events);
        assert!(resolved_types.iter().any(|t| matches!(t, HookTool::Git)));
        assert!(resolved_types.iter().any(|t| matches!(t, HookTool::Agent)));
        assert_eq!(resolved_types.len(), 2);
        assert_eq!(resolved_events, vec![HookEvent::PreCommit]);
    }

    #[test]
    fn test_fallback_types_provided_events_empty() {
        // When types are explicit but events are empty, fallback to [pre-commit]
        let types = vec![HookTool::Git];
        let events: Vec<HookEvent> = vec![];
        let (resolved_types, resolved_events) = apply_yes_fallback(types, events);
        assert_eq!(resolved_types, vec![HookTool::Git]);
        assert_eq!(resolved_events, vec![HookEvent::PreCommit]);
    }

    #[test]
    fn test_lint_content_contains_linthis_s() {
        let content = agent_event_content_generic(&HookEvent::PreCommit);
        assert!(content.contains("linthis -s"));
    }

    #[test]
    fn test_cmsg_content_contains_linthis_cmsg() {
        let content = agent_event_content_generic(&HookEvent::CommitMsg);
        assert!(content.contains("linthis cmsg"));
        assert!(content.contains("feat"));
    }

    #[test]
    fn test_review_content_contains_git_diff() {
        let content = agent_event_content_generic(&HookEvent::PrePush);
        assert!(content.contains("git diff"));
        assert!(content.contains("Critical"));
        assert!(content.contains(".linthis/review/"));
    }

    #[test]
    fn test_skill_path_claude_per_event() {
        use std::path::PathBuf;
        let base = PathBuf::from("/repo");
        let lint_path = agent_skill_path(&base, &AgentProvider::Claude, false, &HookEvent::PreCommit, None);
        assert_eq!(lint_path, PathBuf::from("/repo/.claude/skills/lt-lint/SKILL.md"));

        let cmsg_path = agent_skill_path(&base, &AgentProvider::Claude, false, &HookEvent::CommitMsg, None);
        assert_eq!(cmsg_path, PathBuf::from("/repo/.claude/skills/lt-cmsg/SKILL.md"));

        let review_path = agent_skill_path(&base, &AgentProvider::Claude, false, &HookEvent::PrePush, None);
        assert_eq!(review_path, PathBuf::from("/repo/.claude/skills/lt-review/SKILL.md"));
    }

    #[test]
    fn test_skill_path_claude_global() {
        use std::path::PathBuf;
        let base = PathBuf::from("/home/user");
        let p = agent_skill_path(&base, &AgentProvider::Claude, true, &HookEvent::PreCommit, None);
        assert_eq!(p, PathBuf::from("/home/user/.claude/skills/lt-lint/SKILL.md"));
    }

    #[test]
    fn test_skill_path_cursor_per_event() {
        use std::path::PathBuf;
        let base = PathBuf::from("/repo");
        let p = agent_skill_path(&base, &AgentProvider::Cursor, false, &HookEvent::PrePush, None);
        assert_eq!(p, PathBuf::from("/repo/.cursor/rules/linthis-review.mdc"));
    }

    #[test]
    fn test_skill_path_gemini_per_event() {
        use std::path::PathBuf;
        let base = PathBuf::from("/repo");
        let p = agent_skill_path(&base, &AgentProvider::Gemini, false, &HookEvent::CommitMsg, None);
        assert_eq!(p, PathBuf::from("/repo/.gemini/linthis-cmsg.md"));
    }

    #[test]
    fn test_skill_path_codebuddy_per_event() {
        use std::path::PathBuf;
        let base = PathBuf::from("/repo");
        let p = agent_skill_path(&base, &AgentProvider::Codebuddy, false, &HookEvent::PreCommit, None);
        assert_eq!(p, PathBuf::from("/repo/.codebuddy/skills/lt-lint/SKILL.md"));
    }

    #[test]
    fn test_skill_path_with_custom_names() {
        use std::path::PathBuf;
        use linthis::config::AgentSkillNamesConfig;
        let base = PathBuf::from("/repo");
        let cfg = AgentSkillNamesConfig {
            pre_commit: Some("custom-lint".to_string()),
            commit_msg: None,
            pre_push: Some("my-review".to_string()),
        };
        // Custom pre-commit name
        let p = agent_skill_path(&base, &AgentProvider::Claude, false, &HookEvent::PreCommit, Some(&cfg));
        assert_eq!(p, PathBuf::from("/repo/.claude/skills/custom-lint/SKILL.md"));
        // Default commit-msg (not overridden)
        let p = agent_skill_path(&base, &AgentProvider::Claude, false, &HookEvent::CommitMsg, Some(&cfg));
        assert_eq!(p, PathBuf::from("/repo/.claude/skills/lt-cmsg/SKILL.md"));
        // Custom pre-push name for Gemini
        let p = agent_skill_path(&base, &AgentProvider::Gemini, false, &HookEvent::PrePush, Some(&cfg));
        assert_eq!(p, PathBuf::from("/repo/.gemini/my-review.md"));
        // Custom pre-commit for Codebuddy
        let p = agent_skill_path(&base, &AgentProvider::Codebuddy, false, &HookEvent::PreCommit, Some(&cfg));
        assert_eq!(p, PathBuf::from("/repo/.codebuddy/skills/custom-lint/SKILL.md"));
        // Custom pre-commit for Cursor
        let p = agent_skill_path(&base, &AgentProvider::Cursor, false, &HookEvent::PreCommit, Some(&cfg));
        assert_eq!(p, PathBuf::from("/repo/.cursor/rules/custom-lint.mdc"));
    }

    // ==================== New directory structure tests ====================

    #[test]
    fn test_agent_plugin_id_unified() {
        // All events share a single plugin ID "lt"
        assert_eq!(agent_plugin_id(&HookEvent::PreCommit), "lt");
        assert_eq!(agent_plugin_id(&HookEvent::CommitMsg), "lt");
        assert_eq!(agent_plugin_id(&HookEvent::PrePush), "lt");
    }

    #[test]
    fn test_fixed_agent_plugin_dir_default_fallback() {
        use linthis::hooks::resolver;

        let root = tempfile::TempDir::new().unwrap();
        let default_dir = root.path()
            .join("hooks/agent/plugins/_default/lt");
        std::fs::create_dir_all(&default_dir).unwrap();

        // _default/lt/ exists → should be found
        let result = resolver::fixed_agent_plugin_dir(root.path(), "claude", "lt");
        assert_eq!(result, Some(default_dir));
    }

    #[test]
    fn test_fixed_agent_plugin_dir_provider_override() {
        use linthis::hooks::resolver;

        let root = tempfile::TempDir::new().unwrap();
        let default_dir = root.path()
            .join("hooks/agent/plugins/_default/lt");
        let claude_dir = root.path()
            .join("hooks/agent/plugins/claude/lt");
        std::fs::create_dir_all(&default_dir).unwrap();
        std::fs::create_dir_all(&claude_dir).unwrap();

        // Both exist → provider-specific wins
        let result = resolver::fixed_agent_plugin_dir(root.path(), "claude", "lt");
        assert_eq!(result, Some(claude_dir));

        // Different provider → falls back to _default
        let result2 = resolver::fixed_agent_plugin_dir(root.path(), "codebuddy", "lt");
        assert_eq!(result2, Some(default_dir));
    }

    #[test]
    fn test_fixed_agent_plugin_dir_not_found() {
        use linthis::hooks::resolver;

        let root = tempfile::TempDir::new().unwrap();
        // No directories created
        let result = resolver::fixed_agent_plugin_dir(root.path(), "claude", "lt");
        assert!(result.is_none());
    }

    #[test]
    fn test_install_agent_plugin_from_dir_skill_and_command() {
        // Build a plugin dir with the new flat structure and verify install
        let plugin_root = tempfile::TempDir::new().unwrap();
        let pd = plugin_root.path();

        // Create skills/lt-lint/SKILL.md
        let skill_dir = pd.join("skills/lt-lint");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "---\nname: lt-lint\n---\n# Test Skill\n").unwrap();

        // Create commands/lt-lint.md
        let cmd_dir = pd.join("commands");
        std::fs::create_dir_all(&cmd_dir).unwrap();
        std::fs::write(cmd_dir.join("lt-lint.md"), "# /lt-lint\nRun lint.\n").unwrap();

        // Create memories/TOPLEVEL.md
        let mem_dir = pd.join("memories");
        std::fs::create_dir_all(&mem_dir).unwrap();
        std::fs::write(mem_dir.join("TOPLEVEL.md"), "## Linthis Memory\nRemember this.\n").unwrap();

        // Install into a temp base for Claude
        let base = tempfile::TempDir::new().unwrap();
        install_agent_plugin_from_dir(pd, base.path(), &AgentProvider::Claude, &HookEvent::PreCommit, None, None).unwrap();

        // Verify skill was installed
        let skill_target = base.path().join(".claude/skills/lt-lint/SKILL.md");
        assert!(skill_target.exists(), "SKILL.md should be installed");
        let content = std::fs::read_to_string(&skill_target).unwrap();
        assert!(content.contains("lt-lint"), "Skill content should be preserved");

        // Verify command was installed
        let cmd_target = base.path().join(".claude/commands/linthis/lt-lint.md");
        assert!(cmd_target.exists(), "Command file should be installed");

        // Verify memory was injected into CLAUDE.md
        let claude_md = base.path().join("CLAUDE.md");
        assert!(claude_md.exists(), "CLAUDE.md should be created");
        let mem_content = std::fs::read_to_string(&claude_md).unwrap();
        assert!(mem_content.contains("linthis-memory-lt"), "Memory section marker should exist");
        assert!(mem_content.contains("Linthis Memory"), "Memory content should be injected");
    }

    #[test]
    fn test_install_agent_plugin_from_dir_with_subdirs() {
        // Verify that skill subdirectories (scripts/, references/) are copied
        let plugin_root = tempfile::TempDir::new().unwrap();
        let pd = plugin_root.path();

        // Create skills/lt-lint/ with SKILL.md + scripts/ + references/
        let skill_dir = pd.join("skills/lt-lint");
        std::fs::create_dir_all(skill_dir.join("scripts")).unwrap();
        std::fs::create_dir_all(skill_dir.join("references")).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "---\nname: lt-lint\n---\n# Skill\n").unwrap();
        std::fs::write(skill_dir.join("scripts/check.sh"), "#!/bin/bash\necho ok\n").unwrap();
        std::fs::write(skill_dir.join("references/rules.md"), "# Rules\n").unwrap();

        // Install for Claude (supports skill subdirectories)
        let base = tempfile::TempDir::new().unwrap();
        install_agent_plugin_from_dir(pd, base.path(), &AgentProvider::Claude, &HookEvent::PreCommit, None, None).unwrap();

        let target_dir = base.path().join(".claude/skills/lt-lint");
        assert!(target_dir.join("SKILL.md").exists(), "SKILL.md should exist");
        assert!(target_dir.join("scripts/check.sh").exists(), "scripts/check.sh should be copied");
        assert!(target_dir.join("references/rules.md").exists(), "references/rules.md should be copied");

        // Verify content
        let script = std::fs::read_to_string(target_dir.join("scripts/check.sh")).unwrap();
        assert!(script.contains("echo ok"));
    }

    #[test]
    fn test_install_agent_plugin_from_dir_single_file_provider() {
        // For providers like Gemini that use single files, only SKILL.md content is used
        let plugin_root = tempfile::TempDir::new().unwrap();
        let pd = plugin_root.path();

        let skill_dir = pd.join("skills/lt-lint");
        std::fs::create_dir_all(skill_dir.join("scripts")).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# Gemini Skill Content\n").unwrap();
        std::fs::write(skill_dir.join("scripts/check.sh"), "#!/bin/bash\n").unwrap();

        let base = tempfile::TempDir::new().unwrap();
        install_agent_plugin_from_dir(pd, base.path(), &AgentProvider::Gemini, &HookEvent::PreCommit, None, None).unwrap();

        // Gemini: single file at .gemini/linthis-lint.md
        let target = base.path().join(".gemini/linthis-lint.md");
        assert!(target.exists(), "Gemini skill file should exist");
        let content = std::fs::read_to_string(&target).unwrap();
        assert!(content.contains("Gemini Skill Content"));

        // scripts/ should NOT be copied for single-file providers
        assert!(!base.path().join(".gemini/scripts").exists());
    }

    #[test]
    fn test_install_agent_plugin_from_dir_hooks_json() {
        // Verify hooks.json is installed as stop hook
        let plugin_root = tempfile::TempDir::new().unwrap();
        let pd = plugin_root.path();

        // Minimal skill
        let skill_dir = pd.join("skills/lt-lint");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "---\nname: lt-lint\n---\n# Skill\n").unwrap();

        // hooks/hooks.json
        let hooks_dir = pd.join("hooks");
        std::fs::create_dir_all(&hooks_dir).unwrap();
        std::fs::write(hooks_dir.join("hooks.json"), r#"{"hooks":{"Stop":[{"hooks":[{"type":"prompt","prompt":"test stop hook"}]}]}}"#).unwrap();

        let base = tempfile::TempDir::new().unwrap();
        install_agent_plugin_from_dir(pd, base.path(), &AgentProvider::Claude, &HookEvent::PreCommit, None, None).unwrap();

        // Verify .claude/settings.json was created with stop hook
        let settings = base.path().join(".claude/settings.json");
        assert!(settings.exists(), "settings.json should be created");
        let content = std::fs::read_to_string(&settings).unwrap();
        assert!(content.contains("Stop"), "Should contain Stop hook key");
        assert!(content.contains("test stop hook"), "Should contain hook prompt");
    }

    #[test]
    fn test_copy_dir_recursive() {
        let src = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(src.path().join("a/b")).unwrap();
        std::fs::write(src.path().join("top.txt"), "top").unwrap();
        std::fs::write(src.path().join("a/mid.txt"), "mid").unwrap();
        std::fs::write(src.path().join("a/b/deep.txt"), "deep").unwrap();

        let dst = tempfile::TempDir::new().unwrap();
        let target = dst.path().join("out");
        copy_dir_recursive(src.path(), &target).unwrap();

        assert_eq!(std::fs::read_to_string(target.join("top.txt")).unwrap(), "top");
        assert_eq!(std::fs::read_to_string(target.join("a/mid.txt")).unwrap(), "mid");
        assert_eq!(std::fs::read_to_string(target.join("a/b/deep.txt")).unwrap(), "deep");
    }
}
