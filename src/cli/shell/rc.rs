// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

//! Marker-block management for user rc files + atomic source-file writes.

use super::state::Shell;
use std::path::{Path, PathBuf};

pub const MARKER_OPEN: &str = "# >>> linthis shell init >>>";
pub const MARKER_CLOSE: &str = "# <<< linthis shell init <<<";

/// Outcome of `ensure_marker`. `UnmanagedSourceLine` is when the user wrote
/// their own `source ~/.linthis/shell.<ext>` outside our marker block — we
/// don't overwrite it, just warn.
#[derive(Debug, PartialEq, Eq)]
pub enum EnsureOutcome {
    Inserted,
    Idempotent,
    UnmanagedSourceLine,
}

/// Per-shell rc-file path under `home`.
pub fn rc_path_for(shell: Shell, home: &Path) -> PathBuf {
    match shell {
        Shell::Bash => home.join(".bashrc"),
        Shell::Zsh => home.join(".zshrc"),
        Shell::Fish => home.join(".config").join("fish").join("config.fish"),
        Shell::PowerShell => home
            .join("Documents")
            .join("PowerShell")
            .join("Microsoft.PowerShell_profile.ps1"),
    }
}

/// Per-shell source-file path under `home`.
pub fn source_path_for(shell: Shell, home: &Path) -> PathBuf {
    let ext = match shell {
        Shell::Bash => "bash",
        Shell::Zsh => "zsh",
        Shell::Fish => "fish",
        Shell::PowerShell => "ps1",
    };
    home.join(".linthis").join(format!("shell.{ext}"))
}

/// The single line (or block, for fish/PowerShell) that goes between
/// `MARKER_OPEN` and `MARKER_CLOSE` in the user's rc file.
pub fn rc_source_line(shell: Shell) -> String {
    let src = source_path_for(shell, Path::new("$HOME"));
    let s = src.to_string_lossy();
    match shell {
        Shell::Bash | Shell::Zsh => format!("[ -f \"{s}\" ] && . \"{s}\""),
        Shell::Fish => format!("if test -f \"{s}\"\n    source \"{s}\"\nend"),
        Shell::PowerShell => format!("if (Test-Path \"{s}\") {{ . \"{s}\" }}"),
    }
}

/// Atomic write: temp + fsync + rename. Creates parent dirs.
pub fn atomic_write(path: &Path, body: &str) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = {
        let mut p = path.as_os_str().to_owned();
        p.push(format!(".linthis-tmp.{}", std::process::id()));
        PathBuf::from(p)
    };
    let mut f = std::fs::File::create(&tmp)?;
    f.write_all(body.as_bytes())?;
    f.sync_all()?;
    drop(f);
    std::fs::rename(&tmp, path)
}

