// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Hook status, list, and check commands.

use colored::Colorize;
use std::path::PathBuf;
use std::process::ExitCode;

use super::agent::{
    agent_is_installed, agent_skill_path, agent_stop_hook_settings_path, ALL_AGENT_PROVIDERS,
};
use super::metadata::{load_installed_hooks, InstalledHooksFile};
use super::{dirs, find_git_root, global_hooks_dir, is_linthis_hook_file};
use crate::cli::commands::HookEvent;

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
}

/// Print global hook status section.
fn print_global_hook_status(hook_events: &[HookEvent]) {
    println!();
    println!("{}", "Global Hooks (~/.config/git/hooks/):".bold());
    let core_hooks_path = std::process::Command::new("git")
        .args(["config", "--global", "core.hooksPath"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        });

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
                    println!("    {} Strategy: local hook takes priority", "ℹ".dimmed());
                } else {
                    println!(
                        "{} {} [global, not by linthis]",
                        "⚠".yellow(),
                        hook_path.display()
                    );
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
    let events = [
        HookEvent::PreCommit,
        HookEvent::CommitMsg,
        HookEvent::PrePush,
    ];
    let mut any_installed = false;
    for p in ALL_AGENT_PROVIDERS {
        if agent_is_installed(git_root, p, false, skill_names) {
            any_installed = true;
            println!("{} {}", "✓".green(), p);
            for event in &events {
                let path = agent_skill_path(git_root, p, false, event, skill_names);
                if path.exists() {
                    println!(
                        "  {} {} ({})",
                        "✓".green().dimmed(),
                        path.display(),
                        event.hook_filename()
                    );
                }
            }
            if let Some(settings_path) = agent_stop_hook_settings_path(git_root, p) {
                let has_stop_hook = settings_path.exists()
                    && std::fs::read_to_string(&settings_path)
                        .map(|c| c.contains("linthis"))
                        .unwrap_or(false);
                if has_stop_hook {
                    println!(
                        "  {} Stop Hook ({})",
                        "✓".green().dimmed(),
                        settings_path.display()
                    );
                }
            }
        } else {
            println!("{} {} (not installed)", "✗".red(), p);
        }
    }
    any_installed
}

