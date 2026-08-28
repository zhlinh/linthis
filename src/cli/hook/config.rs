// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Hook configuration: create_hook_config, resolve_hook_override, and related helpers.

use colored::Colorize;
use std::process::ExitCode;

use super::metadata::save_installed_hook;
use super::script::{build_hook_command, hook_action};
use super::{find_git_root, is_command_available, write_hook_script};
use crate::cli::commands::{HookEvent, HookTool};

/// Format a `HookSource` as a TOML inline-table string, e.g. `{ plugin = "lt", file = "..." }`.
pub(crate) fn format_hook_source(source: &linthis::config::HookSource) -> String {
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
                format!(
                    "{{ git = \"{}\", ref = \"{}\", path = \"{}\" }}",
                    git, r, path
                )
            } else {
                format!("{{ git = \"{}\", path = \"{}\" }}", git, path)
            }
        }
        HookSource::Marketplace {
            marketplace,
            plugin,
            file,
        } => {
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
pub(crate) fn describe_hook_source(tool: &HookTool, hook_event: &HookEvent) -> String {
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
pub(crate) fn tool_type_dir(tool: &HookTool) -> Option<&'static str> {
    match tool {
        HookTool::Git => Some("git"),
        HookTool::GitWithAgent => Some("git-with-agent"),
        HookTool::Prek => Some("prek"),
        HookTool::PrekWithAgent => Some("prek-with-agent"),
        HookTool::Agent => None,
    }
}

/// Look up the TOML hook config entry for a given tool and event key.
pub(crate) fn lookup_hook_config_entry<'a>(
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
pub(crate) fn resolve_hook_override(
    tool: &HookTool,
    hook_event: &HookEvent,
) -> Result<Option<String>, ExitCode> {
    use linthis::config::Config;
    use linthis::hooks::resolver;

    let dir = match tool_type_dir(tool) {
        Some(d) => d,
        None => return Ok(None),
    };

    let project_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    // Tier 1: fixed-path auto-discovery
    if let Some(fixed) =
        resolver::fixed_git_hook_path(&project_root, dir, hook_event.hook_filename())
    {
        match std::fs::read_to_string(fixed.as_path()) {
            Ok(content) => return Ok(Some(content)),
            Err(e) => {
                eprintln!(
                    "{}: Failed to read fixed-path override '{}': {}",
                    "Error".red(),
                    fixed.display(),
                    e
                );
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
                eprintln!(
                    "{}: Failed to resolve hook override for '{}/{}': {}",
                    "Error".red(),
                    dir,
                    event_key,
                    e
                );
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
        eprintln!(
            "{}: {} already exists, skipping",
            "Warning".yellow(),
            config_path.display()
        );
        return Ok(());
    }

    if let Some(override_content) = resolve_hook_override(tool, hook_event)? {
        std::fs::write(&config_path, override_content).map_err(|e| {
            eprintln!(
                "{}: Failed to write '{}': {}",
                "Error".red(),
                config_path.display(),
                e
            );
            ExitCode::from(2)
        })?;
        println!(
            "{} Created {} [override]",
            "✓".green(),
            config_path.display()
        );
        return Ok(());
    }

    let hook_cmd = build_hook_command(hook_event, args);
    let stage = hook_event.hook_filename();
    let content = format!(
        "repos:\n  - repo: local\n    hooks:\n      - id: linthis-{}\n        name: linthis ({})\n        entry: {}\n        language: system\n        stages: [{}]\n        pass_filenames: false\n",
        hook_filename, hook_event.description(), hook_cmd, stage
    );

    std::fs::write(&config_path, content).map_err(|e| {
        eprintln!(
            "{}: Failed to create {}: {}",
            "Error".red(),
            config_path.display(),
            e
        );
        ExitCode::from(2)
    })?;

    let tool_name = "prek";
    println!(
        "{} Created {} ({}/pre-commit compatible)",
        "✓".green(),
        config_path.display(),
        tool_name
    );
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
                println!(
                    "\n{} {} hooks are ready!",
                    "✓".green().bold(),
                    hook_filename
                );
                println!(
                    "  Hooks will run automatically on {}",
                    format!("git {}", hook_action(hook_event)).cyan()
                );
            }
            Err(e) => {
                println!("{}", "✗".red());
                eprintln!("{}: {}", "Warning".yellow(), e);
                println!(
                    "\nPlease run manually: {}",
                    format!("{} install --hook-type {}", tool_name, hook_filename).cyan()
                );
            }
        }
    } else {
        println!("\nNext steps:");
        if matches!(tool, HookTool::Prek) {
            let prek_cmd = linthis::python_tool_install_hint("prek").replace("Install: ", "");
            println!("  1. Install prek: {}", prek_cmd.cyan());
            println!(
                "  2. Set up hooks: {}",
                format!("prek install --hook-type {}", hook_filename).cyan()
            );
        } else {
            let precommit_cmd =
                linthis::python_tool_install_hint("pre-commit").replace("Install: ", "");
            println!("  1. Install pre-commit: {}", precommit_cmd.cyan());
            println!(
                "  2. Set up hooks: {}",
                format!("pre-commit install --hook-type {}", hook_filename).cyan()
            );
        }
    }
}

