// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style license that can be
// found at https://opensource.org/license/MIT

//! Integration tests for the TOOL_INSTALLS matrix.
//!
//! Test groups:
//!   1. table_wellformed — static invariants on TOOL_INSTALLS (always runs)
//!   2. command_generation — resolver output shape on current OS (always runs)
//!   3. auto_install_lint_cycle — real install + linthis run (`#[ignore]`, CI only)

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;

use linthis::tools::install::{
    is_tool_supported_on_current_platform, resolve_install_cmds, supported_platforms, Os, ToolRole,
    TOOL_INSTALLS,
};
use linthis::Language;

// ──────────────────────────────── Group 1 ────────────────────────────────

#[test]
fn table_wellformed_no_duplicate_language_role_pairs() {
    let mut seen: HashSet<(Language, ToolRole)> = HashSet::new();
    for spec in TOOL_INSTALLS {
        assert!(
            seen.insert((spec.language, spec.role)),
            "duplicate (language, role) entry for tool '{}': {:?} {:?}",
            spec.tool,
            spec.language,
            spec.role,
        );
    }
}

#[test]
fn table_wellformed_every_platform_has_at_least_one_cmd() {
    for spec in TOOL_INSTALLS {
        for platform in spec.platforms {
            assert!(
                !platform.cmds.is_empty(),
                "{} on {:?} has no commands",
                spec.tool,
                platform.os,
            );
            for cmd in platform.cmds {
                assert!(
                    !cmd.is_empty(),
                    "{} on {:?} has an empty command",
                    spec.tool,
                    platform.os,
                );
                assert!(
                    !cmd[0].is_empty(),
                    "{} on {:?} has an empty program name",
                    spec.tool,
                    platform.os,
                );
            }
        }
    }
}

#[test]
fn table_wellformed_swiftlint_is_macos_only() {
    let platforms = supported_platforms("swiftlint");
    assert_eq!(platforms, vec![Os::MacOs]);
}

#[test]
fn table_wellformed_dart_is_not_registered() {
    assert!(!TOOL_INSTALLS.iter().any(|s| s.language == Language::Dart));
}

// ──────────────────────────────── Group 2 ────────────────────────────────

#[test]
fn resolve_returns_empty_for_unregistered_pair() {
    // Dart is not in the table; resolver returns empty.
    assert!(resolve_install_cmds(Language::Dart, ToolRole::Checker).is_empty());
    assert!(resolve_install_cmds(Language::Dart, ToolRole::Formatter).is_empty());
}

// Swift is only supported on macOS; on non-macOS platforms the resolver must
// yield an empty vec. Cfg-gate the test off macOS so it's only compiled where
// the assertion is meaningful (instead of passing vacuously).
#[cfg(not(target_os = "macos"))]
#[test]
fn resolve_returns_empty_for_unsupported_platform_combo() {
    assert!(resolve_install_cmds(Language::Swift, ToolRole::Checker).is_empty());
    assert!(resolve_install_cmds(Language::Swift, ToolRole::Formatter).is_empty());
}

#[test]
fn stylua_formatter_first_candidate_on_current_platform_matches_expected() {
    let cmds = resolve_install_cmds(Language::Lua, ToolRole::Formatter);
    assert!(!cmds.is_empty(), "stylua must resolve on current platform");
    let first = &cmds[0];
    match Os::current() {
        Os::MacOs => assert_eq!(
            first,
            &vec![
                "brew".to_string(),
                "install".to_string(),
                "stylua".to_string()
            ]
        ),
        Os::Windows => assert_eq!(
            first,
            &vec![
                "scoop".to_string(),
                "install".to_string(),
                "stylua".to_string()
            ]
        ),
        Os::Linux => assert_eq!(
            first,
            &vec![
                "cargo".to_string(),
                "install".to_string(),
                "stylua".to_string()
            ]
        ),
    }
}