/// Show git hook status
pub(crate) fn handle_hook_status() -> ExitCode {
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

    let hook_events = [
        HookEvent::PreCommit,
        HookEvent::PrePush,
        HookEvent::CommitMsg,
    ];

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
    println!(
        "  {} - validates commit message format",
        "commit-msg".cyan()
    );

    let skill_names_cfg = linthis::config::Config::load_merged(&git_root)
        .hook
        .agent
        .skill_names;
    let any_agent_installed = print_agent_status(&git_root, Some(&skill_names_cfg));

    println!("\n{}", "Commands:".bold());
    if !any_hook_installed {
        println!("  Install pre-commit:  {}", "linthis hook install".cyan());
        println!(
            "  Install pre-push:    {}",
            "linthis hook install --event pre-push".cyan()
        );
        println!(
            "  Install commit-msg:  {}",
            "linthis hook install --event commit-msg".cyan()
        );
    } else {
        println!(
            "  Install hook:   {}",
            "linthis hook install --event <event>".cyan()
        );
        println!(
            "  Uninstall hook: {}",
            "linthis hook uninstall --event <event>".cyan()
        );
        println!(
            "  Uninstall all:  {}",
            "linthis hook uninstall --all".cyan()
        );
    }
    if !any_agent_installed {
        println!(
            "  Install agent:  {}",
            "linthis hook install --type agent".cyan()
        );
    } else {
        println!(
            "  Install agent:  {}",
            "linthis hook install --type agent --provider <name>".cyan()
        );
        println!(
            "  Uninstall all:  {}",
            "linthis hook uninstall --all".cyan()
        );
    }

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
    if has_agent
        && (content.contains("claude")
            || content.contains("codex")
            || content.contains("openclaw")
            || content.contains("gemini")
            || content.contains("codebuddy")
            || content.contains("droid")
            || content.contains("auggie")
            || content.contains("cursor-agent"))
    {
        return "git-with-agent".to_string();
    }

    // Fall back to TOML registry
    let project_str = project.to_string_lossy();
    let event_str = event.hook_filename();
    for hook in &toml.hooks {
        if hook.scope == scope
            && hook.event == event_str
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
        if content.contains(pattern)
            && (content.contains("_LINTHIS_AGENT_OK") || content.contains("agent"))
        {
            return name.to_string();
        }
    }

    // Fall back to TOML registry
    let project_str = project.to_string_lossy();
    let event_str = event.hook_filename();
    for hook in &toml.hooks {
        if hook.scope == scope
            && hook.event == event_str
            && (scope == "global" || hook.project == project_str.as_ref())
        {
            return hook.provider.clone();
        }
    }

    String::new()
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
            if provider.is_empty() {
                String::new()
            } else {
                format!("(provider: {})", provider)
            }
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
            .map(|sp| {
                sp.exists()
                    && std::fs::read_to_string(&sp)
                        .map(|c| c.contains("linthis"))
                        .unwrap_or(false)
            })
            .unwrap_or(false);
        println!(
            "  {} {} [{}]{}",
            "✓".green(),
            p,
            event_tags.join(", "),
            if stop_hook {
                format!(" + {}", "stop-hook".dimmed())
            } else {
                String::new()
            }
        );
    }
    if !any {
        let label = if global {
            "No global agent skills installed"
        } else {
            "No agent skills installed"
        };
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
            println!(
                "  Use {} to view project hooks.",
                "linthis hook list".cyan()
            );
        } else {
            println!("No project hooks installed.");
            println!(
                "  Use {} to view global hooks.",
                "linthis hook list -g".cyan()
            );
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
pub(crate) fn handle_hook_list(global: bool) -> ExitCode {
    let toml = load_installed_hooks();

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let skill_names_cfg = linthis::config::Config::load_merged(&cwd)
        .hook
        .agent
        .skill_names;
    let skill_names = Some(&skill_names_cfg);

    let hook_events = [
        HookEvent::PreCommit,
        HookEvent::PrePush,
        HookEvent::CommitMsg,
    ];
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
        let project_root_display = git_root
            .as_ref()
            .map(|r| r.display().to_string())
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
            println!(
                "{} Multiple hook tools detected in {}",
                "⚠".yellow(),
                hook_path.display()
            );
            if content.contains("linthis") {
                println!("  {} linthis", "✓".green());
            }
            if content.contains("prek") {
                println!("  {} prek", "⚠".yellow());
            }
            if content.contains("pre-commit") {
                println!("  {} pre-commit", "⚠".yellow());
            }
            if content.contains("husky") {
                println!("  {} husky", "⚠".yellow());
            }
            warnings.push("Consider using only one hook management tool");
            return (true, warnings);
        }
    }
    (false, warnings)
}

/// Check for hook conflicts
pub(crate) fn handle_hook_check() -> ExitCode {
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
                println!(
                    "{} {} exists but no hook installed",
                    "⚠".yellow(),
                    prek_config.display()
                );
                warnings.push("Run 'prek install' or 'pre-commit install' to activate hooks");
            }
        }
    }

    if husky_dir.exists() && husky_dir.join("pre-commit").exists() {
        println!(
            "{} Husky detected: {}",
            "ℹ".cyan(),
            husky_dir.join("pre-commit").display()
        );
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
        println!(
            "  • Use {} to see current hook setup",
            "linthis hook status".cyan()
        );
        println!("  • Choose one hook tool and stick with it");
        println!("  • For teams, document hook setup in README");
    } else {
        println!("{} No conflicts detected", "✓".green().bold());
    }

    ExitCode::SUCCESS
}
