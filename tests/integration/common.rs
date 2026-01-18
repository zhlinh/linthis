//! Common test utilities for integration tests.

use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

/// Get a Command for the linthis binary
pub fn linthis_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_linthis"))
}

/// Create a temporary project directory with .linthis subdirectory
pub fn create_temp_project() -> TempDir {
    let temp = TempDir::new().expect("Failed to create temp directory");
    std::fs::create_dir_all(temp.path().join(".linthis"))
        .expect("Failed to create .linthis directory");
    temp
}

/// Write content to a file in the given directory
pub fn write_fixture(dir: &Path, name: &str, content: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create parent directory");
    }
    std::fs::write(&path, content).expect("Failed to write fixture file");
}

/// Assert that command output has a specific exit code
pub fn assert_exit_code(output: &Output, expected: i32) {
    let actual = output.status.code().unwrap_or(-1);
    assert_eq!(
        actual, expected,
        "Expected exit code {}, got {}.\nstdout: {}\nstderr: {}",
        expected,
        actual,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Assert that stdout contains a string
pub fn assert_stdout_contains(output: &Output, expected: &str) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(expected),
        "Expected stdout to contain '{}', got:\n{}",
        expected,
        stdout
    );
}

/// Assert that stderr contains a string
pub fn assert_stderr_contains(output: &Output, expected: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected),
        "Expected stderr to contain '{}', got:\n{}",
        expected,
        stderr
    );
}

/// Check if an external tool is available
pub fn tool_available(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if clippy is available
pub fn has_clippy() -> bool {
    tool_available("cargo", &["clippy", "--version"])
}

/// Check if ruff is available
pub fn has_ruff() -> bool {
    tool_available("ruff", &["--version"])
}

/// Check if rustfmt is available
pub fn has_rustfmt() -> bool {
    tool_available("rustfmt", &["--version"])
}

/// Check if git is available
pub fn has_git() -> bool {
    tool_available("git", &["--version"])
}

/// Python fixture: good code (no lint errors)
pub const PYTHON_GOOD: &str = r#""""A simple module with no lint errors."""


def hello(name: str) -> str:
    """Return a greeting message."""
    return f"Hello, {name}!"


if __name__ == "__main__":
    print(hello("World"))
"#;

/// Python fixture: bad code (has lint errors)
pub const PYTHON_BAD: &str = r#"import os
import sys
x=1
y =  2
def foo():
    unused_var = 42
    print("hello")
"#;

/// Python fixture: unformatted code
pub const PYTHON_UNFORMATTED: &str = r#"def foo():print("hello")
x=1
y=2
"#;

/// Rust fixture: good code (no warnings)
pub const RUST_GOOD: &str = r#"fn main() {
    println!("Hello, world!");
}
"#;

/// Rust fixture: bad code (has warnings)
pub const RUST_BAD: &str = r#"fn main() {
    let unused = 42;
    println!("hello");
}
"#;

/// Rust fixture: unformatted code
pub const RUST_UNFORMATTED: &str = r#"fn main(){println!("hello");let x=1;let y=2;}
"#;