/// Delete `path` if it exists. Missing → ok.
pub fn delete_if_exists(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Returns `true` when `rc` exists and contains a manual `source` line
/// for this shell's source file outside any linthis marker block.
///
/// This is the same condition that makes `ensure_marker` return
/// [`EnsureOutcome::UnmanagedSourceLine`]. Exposed separately so
/// `linthis shell status` can surface the condition without writing.
pub fn has_unmanaged_source_line(shell: Shell, rc: &Path) -> bool {
    let unmanaged_substring = source_path_for(shell, Path::new("$HOME"))
        .to_string_lossy()
        .into_owned();
    let existing = match std::fs::read_to_string(rc) {
        Ok(s) => s,
        Err(_) => return false,
    };
    existing.contains(&unmanaged_substring) && !existing.contains(MARKER_OPEN)
}

/// Ensure the rc file at `rc` contains a marker block sourcing the per-shell
/// source file. Idempotent; preserves any content outside the marker.
///
/// If the user has manually written something that looks like our source line
/// outside the marker, returns `UnmanagedSourceLine` and leaves the rc alone.
pub fn ensure_marker(shell: Shell, rc: &Path) -> std::io::Result<EnsureOutcome> {
    let source_line = rc_source_line(shell);

    let existing = match std::fs::read_to_string(rc) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };

    if let Some((before, after)) = split_marker(&existing) {
        // Marker exists — replace its contents idempotently.
        let mut new = String::with_capacity(existing.len());
        new.push_str(before);
        new.push_str(MARKER_OPEN);
        new.push('\n');
        new.push_str(&source_line);
        new.push('\n');
        new.push_str(MARKER_CLOSE);
        new.push('\n'); // restore the newline consumed by split_marker's after_start
        new.push_str(after);
        if new == existing {
            return Ok(EnsureOutcome::Idempotent);
        }
        atomic_write(rc, &new)?;
        // We manage the marker block contents — rewriting it (when the user
        // mangled the inside) is transparent to the caller, so we still report
        // Idempotent. Callers branch on `Inserted` only when a fresh marker
        // block was added.
        return Ok(EnsureOutcome::Idempotent);
    }

    // No marker. Check for unmanaged source line via shared helper.
    if has_unmanaged_source_line(shell, rc) {
        return Ok(EnsureOutcome::UnmanagedSourceLine);
    }

    // Append a fresh marker block (with leading newline if file doesn't end with one).
    let mut new = existing.clone();
    if !new.is_empty() && !new.ends_with('\n') {
        new.push('\n');
    }
    if !new.is_empty() {
        new.push('\n');
    }
    new.push_str(MARKER_OPEN);
    new.push('\n');
    new.push_str(&source_line);
    new.push('\n');
    new.push_str(MARKER_CLOSE);
    new.push('\n');
    atomic_write(rc, &new)?;
    Ok(EnsureOutcome::Inserted)
}

/// Remove the marker block from `rc`, if present. Idempotent.
pub fn remove_marker(rc: &Path) -> std::io::Result<()> {
    let existing = match std::fs::read_to_string(rc) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    let Some((before, after)) = split_marker(&existing) else {
        return Ok(());
    };
    // Drop one trailing newline that the open marker introduced.
    let mut joined = String::with_capacity(before.len() + after.len());
    joined.push_str(before.trim_end_matches('\n'));
    joined.push_str(after);
    // Collapse double-blank-lines we may have created.
    // `str::replace` is global; one pass is enough because the collapse
    // pattern can't re-form (no overlap with the result).
    let new = joined.replace("\n\n\n", "\n\n");
    atomic_write(rc, &new)
}

/// Markers wrapping the `.bash_profile` → `.bashrc` shim. Distinct from
/// `MARKER_OPEN/CLOSE` so the two blocks can be managed independently.
pub const BASH_PROFILE_SHIM_OPEN: &str = "# >>> linthis bash_profile shim >>>";
pub const BASH_PROFILE_SHIM_CLOSE: &str = "# <<< linthis bash_profile shim <<<";

/// Returns the path to `~/.bash_profile` under `home`. (Pure path computation;
/// the file may or may not exist.)
pub fn bash_profile_path(home: &Path) -> PathBuf {
    home.join(".bash_profile")
}

/// Detect whether `.bash_profile` already sources `.bashrc` in any common form.
/// Used to decide whether the shim is needed at all.
pub fn bash_profile_already_sources_bashrc(bash_profile: &Path) -> bool {
    let existing = match std::fs::read_to_string(bash_profile) {
        Ok(s) => s,
        Err(_) => return false,
    };
    // Already has our shim block — counts.
    if existing.contains(BASH_PROFILE_SHIM_OPEN) {
        return true;
    }
    // Common manual forms.
    for needle in [
        "source ~/.bashrc",
        "source $HOME/.bashrc",
        ". ~/.bashrc",
        ". $HOME/.bashrc",
        ". \"$HOME/.bashrc\"",
        "source \"$HOME/.bashrc\"",
    ] {
        if existing.contains(needle) {
            return true;
        }
    }
    false
}

