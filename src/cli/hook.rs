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

/// Handle hook subcommands
pub fn handle_hook_command(action: HookCommands) -> ExitCode {
    match action {
        HookCommands::Install { hook_types, hook_events, force, yes, global, provider, args, provider_args } => {
            // Resolve types and events (dedup + interactive prompt or -y fallback)
            let (hook_types, hook_events) = if yes {
                apply_yes_fallback(
                    deduplicate_hook_types(hook_types),
                    deduplicate_hook_events(hook_events),
                )
            } else {
                let types = deduplicate_hook_types(hook_types);
                let types = if types.is_empty() {
                    match prompt_hook_types(false) {
                        Some(t) => t,
                        None => { println!("Installation cancelled"); return ExitCode::SUCCESS; }
                    }
                } else { types };
                let events = deduplicate_hook_events(hook_events);
                let events = if events.is_empty() {
                    match prompt_hook_events(false) {
                        Some(e) => e,
                        None => { println!("Installation cancelled"); return ExitCode::SUCCESS; }
                    }
                } else { events };
                (types, events)
            };
            handle_hook_install(hook_types, hook_events, force, yes, global, provider, args, provider_args)
        }
        HookCommands::Uninstall { hook_types, hook_events, all, all_types, all_events, yes, global } => {
            // --all / --all-types / --all-events: skip interactive prompts.
            // -y: skip prompt, use whatever was explicitly specified (no default fallback for uninstall).
            let (types, events) = if all || all_types || all_events || yes {
                (
                    deduplicate_hook_types(hook_types),
                    deduplicate_hook_events(hook_events),
                )
            } else {
                let types = deduplicate_hook_types(hook_types);
                let types = if types.is_empty() {
                    match prompt_hook_types(true) {
                        Some(t) => t,
                        None => { println!("Uninstall cancelled"); return ExitCode::SUCCESS; }
                    }
                } else { types };
                let events = deduplicate_hook_events(hook_events);
                let events = if events.is_empty() {
                    match prompt_hook_events(true) {
                        Some(e) => e,
                        None => { println!("Uninstall cancelled"); return ExitCode::SUCCESS; }
                    }
                } else { events };
                (types, events)
            };
            handle_hook_uninstall(types, events, all, all_types, all_events, yes, global)
        }
        HookCommands::Status => {
            handle_hook_status()
        }
        HookCommands::Check => {
            handle_hook_check()
        }
        HookCommands::CommitMsgCheck { msg_or_file } => {
            handle_commit_msg_check(&msg_or_file, false, None)
        }
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

/// Install git hook (pre-commit, pre-push, or commit-msg) for a single type × event pair.
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
    use std::io::{self, Write};

    // *-with-agent types: install base hook + agent fix fallback
    if hook_type.as_ref().map(|t| t.has_agent_fix()).unwrap_or(false) {
        let fix_provider = if let Some(p) = preresolved_fix_provider {
            p
        } else {
            match resolve_agent_fix_provider(provider.as_deref(), yes) {
                Ok(p)  => p,
                Err(e) => return e,
            }
        };
        let base = hook_type.as_ref().unwrap().base_tool().clone();
        return match &base {
            HookTool::Git => handle_git_with_agent_install(&hook_event, force, global, yes, &fix_provider, &args, provider_args.as_deref()),
            HookTool::Prek => {
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
                "openclaw"  => Some(AgentProvider::Openclaw),
                _ => {
                    eprintln!("{}: Unknown agent provider '{}'. Valid options: claude, codex, gemini, cursor, droid, auggie, codebuddy, openclaw", "Error".red(), p);
                    None
                }
            }
        });
        // If provider was given but invalid, exit
        if provider.is_some() && agent_provider.is_none() {
            return ExitCode::from(1);
        }
        return handle_agent_hook_install(agent_provider, &[hook_event.clone()], force, yes, global);
    }

    // Global non-agent hook: install into ~/.config/git/hooks
    if global {
        return handle_global_hook_install(hook_type, &hook_event, force, yes, &args, provider_args.as_deref());
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
    let is_empty_hook = hook_path.exists()
        && std::fs::read_to_string(&hook_path).map(|s| s.trim().is_empty()).unwrap_or(false);
    if hook_path.exists() && !force && !is_empty_hook {
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
    // forwarded correctly.  For pre-commit the hook receives no args so "$@"
    // is empty (a no-op).
    let linthis_cmd_var = match hook_event {
        HookEvent::CommitMsg => linthis_cmd
            .trim_end_matches(" \"$1\"")
            .to_string(),
        _ => linthis_cmd.clone(),
    };

    // For pre-push: git passes <remote-name> <remote-url> as positional args,
    // NOT file paths.  linthis uses `-i <file>` for file inputs and has no
    // positional-arg support, so we must compute the pushed files from git diff
    // and build `-i` flags.  The original remote args are saved for delegating
    // to any local hook (which expects them).
    //
    // For other events (pre-commit receives nothing, commit-msg receives $1 =
    // message file via "$@") the existing "$@" passthrough is correct.
    let (pre_push_preamble, local_hook_orig_args) = if matches!(hook_event, HookEvent::PrePush) {
        let preamble = "# For pre-push: save remote args, compute pushed files as -i flags\n\
             _REMOTE_NAME=\"$1\"\n\
             _REMOTE_URL=\"$2\"\n\
             _BASE=$(git rev-parse '@{u}' 2>/dev/null || \\\n\
             \x20       git rev-parse 'HEAD~1' 2>/dev/null)\n\
             _PUSHED_FILES=$(git diff --name-only \"$_BASE\"..HEAD 2>/dev/null | grep -v '^$')\n\
             set --\n\
             if [ -n \"$_PUSHED_FILES\" ]; then\n\
             \x20 while IFS= read -r _F; do set -- \"$@\" -i \"$_F\"; done <<_EOF_\n\
             $_PUSHED_FILES\n\
             _EOF_\n\
             fi\n\
             \n"
            .to_string();
        (preamble, "\"$_REMOTE_NAME\" \"$_REMOTE_URL\"")
    } else {
        (String::new(), "\"$@\"")
    };

    let error_msg = agent_fix_error_msg(hook_event);
    let new_msg_print = if matches!(hook_event, HookEvent::CommitMsg) {
        agent_fix_show_fixed_cmsg("   ")
    } else {
        String::new()
    };
    let fix_block = match fix_provider {
        None => String::new(),
        Some(p) => {
            let agent_cmd = if matches!(hook_event, HookEvent::CommitMsg) {
                agent_fix_headless_cmd_commit_msg(p, None)
            } else {
                let prompt = agent_fix_prompt_for_event(hook_event);
                agent_fix_headless_cmd(p, &prompt, None)
            };
            let agent_check = shell_agent_availability_check(p);
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
                provider = p,
                agent = agent_cmd,
                agent_check = agent_check,
                error_msg = error_msg,
                new_msg_print = new_msg_print,
            )
        }
    };
    let fix_block_direct = match fix_provider {
        None => String::new(),
        Some(p) => {
            let agent_cmd = if matches!(hook_event, HookEvent::CommitMsg) {
                agent_fix_headless_cmd_commit_msg(p, None)
            } else {
                let prompt = agent_fix_prompt_for_event(hook_event);
                agent_fix_headless_cmd(p, &prompt, None)
            };
            let agent_check = shell_agent_availability_check(p);
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
                provider = p,
                agent = agent_cmd,
                agent_check = agent_check,
                error_msg = error_msg,
                new_msg_print = new_msg_print,
            )
        }
    };

    let event_name = hook_event.hook_filename();

    // For pre-push events, add background review trigger
    let review_block = if matches!(hook_event, HookEvent::PrePush) {
        "\n# Trigger background AI code review (non-blocking)\n\
         linthis review --background 2>/dev/null &\n"
    } else {
        ""
    };

    let timer_block = if fix_provider.is_some() {
        shell_timer_functions()
    } else {
        ""
    };

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
            if existing.trim().is_empty() {
                // Empty file — treat as not installed, fall through to overwrite
            } else if existing.contains("# linthis-hook") || existing.contains("linthis hook run") {
                println!(
                    "{}: Global {} hook already installed at {}",
                    "Info".cyan(),
                    hook_filename,
                    hook_path.display()
                );
                return ExitCode::SUCCESS;
            } else {
                eprintln!(
                    "{}: {} already exists (not by linthis). Use --force to overwrite.",
                    "Warning".yellow(),
                    hook_path.display()
                );
                return ExitCode::from(1);
            }
        }
    }

    // Create directory
    if let Err(e) = fs::create_dir_all(&hooks_dir) {
        eprintln!("{}: Failed to create {}: {}", "Error".red(), hooks_dir.display(), e);
        return ExitCode::from(2);
    }

    // Determine the effective hook_type for the thin wrapper
    let effective_hook_type = hook_type.clone().unwrap_or(HookTool::Git);
    let provider_str = fix_provider.as_ref().map(|p| p.as_str());

    // Generate thin wrapper script (full logic runs from binary at hook execution time)
    let content = build_thin_wrapper_script(hook_event, &effective_hook_type, provider_str, true, provider_args);
    let _ = args; // args are embedded in binary logic at run time, not in the thin wrapper

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

    // Persist metadata for `linthis hook sync`
    save_installed_hook("global", "", hook_event, &effective_hook_type, provider_str, provider_args);

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
            .map(|c| c.contains("# linthis-hook") || c.contains("linthis hook run"))
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
        .any(|e| {
            let p = hooks_dir.join(e.hook_filename());
            p.exists() && fs::read_to_string(&p).map(|c| c.contains("# linthis-hook") || c.contains("linthis hook run")).unwrap_or(false)
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
                    let has_linthis = content.contains("# linthis-hook") || content.contains("linthis hook run");
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
    let status_skill_names_cfg = linthis::config::Config::load_merged(&git_root).hook.agent.skill_names;
    let status_skill_names = Some(&status_skill_names_cfg);
    let mut any_agent_installed = false;
    for p in ALL_AGENT_PROVIDERS {
        let installed = agent_is_installed(&git_root, p, false, status_skill_names);
        if installed {
            any_agent_installed = true;
            println!("{} {}", "✓".green(), p);
            let events = [HookEvent::PreCommit, HookEvent::CommitMsg, HookEvent::PrePush];
            for event in &events {
                let path = agent_skill_path(&git_root, p, false, event, status_skill_names);
                if path.exists() {
                    let event_name = match event {
                        HookEvent::PreCommit => "pre-commit",
                        HookEvent::CommitMsg => "commit-msg",
                        HookEvent::PrePush => "pre-push",
                    };
                    println!("  {} {} ({})", "✓".green().dimmed(), path.display(), event_name);
                }
            }
            if let Some(settings_path) = agent_stop_hook_settings_path(&git_root, p) {
                let has_stop_hook = settings_path.exists()
                    && std::fs::read_to_string(&settings_path)
                        .map(|c| c.contains("linthis"))
                        .unwrap_or(false);
                if has_stop_hook {
                    println!("  {} Stop Hook ({})", "✓".green().dimmed(), settings_path.display());
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

/// Uninstall git hooks for the given types × events combinations.
///
/// Flag semantics:
/// - `--all`        : uninstall every type × every event (both git files and agent hooks)
/// - `--all-types`  : uninstall every type for the specified `--event`(s)
/// - `--all-events` : uninstall every event for the specified `--type`(s)
fn handle_hook_uninstall(
    hook_types: Vec<HookTool>,
    hook_events: Vec<HookEvent>,
    all: bool,
    all_types: bool,
    all_events: bool,
    yes: bool,
    global: bool,
) -> ExitCode {
    const ALL_EVENTS: [HookEvent; 3] = [HookEvent::PreCommit, HookEvent::PrePush, HookEvent::CommitMsg];

    // Resolve effective events: --all / --all-events expand to all three events.
    let effective_events: Vec<HookEvent> = if all || all_events {
        ALL_EVENTS.to_vec()
    } else {
        hook_events.clone()
    };

    // Resolve whether agent hooks should be included:
    // --all / --all-types always include agent; otherwise only if Agent is in the type list.
    let include_agent = all || all_types || hook_types.iter().any(|t| matches!(t, HookTool::Agent));

    // Resolve whether git hook files should be touched:
    // --all / --all-types always include git hooks; otherwise only if a non-agent type is listed.
    let include_git = all || all_types || hook_types.iter().any(|t| !matches!(t, HookTool::Agent));

    if global {
        let mut result = ExitCode::SUCCESS;
        if include_git {
            // When all events are needed, delegate to handle_global_hook_uninstall with all=true
            // (it iterates all three events internally). Otherwise iterate effective_events explicitly.
            if all || all_events {
                result = handle_global_hook_uninstall(None, true, yes);
            } else {
                for event in &effective_events {
                    result = handle_global_hook_uninstall(Some(event.clone()), false, yes);
                }
            }
        }
        if include_agent {
            handle_agent_hook_uninstall(yes, true, &effective_events);
        }
        return result;
    }

    // Find git root
    let git_root = match find_git_root() {
        Some(root) => root,
        None => {
            eprintln!("{}: Not in a git repository", "Error".red());
            return ExitCode::from(1);
        }
    };

    let mut any_uninstalled = false;

    if include_git {
        for event in &effective_events {
            let result = uninstall_single_hook(&git_root, event, yes);
            if result == ExitCode::SUCCESS {
                any_uninstalled = true;
            }
        }
    }

    if include_agent {
        let agent_result = handle_agent_hook_uninstall(yes, false, &effective_events);
        if agent_result == ExitCode::SUCCESS {
            any_uninstalled = true;
        }
    }

    if !any_uninstalled {
        println!("{}: No hooks with linthis found", "Info".cyan());
    }

    ExitCode::SUCCESS
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

    // Remove the TOML record for this hook
    let project_str = git_root.to_str().unwrap_or("").to_string();
    remove_installed_hook("local", &project_str, hook_event);

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
/// - Tier 3 (built-in):   `"built-in → linthis -c --hook-event=pre-push"`
fn describe_hook_source(tool: &HookTool, hook_event: &HookEvent) -> String {
    use linthis::config::Config;
    use linthis::hooks::resolver;

    let tool_type_dir = match tool {
        HookTool::Git => "git",
        HookTool::GitWithAgent => "git-with-agent",
        HookTool::Prek => "prek",
        HookTool::PrekWithAgent => "prek-with-agent",
        HookTool::Agent => return "built-in (agent)".to_string(),
    };

    let project_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    // Tier 1: fixed-path auto-discovery
    if resolver::fixed_git_hook_path(&project_root, tool_type_dir, hook_event.hook_filename()).is_some() {
        return format!("hooks/{}/{} (fixed path)", tool_type_dir, hook_event.hook_filename());
    }

    // Tier 2: TOML source mapping
    let config = Config::load_merged(&project_root);
    let hook_cfg = &config.hook;
    let event_key = hook_event.hook_filename();

    let entry = match tool {
        HookTool::Git => hook_cfg.git.get(event_key),
        HookTool::GitWithAgent => hook_cfg.git_with_agent.get(event_key),
        HookTool::Prek => hook_cfg.prek.get(event_key),
        HookTool::PrekWithAgent => hook_cfg.prek_with_agent.get(event_key),
        HookTool::Agent => unreachable!(),
    };

    if let Some(entry) = entry {
        // e.g. "git-with-agent" → "[hook.git-with-agent]"
        let section = format!("[hook.{}]", tool_type_dir);
        let source_str = format_hook_source(&entry.source);
        return format!("{}\n{} = {{ source = {} }}", section, event_key, source_str);
    }

    // Tier 3: built-in
    let cmd = build_hook_command(hook_event, &None);
    format!("built-in → {}", cmd)
}

/// Create hook configuration file based on the selected tool and event
/// Resolve hook script content from Tier-1 (fixed path) or Tier-2 (TOML config),
/// returning `Ok(Some(content))` if an override is found, `Ok(None)` to fall through
/// to the built-in generator, or `Err(ExitCode)` for a hard resolution error.
fn resolve_hook_override(tool: &HookTool, hook_event: &HookEvent) -> Result<Option<String>, ExitCode> {
    use linthis::config::Config;
    use linthis::hooks::resolver;

    let tool_type_dir = match tool {
        HookTool::Git => "git",
        HookTool::GitWithAgent => "git-with-agent",
        HookTool::Prek => "prek",
        HookTool::PrekWithAgent => "prek-with-agent",
        HookTool::Agent => return Ok(None), // agent handled separately
    };

    let project_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    // Tier 1: fixed-path auto-discovery
    if let Some(fixed) = resolver::fixed_git_hook_path(&project_root, tool_type_dir, hook_event.hook_filename()) {
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
    let hook_cfg = &config.hook;
    let event_key = hook_event.hook_filename(); // "pre-commit", "commit-msg", "pre-push"

    let entry = match tool {
        HookTool::Git => hook_cfg.git.get(event_key),
        HookTool::GitWithAgent => hook_cfg.git_with_agent.get(event_key),
        HookTool::Prek => hook_cfg.prek.get(event_key),
        HookTool::PrekWithAgent => hook_cfg.prek_with_agent.get(event_key),
        HookTool::Agent => return Ok(None),
    };

    if let Some(entry) = entry {
        match resolver::resolve_to_string(&entry.source, &project_root, &hook_cfg.marketplaces) {
            Ok(content) => return Ok(Some(content)),
            Err(e) => {
                eprintln!("{}: Failed to resolve hook override for '{}/{}': {}", "Error".red(), tool_type_dir, event_key, e);
                return Err(ExitCode::from(2));
            }
        }
    }

    // Tier 3: no override — caller uses built-in generator
    Ok(None)
}

fn create_hook_config(tool: &HookTool, hook_event: &HookEvent, force: bool, args: &Option<String>) -> Result<(), ExitCode> {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    let hook_filename = hook_event.hook_filename();

    match tool {
        HookTool::Agent
        | HookTool::GitWithAgent | HookTool::PrekWithAgent => {
            // Handled separately before create_hook_config is called
            return Ok(());
        }
        HookTool::Prek => {
            let config_path = std::path::PathBuf::from(".pre-commit-config.yaml");

            if config_path.exists() && !force {
                eprintln!(
                    "{}: {} already exists, skipping",
                    "Warning".yellow(),
                    config_path.display()
                );
                return Ok(());
            }

            // ── Tier-1/2 override check ──────────────────────────────────────
            if let Some(override_content) = resolve_hook_override(tool, hook_event)? {
                match std::fs::write(&config_path, override_content) {
                    Ok(_) => {
                        println!("{} Created {} [override]", "✓".green(), config_path.display());
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("{}: Failed to write '{}': {}", "Error".red(), config_path.display(), e);
                        return Err(ExitCode::from(2));
                    }
                }
            }
            // ── End override check — fall through to built-in generator ──────

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
                    let tool_name = "prek";
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

            // ── Tier-1/2 override check ──────────────────────────────────────
            // If a fixed-path file or TOML source entry exists, use its content
            // as the complete hook script.  On resolution error, abort (no fallback).
            if let Some(override_content) = resolve_hook_override(tool, hook_event)? {
                let content = if hook_path.exists() && !force {
                    // Append override content to existing hook
                    let mut existing = fs::read_to_string(&hook_path).unwrap_or_default();
                    if !existing.ends_with('\n') { existing.push('\n'); }
                    existing.push_str("\n# linthis-hook (override)\n");
                    existing.push_str(&override_content);
                    existing
                } else {
                    override_content
                };
                match fs::write(&hook_path, &content) {
                    Ok(_) => {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            if let Ok(meta) = fs::metadata(&hook_path) {
                                let mut perms = meta.permissions();
                                perms.set_mode(0o755);
                                let _ = fs::set_permissions(&hook_path, perms);
                            }
                        }
                        println!("{} Created {} [project, override]", "✓".green(), hook_path.display());
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("{}: Failed to write hook '{}': {}", "Error".red(), hook_path.display(), e);
                        return Err(ExitCode::from(2));
                    }
                }
            }
            // ── End override check — fall through to built-in generator ──────

            // Build hook command based on options and event type (used for append detection)
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

                // Check if linthis is already in the hook (direct command or thin wrapper)
                if existing_content.contains(&linthis_hook_line)
                    || existing_content.contains("linthis hook run")
                {
                    println!(
                        "{}: linthis hook already exists in {}",
                        "Info".cyan(),
                        hook_path.display()
                    );
                    return Ok(());
                }

                // Append linthis to the existing hook (keep appended line, not thin wrapper,
                // to avoid overwriting unrelated existing hook logic)
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
                // Create new hook file as thin wrapper (hook logic auto-updates with linthis)
                let content = build_thin_wrapper_script(hook_event, &HookTool::Git, None, false, None);

                match fs::write(&hook_path, &content) {
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
                        println!("  {} Thin wrapper: hook logic auto-updates with linthis", "→".dimmed());
                        #[cfg(not(unix))]
                        {
                            println!("\nNext steps:");
                            println!("  Make sure the hook is executable:");
                            println!("    {}", format!("chmod +x .git/hooks/{}", hook_filename).cyan());
                        }
                        // Persist metadata for `linthis hook sync`
                        let project = git_root.to_str().unwrap_or("").to_string();
                        save_installed_hook("local", &project, hook_event, &HookTool::Git, None, None);
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
     Run 'linthis -s -c' to inspect them. \
     Fix all issues by editing the files directly (do NOT use linthis --fix). \
     Verify with 'linthis -s -c' until it passes cleanly."
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

/// Build the pre-push hook script that ALWAYS triggers an agent code review.
///
/// Unlike pre-commit (agent only called on failure), pre-push always invokes
/// the agent to perform a structured review of the diff before pushing.
/// The agent's exit code gates the push: non-zero = block, zero = allow.
fn build_git_with_agent_prepush_script(linthis_cmd: &str, fix_provider: &AgentFixProvider, provider_args: Option<&str>) -> String {
    let review_prompt = "Perform a structured pre-push code review using the lt.review skill. \
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
        Exit 0 unless Critical issues were found.";
    let agent_cmd = agent_fix_headless_cmd(fix_provider, review_prompt, provider_args);
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
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::io::{self, Write};

    let hook_filename = hook_event.hook_filename();
    let _ = args; // args are embedded in binary logic at run time, not in the thin wrapper

    let (hook_path, scope, project) = if global {
        let hooks_dir = match global_hooks_dir() {
            Some(d) => d,
            None => {
                eprintln!("{}: Could not determine global hooks directory", "Error".red());
                return ExitCode::from(1);
            }
        };
        (hooks_dir.join(hook_filename), "global", String::new())
    } else {
        let git_root = match find_git_root() {
            Some(root) => root,
            None => {
                eprintln!("{}: Not in a git repository", "Error".red());
                return ExitCode::from(1);
            }
        };
        let project_str = git_root.to_str().unwrap_or("").to_string();
        (git_root.join(".git/hooks").join(hook_filename), "local", project_str)
    };

    // Confirm with user for global installs unless --yes was passed.
    // Global installs also set core.hooksPath so warn the user.
    if global && !yes {
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

    let content = build_thin_wrapper_script(
        hook_event,
        &HookTool::GitWithAgent,
        Some(fix_provider.as_str()),
        global,
        provider_args,
    );

    if hook_path.exists() && !force {
        if let Ok(existing) = fs::read_to_string(&hook_path) {
            if existing.contains("# linthis-hook") || existing.contains("linthis hook run") {
                println!(
                    "{}: {} hook already installed at {}",
                    "Info".cyan(),
                    hook_filename,
                    hook_path.display()
                );
                return ExitCode::SUCCESS;
            }
        }
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
            println!("  {} Thin wrapper: hook logic auto-updates with linthis", "→".dimmed());

            // For global installs, also set core.hooksPath
            if global {
                if let Some(hooks_dir) = global_hooks_dir() {
                    let hooks_dir_str = hooks_dir.to_string_lossy().to_string();
                    let _ = std::process::Command::new("git")
                        .args(["config", "--global", "core.hooksPath", &hooks_dir_str])
                        .status();
                    println!("{} Set {} = {}", "✓".green(), "core.hooksPath".cyan(), hooks_dir_str);
                }
            }

            // Persist metadata for `linthis hook sync`
            save_installed_hook(scope, &project, hook_event, &HookTool::GitWithAgent, Some(fix_provider.as_str()), provider_args);
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

## Steps

1. Identify modified code files in this session (files written or edited via Write/Edit tools, or via Bash)
2. Run `linthis -i <file1> -i <file2> -c` on all modified files — use separate `-i` flags for each file
3. Before any `git commit`, also run `linthis -s -c` to check all staged files
4. Read the lint output carefully — each issue includes file path, line number, and rule name
5. If issues are found, fix them by editing the code directly
   - Do **NOT** use `linthis --fix` or `linthis fix` — fixing manually ensures you understand the issue and don't introduce regressions from blind automated transforms
6. Re-run `linthis -i <files> -c` to confirm all issues are resolved
7. Only approve the commit once lint passes with zero errors

## Key Commands

| Action | Command |
|--------|---------|
| Lint staged files | `linthis -s -c` |
| Lint specific files | `linthis -i <file> -c` |
| Format only | `linthis -s -f` |

## Example

```
$ linthis -i src/handler.go -c

src/handler.go:15:1: exported function HandleRequest should have comment (golint)
src/handler.go:23:4: error return value not checked (errcheck)

2 issues found
```

Fix line 15 by adding a doc comment, and line 23 by handling the error return value. Then re-run to confirm zero errors."#
        .to_string()
}

fn agent_cmsg_body() -> String {
    r#"## Companion Skills

When the user asks to commit, if the **lt-lint** skill is also available, both lt-lint and lt-cmsg should be invoked together. Run lt-lint first (to fix code issues), then lt-cmsg (to validate the commit message).

## Goal

Ensure every commit message follows Conventional Commits format and accurately reflects the actual code changes. A well-structured commit history makes code review, changelog generation, and git bisect much easier.

## When to Skip

If the commit message already complies with all rules below, approve immediately with `✅ Commit message OK`.

## Steps

1. Read the commit message from `.git/COMMIT_EDITMSG`
2. Run `git diff --cached --stat` to understand what files actually changed — the type prefix must match the actual diff, not just what the developer wrote
3. Run `git log -n 5 --oneline` to check the recent commit style **and language** (Chinese or English) — match that language for the description part, because consistency in the git log improves readability
4. Validate with: `linthis cmsg "your commit message here"`
5. Evaluate the message against these rules:
   - **Type prefix**: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`
   - **Scope** (optional): `feat(module): description`
   - **Subject line**: ≤72 characters, imperative mood, starts with lowercase after the colon
   - **No trailing period** on subject line
   - **Body** (if present): wrapped at 80 characters, explains *why* not *what*
6. If the message is acceptable, output `✅ Commit message OK` and approve
7. If improvements are needed, choose the correct `type` based on the staged diff, then **automatically rewrite** `.git/COMMIT_EDITMSG` — do NOT ask for confirmation

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
            "prompt": "Before finishing, check if any code files were modified during this session (Write/Edit/Bash tools). If code was modified:\n1. Run `linthis -i <file1> -i <file2> -c` on all modified files to check for lint issues\n2. If issues are found, fix them yourself by editing the code directly (do NOT use `linthis --fix` or `linthis fix`)\n3. Re-run `linthis -i <files> -c` to confirm all issues are resolved\n4. Only approve stopping once lint passes with no errors\n\nIf no code files were modified, approve stopping immediately.\n\nYou MUST respond with valid JSON: {\"ok\": true} to approve stopping, or {\"ok\": false, \"reason\": \"description of remaining lint issues\"} to block."
          }
        ]
      }
    ]
  }
}"#;

fn agent_stop_hook_json_ref() -> &'static str {
    AGENT_STOP_HOOK_JSON
}

/// Get the skill file path for a given agent provider and hook event.
///
/// When `global` is true, `base` is the user home directory; otherwise it is
/// the project git root.  Each event maps to a distinct file so that agents
/// receive focused instructions per hook type.
fn agent_skill_path(
    base: &std::path::Path,
    provider: &AgentProvider,
    global: bool,
    event: &HookEvent,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> PathBuf {
    let event_name = match event {
        HookEvent::PreCommit => "lint",
        HookEvent::CommitMsg => "cmsg",
        HookEvent::PrePush   => "review",
    };
    // Check for a configured skill name override for this event
    let custom_name: Option<&str> = skill_names.and_then(|sn| match event {
        HookEvent::PreCommit => sn.pre_commit.as_deref(),
        HookEvent::CommitMsg => sn.commit_msg.as_deref(),
        HookEvent::PrePush   => sn.pre_push.as_deref(),
    });
    match provider {
        AgentProvider::Claude => {
            // Skills subdirectory: .claude/skills/<name>/SKILL.md
            let dir_name = custom_name.map_or_else(|| format!("lt-{}", event_name), |n| n.to_string());
            base.join(".claude/skills").join(dir_name).join("SKILL.md")
        }
        AgentProvider::Codex => {
            // Section-based: AGENTS.md (path doesn't change per event; event handled by section content)
            if global { base.join(".codex/AGENTS.md") } else { base.join("AGENTS.md") }
        }
        AgentProvider::Gemini => {
            let name = custom_name.map_or_else(|| format!("linthis-{}", event_name), |n| n.to_string());
            base.join(".gemini").join(format!("{}.md", name))
        }
        AgentProvider::Cursor => {
            let name = custom_name.map_or_else(|| format!("linthis-{}", event_name), |n| n.to_string());
            base.join(".cursor/rules").join(format!("{}.mdc", name))
        }
        AgentProvider::Droid => {
            let name = custom_name.map_or_else(|| format!("linthis-{}", event_name), |n| n.to_string());
            base.join(".droid/rules").join(format!("{}.md", name))
        }
        AgentProvider::Auggie => {
            let name = custom_name.map_or_else(|| format!("linthis-{}", event_name), |n| n.to_string());
            base.join(".augment/rules").join(format!("{}.md", name))
        }
        AgentProvider::Codebuddy => {
            let dir_name = custom_name.map_or_else(|| format!("lt-{}", event_name), |n| n.to_string());
            base.join(".codebuddy/skills").join(dir_name).join("SKILL.md")
        }
        AgentProvider::Openclaw => {
            let dir_name = custom_name.map_or_else(|| format!("lt-{}", event_name), |n| n.to_string());
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
        // Skill-dir-based: check for section marker in file (current or legacy)
        AgentProvider::Claude | AgentProvider::Codebuddy | AgentProvider::Openclaw => {
            let path = agent_skill_path(base, provider, global, &HookEvent::PreCommit, skill_names);
            path.exists()
                && std::fs::read_to_string(&path)
                    .map(|c| c.contains(AGENT_SECTION_MARKER) || c.contains(AGENT_SECTION_MARKER_LEGACY))
                    .unwrap_or(false)
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

/// Resolve and install agent plugin components (skill, command, memory, hooks) from a plugin directory.
///
/// New layout — `plugin_dir` must contain:
/// - `skills/<skill_name>/SKILL.md`  (skill_name from `agent_event_skill_metadata`)
/// - `commands/`                      (optional; all files copied)
/// - `memories/TOPLEVEL.md`           (optional; injected into provider root instruction file)
/// - `hooks/hooks.json`               (optional; stop hook settings merged into provider settings)
///
/// Each is optional; missing subdirs are silently skipped.
fn install_agent_plugin_from_dir(
    plugin_dir: &std::path::Path,
    base: &std::path::Path,
    provider: &AgentProvider,
    event: &HookEvent,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
    target: Option<&linthis::config::AgentTargetConfig>,
) -> Result<(), String> {
    use std::fs;

    let (skill_name, _) = agent_event_skill_metadata(event, skill_names);

    // ── skill ───────────────────────────────────────────────────────────
    let skill_src_dir = plugin_dir.join("skills").join(&skill_name);
    let skill_src = skill_src_dir.join("SKILL.md");
    if skill_src.is_file() {
        if let Some(target_skills) = target.and_then(|t| t.skills.as_deref()) {
            // Custom target: skill-dir style — {base}/{target.skills}/{skill_name}/SKILL.md
            let custom_skill_dir = base.join(target_skills).join(&skill_name);
            copy_dir_recursive(&skill_src_dir, &custom_skill_dir)?;
        } else {
            let skill_path = agent_skill_path(base, provider, false, event, skill_names);
            match provider {
                AgentProvider::Codex => {
                    let content = fs::read_to_string(&skill_src)
                        .map_err(|e| format!("Failed to read skill file '{}': {}", skill_src.display(), e))?;
                    let section_marker = agent_event_section_marker(event);
                    install_agent_append_section(&skill_path, &content, section_marker, "# Agent Instructions\n")?;
                }
                AgentProvider::Claude | AgentProvider::Codebuddy | AgentProvider::Openclaw => {
                    let target_dir = skill_path.parent().unwrap();
                    copy_dir_recursive(&skill_src_dir, target_dir)?;
                    if matches!(provider, AgentProvider::Openclaw) {
                        openclaw_post_install_skill(target_dir);
                    }
                }
                _ => {
                    let content = fs::read_to_string(&skill_src)
                        .map_err(|e| format!("Failed to read skill file '{}': {}", skill_src.display(), e))?;
                    install_agent_dedicated_file(&skill_path, &content)?;
                }
            }
        }
    }

    // ── command ─────────────────────────────────────────────────────────
    let cmd_src_dir = plugin_dir.join("commands");
    if cmd_src_dir.is_dir() {
        let cmd_dir_opt = if let Some(target_commands) = target.and_then(|t| t.commands.as_deref()) {
            Some(base.join(target_commands))
        } else {
            agent_command_dir(base, provider)
        };
        if let Some(cmd_dir) = cmd_dir_opt {
            if let Ok(entries) = fs::read_dir(&cmd_src_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_file() {
                        let filename = entry.file_name();
                        let cmd_target = cmd_dir.join(&filename);
                        let content = fs::read_to_string(entry.path())
                            .map_err(|e| format!("Failed to read command file '{}': {}", entry.path().display(), e))?;
                        install_agent_dedicated_file(&cmd_target, &content)?;
                    }
                }
            }
        }
    }

    // ── memory ──────────────────────────────────────────────────────────
    let mem_src = plugin_dir.join("memories").join("TOPLEVEL.md");
    if mem_src.is_file() {
        let memory_target = if let Some(target_memory) = target.and_then(|t| t.memory.as_deref()) {
            Some(base.join(target_memory))
        } else {
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
        };
        if let Some(mem_target) = memory_target {
            let content = fs::read_to_string(&mem_src)
                .map_err(|e| format!("Failed to read memory file '{}': {}", mem_src.display(), e))?;
            let plugin_id = agent_plugin_id(event);
            let section_marker = &format!("linthis-memory-{}", plugin_id);
            install_agent_append_section(&mem_target, &content, section_marker, "")?;
        }
    }

    // ── stop hook (from plugin's hooks/hooks.json) ──────────────────────
    let hooks_json_src = plugin_dir.join("hooks").join("hooks.json");
    if hooks_json_src.is_file() {
        if matches!(provider, AgentProvider::Openclaw) {
            // OpenClaw uses its own event-based hooks system (HOOK.md + handler.ts).
            // No direct mapping to the stop-hook concept; skip for now.
        } else {
            let settings_path_opt = if let Some(target_settings) = target.and_then(|t| t.settings.as_deref()) {
                Some(base.join(target_settings))
            } else {
                agent_stop_hook_settings_path(base, provider)
            };
            if let Some(settings_path) = settings_path_opt {
                let override_json = fs::read_to_string(&hooks_json_src)
                    .map_err(|e| format!("Failed to read hooks.json '{}': {}", hooks_json_src.display(), e))?;
                install_agent_stop_hook_from_json(base, &settings_path, &override_json)?;
            }
        }
    }

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
            "对暂存/修改的代码文件运行 linthis 代码检查，提交前修复所有问题。使用 `linthis -i <file> -c` 按项目编码规范检查，必须手动编辑修复（不能用 linthis --fix）。提交时若 lt-cmsg skill 也存在应一起触发。Run linthis lint checks on staged/modified code files and fix all issues before committing. Uses `linthis -i <file> -c`. Issues must be fixed by editing code directly. If the lt-cmsg skill also exists, both should be invoked together when committing.",
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

fn handle_agent_hook_install(
    provider: Option<AgentProvider>,
    events: &[HookEvent],
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
                eprintln!("  Run this command from within a git repository, or use --global / -g to install user-level skills");
                return ExitCode::from(1);
            }
        }
    };

    let scope = if global { "global" } else { "local" };
    let project_str = if global {
        String::new()
    } else {
        base.to_str().unwrap_or("").to_string()
    };

    // Load configured skill name aliases
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let skill_names_cfg = linthis::config::Config::load_merged(&project_root).hook.agent.skill_names;
    let skill_names = Some(&skill_names_cfg);

    println!("{}", "🤖 AI Coding Agent Integration".bold());
    if global {
        println!("  {} Installing user-level skills in {}", "→".dimmed(), base.display());
    }
    println!();

    // If a specific provider was given, install just that one
    if let Some(ref p) = provider {
        let installed = agent_is_installed(&base, p, global, skill_names);
        if installed && !force {
            println!(
                "{}: {} is already installed",
                "Info".cyan(),
                p
            );
            print_agent_installed_info(&base, p, global, skill_names);
            return ExitCode::SUCCESS;
        }

        warn_legacy_if_present(&base, p);
        let provider_name = format!("{}", p).to_lowercase();
        for event in events {
            match install_agent_skill(&base, p, global, event, skill_names) {
                Ok(_) => {
                    let path = agent_skill_path(&base, p, global, event, skill_names);
                    println!("{} Installed {} ({}) → {}", "✓".green(), p, event.hook_filename(), path.display());
                    add_skill_provider_to_hook(scope, &project_str, event, &provider_name);
                }
                Err(e) => {
                    eprintln!("{}: Failed to install {} ({}): {}", "Error".red(), p, event.hook_filename(), e);
                    return ExitCode::from(2);
                }
            }
        }
        print_extra_installed(&base, p);
        return ExitCode::SUCCESS;
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
            if agent_is_installed(&base, p, global, skill_names) && !force {
                println!("{}: {} already installed", "Info".cyan(), p);
                print_agent_installed_info(&base, p, global, skill_names);
                continue;
            }
            warn_legacy_if_present(&base, p);
            let provider_name = format!("{}", p).to_lowercase();
            let mut provider_ok = true;
            for event in events {
                match install_agent_skill(&base, p, global, event, skill_names) {
                    Ok(_) => {
                        let path = agent_skill_path(&base, p, global, event, skill_names);
                        println!("{} Installed {} ({}) → {}", "✓".green(), p, event.hook_filename(), path.display());
                        add_skill_provider_to_hook(scope, &project_str, event, &provider_name);
                    }
                    Err(e) => {
                        eprintln!("{}: Failed to install {} ({}): {}", "Error".red(), p, event.hook_filename(), e);
                        provider_ok = false;
                    }
                }
            }
            if provider_ok {
                print_extra_installed(&base, p);
                any_installed = true;
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
            || agent_is_installed(&base, p, global, skill_names)
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
        let is_installed = agent_is_installed(&base, p, global, skill_names);
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
        if agent_is_installed(&base, p, global, skill_names) && !force {
            println!("{}: {} already installed", "Info".cyan(), p);
            print_agent_installed_info(&base, p, global, skill_names);
            continue;
        }
        warn_legacy_if_present(&base, p);
        let provider_name = format!("{}", p).to_lowercase();
        let mut provider_ok = true;
        for event in events {
            match install_agent_skill(&base, p, global, event, skill_names) {
                Ok(_) => {
                    let path = agent_skill_path(&base, p, global, event, skill_names);
                    println!("{} Installed {} ({}) → {}", "✓".green(), p, event.hook_filename(), path.display());
                    add_skill_provider_to_hook(scope, &project_str, event, &provider_name);
                }
                Err(e) => {
                    eprintln!("{}: Failed to install {} ({}): {}", "Error".red(), p, event.hook_filename(), e);
                    provider_ok = false;
                }
            }
        }
        if provider_ok {
            print_extra_installed(&base, p);
            any_installed = true;
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
/// When `global` is true, removes skills from the user home directory;
/// otherwise removes from the project git root.
fn handle_agent_hook_uninstall(yes: bool, global: bool, events: &[HookEvent]) -> ExitCode {
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

    // Load configured skill name aliases
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let skill_names_cfg = linthis::config::Config::load_merged(&project_root).hook.agent.skill_names;
    let skill_names = Some(&skill_names_cfg);

    // Find all installed providers in the target scope
    let installed: Vec<&AgentProvider> = ALL_AGENT_PROVIDERS
        .iter()
        .filter(|p| agent_is_installed(&base, p, global, skill_names))
        .collect();

    if installed.is_empty() {
        return ExitCode::from(1); // Nothing to uninstall
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
    let project_str = if global {
        String::new()
    } else {
        base.to_str().unwrap_or("").to_string()
    };

    let mut any_removed = false;
    for p in &installed {
        let mut provider_ok = true;
        let provider_name = format!("{}", p).to_lowercase();
        for event in events {
            let event_name = event.hook_filename();
            match uninstall_agent_skill(&base, p, global, event, skill_names) {
                Ok(_) => {
                    println!("{} Uninstalled {} ({}) skill", "✓".green(), p, event_name);
                    // Remove this provider from the skill_providers list in TOML
                    remove_skill_provider_from_hook(scope, &project_str, event, &provider_name);
                }
                Err(e) => {
                    eprintln!("{}: Failed to uninstall {} ({}): {}", "Error".red(), p, event_name, e);
                    provider_ok = false;
                }
            }
        }
        if provider_ok {
            uninstall_agent_legacy(&base, p);
            any_removed = true;
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
pub fn handle_commit_msg_check(msg_or_file: &str, auto_fix: bool, provider: Option<&str>) -> ExitCode {
    use linthis::config::Config;
    use regex::Regex;
    use std::fs;

    // Load config to get hooks settings
    let project_root = linthis::utils::get_project_root();
    let config = Config::load_merged(&project_root);

    // Accept either a file path or a raw message string
    let path = std::path::Path::new(msg_or_file);
    let is_file = path.exists();
    let commit_msg = if is_file {
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

    // Collect validation errors for auto-fix context
    let mut errors: Vec<String> = Vec::new();

    // Check main pattern
    if !regex.is_match(first_line) {
        errors.push(format!(
            "Does not match Conventional Commits format (type(scope)?: description). Valid types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert"
        ));
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
            errors.push(format!(
                "Missing ticket reference (pattern: {}). Example: feat: [PROJ-123] add feature",
                ticket_pattern
            ));
        }
    }

    if errors.is_empty() {
        // Box width matches the failure box (42 chars total, 40 inner dashes)
        println!("{}", "╭────────────────────────────────────────╮".green());
        println!("{}", "│ ✓ Linthis 📝 [Commit-msg] Hook Passed  │".green());
        println!("{}", "├────────────────────────────────────────┤".green());
        println!("{}", "│ Commit message is valid                │".green());
        println!("{}", "╰────────────────────────────────────────╯".green());
        // Print hook file paths (Global / Local) same as pre-commit success output
        let paths = linthis::utils::output::format_hook_paths_footer_pub(Some("commit-msg"));
        if !paths.is_empty() {
            println!("{}", paths);
        }
        return ExitCode::SUCCESS;
    }

    // Validation failed - try auto-fix if enabled
    if auto_fix {
        return handle_cmsg_auto_fix(
            &commit_msg,
            &errors,
            is_file,
            path,
            provider,
            config.ai.provider.as_deref(),
        );
    }

    // No auto-fix - print errors normally
    if errors.iter().any(|e| e.contains("Conventional Commits")) {
        print_commit_msg_error(first_line);
    } else {
        // Ticket reference error
        let ticket_pattern = config.cmsg.ticket_pattern.as_deref()
            .unwrap_or(r"\[\w+-\d+\]");
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
    ExitCode::from(1)
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
    use linthis::ai::{AiProvider, AiProviderConfig, AiProviderKind, AiProviderTrait};
    use std::fs;

    let provider_name = resolve_ai_provider(cli_provider, config_provider);
    let kind: AiProviderKind = match provider_name.parse() {
        Ok(k) => k,
        Err(_) => {
            eprintln!("{}: Unknown AI provider: {}", "Error".red(), provider_name);
            return ExitCode::from(2);
        }
    };

    let provider_config = match &kind {
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
    };
    let provider = AiProvider::new(provider_config);

    eprintln!(
        "{} Rewriting commit message with AI (provider: {})...",
        "→".cyan(),
        provider_name.cyan()
    );

    let first_line = original_msg.lines().next().unwrap_or("").trim();
    let rest_of_msg: String = original_msg.lines().skip(1).collect::<Vec<_>>().join("\n");

    let error_desc = errors.join("; ");
    let prompt = format!(
        "Rewrite the following git commit message to conform to the Conventional Commits format.\n\
         \n\
         Original message: {}\n\
         \n\
         Validation errors: {}\n\
         \n\
         Rules:\n\
         - Format: type(scope)?: description\n\
         - Valid types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert\n\
         - Keep the original intent and meaning\n\
         - Output ONLY the rewritten first line, nothing else (no quotes, no explanation)",
        first_line, error_desc
    );

    match provider.complete(&prompt, Some("You are a git commit message formatter. Output only the corrected commit message first line.")) {
        Ok(fixed_line) => {
            let fixed_line = fixed_line.trim().trim_matches('"').trim_matches('\'').trim();

            // Reassemble: fixed first line + rest of original message
            let fixed_msg = if rest_of_msg.is_empty() {
                format!("{}\n", fixed_line)
            } else {
                format!("{}\n{}", fixed_line, rest_of_msg)
            };

            if is_file {
                if let Err(e) = fs::write(file_path, &fixed_msg) {
                    eprintln!("{}: Failed to write fixed message: {}", "Error".red(), e);
                    return ExitCode::from(1);
                }
                eprintln!(
                    "{} Commit message rewritten: {} → {}",
                    "✓".green(),
                    first_line.dimmed(),
                    fixed_line.green()
                );
            } else {
                // Can't write back to a string arg, just print the fixed message
                eprintln!(
                    "{} Suggested rewrite: {}",
                    "✓".green(),
                    fixed_line.green()
                );
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
    eprintln!("{}", "╭────────────────────────────────────────╮".red());
    eprintln!("{}", "│ X Linthis 📝 [Commit-msg] Hook Failed  │".red());
    eprintln!("{}", "├────────────────────────────────────────┤".red());
    eprintln!("{}", "│ Validation Failed!                     │".red());
    eprintln!("│                                        │");
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
    // Show hook file paths (with type suffix extracted from thin wrapper)
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
/// `sh -c`, forwarding any passthrough arguments from the original git hook
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

fn handle_hook_run(
    event: &HookEvent,
    hook_type: &HookTool,
    raw_provider: Option<&str>,
    raw_provider_args: Option<&str>,
    _global: bool,
    hook_args: &[String],
) -> i32 {
    // Support provider/model syntax at runtime too (e.g. "claude/opus")
    let (provider_name, merged_pa) = if let Some(raw) = raw_provider {
        let (name, model) = parse_provider_with_model(raw);
        (Some(name), merge_model_into_provider_args(model, raw_provider_args))
    } else {
        (None, raw_provider_args.map(|s| s.to_string()))
    };
    let provider: Option<&str> = provider_name;
    let provider_args: Option<&str> = merged_pa.as_deref();

    // Detect re-entrant calls: the parent hook execution sets LINTHIS_HOOK_RUNNING_<pid>=1
    // before exec-ing the child hook.  If any such var is present, skip local delegation.
    let already_running = std::env::vars()
        .any(|(k, _)| k.starts_with(LINTHIS_HOOK_RUNNING_PREFIX));

    let script = match hook_type {
        HookTool::Git => {
            if already_running {
                // Re-entrant: we were delegated to from a parent hook invocation.
                // Run linthis directly without further local-hook delegation.
                let linthis_cmd = build_hook_command(event, &None);
                if matches!(event, HookEvent::PrePush) {
                    // For pre-push, $@ = remote name/url (NOT file paths).
                    // Compute the pushed files from git diff and pass with -i.
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
            } else {
                // First invocation: full script that handles local-hook delegation.
                build_global_hook_script_for_event(event, &None, None)
            }
        }
        HookTool::GitWithAgent => {
            let fix_provider = match provider
                .and_then(|p| match p.to_lowercase().as_str() {
                    "claude"    => Some(AgentFixProvider::Claude),
                    "codex"     => Some(AgentFixProvider::Codex),
                    "gemini"    => Some(AgentFixProvider::Gemini),
                    "cursor"    => Some(AgentFixProvider::Cursor),
                    "droid"     => Some(AgentFixProvider::Droid),
                    "auggie" | "aug" | "augment" => Some(AgentFixProvider::Auggie),
                    "codebuddy" => Some(AgentFixProvider::Codebuddy),
                    _ => None,
                })
                .or(Some(AgentFixProvider::Claude))
            {
                Some(p) => p,
                None => {
                    eprintln!("{}: hook run: unknown provider '{}'", "Error".red(), provider.unwrap_or(""));
                    return 1;
                }
            };
            let linthis_cmd = build_hook_command(event, &None);
            build_git_with_agent_hook_script(&linthis_cmd, &fix_provider, event, provider_args)
        }
        _ => {
            eprintln!("{}: hook run: unsupported hook type '{}' (supported: git, git-with-agent)", "Error".red(), hook_type.as_str());
            return 1;
        }
    };

    // Show where this hook's behavior is configured from (Tier 1/2/3)
    {
        let description = describe_hook_source(hook_type, event);
        eprintln!("{}", format!("📄 Config: {}", description).dimmed());
    }

    // Inject LINTHIS_HOOK_RUNNING_<pid>=1 so delegated child hooks can detect re-entry.
    // Using PID as part of the variable NAME (not value) ensures concurrent commit
    // operations don't interfere: each process has its own uniquely-named env var.
    let pid = std::process::id().to_string();
    let env_key = format!("{}{}", LINTHIS_HOOK_RUNNING_PREFIX, pid);

    // Execute the generated script, passing through git hook arguments
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(&script)
        .arg("--") // placeholder for $0 (script name)
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

/// Scan `hook_dir` for old-format linthis hook scripts, migrate each to a thin
/// wrapper, save metadata, and return the number of hooks migrated.
///
/// A file is considered a linthis hook if its content contains the marker
/// `# linthis-hook` or already calls `linthis hook run`.
fn detect_and_migrate_existing_hooks(hook_dir: &std::path::Path, global: bool, project: &str) -> usize {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    // Map filename → HookEvent
    let event_map: &[(&str, HookEvent)] = &[
        ("pre-commit", HookEvent::PreCommit),
        ("pre-push", HookEvent::PrePush),
        ("commit-msg", HookEvent::CommitMsg),
    ];

    let mut migrated = 0_usize;

    let entries = match fs::read_dir(hook_dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };

    for entry in entries.flatten() {
        let filename = entry.file_name();
        let name = match filename.to_str() {
            Some(n) => n,
            None => continue,
        };

        // Only consider known git hook filenames
        let event_opt = event_map.iter().find(|(n, _)| *n == name);
        let event = match event_opt {
            Some((_, e)) => e,
            None => continue,
        };

        let path = entry.path();
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Skip if not a linthis-managed hook (old format or thin wrapper)
        let is_old_format = content.contains("# linthis-hook");
        let is_thin_wrapper = content.contains("linthis hook run");
        if !is_old_format && !is_thin_wrapper {
            continue;
        }

        // If already a thin wrapper, just record metadata if missing
        if is_thin_wrapper {
            // Parse provider from existing thin wrapper: --provider <p>
            let provider_opt = content
                .split("--provider ")
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
                .map(|s| s.trim_end_matches('"').to_string());
            let hook_type = if content.contains("--type git-with-agent") {
                HookTool::GitWithAgent
            } else if content.contains("--type agent") {
                HookTool::Agent
            } else if content.contains("--type prek-with-agent") {
                HookTool::PrekWithAgent
            } else if content.contains("--type prek") {
                HookTool::Prek
            } else {
                HookTool::Git
            };
            let scope = if global { "global" } else { "local" };
            save_installed_hook(scope, project, event, &hook_type, provider_opt.as_deref(), None);
            println!("  {} recorded thin wrapper {} {} ({})", "✓".green(), name, hook_type.as_str(), scope);
            migrated += 1;
            continue;
        }

        // Old-format hook — detect hook_type from content heuristics
        // Presence of "start_timer" or agent-specific patterns → git-with-agent
        let has_agent = content.contains("start_timer")
            || content.contains("AGENT_PROVIDER")
            || content.contains("claude")
            || content.contains("codebuddy")
            || content.contains("codex");
        let hook_type = if has_agent {
            HookTool::GitWithAgent
        } else {
            HookTool::Git
        };

        // Try to detect provider from old script content
        let provider_opt: Option<&str> = if content.contains("codebuddy") {
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
        };

        // Build thin wrapper and overwrite the hook file
        let thin = build_thin_wrapper_script(event, &hook_type, provider_opt, global, None);
        match fs::write(&path, &thin) {
            Ok(_) => {
                #[cfg(unix)]
                {
                    if let Ok(meta) = fs::metadata(&path) {
                        let mut perms = meta.permissions();
                        perms.set_mode(0o755);
                        let _ = fs::set_permissions(&path, perms);
                    }
                }
                let scope = if global { "global" } else { "local" };
                save_installed_hook(scope, project, event, &hook_type, provider_opt, None);
                println!(
                    "  {} migrated {} → thin wrapper {} ({})",
                    "✓".green(),
                    name,
                    hook_type.as_str(),
                    scope
                );
                eprintln!(
                    "  {} Hook type inferred from old script content (heuristic). \
                     If incorrect, re-install with the right type:\n  \
                     linthis hook install{} --event {} --type <type> --force",
                    "⚠".yellow(),
                    if global { " -g" } else { "" },
                    event.as_str(),
                );
                migrated += 1;
            }
            Err(e) => {
                eprintln!("  {} Failed to migrate {}: {}", "✗".red(), name, e);
            }
        }
    }

    migrated
}

/// Re-sync installed hooks for local project or global scope.
///
/// Reads `~/.linthis/installed-hooks.toml` and re-generates thin wrapper
/// scripts and agent skill components for each recorded hook installation.
pub fn handle_hook_sync(global: bool, _yes: bool) -> i32 {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    let hooks_file = load_installed_hooks();
    let target_scope = if global { "global" } else { "local" };

    // Determine project root for local scope
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

    // Load configured skill name aliases
    let sync_project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let skill_names_cfg = linthis::config::Config::load_merged(&sync_project_root).hook.agent.skill_names;
    let skill_names = Some(&skill_names_cfg);

    let filtered: Vec<&InstalledHook> = hooks_file
        .hooks
        .iter()
        .filter(|h| h.scope == target_scope)
        .filter(|h| {
            if global {
                true
            } else {
                h.project.is_empty()
                    || h.project == project_root.to_str().unwrap_or("")
            }
        })
        .collect();

    if filtered.is_empty() {
        // No metadata recorded — try auto-detecting existing linthis hooks on disk
        // (hooks installed before the thin-wrapper / metadata feature was added).
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
            &hook_dir,
            global,
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
        return 0;
    }

    // Group entries by hook_type for structured output
    let type_order = ["agent", "git-with-agent", "prek-with-agent", "git", "prek"];
    let mut grouped: Vec<(&str, Vec<&&InstalledHook>)> = Vec::new();
    for ht in &type_order {
        let group: Vec<&&InstalledHook> = filtered.iter().filter(|h| h.hook_type == *ht).collect();
        if !group.is_empty() {
            grouped.push((ht, group));
        }
    }
    // Catch any hook_types not in type_order
    for hook in &filtered {
        if !type_order.contains(&hook.hook_type.as_str()) {
            let existing = grouped.iter().any(|(ht, _)| *ht == hook.hook_type.as_str());
            if !existing {
                let group: Vec<&&InstalledHook> = filtered.iter().filter(|h| h.hook_type == hook.hook_type).collect();
                grouped.push((hook.hook_type.as_str(), group));
            }
        }
    }

    println!("{} Syncing {} hook(s)...", "→".cyan(), filtered.len());

    let mut errors = 0_u32;

    let mut hook_index = 0_usize;

    for (group_type, group_hooks) in &grouped {
    println!();
    println!("{} Type: {} ({} hook{})", "→".cyan(), group_type.cyan(), group_hooks.len(), if group_hooks.len() == 1 { "" } else { "s" });

    for hook in group_hooks {
        // Parse event and hook_type back from stored strings
        let event = match hook.event.as_str() {
            "pre-commit" => HookEvent::PreCommit,
            "pre-push"   => HookEvent::PrePush,
            "commit-msg" => HookEvent::CommitMsg,
            other => {
                eprintln!("  {} Unknown event '{}', skipping", "✗".red(), other);
                errors += 1;
                continue;
            }
        };
        let hook_type = match hook.hook_type.as_str() {
            "git"                  => HookTool::Git,
            "git-with-agent"       => HookTool::GitWithAgent,
            "agent"                => HookTool::Agent,
            "prek"                 => HookTool::Prek,
            "prek-with-agent"      => HookTool::PrekWithAgent,
            other => {
                eprintln!("  {} Unknown hook type '{}', skipping", "✗".red(), other);
                errors += 1;
                continue;
            }
        };
        let provider_opt: Option<&str> = if hook.provider.is_empty() {
            None
        } else {
            Some(&hook.provider)
        };

        // 1. Re-generate thin wrapper git hook script
        let hook_dir = if global {
            match global_hooks_dir() {
                Some(d) => d,
                None => {
                    eprintln!("  {} Could not determine global hooks directory", "✗".red());
                    errors += 1;
                    continue;
                }
            }
        } else {
            project_root.join(".git/hooks")
        };

        // Only re-write thin wrapper for types that have one
        if !matches!(hook_type, HookTool::Agent | HookTool::Prek) {
            let hook_file = hook_dir.join(event.hook_filename());
            let pa_opt: Option<&str> = if hook.provider_args.is_empty() { None } else { Some(&hook.provider_args) };
            let thin_script = build_thin_wrapper_script(&event, &hook_type, provider_opt, global, pa_opt);
            if let Some(parent) = hook_file.parent() {
                let _ = fs::create_dir_all(parent);
            }
            match fs::write(&hook_file, &thin_script) {
                Ok(_) => {
                    #[cfg(unix)]
                    {
                        if let Ok(meta) = fs::metadata(&hook_file) {
                            let mut perms = meta.permissions();
                            perms.set_mode(0o755);
                            let _ = fs::set_permissions(&hook_file, perms);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("  {} Failed to write {}: {}", "✗".red(), hook_file.display(), e);
                    errors += 1;
                    continue;
                }
            }
        }

        // Print summary line FIRST (总), then details (分)
        hook_index += 1;
        let mut details = Vec::new();
        details.push(target_scope.to_string());
        if let Some(fp) = provider_opt {
            if !fp.is_empty() {
                details.push(format!("fix: {}", fp));
            }
        }
        if !hook.skill_providers.is_empty() {
            details.push(format!("skills: {}", hook.skill_providers.join(",")));
        }
        println!(
            "  {}. {} synced {} {} ({})",
            hook_index,
            "✓".green(),
            hook.event,
            hook.hook_type,
            details.join(", ")
        );

        // 2. Re-sync agent skills (only for "agent" type entries).
        //    git-with-agent / prek-with-agent entries only sync thin wrappers;
        //    their agent skills are tracked via separate "agent" TOML entries.
        if matches!(hook_type, HookTool::Agent) {
            let base = if global {
                dirs::home_dir().unwrap_or_default()
            } else {
                project_root.clone()
            };

            // Build target list from TOML skill_providers
            let mut skill_targets: Vec<AgentProvider> = hook.skill_providers.iter().filter_map(|name| {
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
            }).collect();

            // Backward compatibility: if no skill_providers recorded, fall back to fix provider
            if skill_targets.is_empty() {
                if let Some(fb) = provider_opt.and_then(|p| match p.to_lowercase().as_str() {
                    "claude"    => Some(AgentProvider::Claude),
                    "codex"     => Some(AgentProvider::Codex),
                    "gemini"    => Some(AgentProvider::Gemini),
                    "cursor"    => Some(AgentProvider::Cursor),
                    "droid"     => Some(AgentProvider::Droid),
                    "auggie" | "aug" | "augment" => Some(AgentProvider::Auggie),
                    "codebuddy" => Some(AgentProvider::Codebuddy),
                    "openclaw"  => Some(AgentProvider::Openclaw),
                    _ => None,
                }) {
                    skill_targets.push(fb);
                }
            }

            for provider in &skill_targets {
                let skill_path = agent_skill_path(&base, provider, global, &event, skill_names);
                if let Err(e) = install_agent_skill(&base, provider, global, &event, skill_names) {
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
        }
    } // end for hook in group_hooks
    } // end for (group_type, group_hooks) in &grouped

    // ── Disk-scan pass: refresh skills for providers not in the TOML ────────────
    // Covers backward-compat: if the user previously installed codebuddy skills
    // then reinstalled with claude (overwriting the TOML entry), the skill files
    // on disk still exist but no TOML entry records them.  Refresh any we find.
    let base_for_scan = if global {
        dirs::home_dir().unwrap_or_default()
    } else {
        project_root.clone()
    };
    let all_scan_providers = [
        AgentProvider::Claude,
        AgentProvider::Codebuddy,
        AgentProvider::Gemini,
        AgentProvider::Cursor,
        AgentProvider::Droid,
        AgentProvider::Auggie,
    ];
    let all_scan_events = [HookEvent::PreCommit, HookEvent::CommitMsg, HookEvent::PrePush];
    for scan_event in &all_scan_events {
        for scan_provider in &all_scan_providers {
            let skill_path = agent_skill_path(&base_for_scan, scan_provider, global, scan_event, skill_names);
            if !skill_path.exists() {
                continue;
            }
            // Already handled above (registered in TOML skill_providers)? skip to avoid double output.
            let provider_name_lower = format!("{}", scan_provider).to_lowercase();
            let already_registered = filtered.iter().any(|h| {
                h.event == scan_event.as_str()
                    && matches!(h.hook_type.as_str(), "git-with-agent" | "agent" | "prek-with-agent")
                    && h.skill_providers.iter().any(|sp| sp.to_lowercase() == provider_name_lower)
            });
            if already_registered {
                continue;
            }
            // Unregistered but exists on disk — refresh silently
            if let Err(e) = install_agent_skill(&base_for_scan, scan_provider, global, scan_event, skill_names) {
                eprintln!("  {} skill refresh error ({:?}/{}): {}", "⚠".yellow(), scan_provider, scan_event.as_str(), e);
            }
        }
    }
    // ── End disk-scan pass ──────────────────────────────────────────────────────

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
        install_agent_plugin_from_dir(pd, base.path(), &AgentProvider::Claude, &HookEvent::PreCommit, None).unwrap();

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
        install_agent_plugin_from_dir(pd, base.path(), &AgentProvider::Claude, &HookEvent::PreCommit, None).unwrap();

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
        install_agent_plugin_from_dir(pd, base.path(), &AgentProvider::Gemini, &HookEvent::PreCommit, None).unwrap();

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
        install_agent_plugin_from_dir(pd, base.path(), &AgentProvider::Claude, &HookEvent::PreCommit, None).unwrap();

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