/// Install hooks using the specified tool
fn install_hooks(tool: &HookTool, hook_event: &HookEvent) -> Result<(), String> {
    use std::process::Command;

    let (cmd, tool_name) = match tool {
        HookTool::Prek => ("prek", "prek"),
        HookTool::Git | HookTool::Agent | HookTool::GitWithAgent | HookTool::PrekWithAgent => {
            return Ok(()); // handled separately
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

/// Create a git hook file (thin wrapper or append to existing).
fn create_git_hook_config(
    tool: &HookTool,
    hook_event: &HookEvent,
    force: bool,
    _args: &Option<String>,
) -> Result<(), ExitCode> {
    use std::fs;

    let hook_filename = hook_event.hook_filename();
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

    if !git_hooks_dir.exists() {
        fs::create_dir_all(&git_hooks_dir).map_err(|e| {
            eprintln!(
                "{}: Failed to create hooks directory {}: {}",
                "Error".red(),
                git_hooks_dir.display(),
                e
            );
            ExitCode::from(2)
        })?;
    }

    // Tier-1/2 override check
    if let Some(override_content) = resolve_hook_override(tool, hook_event)? {
        write_git_hook_override(&hook_path, &override_content, force)?;
        // Record it like any other install: without this the hook is invisible
        // to `hook sync`, so a plugin-sourced script never gets refreshed when
        // linthis or the plugin is upgraded.
        let project = git_root.to_str().unwrap_or("").to_string();
        save_installed_hook("local", &project, hook_event, &HookTool::Git, None, None);
        return Ok(());
    }

    // Merge into whatever is already there: another tool's hook keeps working,
    // and re-installing just refreshes our block. `--force` replaces the file
    // outright, which is the only way to evict a hook linthis did not write.
    let existing = std::fs::read_to_string(&hook_path).ok();
    let chained = !force
        && existing
            .as_deref()
            .is_some_and(super::block::has_foreign_content);

    let block = super::block::build_block(hook_event, &HookTool::Git, None, false, None);
    if force {
        // --force replaces the file, the only way to evict a hook linthis did
        // not write.
        let super::block::Upsert::Merged(fresh) = super::block::upsert_block(None, &block) else {
            unreachable!("an empty file has nothing hand-written in it")
        };
        write_hook_script(&hook_path, &fresh)?;
    } else {
        super::write_hook_block(&hook_path, &block)?;
    }

    println!("{} Created {} [project]", "✓".green(), hook_path.display());
    warn_if_hooks_path_overrides(&hook_path);
    if chained {
        println!(
            "  {} Chained: linthis runs before the hook that was already here",
            "→".dimmed()
        );
    }
    println!(
        "  {} Thin wrapper: hook logic auto-updates with linthis",
        "→".dimmed()
    );
    #[cfg(not(unix))]
    {
        println!("\nNext steps:");
        println!("  Make sure the hook is executable:");
        println!(
            "    {}",
            format!("chmod +x .git/hooks/{}", hook_filename).cyan()
        );
    }
    let project = git_root.to_str().unwrap_or("").to_string();
    save_installed_hook("local", &project, hook_event, &HookTool::Git, None, None);
    Ok(())
}

/// Warn when `core.hooksPath` means git will never look at `.git/hooks`.
///
/// A project hook written there is silently dead, which is confusing enough on
/// its own — and it is how a global hook ends up being the only one that runs.
fn warn_if_hooks_path_overrides(hook_path: &std::path::Path) {
    let configured = std::process::Command::new("git")
        .args(["config", "core.hooksPath"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    let Some(configured) = configured else {
        return;
    };
    // The hook we just wrote is under .git/hooks; if that is where git looks,
    // there is nothing to warn about.
    if hook_path.parent().is_some_and(|p| p.ends_with(&configured)) {
        return;
    }

    eprintln!(
        "{}: core.hooksPath is set to {}, so git ignores .git/hooks entirely",
        "Warning".yellow(),
        configured.cyan()
    );
    eprintln!(
        "  {} this hook will not run. Install into that directory instead: {}",
        "→".dimmed(),
        "linthis hook add -g".cyan()
    );
}

/// Write an override hook script, optionally appending to existing content.
fn write_git_hook_override(
    hook_path: &std::path::Path,
    override_content: &str,
    force: bool,
) -> Result<(), ExitCode> {
    let content = if hook_path.exists() && !force {
        let mut existing = std::fs::read_to_string(hook_path).unwrap_or_default();
        if !existing.ends_with('\n') {
            existing.push('\n');
        }
        existing.push_str("\n# linthis-hook (override)\n");
        existing.push_str(override_content);
        existing
    } else {
        override_content.to_string()
    };
    write_hook_script(hook_path, &content)?;
    println!(
        "{} Created {} [project, override]",
        "✓".green(),
        hook_path.display()
    );
    Ok(())
}


pub(crate) fn create_hook_config(
    tool: &HookTool,
    hook_event: &HookEvent,
    force: bool,
    args: &Option<String>,
) -> Result<(), ExitCode> {
    match tool {
        HookTool::Agent | HookTool::GitWithAgent | HookTool::PrekWithAgent => Ok(()),
        HookTool::Prek => create_prek_config(tool, hook_event, force, args),
        HookTool::Git => create_git_hook_config(tool, hook_event, force, args),
    }
}
