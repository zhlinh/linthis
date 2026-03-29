// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Installed hooks metadata (persisted to ~/.linthis/installed-hooks.toml).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::dirs;
use crate::cli::commands::{HookEvent, HookTool};

/// Record of a single installed hook (stored in installed-hooks.toml).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct InstalledHook {
    /// "local" or "global"
    pub scope: String,
    /// Absolute path to git repo root (empty for global scope)
    pub project: String,
    /// Hook event name (e.g. "pre-commit", "commit-msg")
    pub event: String,
    /// Hook tool name (e.g. "git", "git-with-agent")
    pub hook_type: String,
    /// AI provider name for the fix fallback (empty string if none)
    pub provider: String,
    /// All agent providers that have skills installed for this hook.
    /// Used by `hook sync` to know which providers to refresh.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skill_providers: Vec<String>,
    /// Extra arguments passed verbatim to the AI agent CLI (e.g. "--model opus").
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub provider_args: String,
}

/// Top-level structure of ~/.linthis/installed-hooks.toml.
#[derive(Serialize, Deserialize, Default, Debug)]
pub(crate) struct InstalledHooksFile {
    #[serde(default)]
    pub hooks: Vec<InstalledHook>,
}

/// Returns the path to the installed-hooks.toml file.
fn installed_hooks_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".linthis").join("installed-hooks.toml"))
}

/// Load the installed-hooks.toml file (returns empty struct if missing or unreadable).
pub(crate) fn load_installed_hooks() -> InstalledHooksFile {
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
pub(crate) fn save_installed_hook(
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
pub(crate) fn add_skill_provider_to_hook(
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
pub(crate) fn remove_installed_hook(scope: &str, project: &str, event: &HookEvent) {
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
pub(crate) fn remove_skill_provider_from_hook(
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
pub(crate) fn deduplicate_hook_types(types: Vec<HookTool>) -> Vec<HookTool> {
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
pub(crate) fn deduplicate_hook_events(events: Vec<HookEvent>) -> Vec<HookEvent> {
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
pub(crate) fn apply_yes_fallback(
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
