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

mod agent;
mod cmsg;
mod config;
mod install;
mod metadata;
mod run;
mod script;
mod status;
mod sync;
mod uninstall;

use colored::Colorize;
use std::path::PathBuf;
use std::process::ExitCode;

use super::commands::HookCommands;

// Re-exports: public API
pub use agent::detect_agent_providers_lightweight;
pub use cmsg::handle_commit_msg_check;
pub use sync::{handle_hook_sync, handle_hook_sync_after_plugin_sync};

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
            let (hook_types, hook_events) = match install::resolve_install_types_events(hook_types, hook_events, yes) {
                Ok(r) => r,
                Err(code) => return code,
            };
            install::handle_hook_install(hook_types, hook_events, force, yes, global, provider, args, provider_args)
        }
        HookCommands::Uninstall { hook_types, hook_events, all, all_types, all_events, yes, global } => {
            let skip_prompt = all || all_types || all_events || yes;
            let (types, events) = match install::resolve_uninstall_types_events(hook_types, hook_events, skip_prompt) {
                Ok(r) => r,
                Err(code) => return code,
            };
            uninstall::handle_hook_uninstall(types, events, all, all_types, all_events, yes, global)
        }
        HookCommands::Status => status::handle_hook_status(),
        HookCommands::List { global } => status::handle_hook_list(global),
        HookCommands::Check => status::handle_hook_check(),
        HookCommands::CommitMsgCheck { msg_or_file } => handle_commit_msg_check(&msg_or_file, false, None),
        HookCommands::Run { event, hook_type, provider, provider_args, global, hook_args } => {
            let code = run::handle_hook_run(&event, &hook_type, provider.as_deref(), provider_args.as_deref(), global, &hook_args);
            ExitCode::from(code as u8)
        }
        HookCommands::Sync { global, yes } => {
            let code = handle_hook_sync(global, yes);
            ExitCode::from(code as u8)
        }
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

        match current_dir.parent() {
            Some(parent) => current_dir = parent.to_path_buf(),
            None => return None,
        }
    }
}

/// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    std::process::Command::new("which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Global hooks directory (XDG standard).
fn global_hooks_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".config/git/hooks"))
}

