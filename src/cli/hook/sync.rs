// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! `linthis hook sync` — re-sync installed hooks from persisted metadata.

use colored::Colorize;
use std::path::PathBuf;

use super::agent::{agent_skill_path, agent_stop_hook_settings_path, install_agent_skill};
use super::metadata::{load_installed_hooks, save_installed_hook, InstalledHook};
use super::script::build_thin_wrapper_script;
use super::{dirs, find_git_root, global_hooks_dir, write_hook_script};
use crate::cli::commands::{AgentProvider, HookEvent, HookTool};

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
    save_installed_hook(
        scope,
        project,
        event,
        &hook_type,
        provider_opt.as_deref(),
        None,
    );
    println!(
        "  {} recorded thin wrapper {} {} ({})",
        "✓".green(),
        name,
        hook_type.as_str(),
        scope
    );
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
    let hook_type = if old_hook_has_agent(content) {
        HookTool::GitWithAgent
    } else {
        HookTool::Git
    };
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
    true
}

/// Scan `hook_dir` for old-format linthis hook scripts, migrate each to a thin
/// wrapper, save metadata, and return the number of hooks migrated.
pub(crate) fn detect_and_migrate_existing_hooks(
    hook_dir: &std::path::Path,
    global: bool,
    project: &str,
) -> usize {
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
        "pre-push" => Some(HookEvent::PrePush),
        "commit-msg" => Some(HookEvent::CommitMsg),
        _ => None,
    }
}

/// Parse hook type string back to HookTool enum.
fn parse_hook_tool(s: &str) -> Option<HookTool> {
    match s {
        "git" => Some(HookTool::Git),
        "git-with-agent" => Some(HookTool::GitWithAgent),
        "agent" => Some(HookTool::Agent),
        "prek" => Some(HookTool::Prek),
        "prek-with-agent" => Some(HookTool::PrekWithAgent),
        _ => None,
    }
}

/// Parse a provider name string to AgentProvider.
fn parse_sync_agent_provider(name: &str) -> Option<AgentProvider> {
    match name.to_lowercase().as_str() {
        "claude" => Some(AgentProvider::Claude),
        "codex" => Some(AgentProvider::Codex),
        "gemini" => Some(AgentProvider::Gemini),
        "cursor" => Some(AgentProvider::Cursor),
        "droid" => Some(AgentProvider::Droid),
        "auggie" | "aug" | "augment" => Some(AgentProvider::Auggie),
        "codebuddy" => Some(AgentProvider::Codebuddy),
        "openclaw" => Some(AgentProvider::Openclaw),
        _ => None,
    }
}

/// Group filtered hooks by hook_type for structured output.
fn group_hooks_by_type<'a>(
    filtered: &'a [&'a InstalledHook],
) -> Vec<(&'a str, Vec<&'a &'a InstalledHook>)> {
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
                let group: Vec<&&InstalledHook> = filtered
                    .iter()
                    .filter(|h| h.hook_type == hook.hook_type)
                    .collect();
                grouped.push((hook.hook_type.as_str(), group));
            }
        }
    }
    grouped
}

/// Re-write a thin wrapper hook script for a single hook entry.
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
    let pa_opt: Option<&str> = if hook.provider_args.is_empty() {
        None
    } else {
        Some(&hook.provider_args)
    };
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
    let base = if global {
        dirs::home_dir().unwrap_or_default()
    } else {
        project_root.to_path_buf()
    };

    let mut skill_targets: Vec<AgentProvider> = hook
        .skill_providers
        .iter()
        .filter_map(|name| parse_sync_agent_provider(name))
        .collect();

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
        println!(
            "     {} {} skill → {}",
            "↳".dimmed(),
            provider,
            skill_path.display()
        );
        if let Some(cmd_dir) = agent_command_dir(&base, provider) {
            if cmd_dir.exists() {
                println!(
                    "     {} {} command → {}",
                    "↳".dimmed(),
                    provider,
                    cmd_dir.display()
                );
            }
        }
        if matches!(event, HookEvent::PreCommit) {
            if let Some(settings_path) = agent_stop_hook_settings_path(&base, provider) {
                if settings_path.exists() {
                    println!(
                        "     {} {} stop hook → {}",
                        "↳".dimmed(),
                        provider,
                        settings_path.display()
                    );
                }
            }
        }
    }
    errors
}

/// Target directory for agent slash commands per provider (sync-local copy).
fn agent_command_dir(
    base: &std::path::Path,
    provider: &AgentProvider,
) -> Option<std::path::PathBuf> {
    match provider {
        AgentProvider::Claude => Some(base.join(".claude/commands/linthis")),
        AgentProvider::Codebuddy => Some(base.join(".codebuddy/commands/linthis")),
        AgentProvider::Gemini => Some(base.join(".gemini/commands")),
        AgentProvider::Cursor => Some(base.join(".cursor/commands")),
        AgentProvider::Droid => Some(base.join(".droid/commands")),
        AgentProvider::Auggie => Some(base.join(".augment/commands")),
        AgentProvider::Codex => None,
        AgentProvider::Openclaw => Some(base.join(".openclaw/commands")),
    }
}

