// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Per-hook bypass via the `LINTHIS_SKIP` environment variable.
//!
//! Git's `--no-verify` skips every hook at once. `LINTHIS_SKIP` lets users
//! bypass a specific hook (e.g. commit-msg) while leaving the others running.
//!
//! Accepted tokens (comma-separated, case-insensitive):
//! - `check` / `pc`         — pre-commit + post-commit (they run as a pair)
//! - `pre-commit`           — only pre-commit
//! - `post-commit`          — only post-commit
//! - `cmsg` / `cm` / `commit-msg` — commit-msg
//! - `pp` / `pre-push`      — pre-push
//! - `all`                  — every hook

use colored::Colorize;

use crate::cli::commands::HookEvent;

/// Environment variable name.
pub const ENV_VAR: &str = "LINTHIS_SKIP";

/// Parse a `LINTHIS_SKIP` value and return the set of hook events to skip.
/// Returns an error string on unknown tokens so the caller can fail loudly.
pub fn parse(value: &str) -> Result<Vec<HookEvent>, String> {
    let mut out: Vec<HookEvent> = Vec::new();
    for raw in value.split(',') {
        let tok = raw.trim().to_lowercase();
        if tok.is_empty() {
            continue;
        }
        let events: &[HookEvent] = match tok.as_str() {
            "all" => &[
                HookEvent::PreCommit,
                HookEvent::PostCommit,
                HookEvent::CommitMsg,
                HookEvent::PrePush,
            ],
            "check" | "pc" => &[HookEvent::PreCommit, HookEvent::PostCommit],
            "pre-commit" => &[HookEvent::PreCommit],
            "post-commit" => &[HookEvent::PostCommit],
            "cmsg" | "cm" | "commit-msg" => &[HookEvent::CommitMsg],
            "pp" | "pre-push" => &[HookEvent::PrePush],
            other => {
                return Err(format!(
                    "{ENV_VAR}: unknown value '{other}'. \
                     Supported: check|pc, pre-commit, post-commit, cmsg|cm|commit-msg, pp|pre-push, all"
                ));
            }
        };
        for ev in events {
            if !out.contains(ev) {
                out.push(ev.clone());
            }
        }
    }
    Ok(out)
}

/// Return `true` if `LINTHIS_SKIP` is set and asks to skip `event`.
/// Prints an `Error: ...` line and still returns `false` on a malformed value
/// (fail-safe: don't silently skip when the user's intent is unclear).
pub fn should_skip(event: &HookEvent) -> bool {
    let value = match std::env::var(ENV_VAR) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return false,
    };
    match parse(&value) {
        Ok(events) => {
            if events.contains(event) {
                eprintln!(
                    "{}",
                    format!(
                        "⏭  linthis {} skipped via {ENV_VAR}={value}",
                        event.as_str()
                    )
                    .dimmed()
                );
                true
            } else {
                false
            }
        }
        Err(msg) => {
            eprintln!("{}: {msg}", "Error".red());
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[HookEvent]) -> Vec<HookEvent> {
        items.to_vec()
    }

    #[test]
    fn parses_all_token() {
        let got = parse("all").unwrap();
        assert!(got.contains(&HookEvent::PreCommit));
        assert!(got.contains(&HookEvent::PostCommit));
        assert!(got.contains(&HookEvent::CommitMsg));
        assert!(got.contains(&HookEvent::PrePush));
    }

    #[test]
    fn check_expands_to_pre_and_post_commit() {
        assert_eq!(
            parse("check").unwrap(),
            set(&[HookEvent::PreCommit, HookEvent::PostCommit])
        );
        assert_eq!(
            parse("pc").unwrap(),
            set(&[HookEvent::PreCommit, HookEvent::PostCommit])
        );
    }

    #[test]
    fn cmsg_aliases() {
        for tok in ["cmsg", "cm", "commit-msg", "CMSG"] {
            assert_eq!(
                parse(tok).unwrap(),
                set(&[HookEvent::CommitMsg]),
                "tok={tok}"
            );
        }
    }

    #[test]
    fn prepush_aliases() {
        for tok in ["pp", "pre-push", "PRE-PUSH"] {
            assert_eq!(parse(tok).unwrap(), set(&[HookEvent::PrePush]), "tok={tok}");
        }
    }

    #[test]
    fn explicit_only_one_of_the_pair() {
        assert_eq!(parse("pre-commit").unwrap(), set(&[HookEvent::PreCommit]));
        assert_eq!(parse("post-commit").unwrap(), set(&[HookEvent::PostCommit]));
    }

    #[test]
    fn combines_tokens_with_comma() {
        let got = parse("cmsg, pp").unwrap();
        assert!(got.contains(&HookEvent::CommitMsg));
        assert!(got.contains(&HookEvent::PrePush));
        assert_eq!(got.len(), 2);
    }

    #[test]
    fn dedupes_repeats() {
        let got = parse("pc,pre-commit,check").unwrap();
        assert_eq!(got.len(), 2);
        assert!(got.contains(&HookEvent::PreCommit));
        assert!(got.contains(&HookEvent::PostCommit));
    }

    #[test]
    fn empty_input_yields_empty() {
        assert!(parse("").unwrap().is_empty());
        assert!(parse(" , , ").unwrap().is_empty());
    }

    #[test]
    fn unknown_token_errors() {
        let err = parse("foo").unwrap_err();
        assert!(err.contains("unknown value 'foo'"));
    }
}
