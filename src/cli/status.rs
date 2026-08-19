// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! `linthis status` — one screen of "what is linthis actually doing here":
//! enable state, installed hooks, active config/rule files and plugins.

use colored::Colorize;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use linthis::state::{self, Scope};

/// Hook events shown in the hooks section, in execution order.
const EVENTS: &[&str] = &["pre-commit", "commit-msg", "post-commit", "pre-push"];

pub fn handle_status_command() -> ExitCode {
    print_header();
    print_state();
    print_hooks();
    print_configs();
    print_plugins();
    ExitCode::SUCCESS
}

fn label(text: &str) -> String {
    format!("{:<9}", text)
}

fn tick(ok: bool) -> colored::ColoredString {
    if ok {
        "\u{2713}".green()
    } else {
        "\u{2717}".red()
    }
}

fn print_header() {
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "?".into());
    println!(
        "{} {}   {}",
        "linthis".bold(),
        env!("CARGO_PKG_VERSION"),
        exe.dimmed()
    );
    println!();
}

fn print_state() {
    match state::active() {
        Some((scope, disabled)) => {
            println!(
                "{}{} disabled ({}) \u{b7} {}",
                label("State"),
                tick(false),
                scope.as_str(),
                disabled.describe()
            );
            if let Some(p) = state::state_path(scope) {
                println!("{}{}", label(""), p.display().to_string().dimmed());
            }
            println!(
                "{}{}",
                label(""),
                "git hooks skipped; manual runs still work".dimmed()
            );
        }
        None => println!("{}{} enabled", label("State"), tick(true)),
    }
    println!();
}

/// Effective hooks directory for a scope, honoring `core.hooksPath`.
fn hooks_dir(global: bool) -> Option<PathBuf> {
    let scope_flag = if global { "--global" } else { "--local" };
    if let Some(p) = git_output(&["config", scope_flag, "core.hooksPath"]) {
        return Some(expand_tilde(&p));
    }
    if global {
        return None;
    }
    // Default location; --git-path keeps this correct inside worktrees.
    let rel = git_output(&["rev-parse", "--git-path", "hooks"])?;
    let path = PathBuf::from(&rel);
    Some(if path.is_absolute() {
        path
    } else {
        std::env::current_dir().ok()?.join(path)
    })
}

fn expand_tilde(raw: &str) -> PathBuf {
    match raw.strip_prefix("~/") {
        Some(rest) => linthis::utils::home_dir()
            .map(|h| h.join(rest))
            .unwrap_or_else(|| PathBuf::from(raw)),
        None => PathBuf::from(raw),
    }
}