/// Disk-scan pass: refresh skills for providers that exist on disk but aren't in the TOML.
fn sync_disk_scan_pass(
    base: &std::path::Path,
    global: bool,
    filtered: &[&InstalledHook],
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) {
    let all_scan_providers = [
        AgentProvider::Claude,
        AgentProvider::Codebuddy,
        AgentProvider::Gemini,
        AgentProvider::Cursor,
        AgentProvider::Droid,
        AgentProvider::Auggie,
    ];
    let all_scan_events = [
        HookEvent::PreCommit,
        HookEvent::CommitMsg,
        HookEvent::PrePush,
    ];
    for scan_event in &all_scan_events {
        for scan_provider in &all_scan_providers {
            let skill_path = agent_skill_path(base, scan_provider, global, scan_event, skill_names);
            if !skill_path.exists() {
                continue;
            }
            let provider_name_lower = format!("{}", scan_provider).to_lowercase();
            let already_registered = filtered.iter().any(|h| {
                h.event == scan_event.as_str()
                    && matches!(
                        h.hook_type.as_str(),
                        "git-with-agent" | "agent" | "prek-with-agent"
                    )
                    && h.skill_providers
                        .iter()
                        .any(|sp| sp.to_lowercase() == provider_name_lower)
            });
            if already_registered {
                continue;
            }
            if let Err(e) =
                install_agent_skill(base, scan_provider, global, scan_event, skill_names)
            {
                eprintln!(
                    "  {} skill refresh error ({:?}/{}): {}",
                    "⚠".yellow(),
                    scan_provider,
                    scan_event.as_str(),
                    e
                );
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
                eprintln!(
                    "{}: Could not determine global hooks directory",
                    "Error".red()
                );
                return 1;
            }
        }
    } else {
        project_root.join(".git/hooks")
    };

    let detected = detect_and_migrate_existing_hooks(
        &hook_dir,
        global,
        if global {
            ""
        } else {
            project_root.to_str().unwrap_or("")
        },
    );

    if detected == 0 {
        if global {
            println!("No global linthis hooks found to sync.");
            println!(
                "  Run {} to install global hooks",
                "linthis hook install -g".cyan()
            );
        } else {
            println!("No local linthis hooks found for this project.");
            println!(
                "  Run {} to install and record hooks",
                "linthis hook install".cyan()
            );
            println!(
                "  Use {} to sync global hooks.",
                "linthis hook sync -g".cyan()
            );
        }
    }
    0
}

/// Sync a single hook entry.
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
        None => {
            eprintln!("  {} Unknown event '{}', skipping", "✗".red(), hook.event);
            return 1;
        }
    };
    let hook_type = match parse_hook_tool(&hook.hook_type) {
        Some(t) => t,
        None => {
            eprintln!(
                "  {} Unknown hook type '{}', skipping",
                "✗".red(),
                hook.hook_type
            );
            return 1;
        }
    };
    let prov_str: &str = &hook.provider;
    let provider_opt: Option<&str> = if prov_str.is_empty() {
        None
    } else {
        Some(prov_str)
    };

    if sync_thin_wrapper(hook, &event, &hook_type, provider_opt, global, project_root).is_err() {
        return 1;
    }

    *hook_index += 1;
    let mut details = vec![target_scope.to_string()];
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

    if matches!(hook_type, HookTool::Agent) {
        errors += sync_agent_skills(
            hook,
            &event,
            provider_opt,
            global,
            project_root,
            skill_names,
        );
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
    let skill_names_cfg = linthis::config::Config::load_merged(&sync_project_root)
        .hook
        .agent
        .skill_names;
    let skill_names = Some(&skill_names_cfg);

    let filtered: Vec<&InstalledHook> = hooks_file
        .hooks
        .iter()
        .filter(|h| h.scope == target_scope)
        .filter(|h| {
            global || h.project.is_empty() || h.project == project_root.to_str().unwrap_or("")
        })
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
        println!(
            "{} Type: {} ({} hook{})",
            "→".cyan(),
            gt.cyan(),
            gh_len,
            if gh_len == 1 { "" } else { "s" }
        );

        for hook in group_hooks {
            errors += sync_single_hook(
                hook,
                &mut hook_index,
                target_scope,
                global,
                &project_root,
                skill_names,
            );
        }
    }

    let base_for_scan = if global {
        dirs::home_dir().unwrap_or_default()
    } else {
        project_root.clone()
    };
    sync_disk_scan_pass(&base_for_scan, global, &filtered, skill_names);

    if errors > 0 {
        eprintln!("{} {} error(s) during sync", "⚠".yellow(), errors);
        1
    } else {
        println!("{} Hook sync complete", "✓".green());
        0
    }
}

/// Called automatically after `linthis plugin sync` to refresh agent skill files.
pub fn handle_hook_sync_after_plugin_sync(global: bool) {
    let code = handle_hook_sync(global, true);
    if code != 0 {
        eprintln!(
            "{}: Agent hook sync encountered errors (exit {})",
            "Warning".yellow(),
            code
        );
    }
}
