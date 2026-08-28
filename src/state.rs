// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Persistent enable/disable state for linthis git hooks.
//!
//! `linthis disable` writes `[disabled]` into a state file; every hook run
//! consults it and exits 0 silently while it is in effect. Manual runs
//! (`linthis -s`, `linthis check`, ...) are never gated.
//!
//! Two scopes, global wins:
//! - project: `<project-root>/.linthis/state.toml` (default)
//! - global:  `~/.linthis/state.toml` (`-g`)
//!
//! Three flavours of TTL, at most one field set:
//! - forever: neither `until` nor `remaining`
//! - time:    `until` (RFC 3339)
//! - count:   `remaining` + `head`, consumed per git operation (see
//!   [`consume`])

use chrono::{DateTime, Duration, Local, NaiveTime};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// File name used in both scopes.
const STATE_FILE: &str = "state.toml";

/// Which state file a disable applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Project,
    Global,
}

impl Scope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Scope::Project => "project",
            Scope::Global => "global",
        }
    }
}

/// A parsed `--ttl` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ttl {
    /// No expiry — stays disabled until `linthis enable`.
    Forever,
    /// Expires at a wall-clock instant.
    Until(DateTime<Local>),
    /// Expires after N git operations (commit / push).
    Count(u32),
}

/// Parse a `--ttl` value: `3pcs`, `today`, `10s`, `30m`, `2h`, `1d`, `1w`.
///
/// `m` is minutes — months are not supported (use `30d`).
pub fn parse_ttl(raw: &str) -> Result<Ttl, String> {
    let s = raw.trim().to_lowercase();
    if s.is_empty() {
        return Err(ttl_error(raw));
    }

    if s == "today" {
        // End of the current local day.
        let end = Local::now()
            .date_naive()
            .and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap())
            .and_local_timezone(Local)
            .single()
            .ok_or_else(|| ttl_error(raw))?;
        return Ok(Ttl::Until(end));
    }

    if let Some(num) = s.strip_suffix("pcs") {
        let n: u32 = num.parse().map_err(|_| ttl_error(raw))?;
        if n == 0 {
            return Err(ttl_error(raw));
        }
        return Ok(Ttl::Count(n));
    }

    let duration = parse_duration(&s).ok_or_else(|| ttl_error(raw))?;
    Ok(Ttl::Until(Local::now() + duration))
}

/// Parse `10s` / `30m` / `2h` / `1d` / `1w`. `m` is minutes.
fn parse_duration(s: &str) -> Option<Duration> {
    let (num, unit) = s.split_at(s.len().checked_sub(1)?);
    let n: i64 = num.parse().ok()?;
    if n <= 0 {
        return None;
    }
    match unit {
        "s" => Some(Duration::seconds(n)),
        "m" => Some(Duration::minutes(n)),
        "h" => Some(Duration::hours(n)),
        "d" => Some(Duration::days(n)),
        "w" => Some(Duration::weeks(n)),
        _ => None,
    }
}

fn ttl_error(raw: &str) -> String {
    format!(
        "invalid --ttl '{}'. Use: <N>pcs (git operations), today, \
         or a duration like 10s / 30m / 2h / 1d / 1w (m = minutes)",
        raw.trim()
    )
}

/// The `[disabled]` table of a state file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Disabled {
    /// Wall-clock expiry (RFC 3339). Absent for forever/count disables.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
    /// Remaining git operations. Absent for forever/time disables.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remaining: Option<u32>,
    /// HEAD when the count disable was armed — see [`consume`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
    /// When the disable was set, for `linthis status`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_at: Option<String>,
}

/// A state file (`state.toml`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Disabled>,
}

impl Disabled {
    /// Parse `until` back into a timestamp.
    pub fn until_time(&self) -> Option<DateTime<Local>> {
        self.until
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Local))
    }

    /// Has a time-based disable run out?
    pub fn is_expired(&self) -> bool {
        match self.until_time() {
            Some(until) => Local::now() >= until,
            // A count disable with nothing left is expired too.
            None => matches!(self.remaining, Some(0)),
        }
    }

    /// Human-readable remainder, e.g. "剩余 2 次" / "until 18:00".
    pub fn describe(&self) -> String {
        if let Some(n) = self.remaining {
            return format!("{} git operation(s) left", n);
        }
        match self.until_time() {
            Some(until) => {
                let left = until - Local::now();
                format!(
                    "until {} ({} left)",
                    until.format("%Y-%m-%d %H:%M"),
                    humanize(left)
                )
            }
            None => "no expiry".to_string(),
        }
    }
}

