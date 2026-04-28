// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

//! Shell integration commands: completion + built-in aliases.
//!
//! Adds tab-completion and a small set of aliases (`lt`, `lts`, `ltm`, `ltr`)
//! to the user's bash / zsh / fish / PowerShell. State lives in
//! `~/.linthis/shell-state.toml`; the per-shell source files
//! `~/.linthis/shell.{bash,zsh,fish,ps1}` are fully regenerated from that
//! state on every `add`/`remove`. The user's rc file gets a marker block
//! that sources the per-shell file.

mod completion;
mod detect;
mod rc;
mod render;
mod state;

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use super::commands::ShellCommands;
use state::{Shell, ShellState};

/// What the user is enabling/disabling.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Feature {
    Ac,
    Alias,
    All,
}

pub(crate) fn parse_feature(name: &str) -> Result<Feature, String> {
    match name.trim().to_ascii_lowercase().as_str() {
        "ac" => Ok(Feature::Ac),
        "alias" => Ok(Feature::Alias),
        "all" => Ok(Feature::All),
        other => Err(format!(
            "unknown feature `{other}` (expected: ac, alias, or all)"
        )),
    }
}

/// Resolve the `--shell` argument to a list of target shells.
/// `Some("all")` → all four. Otherwise parse a single shell name.
pub(crate) fn shells_for_target(value: &str) -> Result<Vec<Shell>, String> {
    if value.trim().eq_ignore_ascii_case("all") {
        return Ok(Shell::ALL.to_vec());
    }
    detect::parse_shell_name(value)
        .map(|s| vec![s])
        .map_err(|e| e.to_string())
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
}

/// Resolve target shells from an optional `--shell` flag (None → auto-detect).
fn resolve_shells(explicit: Option<&str>) -> Result<Vec<Shell>, String> {
    if let Some(v) = explicit {
        return shells_for_target(v);
    }
    detect::detect(None)
        .map(|s| vec![s])
        .map_err(|e| e.to_string())
}

fn set_feature(state: &mut ShellState, shell: Shell, feature: Feature, on: bool) {
    let f = state.flags_mut(shell);
    match feature {
        Feature::Ac => f.ac = on,
        Feature::Alias => f.alias = on,
        Feature::All => {
            f.ac = on;
            f.alias = on;
        }
    }
}

/// (Re)write the per-shell source file from `flags`. If both flags are off,
/// delete the source file. Caller-supplied `home` lets tests use a tempdir.
fn render_and_write_source(
    shell: Shell,
    flags: state::ShellFlags,
    home: &std::path::Path,
) -> io::Result<()> {
    let path = rc::source_path_for(shell, home);
    match render::render(shell, flags) {
        Some(body) => rc::atomic_write(&path, &body),
        None => rc::delete_if_exists(&path),
    }
}

/// Apply state mutation for a single shell: write source file, ensure or
/// remove the rc marker block. Returns the rc outcome for messaging.
fn apply_shell(
    shell: Shell,
    flags: state::ShellFlags,
    home: &std::path::Path,
) -> io::Result<rc::EnsureOutcome> {
    render_and_write_source(shell, flags, home)?;
    let rc_path = rc::rc_path_for(shell, home);
    if flags.is_empty() {
        rc::remove_marker(&rc_path)?;
        Ok(rc::EnsureOutcome::Idempotent)
    } else {
        rc::ensure_marker(shell, &rc_path)
    }
}

/// Common context for add/remove operations.
struct OperationContext {
    home: PathBuf,
    feature: Feature,
    targets: Vec<Shell>,
    state: ShellState,
    state_path: PathBuf,
}

