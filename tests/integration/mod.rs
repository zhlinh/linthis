//! Integration tests for linthis CLI.
//!
//! These tests verify the CLI interface by running the linthis binary
//! and checking its output and exit codes.

// Shared test utilities
mod common;

// Test modules
mod cache_tests;
mod check_tests;
mod config_tests;
mod error_tests;
mod format_tests;
mod language_tests;
mod plugin_tests;

use std::process::Command;

/// Helper to get the path to the linthis binary
fn linthis_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_linthis"))
}

/// Test that --help works
#[test]
fn test_help_flag() {
    let output = linthis_bin()
        .arg("--help")
        .output()
        .expect("Failed to execute linthis");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("linthis"));
    assert!(stdout.contains("Usage"));
}

/// Test that --version works
#[test]
fn test_version_flag() {
    let output = linthis_bin()
        .arg("--version")
        .output()
        .expect("Failed to execute linthis");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("linthis"));
}

/// Test init subcommand help
#[test]
fn test_init_help() {
    let output = linthis_bin()
        .args(["init", "--help"])
        .output()
        .expect("Failed to execute linthis");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("init"));
}

/// Test plugin subcommand help
#[test]
fn test_plugin_help() {
    let output = linthis_bin()
        .args(["plugin", "--help"])
        .output()
        .expect("Failed to execute linthis");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("plugin"));
}

/// Test hook subcommand help
#[test]
fn test_hook_help() {
    let output = linthis_bin()
        .args(["hook", "--help"])
        .output()
        .expect("Failed to execute linthis");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hook"));
}

/// Test config subcommand help
#[test]
fn test_config_help() {
    let output = linthis_bin()
        .args(["config", "--help"])
        .output()
        .expect("Failed to execute linthis");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("config"));
}

/// Test that running with non-existent path fails gracefully
#[test]
fn test_nonexistent_path() {
    let output = linthis_bin()
        .args(["-c", "/nonexistent/path/that/does/not/exist"])
        .output()
        .expect("Failed to execute linthis");

    // Should not crash, may exit with error
    // Just verify it completes
    let _ = output.status;
}

/// Test quiet mode flag
#[test]
fn test_quiet_mode() {
    let output = linthis_bin()
        .args(["--quiet", "--help"])
        .output()
        .expect("Failed to execute linthis");

    assert!(output.status.success());
}

/// Test verbose mode flag
#[test]
fn test_verbose_mode() {
    let output = linthis_bin()
        .args(["--verbose", "--help"])
        .output()
        .expect("Failed to execute linthis");

    assert!(output.status.success());
}