/// Render a duration as the largest sensible unit.
fn humanize(d: Duration) -> String {
    let secs = d.num_seconds().max(0);
    match secs {
        0..=59 => format!("{}s", secs),
        60..=3599 => format!("{}m", secs / 60),
        3600..=86399 => format!("{}h", secs / 3600),
        _ => format!("{}d", secs / 86400),
    }
}

/// Path of the state file for a scope. `None` when the home directory is
/// unknown (global scope only).
pub fn state_path(scope: Scope) -> Option<PathBuf> {
    match scope {
        // Under ~/.linthis, keyed by project, not inside the repository.
        // Whether *you* have linthis switched off is not something your
        // colleagues should find in `git status`, let alone review.
        Scope::Project => Some(crate::utils::get_global_project_dir().join(STATE_FILE)),
        Scope::Global => crate::utils::home_dir().map(|h| h.join(".linthis").join(STATE_FILE)),
    }
}

/// Where project state used to live, inside the repository.
fn legacy_state_path() -> PathBuf {
    crate::utils::get_effective_project_root()
        .join(".linthis")
        .join(STATE_FILE)
}

/// Move a disable written by an older linthis out of the repository.
///
/// Returns the state it found, if any. The old file is deleted either way, and
/// its directory too when nothing else was in it — leaving `.linthis/` behind
/// is what put an untracked directory in people's repositories.
fn migrate_legacy_state() -> Option<State> {
    let legacy = legacy_state_path();
    let text = std::fs::read_to_string(&legacy).ok()?;
    let state: State = toml::from_str(&text).ok()?;

    let _ = std::fs::remove_file(&legacy);
    if let Some(dir) = legacy.parent() {
        // Only if it is now empty: `.linthis/` also holds config and rules.
        if dir.read_dir().is_ok_and(|mut d| d.next().is_none()) {
            let _ = std::fs::remove_dir(dir);
        }
    }

    (state.disabled.is_some()).then_some(state)
}

/// Read a state file. A missing or malformed file reads as "no state" —
/// linthis must never break because its own scratch file got mangled.
pub fn load(scope: Scope) -> State {
    let current: Option<State> = state_path(scope)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| toml::from_str(&s).ok());

    if let Some(state) = current {
        return state;
    }
    if scope == Scope::Project {
        if let Some(migrated) = migrate_legacy_state() {
            let _ = save(scope, &migrated);
            return migrated;
        }
    }
    State::default()
}

/// Write a state file, creating `.linthis/` if needed. Removes the file
/// entirely when the state is empty, so no stale scratch files linger.
pub fn save(scope: Scope, state: &State) -> Result<(), String> {
    let path = state_path(scope).ok_or("cannot resolve home directory")?;

    if state.disabled.is_none() {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let body = toml::to_string(state).map_err(|e| e.to_string())?;
    std::fs::write(&path, body).map_err(|e| e.to_string())
}

/// Build the `[disabled]` entry for a TTL.
pub fn disabled_from_ttl(ttl: &Ttl) -> Disabled {
    let mut d = Disabled {
        set_at: Some(Local::now().to_rfc3339()),
        ..Default::default()
    };
    match ttl {
        Ttl::Forever => {}
        Ttl::Until(t) => d.until = Some(t.to_rfc3339()),
        Ttl::Count(n) => {
            d.remaining = Some(*n);
            d.head = current_head();
        }
    }
    d
}

/// Current `HEAD` sha, if this is a git repo with at least one commit.
fn current_head() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!sha.is_empty()).then_some(sha)
}

/// The active disable, if any: global wins over project, and expired
/// entries are cleaned up as they are found.
pub fn active() -> Option<(Scope, Disabled)> {
    for scope in [Scope::Global, Scope::Project] {
        let state = load(scope);
        if let Some(d) = state.disabled {
            if d.is_expired() {
                let _ = save(scope, &State::default());
                continue;
            }
            return Some((scope, d));
        }
    }
    None
}

/// What should happen to a count-based disable when a hook fires.
#[derive(Debug, Clone, PartialEq)]
enum Next {
    /// Still covered, state unchanged.
    Keep,
    /// Nothing left — drop the disable and check this operation.
    Retire,
    /// Still covered, but one operation was used up.
    Replace(Disabled),
}