fn git_output(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// Read the value following `flag` in an installed hook wrapper
/// (`exec linthis hook run --event pre-commit --type git ...`).
fn wrapper_arg(content: &str, flag: &str) -> Option<String> {
    let mut it = content.split_whitespace();
    while let Some(tok) = it.next() {
        if tok == flag {
            return it.next().map(|s| s.trim_matches('"').to_string());
        }
    }
    None
}

fn print_hooks() {
    let mut any = false;
    let mut seen: Vec<PathBuf> = Vec::new();
    for (name, global) in [("global", true), ("project", false)] {
        let Some(dir) = hooks_dir(global) else {
            continue;
        };
        // `core.hooksPath` makes both scopes resolve to the same directory —
        // print it once instead of pretending there are two installs.
        if seen.contains(&dir) {
            continue;
        }
        seen.push(dir.clone());
        any = true;
        println!(
            "{}{:<8} {}",
            label("Hooks"),
            name,
            dir.display().to_string().dimmed()
        );
        for event in EVENTS {
            println!("{}", hook_line(&dir.join(event), event));
        }
    }
    if !any {
        println!("{}{} no hooks directory", label("Hooks"), tick(false));
    }
    println!();
}

fn hook_line(path: &Path, event: &str) -> String {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    if content.is_empty() {
        return format!("{}  {} {:<12}", label(""), tick(false), event);
    }
    if !content.contains("linthis") {
        return format!(
            "{}  {} {:<12} {}",
            label(""),
            tick(false),
            event,
            "occupied by another tool".yellow()
        );
    }
    let kind = wrapper_arg(&content, "--type").unwrap_or_else(|| "git".into());
    let provider = wrapper_arg(&content, "--provider")
        .map(|p| format!(" \u{b7} {}", p))
        .unwrap_or_default();
    format!(
        "{}  {} {:<12} {}{}",
        label(""),
        tick(true),
        event,
        kind.dimmed(),
        provider.dimmed()
    )
}

fn print_configs() {
    let root = linthis::utils::get_project_root();
    let paths = linthis::config::Config::get_active_config_paths(&root);

    if paths.is_empty() {
        println!("{}{} none (built-in defaults)", label("Config"), tick(false));
    } else {
        for (i, p) in paths.iter().enumerate() {
            let head = if i == 0 { label("Config") } else { label("") };
            println!("{}{} {}", head, tick(true), p.display());
        }
    }

    let ignore = root.join(".linthisignore");
    let exists = ignore.exists();
    println!(
        "{}{} {}",
        label(""),
        tick(exists),
        if exists {
            ignore.display().to_string()
        } else {
            format!("{} (absent)", ignore.display())
        }
    );
    println!();
}

/// Configured plugins from both scopes: (scope, name, url, git_ref).
fn configured_plugins() -> Vec<(&'static str, String, String, Option<String>)> {
    use linthis::plugin::PluginConfigManager;
    let mut out = Vec::new();
    for (scope, mgr) in [
        ("project", PluginConfigManager::project()),
        ("global", PluginConfigManager::global()),
    ] {
        let Ok(mgr) = mgr else { continue };
        for (name, url, git_ref) in mgr.list_plugins().unwrap_or_default() {
            out.push((scope, name, url, git_ref));
        }
    }
    out
}

fn print_plugins() {
    use linthis::plugin::{PluginCache, PluginLoader, PluginSource};

    let plugins = configured_plugins();
    if plugins.is_empty() {
        println!("{}{} none configured", label("Plugins"), tick(false));
        println!();
        return;
    }

    let cache = PluginCache::new().ok();
    let loader = PluginLoader::with_verbose(false).ok();
    let mut rules: Vec<String> = Vec::new();

    for (i, (scope, name, url, git_ref)) in plugins.iter().enumerate() {
        let source = match git_ref {
            Some(r) => PluginSource::new(url).with_ref(r),
            None => PluginSource::new(url),
        };
        let path = cache.as_ref().and_then(|c| c.get_cache_path(&source));
        let head = if i == 0 { label("Plugins") } else { label("") };
        println!(
            "{}{} {:<12} {:<8} {}",
            head,
            tick(path.as_ref().is_some_and(|p| p.exists())),
            name,
            scope.dimmed(),
            path.map(|p| p.display().to_string())
                .unwrap_or_else(|| url.clone())
                .dimmed()
        );

        // Rule/config files this plugin contributes (python/ruff → ruff.toml).
        let configs = loader
            .as_ref()
            .and_then(|l| l.load_configs(&[source], false).ok())
            .unwrap_or_default();
        rules.extend(configs.iter().map(|cfg| {
            format!(
                "{}/{}  {}",
                cfg.language,
                cfg.tool,
                cfg.config_path.display()
            )
        }));
    }
    println!();
    print_rules(rules);
}

/// Rule/config files the configured plugins contribute.
fn print_rules(mut rules: Vec<String>) {
    if rules.is_empty() {
        println!("{}{} none from plugins", label("Rules"), tick(false));
        println!();
        return;
    }

    // Keep status to one screen; the full list is `linthis plugin apply`'s job.
    const SHOWN: usize = 6;
    rules.sort();
    for (i, r) in rules.iter().take(SHOWN).enumerate() {
        let head = if i == 0 { label("Rules") } else { label("") };
        println!("{}{} {}", head, tick(true), r.dimmed());
    }
    if rules.len() > SHOWN {
        println!(
            "{}{}",
            label(""),
            format!("\u{2026} {} more", rules.len() - SHOWN).dimmed()
        );
    }
    println!();
}

/// `linthis disable [-t <ttl>] [-g]`
pub fn handle_disable_command(ttl: Option<String>, global: bool) -> ExitCode {
    let ttl = match ttl.as_deref().map(state::parse_ttl) {
        Some(Ok(t)) => t,
        Some(Err(e)) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(2);
        }
        None => state::Ttl::Forever,
    };
    let scope = pick_scope(global);

    if let Err(e) = state::disable(scope, &ttl) {
        eprintln!("{}: failed to write state: {}", "Error".red(), e);
        return ExitCode::from(1);
    }

    let disabled = state::load(scope).disabled.unwrap_or_default();
    println!(
        "{} linthis hooks disabled ({}) \u{b7} {}",
        "\u{2713}".green(),
        scope.as_str(),
        disabled.describe()
    );
    println!(
        "{}",
        "  manual runs (linthis, linthis -s) are unaffected".dimmed()
    );
    println!("{}", "  re-enable: linthis enable".dimmed());
    ExitCode::SUCCESS
}

/// `linthis enable [-g]`
pub fn handle_enable_command(global: bool) -> ExitCode {
    let scope = pick_scope(global);
    match state::enable(scope) {
        Ok(true) => println!(
            "{} linthis hooks enabled ({})",
            "\u{2713}".green(),
            scope.as_str()
        ),
        Ok(false) => println!(
            "{} linthis hooks were already enabled ({})",
            "\u{2713}".green(),
            scope.as_str()
        ),
        Err(e) => {
            eprintln!("{}: failed to write state: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    }

    // A project enable cannot lift a global disable — say so instead of
    // letting the user wonder why hooks are still silent.
    if !global {
        if let Some((Scope::Global, d)) = state::active() {
            println!(
                "{}: still disabled globally \u{b7} {} (lift with `linthis enable -g`)",
                "Note".yellow(),
                d.describe()
            );
        }
    }
    ExitCode::SUCCESS
}

fn pick_scope(global: bool) -> Scope {
    if global {
        Scope::Global
    } else {
        Scope::Project
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_arg_reads_installed_hook_flags() {
        let content = "#!/bin/sh\nexec linthis hook run --event pre-commit \
                       --type git-with-agent --provider claude --global \"$@\"\n";
        assert_eq!(wrapper_arg(content, "--type").unwrap(), "git-with-agent");
        assert_eq!(wrapper_arg(content, "--provider").unwrap(), "claude");
        assert!(wrapper_arg(content, "--missing").is_none());
        // A trailing flag with no value must not panic.
        assert!(wrapper_arg("exec linthis hook run --type", "--type").is_none());
    }

    #[test]
    fn expand_tilde_resolves_home_only_for_prefix() {
        let expanded = expand_tilde("~/hooks");
        assert!(expanded.ends_with("hooks"));
        assert!(!expanded.to_string_lossy().starts_with('~'));
        assert_eq!(expand_tilde("/abs/hooks"), PathBuf::from("/abs/hooks"));
    }
}