#[test]
fn is_tool_supported_agrees_with_resolve() {
    for spec in TOOL_INSTALLS {
        let in_platforms = spec.platforms.iter().any(|p| p.os == Os::current());
        if in_platforms {
            assert!(
                is_tool_supported_on_current_platform(spec.tool),
                "{} should be supported on current platform",
                spec.tool,
            );
        } else {
            assert!(
                !is_tool_supported_on_current_platform(spec.tool),
                "{} should NOT be supported on current platform",
                spec.tool,
            );
        }
    }
}

// ──────────────────────────────── Group 3 ────────────────────────────────

/// Returns the path to the linthis binary built by `cargo build --release`.
fn linthis_bin() -> PathBuf {
    // CARGO_BIN_EXE_linthis is set by Cargo at test-compile time and automatically
    // appends `.exe` on Windows, so this works on all platforms without manual logic.
    PathBuf::from(env!("CARGO_BIN_EXE_linthis"))
}

fn fixture_dir(lang_subdir: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/all-langs")
        .join(lang_subdir)
}

fn which(bin: &str) -> Option<PathBuf> {
    let output = Command::new(if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    })
    .arg(bin)
    .output()
    .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().next().map(|line| PathBuf::from(line.trim()))
}

/// Map a spec tool name to the actual binary name on PATH.
///
/// Most tools install a binary that matches the spec name exactly.
/// Exceptions:
///  - `clippy`: `rustup component add clippy` installs `cargo-clippy`, not a
///    standalone `clippy` binary.
fn path_binary_for(tool: &str) -> &str {
    match tool {
        "clippy" => "cargo-clippy",
        other => other,
    }
}

fn lang_subdir(lang: Language) -> &'static str {
    match lang {
        Language::Python => "python",
        Language::Rust => "rust",
        Language::Go => "go",
        Language::TypeScript | Language::JavaScript => "typescript",
        Language::Cpp | Language::ObjectiveC => "cpp",
        Language::Java => "java",
        Language::Shell => "shell",
        Language::Lua => "lua",
        Language::Kotlin => "kotlin",
        Language::Ruby => "ruby",
        Language::Php => "php",
        Language::Scala => "scala",
        Language::CSharp => "csharp",
        Language::Swift => "swift",
        Language::Dart => "dart",
    }
}

/// End-to-end: for every tool supported on the current platform, ensure it
/// ends up on PATH after `linthis -i <fixture>` runs with LINTHIS_INSTALL_MODE=auto.
///
/// Gated behind `#[ignore]` so developers don't trigger real installs locally.
/// CI invokes with `cargo test -- --include-ignored`.
#[test]
#[ignore]
fn auto_install_lint_cycle() {
    let linthis = linthis_bin();

    for spec in TOOL_INSTALLS {
        if !spec.platforms.iter().any(|p| p.os == Os::current()) {
            eprintln!("skip: {} not supported on {:?}", spec.tool, Os::current());
            continue;
        }

        let lang_dir = fixture_dir(lang_subdir(spec.language));
        if !lang_dir.exists() {
            eprintln!("skip: no fixture for {}", spec.tool);
            continue;
        }

        // Pre-existing? Great — skip the install and trust PATH.
        let bin_name = path_binary_for(spec.tool);
        if which(bin_name).is_some() {
            eprintln!("pre-existing: {} (binary={})", spec.tool, bin_name);
            continue;
        }

        let status = Command::new(&linthis)
            .args(["-i", lang_dir.to_str().unwrap()])
            .env("LINTHIS_INSTALL_MODE", "auto")
            .status()
            .expect("failed to spawn linthis");

        // linthis is expected to exit non-zero on fixtures with findings; just
        // verify the process actually ran (not a spawn failure).
        assert!(
            status.code().is_some() || cfg!(target_os = "windows"),
            "linthis did not exit normally for tool {} (status={:?})",
            spec.tool,
            status,
        );

        assert!(
            which(bin_name).is_some(),
            "{} (binary={}) not found on PATH after auto-install (language={:?})",
            spec.tool,
            bin_name,
            spec.language,
        );
    }
}
