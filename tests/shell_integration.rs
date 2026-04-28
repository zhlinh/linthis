// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

//! End-to-end test for `linthis shell` against a temp $HOME.

use std::path::PathBuf;
use std::process::Command;

fn linthis_bin() -> PathBuf {
    // CARGO_BIN_EXE_<name> is set by Cargo for integration tests.
    PathBuf::from(env!("CARGO_BIN_EXE_linthis"))
}

fn run(home: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(linthis_bin())
        .env("HOME", home)
        .env_remove("LINTHIS_HOOK_COLOR")
        .args(args)
        .output()
        .expect("spawn linthis")
}

#[test]
fn add_alias_then_remove_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();

    // 1) add alias for bash → state, source, rc all written.
    let out = run(home, &["shell", "add", "alias", "--shell", "bash"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let bashrc = std::fs::read_to_string(home.join(".bashrc")).unwrap();
    assert!(bashrc.contains("# >>> linthis shell init >>>"));
    assert!(bashrc.contains(".linthis/shell.bash"));

    let source = std::fs::read_to_string(home.join(".linthis/shell.bash")).unwrap();
    assert!(source.contains("alias lt='linthis'"));
    assert!(source.contains("ltr() { linthis report show \"$@\"; }"));
    assert!(
        !source.contains("eval \"$(linthis shell completion bash"),
        "ac was not requested but appeared in source: {source}"
    );

    let state = std::fs::read_to_string(home.join(".linthis/shell-state.toml")).unwrap();
    assert!(state.contains("[bash]") && state.contains("alias = true"));

    // 2) Second add is idempotent — no error, content unchanged.
    let out2 = run(home, &["shell", "add", "alias", "--shell", "bash"]);
    assert!(out2.status.success());
    let source2 = std::fs::read_to_string(home.join(".linthis/shell.bash")).unwrap();
    assert_eq!(source, source2);

    // 3) Now add ac too — source should grow to include both sections.
    let out3 = run(home, &["shell", "add", "ac", "--shell", "bash"]);
    assert!(out3.status.success());
    let source3 = std::fs::read_to_string(home.join(".linthis/shell.bash")).unwrap();
    assert!(source3.contains("eval \"$(linthis shell completion bash"));
    assert!(source3.contains("alias lt='linthis'"));

    // 4) Remove ac — source keeps only alias.
    let out4 = run(home, &["shell", "remove", "ac", "--shell", "bash"]);
    assert!(out4.status.success());
    let source4 = std::fs::read_to_string(home.join(".linthis/shell.bash")).unwrap();
    assert!(!source4.contains("eval \"$(linthis shell completion bash"));
    assert!(source4.contains("alias lt='linthis'"));
    assert!(
        home.join(".bashrc").exists(),
        "rc still has alias section, marker should remain"
    );

    // 5) Remove alias — source file deleted, rc marker block gone.
    let out5 = run(home, &["shell", "remove", "alias", "--shell", "bash"]);
    assert!(out5.status.success());
    assert!(
        !home.join(".linthis/shell.bash").exists(),
        "source file should be deleted"
    );
    let bashrc_final = std::fs::read_to_string(home.join(".bashrc")).unwrap();
    assert!(
        !bashrc_final.contains("# >>> linthis shell init >>>"),
        "marker block should be gone: {bashrc_final}"
    );
}

#[test]
fn add_all_targets_all_four_shells() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();

    let out = run(home, &["shell", "add", "all", "--shell", "all"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    for ext in ["bash", "zsh", "fish", "ps1"] {
        let p = home.join(".linthis").join(format!("shell.{ext}"));
        assert!(p.exists(), "missing {p:?}");
    }
    assert!(home.join(".bashrc").exists());
    assert!(home.join(".zshrc").exists());
    assert!(home.join(".config/fish/config.fish").exists());
    assert!(home
        .join("Documents/PowerShell/Microsoft.PowerShell_profile.ps1")
        .exists());
}

#[test]
fn unknown_feature_exits_nonzero() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(
        dir.path(),
        &["shell", "add", "everything", "--shell", "bash"],
    );
    assert!(!out.status.success(), "expected nonzero exit");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("unknown feature"), "stderr: {err}");
}

#[test]
fn unknown_shell_exits_nonzero() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(dir.path(), &["shell", "add", "ac", "--shell", "elvish"]);
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("unrecognized") || err.contains("elvish"));
}