/// Decide the fate of a count-based disable. Pure, so the HEAD bookkeeping
/// is testable without a git repo.
///
/// A single commit fires pre-commit → commit-msg → post-commit as separate
/// processes, so "one operation" cannot mean "one invocation". `HEAD` is the
/// marker instead: it does not move within a commit's hook chain, and it has
/// moved by the time the next chain starts. A push never moves `HEAD` and is
/// a chain of one, so it is covered first and retired immediately after.
fn decide(disabled: &Disabled, head: Option<String>, is_push: bool) -> Next {
    let Some(remaining) = disabled.remaining else {
        return Next::Keep; // forever / time-based
    };
    if !is_push && head == disabled.head {
        return Next::Keep; // same commit chain
    }

    let left = remaining.saturating_sub(1);
    if left == 0 {
        return Next::Retire;
    }
    Next::Replace(Disabled {
        remaining: Some(left),
        head: if is_push { disabled.head.clone() } else { head },
        ..disabled.clone()
    })
}

/// The disable in effect for a hook that is about to run, retiring
/// count-based entries whose operation has finished.
///
/// `None` means "run the hook normally". A push is still covered by the
/// entry it consumes — it is retired for the *next* push, not this one.
pub fn gate(is_push: bool) -> Option<(Scope, Disabled)> {
    let (scope, disabled) = active()?;

    match decide(&disabled, current_head(), is_push) {
        Next::Keep => Some((scope, disabled)),
        Next::Retire => {
            let _ = save(scope, &State::default());
            // The push that used the last operation still gets skipped.
            is_push.then_some((scope, disabled))
        }
        Next::Replace(next) => {
            let _ = save(
                scope,
                &State {
                    disabled: Some(next.clone()),
                },
            );
            Some((scope, if is_push { disabled } else { next }))
        }
    }
}

/// Gate a hook invocation: report whether hooks are disabled and, if so,
/// print the skip notice on stderr.
///
/// Hooks reach linthis two ways — the thin `linthis hook run` wrapper, and
/// plugin/legacy scripts that invoke `linthis --hook-event <event>` directly.
/// Both call this, so a disable covers either flavour without reinstalling
/// anything.
pub fn skip_hook(event: &str) -> bool {
    let Some((scope, disabled)) = gate(event == "pre-push") else {
        return false;
    };
    eprintln!(
        "{}",
        format!(
            "\u{23ed}  linthis {} skipped \u{b7} disabled ({}) \u{b7} {}",
            event,
            scope.as_str(),
            disabled.describe()
        )
        .dimmed()
    );
    true
}

/// Disable linthis hooks in `scope`.
pub fn disable(scope: Scope, ttl: &Ttl) -> Result<(), String> {
    save(
        scope,
        &State {
            disabled: Some(disabled_from_ttl(ttl)),
        },
    )
}