/// Check if a hook file is a linthis-managed hook.
fn is_linthis_hook_file(hook_path: &std::path::Path) -> bool {
    std::fs::read_to_string(hook_path)
        .map(|c| c.contains("# linthis-hook") || c.contains("linthis hook run"))
        .unwrap_or(false)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::{AgentProvider, HookEvent, HookTool};
    use super::metadata::{deduplicate_hook_types, deduplicate_hook_events, apply_yes_fallback};
    use super::agent::{
        agent_skill_path, agent_event_skill_metadata, install_agent_plugin_from_dir,
        copy_dir_recursive, ALL_AGENT_PROVIDERS,
    };
    use super::script::build_hook_command;

    fn agent_plugin_id(_event: &HookEvent) -> &'static str {
        "lt"
    }

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
        assert_eq!(resolved_events.len(), 3);
    }

    #[test]
    fn test_fallback_mixed_yes() {
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
        let types = vec![HookTool::Git];
        let events: Vec<HookEvent> = vec![];
        let (resolved_types, resolved_events) = apply_yes_fallback(types, events);
        assert_eq!(resolved_types, vec![HookTool::Git]);
        assert_eq!(resolved_events, vec![HookEvent::PreCommit]);
    }

    #[test]
    fn test_lint_content_contains_linthis_s() {
        let content = super::agent::agent_event_content_generic_test(&HookEvent::PreCommit);
        assert!(content.contains("linthis -s"));
    }

    #[test]
    fn test_cmsg_content_contains_linthis_cmsg() {
        let content = super::agent::agent_event_content_generic_test(&HookEvent::CommitMsg);
        assert!(content.contains("linthis cmsg"));
        assert!(content.contains("feat"));
    }

    #[test]
    fn test_review_content_contains_git_diff() {
        let content = super::agent::agent_event_content_generic_test(&HookEvent::PrePush);
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
        let p = agent_skill_path(&base, &AgentProvider::Claude, false, &HookEvent::PreCommit, Some(&cfg));
        assert_eq!(p, PathBuf::from("/repo/.claude/skills/custom-lint/SKILL.md"));
        let p = agent_skill_path(&base, &AgentProvider::Claude, false, &HookEvent::CommitMsg, Some(&cfg));
        assert_eq!(p, PathBuf::from("/repo/.claude/skills/lt-cmsg/SKILL.md"));
        let p = agent_skill_path(&base, &AgentProvider::Gemini, false, &HookEvent::PrePush, Some(&cfg));
        assert_eq!(p, PathBuf::from("/repo/.gemini/my-review.md"));
        let p = agent_skill_path(&base, &AgentProvider::Codebuddy, false, &HookEvent::PreCommit, Some(&cfg));
        assert_eq!(p, PathBuf::from("/repo/.codebuddy/skills/custom-lint/SKILL.md"));
        let p = agent_skill_path(&base, &AgentProvider::Cursor, false, &HookEvent::PreCommit, Some(&cfg));
        assert_eq!(p, PathBuf::from("/repo/.cursor/rules/custom-lint.mdc"));
    }

    #[test]
    fn test_agent_plugin_id_unified() {
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

        let result = resolver::fixed_agent_plugin_dir(root.path(), "claude", "lt");
        assert_eq!(result, Some(claude_dir));

        let result2 = resolver::fixed_agent_plugin_dir(root.path(), "codebuddy", "lt");
        assert_eq!(result2, Some(default_dir));
    }

    #[test]
    fn test_fixed_agent_plugin_dir_not_found() {
        use linthis::hooks::resolver;

        let root = tempfile::TempDir::new().unwrap();
        let result = resolver::fixed_agent_plugin_dir(root.path(), "claude", "lt");
        assert!(result.is_none());
    }

    #[test]
    fn test_install_agent_plugin_from_dir_skill_and_command() {
        let plugin_root = tempfile::TempDir::new().unwrap();
        let pd = plugin_root.path();

        let skill_dir = pd.join("skills/lt-lint");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "---\nname: lt-lint\n---\n# Test Skill\n").unwrap();

        let cmd_dir = pd.join("commands");
        std::fs::create_dir_all(&cmd_dir).unwrap();
        std::fs::write(cmd_dir.join("lt-lint.md"), "# /lt-lint\nRun lint.\n").unwrap();

        let mem_dir = pd.join("memories");
        std::fs::create_dir_all(&mem_dir).unwrap();
        std::fs::write(mem_dir.join("TOPLEVEL.md"), "## Linthis Memory\nRemember this.\n").unwrap();

        let base = tempfile::TempDir::new().unwrap();
        install_agent_plugin_from_dir(pd, base.path(), &AgentProvider::Claude, &HookEvent::PreCommit, None, None).unwrap();

        let skill_target = base.path().join(".claude/skills/lt-lint/SKILL.md");
        assert!(skill_target.exists(), "SKILL.md should be installed");
        let content = std::fs::read_to_string(&skill_target).unwrap();
        assert!(content.contains("lt-lint"), "Skill content should be preserved");

        let cmd_target = base.path().join(".claude/commands/linthis/lt-lint.md");
        assert!(cmd_target.exists(), "Command file should be installed");

        let claude_md = base.path().join("CLAUDE.md");
        assert!(claude_md.exists(), "CLAUDE.md should be created");
        let mem_content = std::fs::read_to_string(&claude_md).unwrap();
        assert!(mem_content.contains("linthis-memory-lt"), "Memory section marker should exist");
        assert!(mem_content.contains("Linthis Memory"), "Memory content should be injected");
    }

    #[test]
    fn test_install_agent_plugin_from_dir_with_subdirs() {
        let plugin_root = tempfile::TempDir::new().unwrap();
        let pd = plugin_root.path();

        let skill_dir = pd.join("skills/lt-lint");
        std::fs::create_dir_all(skill_dir.join("scripts")).unwrap();
        std::fs::create_dir_all(skill_dir.join("references")).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "---\nname: lt-lint\n---\n# Skill\n").unwrap();
        std::fs::write(skill_dir.join("scripts/check.sh"), "#!/bin/bash\necho ok\n").unwrap();
        std::fs::write(skill_dir.join("references/rules.md"), "# Rules\n").unwrap();

        let base = tempfile::TempDir::new().unwrap();
        install_agent_plugin_from_dir(pd, base.path(), &AgentProvider::Claude, &HookEvent::PreCommit, None, None).unwrap();

        let target_dir = base.path().join(".claude/skills/lt-lint");
        assert!(target_dir.join("SKILL.md").exists(), "SKILL.md should exist");
        assert!(target_dir.join("scripts/check.sh").exists(), "scripts/check.sh should be copied");
        assert!(target_dir.join("references/rules.md").exists(), "references/rules.md should be copied");

        let script = std::fs::read_to_string(target_dir.join("scripts/check.sh")).unwrap();
        assert!(script.contains("echo ok"));
    }

    #[test]
    fn test_install_agent_plugin_from_dir_single_file_provider() {
        let plugin_root = tempfile::TempDir::new().unwrap();
        let pd = plugin_root.path();

        let skill_dir = pd.join("skills/lt-lint");
        std::fs::create_dir_all(skill_dir.join("scripts")).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# Gemini Skill Content\n").unwrap();
        std::fs::write(skill_dir.join("scripts/check.sh"), "#!/bin/bash\n").unwrap();

        let base = tempfile::TempDir::new().unwrap();
        install_agent_plugin_from_dir(pd, base.path(), &AgentProvider::Gemini, &HookEvent::PreCommit, None, None).unwrap();

        let target = base.path().join(".gemini/linthis-lint.md");
        assert!(target.exists(), "Gemini skill file should exist");
        let content = std::fs::read_to_string(&target).unwrap();
        assert!(content.contains("Gemini Skill Content"));

        assert!(!base.path().join(".gemini/scripts").exists());
    }

    #[test]
    fn test_install_agent_plugin_from_dir_hooks_json() {
        let plugin_root = tempfile::TempDir::new().unwrap();
        let pd = plugin_root.path();

        let skill_dir = pd.join("skills/lt-lint");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "---\nname: lt-lint\n---\n# Skill\n").unwrap();

        let hooks_dir = pd.join("hooks");
        std::fs::create_dir_all(&hooks_dir).unwrap();
        std::fs::write(hooks_dir.join("hooks.json"), r#"{"hooks":{"Stop":[{"hooks":[{"type":"prompt","prompt":"test stop hook"}]}]}}"#).unwrap();

        let base = tempfile::TempDir::new().unwrap();
        install_agent_plugin_from_dir(pd, base.path(), &AgentProvider::Claude, &HookEvent::PreCommit, None, None).unwrap();

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