/// Prepare context for add/remove operations. Returns Err(exit_code) on failure.
fn prepare_context(
    feature_arg: &str,
    shell_arg: Option<&str>,
) -> Result<OperationContext, ExitCode> {
    let Some(home) = home_dir() else {
        eprintln!("[linthis shell] $HOME / $USERPROFILE not set");
        return Err(ExitCode::from(1));
    };
    let feature = parse_feature(feature_arg).map_err(|e| {
        eprintln!("[linthis shell] {e}");
        ExitCode::from(1)
    })?;
    let targets = resolve_shells(shell_arg).map_err(|e| {
        eprintln!("[linthis shell] {e}");
        ExitCode::from(1)
    })?;
    let state_path = state::default_state_path(&home);
    let state = state::load(&state_path).map_err(|e| {
        eprintln!("[linthis shell] {e}");
        ExitCode::from(2)
    })?;
    Ok(OperationContext {
        home,
        feature,
        targets,
        state,
        state_path,
    })
}

/// Print outcome message for add operation.
fn print_add_outcome(shell: Shell, outcome: &rc::EnsureOutcome) {
    match outcome {
        rc::EnsureOutcome::Inserted => {
            eprintln!("[linthis shell] {}: marker added", shell.key())
        }
        rc::EnsureOutcome::Idempotent => {
            eprintln!("[linthis shell] {}: up to date", shell.key())
        }
        rc::EnsureOutcome::UnmanagedSourceLine => {
            eprintln!(
                "[linthis shell] {}: warning — your rc already sources \
                 ~/.linthis/shell.{} outside our marker block; not modifying it.",
                shell.key(),
                shell.key()
            );
        }
    }
}

fn handle_add(feature_arg: &str, shell_arg: Option<&str>) -> ExitCode {
    let ctx = match prepare_context(feature_arg, shell_arg) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let mut s = ctx.state;
    let snapshot = s;

    for sh in &ctx.targets {
        set_feature(&mut s, *sh, ctx.feature, true);
    }

    for sh in &ctx.targets {
        match apply_shell(*sh, s.flags(*sh), &ctx.home) {
            Ok(outcome) => print_add_outcome(*sh, &outcome),
            Err(e) => {
                eprintln!("[linthis shell] {}: write failed — {e}", sh.key());
                if let Err(re) = state::save(&ctx.state_path, &snapshot) {
                    eprintln!("[linthis shell] warning: state rollback also failed — {re}");
                }
                return ExitCode::from(1);
            }
        }
    }

    if let Err(e) = state::save(&ctx.state_path, &s) {
        eprintln!("[linthis shell] failed to persist state: {e}");
        return ExitCode::from(2);
    }

    eprintln!("[linthis shell] open a new terminal to pick up the changes.");
    ExitCode::SUCCESS
}

fn handle_remove(feature_arg: &str, shell_arg: Option<&str>) -> ExitCode {
    let ctx = match prepare_context(feature_arg, shell_arg) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let mut s = ctx.state;
    let snapshot = s;

    for sh in &ctx.targets {
        set_feature(&mut s, *sh, ctx.feature, false);
    }

    for sh in &ctx.targets {
        match apply_shell(*sh, s.flags(*sh), &ctx.home) {
            Ok(_) => eprintln!("[linthis shell] {}: removed", sh.key()),
            Err(e) => {
                eprintln!("[linthis shell] {}: remove failed — {e}", sh.key());
                if let Err(re) = state::save(&ctx.state_path, &snapshot) {
                    eprintln!("[linthis shell] warning: state rollback also failed — {re}");
                }
                return ExitCode::from(1);
            }
        }
    }

    if let Err(e) = state::save(&ctx.state_path, &s) {
        eprintln!("[linthis shell] failed to persist state: {e}");
        return ExitCode::from(2);
    }
    eprintln!("[linthis shell] removed.");
    ExitCode::SUCCESS
}

