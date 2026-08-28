// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! linthis's share of a git hook file, delimited so the rest survives.
//!
//! A hook name belongs to whoever writes the file, and more than one tool
//! wants the same names: Git LFS installs `pre-push`, `post-commit`,
//! `post-checkout` and `post-merge`. Overwriting one of those stops large
//! files from being uploaded, silently. So linthis owns a marked block and
//! treats everything outside it as somebody else's.

use crate::cli::commands::{HookEvent, HookTool};

/// Opening marker for the region linthis rewrites.
pub(crate) const BLOCK_START: &str = "# >>> linthis managed block >>>";
/// Closing marker.
pub(crate) const BLOCK_END: &str = "# <<< linthis managed block <<<";

const SHEBANG: &str = "#!/bin/sh";

/// Build the linthis half of a hook.
///
/// Deliberately not `exec`: the whole point is that something can follow.
/// Failure propagates with the original exit code, so a blocked commit still
/// blocks.
pub(crate) fn build_block(
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

    let invocation = format!(
        "linthis hook run --event {} --type {}{}{}{}",
        event.as_str(),
        hook_type.as_str(),
        provider_arg,
        provider_args_arg,
        global_arg,
    );

    let body = if matches!(event, HookEvent::PrePush) {
        // git feeds the refs to push on stdin, and linthis reads all of it.
        // Without this the next hook in the file sees EOF and concludes there
        // is nothing to do — which is how LFS ends up uploading nothing.
        format!(
            "\x20 _LINTHIS_STDIN=$(mktemp) || exit 1\n\
             \x20 cat > \"$_LINTHIS_STDIN\"\n\
             \x20 {invocation} \"$@\" < \"$_LINTHIS_STDIN\"\n\
             \x20 _LINTHIS_CODE=$?\n\
             \x20 if [ \"$_LINTHIS_CODE\" -ne 0 ]; then\n\
             \x20\x20\x20 rm -f \"$_LINTHIS_STDIN\"\n\
             \x20\x20\x20 exit \"$_LINTHIS_CODE\"\n\
             \x20 fi\n\
             \x20 # Hand the same refs to whatever follows this block.\n\
             \x20 exec 0< \"$_LINTHIS_STDIN\"\n\
             \x20 rm -f \"$_LINTHIS_STDIN\"\n"
        )
    } else {
        format!("\x20 {invocation} \"$@\" || exit $?\n")
    };

    format!(
        "{BLOCK_START}\n\
         # Written by `linthis hook add`; `linthis hook sync` rewrites it.\n\
         # Anything outside these markers is left alone — put other tools' hooks there.\n\
         if command -v linthis >/dev/null 2>&1; then\n\
         {body}\
         fi\n\
         {BLOCK_END}\n"
    )
}

/// What `upsert_block` did, or why it refused.
pub(crate) enum Upsert {
    /// The file's new contents.
    Merged(String),
    /// The file calls linthis outside the markers, in code linthis did not
    /// write. Rewriting it is the caller's business to raise with the user:
    /// deleting the invocation would leave the surrounding shell behind, and
    /// leaving it would run linthis twice.
    HandWritten,
}

/// Put `block` into `existing`, keeping every line that is not linthis's.
///
/// The block replaces a previous one, or a hook linthis itself wrote in an
/// older format; otherwise it goes right after the shebang, ahead of whatever
/// else the file does. Hand-written invocations are never edited.
pub(crate) fn upsert_block(existing: Option<&str>, block: &str) -> Upsert {
    let Some(existing) = existing.filter(|s| !s.trim().is_empty()) else {
        return Upsert::Merged(format!("{SHEBANG}\n{block}"));
    };

    if let Some(replaced) = replace_marked_block(existing, block) {
        return Upsert::Merged(replaced);
    }

    let mut lines: Vec<&str> = existing.lines().collect();
    if is_linthis_only(&lines) {
        // A file linthis wrote and owns: replace it outright.
        return Upsert::Merged(format!("{SHEBANG}\n{block}"));
    }
    if lines.iter().any(|l| invokes_linthis(l)) {
        return Upsert::HandWritten;
    }

    let insert_at = usize::from(lines.first().is_some_and(|l| l.starts_with("#!")));
    let mut out: Vec<String> = lines[..insert_at].iter().map(|l| l.to_string()).collect();
    if insert_at == 0 {
        out.push(SHEBANG.to_string());
    }
    out.push(block.trim_end().to_string());
    out.extend(lines.drain(insert_at..).map(|l| l.to_string()));

    let mut text = out.join("\n");
    text.push('\n');
    Upsert::Merged(text)
}