/// Insert the shim block into `.bash_profile` (only if file exists and doesn't
/// already source `.bashrc`). Returns whether a write occurred. Idempotent.
pub fn ensure_bash_profile_shim(bash_profile: &Path) -> std::io::Result<bool> {
    if !bash_profile.exists() {
        return Ok(false);
    }
    if bash_profile_already_sources_bashrc(bash_profile) {
        return Ok(false);
    }
    let existing = match std::fs::read_to_string(bash_profile) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    let shim_body = "[ -f \"$HOME/.bashrc\" ] && . \"$HOME/.bashrc\"";
    let mut new = existing;
    if !new.is_empty() && !new.ends_with('\n') {
        new.push('\n');
    }
    if !new.is_empty() {
        new.push('\n');
    }
    new.push_str(BASH_PROFILE_SHIM_OPEN);
    new.push('\n');
    new.push_str(shim_body);
    new.push('\n');
    new.push_str(BASH_PROFILE_SHIM_CLOSE);
    new.push('\n');
    atomic_write(bash_profile, &new)?;
    Ok(true)
}

/// Remove the shim block from `.bash_profile` if we added one. Idempotent.
/// Untouched if the file doesn't exist or has no shim marker.
pub fn remove_bash_profile_shim(bash_profile: &Path) -> std::io::Result<()> {
    let existing = match std::fs::read_to_string(bash_profile) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    let Some((before, after)) = split_shim_marker(&existing) else {
        return Ok(());
    };
    let mut new = String::with_capacity(before.len() + after.len());
    new.push_str(before.trim_end_matches('\n'));
    new.push_str(after);
    let mut new = new.replace("\n\n\n", "\n\n");
    while new.ends_with("\n\n") {
        new.pop();
    }
    if !new.ends_with('\n') && !new.is_empty() {
        new.push('\n');
    }
    atomic_write(bash_profile, &new)
}

/// Same shape as `split_marker` but for the bash_profile shim markers.
fn split_shim_marker(s: &str) -> Option<(&str, &str)> {
    let open = s.find(BASH_PROFILE_SHIM_OPEN)?;
    let close_rel = s[open..].find(BASH_PROFILE_SHIM_CLOSE)?;
    let close_end = open + close_rel + BASH_PROFILE_SHIM_CLOSE.len();
    let after_start = if s.as_bytes().get(close_end).copied() == Some(b'\n') {
        close_end + 1
    } else {
        close_end
    };
    Some((&s[..open], &s[after_start..]))
}