fn handle_status() -> ExitCode {
    let Some(home) = home_dir() else {
        eprintln!("[linthis shell] $HOME / $USERPROFILE not set");
        return ExitCode::from(1);
    };
    let state_path = state::default_state_path(&home);
    let s = match state::load(&state_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[linthis shell] {e}");
            return ExitCode::from(2);
        }
    };
    println!("Shell integration status (from {}):", state_path.display());
    for sh in Shell::ALL {
        let f = s.flags(sh);
        let mark = |on: bool| if on { "on " } else { "off" };
        let rc_path = rc::rc_path_for(sh, &home);
        let tag = if rc::has_unmanaged_source_line(sh, &rc_path) {
            "  (unmanaged source line in rc \u{2014} run 'linthis shell add' to review)"
        } else {
            ""
        };
        println!(
            "  {:<11}  ac:{}  alias:{}{}",
            sh.key(),
            mark(f.ac),
            mark(f.alias),
            tag
        );
    }
    ExitCode::SUCCESS
}

fn handle_init(shell_arg: Option<&str>) -> ExitCode {
    let Some(home) = home_dir() else {
        eprintln!("[linthis shell] $HOME / $USERPROFILE not set");
        return ExitCode::from(1);
    };
    let state_path = state::default_state_path(&home);
    let s = match state::load(&state_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[linthis shell] {e}");
            return ExitCode::from(2);
        }
    };
    let targets = match resolve_shells(shell_arg) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[linthis shell] {e}");
            return ExitCode::from(1);
        }
    };
    for sh in targets {
        if let Err(e) = render_and_write_source(sh, s.flags(sh), &home) {
            eprintln!("[linthis shell] {}: init failed — {e}", sh.key());
            return ExitCode::from(1);
        }
    }
    ExitCode::SUCCESS
}

fn handle_completion(shell_arg: &str) -> ExitCode {
    let shell = match detect::parse_shell_name(shell_arg) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[linthis shell] {e}");
            return ExitCode::from(1);
        }
    };
    // Buffer first so the panic path inside clap_complete::generate
    // (which unwraps Write errors) can't fire on a closed pipe.
    let mut buf: Vec<u8> = Vec::new();
    if let Err(e) = completion::write_completion(shell, &mut buf) {
        eprintln!("[linthis shell] completion failed: {e}");
        return ExitCode::from(1);
    }
    let mut out = io::stdout().lock();
    match out.write_all(&buf) {
        Ok(()) => ExitCode::SUCCESS,
        // BrokenPipe = downstream consumer closed (head, less, etc.) — fine.
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("[linthis shell] completion write failed: {e}");
            ExitCode::from(1)
        }
    }
}

pub fn handle_shell_command(action: ShellCommands) -> ExitCode {
    match action {
        ShellCommands::Add { feature, shell } => handle_add(&feature, shell.as_deref()),
        ShellCommands::Remove { feature, shell } => handle_remove(&feature, shell.as_deref()),
        ShellCommands::Status => handle_status(),
        ShellCommands::Init { shell } => handle_init(shell.as_deref()),
        ShellCommands::Completion { shell } => handle_completion(&shell),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_feature_accepts_canonical_names() {
        assert_eq!(parse_feature("ac").unwrap(), Feature::Ac);
        assert_eq!(parse_feature("alias").unwrap(), Feature::Alias);
        assert_eq!(parse_feature("all").unwrap(), Feature::All);
    }

    #[test]
    fn parse_feature_is_case_insensitive() {
        assert_eq!(parse_feature("AC").unwrap(), Feature::Ac);
        assert_eq!(parse_feature("Alias").unwrap(), Feature::Alias);
    }

    #[test]
    fn parse_feature_rejects_unknown() {
        assert!(parse_feature("everything").is_err());
        assert!(parse_feature("").is_err());
    }

    #[test]
    fn shells_for_target_all_returns_four() {
        let v = shells_for_target("all").unwrap();
        assert_eq!(v.len(), 4);
    }

    #[test]
    fn shells_for_target_specific_returns_one() {
        let v = shells_for_target("zsh").unwrap();
        assert_eq!(v, vec![state::Shell::Zsh]);
    }
}