/// Whether every meaningful line is one linthis wrote: the shebang, comments,
/// and its own invocation. Such a file can be replaced wholesale.
fn is_linthis_only(lines: &[&str]) -> bool {
    let mut saw_invocation = false;
    for line in lines {
        let t = line.trim();
        if t.is_empty() || t.starts_with("#!") || t.starts_with('#') {
            continue;
        }
        if invokes_linthis(line) {
            saw_invocation = true;
            continue;
        }
        return false;
    }
    saw_invocation
}

/// Whether a line runs linthis as a hook.
fn invokes_linthis(line: &str) -> bool {
    let t = line.trim();
    t.contains("linthis hook run") || t.contains("--hook-event")
}

/// Remove linthis's block. `None` when nothing but a shebang would remain, so
/// the caller can delete the file instead of leaving an empty hook.
pub(crate) fn remove_block(existing: &str) -> Option<String> {
    let mut lines: Vec<&str> = existing.lines().collect();
    match (
        lines.iter().position(|l| l.trim() == BLOCK_START),
        lines.iter().position(|l| l.trim() == BLOCK_END),
    ) {
        (Some(start), Some(end)) if end >= start => {
            lines.drain(start..=end);
        }
        // A file linthis wrote entirely can go; anything else keeps its own
        // code, including a hand-written invocation we must not edit.
        _ if is_linthis_only(&lines) => return None,
        _ => {}
    }

    let has_content = lines
        .iter()
        .any(|l| !l.trim().is_empty() && !l.starts_with("#!") && !l.trim_start().starts_with('#'));
    if !has_content {
        return None;
    }

    let mut text = lines.join("\n");
    text.push('\n');
    Some(text)
}

/// Whether this file already carries linthis's block.
pub(crate) fn has_block(content: &str) -> bool {
    content.contains(BLOCK_START)
}

/// Whether the file does anything besides run linthis.
pub(crate) fn has_foreign_content(content: &str) -> bool {
    let mut lines: Vec<&str> = content.lines().collect();
    match (
        lines.iter().position(|l| l.trim() == BLOCK_START),
        lines.iter().position(|l| l.trim() == BLOCK_END),
    ) {
        (Some(start), Some(end)) if end >= start => {
            lines.drain(start..=end);
        }
        _ if is_linthis_only(&lines) => return false,
        _ => {}
    }
    lines
        .iter()
        .any(|l| !l.trim().is_empty() && !l.starts_with("#!") && !l.trim_start().starts_with('#'))
}

/// Replace an existing marked block in place.
fn replace_marked_block(existing: &str, block: &str) -> Option<String> {
    let lines: Vec<&str> = existing.lines().collect();
    let start = lines.iter().position(|l| l.trim() == BLOCK_START)?;
    let end = lines.iter().position(|l| l.trim() == BLOCK_END)?;
    if end < start {
        return None;
    }

    let mut out: Vec<String> = lines[..start].iter().map(|l| l.to_string()).collect();
    out.push(block.trim_end().to_string());
    out.extend(lines[end + 1..].iter().map(|l| l.to_string()));

    let mut text = out.join("\n");
    text.push('\n');
    Some(text)
}


#[cfg(test)]
mod tests {
    use super::*;

    fn block() -> String {
        build_block(&HookEvent::PrePush, &HookTool::Git, None, true, None)
    }

    fn merged(existing: Option<&str>) -> String {
        match upsert_block(existing, &block()) {
            Upsert::Merged(text) => text,
            Upsert::HandWritten => panic!("expected a merge, got a refusal"),
        }
    }