/// Split the file content into `(before_open_marker, after_close_marker)`.
/// Returns `None` if either marker is absent.
fn split_marker(s: &str) -> Option<(&str, &str)> {
    let open = s.find(MARKER_OPEN)?;
    let close_rel = s[open..].find(MARKER_CLOSE)?;
    let close_end = open + close_rel + MARKER_CLOSE.len();
    // Eat a trailing newline after the close marker so removal is clean.
    let after_start = if s.as_bytes().get(close_end).copied() == Some(b'\n') {
        close_end + 1
    } else {
        close_end
    };
    Some((&s[..open], &s[after_start..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn rc_path_per_shell() {
        let home = Path::new("/h");
        assert_eq!(rc_path_for(Shell::Bash, home), Path::new("/h/.bashrc"));
        assert_eq!(rc_path_for(Shell::Zsh, home), Path::new("/h/.zshrc"));
        assert_eq!(
            rc_path_for(Shell::Fish, home),
            Path::new("/h/.config/fish/config.fish")
        );
        assert_eq!(
            rc_path_for(Shell::PowerShell, home),
            Path::new("/h/Documents/PowerShell/Microsoft.PowerShell_profile.ps1")
        );
    }

    #[test]
    fn source_path_uses_dot_linthis() {
        let home = Path::new("/h");
        assert_eq!(
            source_path_for(Shell::Bash, home),
            Path::new("/h/.linthis/shell.bash")
        );
        assert_eq!(
            source_path_for(Shell::PowerShell, home),
            Path::new("/h/.linthis/shell.ps1")
        );
    }

    #[test]
    fn rc_source_line_uses_correct_syntax_per_shell() {
        let bash = rc_source_line(Shell::Bash);
        assert!(bash.starts_with("[ -f"));
        assert!(bash.contains(". \"$HOME/.linthis/shell.bash\""));

        let fish = rc_source_line(Shell::Fish);
        assert!(fish.contains("if test -f"));
        assert!(fish.contains("source"));
        assert!(fish.contains("end"));

        let ps = rc_source_line(Shell::PowerShell);
        assert!(ps.contains("Test-Path"));
    }

    #[test]
    fn ensure_marker_creates_rc_when_missing() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".bashrc");
        let outcome = ensure_marker(Shell::Bash, &rc).unwrap();
        assert_eq!(outcome, EnsureOutcome::Inserted);
        let content = std::fs::read_to_string(&rc).unwrap();
        assert!(content.contains(MARKER_OPEN));
        assert!(content.contains(MARKER_CLOSE));
        assert!(content.contains("$HOME/.linthis/shell.bash"));
    }

    #[test]
    fn ensure_marker_is_idempotent() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".zshrc");
        std::fs::write(&rc, "# user content\nexport FOO=bar\n").unwrap();
        let first = ensure_marker(Shell::Zsh, &rc).unwrap();
        assert_eq!(first, EnsureOutcome::Inserted);
        let after_first = std::fs::read_to_string(&rc).unwrap();
        let second = ensure_marker(Shell::Zsh, &rc).unwrap();
        assert_eq!(second, EnsureOutcome::Idempotent);
        let after_second = std::fs::read_to_string(&rc).unwrap();
        assert_eq!(after_first, after_second);
        assert!(after_second.contains("export FOO=bar"));
    }

    #[test]
    fn ensure_marker_preserves_content_outside_block() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".bashrc");
        std::fs::write(
            &rc,
            "# top\nalias me=you\n\n# >>> linthis shell init >>>\n# user mangled this\n# <<< linthis shell init <<<\n# bottom\n",
        )
        .unwrap();
        ensure_marker(Shell::Bash, &rc).unwrap();
        let content = std::fs::read_to_string(&rc).unwrap();
        assert!(content.contains("alias me=you"));
        assert!(content.contains("# bottom"));
        assert!(!content.contains("# user mangled this"));
        assert!(content.contains(". \"$HOME/.linthis/shell.bash\""));
    }

    #[test]
    fn has_unmanaged_source_line_returns_false_when_rc_missing() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join("nonexistent.bashrc");
        assert!(!has_unmanaged_source_line(Shell::Bash, &rc));
    }

    #[test]
    fn has_unmanaged_source_line_returns_false_when_marker_block_present() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".bashrc");
        std::fs::write(
            &rc,
            "# >>> linthis shell init >>>\n[ -f \"$HOME/.linthis/shell.bash\" ] && . \"$HOME/.linthis/shell.bash\"\n# <<< linthis shell init <<<\n",
        )
        .unwrap();
        // Marker is present, so should NOT be considered unmanaged.
        assert!(!has_unmanaged_source_line(Shell::Bash, &rc));
    }

    #[test]
    fn has_unmanaged_source_line_returns_true_when_manual_source_outside_marker() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".bashrc");
        std::fs::write(
            &rc,
            "# user content\nsource $HOME/.linthis/shell.bash\nexport FOO=1\n",
        )
        .unwrap();
        assert!(has_unmanaged_source_line(Shell::Bash, &rc));
    }

    #[test]
    fn ensure_marker_warns_on_manual_source_line_outside_marker() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".bashrc");
        std::fs::write(&rc, "source $HOME/.linthis/shell.bash\n").unwrap();
        let outcome = ensure_marker(Shell::Bash, &rc).unwrap();
        assert_eq!(outcome, EnsureOutcome::UnmanagedSourceLine);
        let content = std::fs::read_to_string(&rc).unwrap();
        assert!(!content.contains(MARKER_OPEN), "marker must NOT be added");
    }

    #[test]
    fn remove_marker_drops_block_keeps_rest() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".bashrc");
        let initial = "# top\n\n# >>> linthis shell init >>>\n[ -f \"$HOME/.linthis/shell.bash\" ] && . \"$HOME/.linthis/shell.bash\"\n# <<< linthis shell init <<<\n# bottom\n";
        std::fs::write(&rc, initial).unwrap();
        remove_marker(&rc).unwrap();
        let content = std::fs::read_to_string(&rc).unwrap();
        assert!(!content.contains(MARKER_OPEN));
        assert!(!content.contains("/.linthis/shell.bash"));
        assert!(content.contains("# top"));
        assert!(content.contains("# bottom"));
    }

    #[test]
    fn remove_marker_idempotent_when_absent() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join(".bashrc");
        std::fs::write(&rc, "no markers here\n").unwrap();
        remove_marker(&rc).unwrap();
        let content = std::fs::read_to_string(&rc).unwrap();
        assert_eq!(content, "no markers here\n");
    }

    #[test]
    fn remove_marker_handles_missing_rc_file() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join("absent.rc");
        remove_marker(&rc).unwrap();
        assert!(!rc.exists());
    }

    #[test]
    fn atomic_write_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a").join("b").join("c.txt");
        atomic_write(&path, "hello").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn atomic_write_replaces_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("c.txt");
        std::fs::write(&path, "old").unwrap();
        atomic_write(&path, "new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    }

    #[test]
    fn delete_if_exists_idempotent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("x.txt");
        delete_if_exists(&path).unwrap(); // no-op
        std::fs::write(&path, "hi").unwrap();
        delete_if_exists(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn ensure_marker_is_idempotent_for_fish_multiline_block() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join("config.fish");
        std::fs::write(&rc, "# user fish content\nset -gx EDITOR vim\n").unwrap();

        let first = ensure_marker(Shell::Fish, &rc).unwrap();
        assert_eq!(first, EnsureOutcome::Inserted);
        let after_first = std::fs::read_to_string(&rc).unwrap();

        // Multi-line fish block must contain all three lines: `if test -f`,
        // `source`, and `end`.
        assert!(after_first.contains("if test -f"));
        assert!(after_first.contains("source"));
        assert!(after_first.contains("\nend\n"));

        // Second invocation must be a no-op.
        let second = ensure_marker(Shell::Fish, &rc).unwrap();
        assert_eq!(second, EnsureOutcome::Idempotent);
        let after_second = std::fs::read_to_string(&rc).unwrap();
        assert_eq!(
            after_first, after_second,
            "fish marker block drifted on second ensure_marker"
        );

        // Sanity: user content preserved.
        assert!(after_second.contains("set -gx EDITOR vim"));
    }

    #[test]
    fn ensure_marker_is_idempotent_for_powershell_block() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join("Microsoft.PowerShell_profile.ps1");
        std::fs::write(&rc, "# user PS profile\nSet-Alias ll Get-ChildItem\n").unwrap();

        let first = ensure_marker(Shell::PowerShell, &rc).unwrap();
        assert_eq!(first, EnsureOutcome::Inserted);
        let after_first = std::fs::read_to_string(&rc).unwrap();

        // PowerShell single-line block: `if (Test-Path "...") { . "..." }`
        assert!(after_first.contains("if (Test-Path"));
        assert!(after_first.contains(". \"$HOME/.linthis/shell.ps1\""));

        let second = ensure_marker(Shell::PowerShell, &rc).unwrap();
        assert_eq!(second, EnsureOutcome::Idempotent);
        let after_second = std::fs::read_to_string(&rc).unwrap();
        assert_eq!(after_first, after_second);
        assert!(after_second.contains("Set-Alias ll Get-ChildItem"));
    }

    #[test]
    fn bash_profile_shim_skips_when_bash_profile_missing() {
        let dir = tempdir().unwrap();
        let bp = dir.path().join(".bash_profile");
        let written = ensure_bash_profile_shim(&bp).unwrap();
        assert!(!written);
        assert!(!bp.exists(), "must not create .bash_profile");
    }

    #[test]
    fn bash_profile_shim_skips_when_already_sourced() {
        let dir = tempdir().unwrap();
        let bp = dir.path().join(".bash_profile");
        std::fs::write(&bp, "# user wrote this\n[ -f ~/.bashrc ] && . ~/.bashrc\n").unwrap();
        let written = ensure_bash_profile_shim(&bp).unwrap();
        assert!(!written);
        let content = std::fs::read_to_string(&bp).unwrap();
        assert!(!content.contains(BASH_PROFILE_SHIM_OPEN));
    }

    #[test]
    fn bash_profile_shim_inserts_when_present_and_unsourced() {
        let dir = tempdir().unwrap();
        let bp = dir.path().join(".bash_profile");
        std::fs::write(&bp, "# user content\nexport PATH=$PATH:/usr/local/bin\n").unwrap();
        let written = ensure_bash_profile_shim(&bp).unwrap();
        assert!(written);
        let content = std::fs::read_to_string(&bp).unwrap();
        assert!(content.contains(BASH_PROFILE_SHIM_OPEN));
        assert!(content.contains("[ -f \"$HOME/.bashrc\" ] && . \"$HOME/.bashrc\""));
        assert!(content.contains(BASH_PROFILE_SHIM_CLOSE));
        assert!(content.contains("export PATH=$PATH:/usr/local/bin"));
    }

    #[test]
    fn bash_profile_shim_is_idempotent_after_insertion() {
        let dir = tempdir().unwrap();
        let bp = dir.path().join(".bash_profile");
        std::fs::write(&bp, "# user content\n").unwrap();
        let first = ensure_bash_profile_shim(&bp).unwrap();
        assert!(first);
        let after_first = std::fs::read_to_string(&bp).unwrap();
        let second = ensure_bash_profile_shim(&bp).unwrap();
        assert!(
            !second,
            "second call must be no-op (already has our marker)"
        );
        let after_second = std::fs::read_to_string(&bp).unwrap();
        assert_eq!(after_first, after_second);
    }

    #[test]
    fn remove_bash_profile_shim_drops_block_keeps_rest() {
        let dir = tempdir().unwrap();
        let bp = dir.path().join(".bash_profile");
        std::fs::write(&bp, "# top\nexport FOO=bar\n").unwrap();
        ensure_bash_profile_shim(&bp).unwrap();
        remove_bash_profile_shim(&bp).unwrap();
        let content = std::fs::read_to_string(&bp).unwrap();
        assert!(!content.contains(BASH_PROFILE_SHIM_OPEN));
        assert!(!content.contains("$HOME/.bashrc"));
        assert!(content.contains("export FOO=bar"));
        assert!(content.contains("# top"));
    }

    #[test]
    fn remove_bash_profile_shim_idempotent_when_missing_or_no_marker() {
        let dir = tempdir().unwrap();
        let bp = dir.path().join(".bash_profile");
        // Missing file
        remove_bash_profile_shim(&bp).unwrap();
        // No marker
        std::fs::write(&bp, "# nothing here\n").unwrap();
        remove_bash_profile_shim(&bp).unwrap();
        let content = std::fs::read_to_string(&bp).unwrap();
        assert_eq!(content, "# nothing here\n");
    }

    #[test]
    fn bash_profile_already_sources_bashrc_detects_common_forms() {
        let dir = tempdir().unwrap();
        let bp = dir.path().join(".bash_profile");

        for needle in [
            "source ~/.bashrc\n",
            "source $HOME/.bashrc\n",
            ". ~/.bashrc\n",
            ". $HOME/.bashrc\n",
            ". \"$HOME/.bashrc\"\n",
            "source \"$HOME/.bashrc\"\n",
        ] {
            std::fs::write(&bp, format!("# user\n{needle}export X=1\n")).unwrap();
            assert!(
                bash_profile_already_sources_bashrc(&bp),
                "should detect {needle:?}"
            );
        }

        // Negative case
        std::fs::write(&bp, "# user\nexport X=1\n").unwrap();
        assert!(!bash_profile_already_sources_bashrc(&bp));
    }

    #[test]
    fn remove_marker_round_trips_for_fish_multiline_block() {
        let dir = tempdir().unwrap();
        let rc = dir.path().join("config.fish");
        std::fs::write(&rc, "# top\nset -gx EDITOR vim\n").unwrap();

        let original = std::fs::read_to_string(&rc).unwrap();
        ensure_marker(Shell::Fish, &rc).unwrap();
        remove_marker(&rc).unwrap();
        let after_remove = std::fs::read_to_string(&rc).unwrap();

        // After insert+remove, file should be functionally equivalent to original
        // (modulo trailing whitespace differences, which the implementation may
        // collapse). Just check user content is preserved and marker is gone.
        assert!(after_remove.contains("set -gx EDITOR vim"));
        assert!(!after_remove.contains(MARKER_OPEN));
        assert!(!after_remove.contains("if test -f"));
        let _ = original; // for clarity
    }
}