/// Re-enable linthis hooks in `scope`. Returns whether anything was disabled.
pub fn enable(scope: Scope) -> Result<bool, String> {
    let was_disabled = load(scope).disabled.is_some();
    save(scope, &State::default())?;
    Ok(was_disabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_count_ttl() {
        assert_eq!(parse_ttl("1pcs").unwrap(), Ttl::Count(1));
        assert_eq!(parse_ttl(" 3PCS ").unwrap(), Ttl::Count(3));
        assert!(parse_ttl("0pcs").is_err());
        assert!(parse_ttl("pcs").is_err());
    }

    #[test]
    fn parses_duration_ttl() {
        let now = Local::now();
        for (raw, min_secs) in [("10s", 9), ("30m", 1790), ("2h", 7190), ("1d", 86390)] {
            match parse_ttl(raw).unwrap() {
                Ttl::Until(t) => assert!(
                    (t - now).num_seconds() >= min_secs,
                    "{raw} expired too early"
                ),
                other => panic!("{raw} parsed as {other:?}"),
            }
        }
    }

    #[test]
    fn today_expires_at_end_of_day() {
        match parse_ttl("today").unwrap() {
            Ttl::Until(t) => {
                assert_eq!(t.date_naive(), Local::now().date_naive());
                assert_eq!(t.format("%H:%M:%S").to_string(), "23:59:59");
            }
            other => panic!("today parsed as {other:?}"),
        }
    }

    #[test]
    fn rejects_unknown_ttl() {
        for raw in ["", "1mo", "abc", "-1d", "1y"] {
            let err = parse_ttl(raw).unwrap_err();
            assert!(err.contains("invalid --ttl"), "raw={raw} err={err}");
        }
    }

    #[test]
    fn expiry_uses_until_timestamp() {
        let past = Disabled {
            until: Some((Local::now() - Duration::minutes(1)).to_rfc3339()),
            ..Default::default()
        };
        assert!(past.is_expired());

        let future = Disabled {
            until: Some((Local::now() + Duration::minutes(1)).to_rfc3339()),
            ..Default::default()
        };
        assert!(!future.is_expired());

        // Forever never expires; a used-up count does.
        assert!(!Disabled::default().is_expired());
        assert!(Disabled {
            remaining: Some(0),
            ..Default::default()
        }
        .is_expired());
    }

    #[test]
    fn state_roundtrips_through_toml() {
        let state = State {
            disabled: Some(Disabled {
                remaining: Some(2),
                head: Some("abc123".into()),
                set_at: Some(Local::now().to_rfc3339()),
                ..Default::default()
            }),
        };
        let text = toml::to_string(&state).unwrap();
        let back: State = toml::from_str(&text).unwrap();
        let d = back.disabled.unwrap();
        assert_eq!(d.remaining, Some(2));
        assert_eq!(d.head.as_deref(), Some("abc123"));
        assert!(d.until.is_none());
    }

    fn armed(remaining: u32, head: &str) -> Disabled {
        Disabled {
            remaining: Some(remaining),
            head: Some(head.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn commit_chain_is_one_operation() {
        let d = armed(1, "aaa");
        // pre-commit / commit-msg / post-commit all see the armed HEAD.
        assert_eq!(decide(&d, Some("aaa".into()), false), Next::Keep);
        // Next commit: HEAD moved, last operation used up.
        assert_eq!(decide(&d, Some("bbb".into()), false), Next::Retire);
    }

    #[test]
    fn multi_count_rearms_on_new_head() {
        let d = armed(2, "aaa");
        match decide(&d, Some("bbb".into()), false) {
            Next::Replace(next) => {
                assert_eq!(next.remaining, Some(1));
                assert_eq!(next.head.as_deref(), Some("bbb"));
            }
            other => panic!("expected Replace, got {other:?}"),
        }
    }

    #[test]
    fn push_consumes_immediately() {
        // HEAD does not move on push, so the chain rule must not apply.
        assert_eq!(decide(&armed(1, "aaa"), Some("aaa".into()), true), Next::Retire);
        match decide(&armed(2, "aaa"), Some("aaa".into()), true) {
            Next::Replace(next) => {
                assert_eq!(next.remaining, Some(1));
                // Keeps the armed HEAD: a later commit chain still compares
                // against the commit that armed the disable.
                assert_eq!(next.head.as_deref(), Some("aaa"));
            }
            other => panic!("expected Replace, got {other:?}"),
        }
    }

    #[test]
    fn non_count_disables_are_untouched() {
        assert_eq!(decide(&Disabled::default(), None, false), Next::Keep);
        let timed = Disabled {
            until: Some((Local::now() + Duration::hours(1)).to_rfc3339()),
            ..Default::default()
        };
        assert_eq!(decide(&timed, Some("bbb".into()), true), Next::Keep);
    }

    #[test]
    fn skip_hook_is_a_noop_when_nothing_is_disabled() {
        // No state file in a scratch cwd → hooks must run normally. Guards the
        // gate against defaulting to "disabled" if the state file is missing.
        assert!(!skip_hook("pre-commit"));
    }

    #[test]
    fn project_state_lives_outside_the_repository() {
        let path = state_path(Scope::Project).expect("project scope always resolves");
        // A personal switch must not show up in `git status`, so it belongs
        // under ~/.linthis/projects/<slug>/, not in the working tree.
        assert!(
            path.to_string_lossy().contains(".linthis/projects/"),
            "unexpected location: {}",
            path.display()
        );
        assert_ne!(path, legacy_state_path());
    }

    #[test]
    fn describe_mentions_remaining_operations() {
        let d = Disabled {
            remaining: Some(2),
            ..Default::default()
        };
        assert!(d.describe().contains('2'));

        let t = Disabled {
            until: Some((Local::now() + Duration::hours(2)).to_rfc3339()),
            ..Default::default()
        };
        assert!(t.describe().contains("1h") || t.describe().contains("2h"));
    }
}