    /// A chain somebody wrote by hand, of the shape people actually produce
    /// when linthis and Git LFS collide.
    const HAND_WRITTEN: &str = "#!/bin/sh\n\
        # pre-push: run linthis, then Git LFS.\n\
        if command -v linthis >/dev/null 2>&1; then\n\
        \x20 linthis hook run --event pre-push --type git --global \"$@\"\n\
        \x20 LT_EXIT=$?\n\
        \x20 if [ \"$LT_EXIT\" -ne 0 ]; then\n\
        \x20\x20\x20 exit \"$LT_EXIT\"\n\
        \x20 fi\n\
        fi\n\
        git lfs pre-push \"$@\"\n";

    #[test]
    fn a_hand_written_chain_is_never_edited() {
        // Removing just the invocation would leave `LT_EXIT=$?` reading the
        // exit code of `command -v`, inside an `if` that no longer does
        // anything — which is what an earlier version of this did.
        assert!(matches!(
            upsert_block(Some(HAND_WRITTEN), &block()),
            Upsert::HandWritten
        ));
        // Uninstall leaves it alone for the same reason.
        assert_eq!(remove_block(HAND_WRITTEN).as_deref(), Some(HAND_WRITTEN));
    }

    #[test]
    fn a_foreign_hook_survives_installation() {
        let lfs = "#!/bin/sh\ncommand -v git-lfs >/dev/null 2>&1 || exit 2\ngit lfs pre-push \"$@\"\n";
        let Upsert::Merged(out) = upsert_block(Some(lfs), &block()) else {
            panic!("a foreign hook with no linthis call must merge")
        };

        assert!(out.contains("git lfs pre-push"), "LFS hook was dropped:\n{out}");
        assert!(out.contains(BLOCK_START));
        // linthis runs first, so a failed check does not waste the upload.
        assert!(out.find(BLOCK_START) < out.find("git lfs pre-push"));
        assert!(out.starts_with("#!/bin/sh\n"));
    }

    #[test]
    fn syncing_twice_does_not_stack_blocks() {
        let once = merged(Some("#!/bin/sh\ngit lfs pre-push \"$@\"\n"));
        let twice = merged(Some(&once));
        assert_eq!(once, twice);
        assert_eq!(twice.matches(BLOCK_START).count(), 1);
    }

    #[test]
    fn an_old_wrapper_is_replaced_not_appended() {
        let legacy = "#!/bin/sh\nexec linthis hook run --event pre-push --type git --global \"$@\"\n";
        let out = merged(Some(legacy));
        assert_eq!(out.matches("linthis hook run").count(), 1);
        assert!(!out.contains("exec linthis"));
    }

    #[test]
    fn pre_push_replays_stdin_for_the_next_hook() {
        // Without this the following hook reads EOF and uploads nothing.
        let out = block();
        assert!(out.contains("cat > \"$_LINTHIS_STDIN\""));
        assert!(out.contains("exec 0< \"$_LINTHIS_STDIN\""));
        // exec would leave nothing to chain to.
        assert!(!out.contains("exec linthis"));
    }

    #[test]
    fn other_events_do_not_touch_stdin() {
        let out = build_block(&HookEvent::PreCommit, &HookTool::Git, None, false, None);
        assert!(!out.contains("_LINTHIS_STDIN"));
        assert!(out.contains("|| exit $?"));
    }

    #[test]
    fn uninstall_keeps_the_other_tool() {
        let both = merged(Some("#!/bin/sh\ngit lfs pre-push \"$@\"\n"));
        let left = remove_block(&both).expect("LFS hook must remain");
        assert!(left.contains("git lfs pre-push"));
        assert!(!left.contains("linthis"));
    }

    #[test]
    fn uninstall_removes_a_linthis_only_hook() {
        let only = merged(None);
        assert!(remove_block(&only).is_none());
    }

    #[test]
    fn foreign_content_is_recognized() {
        let both = merged(Some("#!/bin/sh\ngit lfs pre-push \"$@\"\n"));
        assert!(has_foreign_content(&both));
        assert!(has_block(&both));

        let only = merged(None);
        assert!(!has_foreign_content(&only));
    }
}
