//! Common test utilities for integration tests.

#![allow(dead_code)]

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

// Go tools
/// Check if Go is available
pub fn has_go() -> bool {
    tool_available("go", &["version"])
}

/// Check if golangci-lint is available
pub fn has_golangci_lint() -> bool {
    tool_available("golangci-lint", &["--version"])
}

/// Check if goimports is available
pub fn has_goimports() -> bool {
    tool_available("goimports", &["--help"])
}

// TypeScript/JavaScript tools
/// Check if eslint is available
pub fn has_eslint() -> bool {
    tool_available("eslint", &["--version"])
}

/// Check if prettier is available
pub fn has_prettier() -> bool {
    tool_available("prettier", &["--version"])
}

// C++ tools
/// Check if cpplint is available
pub fn has_cpplint() -> bool {
    tool_available("cpplint", &["--version"])
}

/// Check if clang-format is available
pub fn has_clang_format() -> bool {
    tool_available("clang-format", &["--version"])
}

// Java tools
/// Check if checkstyle is available
pub fn has_checkstyle() -> bool {
    tool_available("checkstyle", &["--version"])
}

// Dart tools
/// Check if dart is available
pub fn has_dart() -> bool {
    tool_available("dart", &["--version"])
}

// Kotlin tools
/// Check if ktlint is available
pub fn has_ktlint() -> bool {
    tool_available("ktlint", &["--version"])
}

// Lua tools
/// Check if luacheck is available
pub fn has_luacheck() -> bool {
    tool_available("luacheck", &["--version"])
}

/// Check if stylua is available
pub fn has_stylua() -> bool {
    tool_available("stylua", &["--version"])
}

// Swift tools
/// Check if swiftlint is available
pub fn has_swiftlint() -> bool {
    tool_available("swiftlint", &["--version"])
}

/// Check if swift-format is available
pub fn has_swift_format() -> bool {
    tool_available("swift-format", &["--version"])
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

// Go fixtures
/// Go fixture: good code
pub const GO_GOOD: &str = r#"package main

import "fmt"

func main() {
	fmt.Println("Hello, World!")
}
"#;

/// Go fixture: unformatted code
pub const GO_UNFORMATTED: &str = r#"package main
import "fmt"
func main(){fmt.Println("Hello")}
"#;

// TypeScript fixtures
/// TypeScript fixture: good code
pub const TS_GOOD: &str = r#"function hello(name: string): string {
  return `Hello, ${name}!`;
}

console.log(hello("World"));
"#;

/// TypeScript fixture: bad code (lint errors)
pub const TS_BAD: &str = r#"var x = 1
let unused = 42
function foo(){console.log("hello")}
"#;

/// TypeScript fixture: unformatted code
pub const TS_UNFORMATTED: &str = r#"function hello(name:string):string{return `Hello, ${name}!`}
"#;

// C++ fixtures
/// C++ fixture: good code
pub const CPP_GOOD: &str = r#"#include <iostream>

int main() {
  std::cout << "Hello, World!" << std::endl;
  return 0;
}
"#;

/// C++ fixture: bad code (lint errors)
pub const CPP_BAD: &str = r#"#include <iostream>
int main(){
int x=1;
std::cout<<"hello"<<std::endl;
return 0;}
"#;

// Java fixtures
/// Java fixture: good code
pub const JAVA_GOOD: &str = r#"public class Hello {
    public static void main(String[] args) {
        System.out.println("Hello, World!");
    }
}
"#;

/// Java fixture: bad code (lint errors)
pub const JAVA_BAD: &str = r#"public class Hello{
public static void main(String[] args){
int x=1;
System.out.println("hello");}}
"#;

// Dart fixtures
/// Dart fixture: good code
pub const DART_GOOD: &str = r#"void main() {
  print('Hello, World!');
}
"#;

/// Dart fixture: bad code (lint errors)
pub const DART_BAD: &str = r#"void main(){
var x=1;
print("hello");}
"#;

// Kotlin fixtures
/// Kotlin fixture: good code
pub const KOTLIN_GOOD: &str = r#"fun main() {
    println("Hello, World!")
}
"#;

/// Kotlin fixture: bad code (lint errors)
pub const KOTLIN_BAD: &str = r#"fun main(){
val x=1
println("hello")}
"#;

// Lua fixtures
/// Lua fixture: good code
pub const LUA_GOOD: &str = r#"local function hello(name)
    return "Hello, " .. name .. "!"
end

print(hello("World"))
"#;

/// Lua fixture: bad code (lint errors)
pub const LUA_BAD: &str = r#"function hello(name)
local unused = 1
return "Hello, "..name.."!"
end
print(hello("World"))
"#;

// Swift fixtures
/// Swift fixture: good code
pub const SWIFT_GOOD: &str = r#"import Foundation

func hello(name: String) -> String {
    return "Hello, \(name)!"
}

print(hello(name: "World"))
"#;

/// Swift fixture: bad code (lint errors)
pub const SWIFT_BAD: &str = r#"import Foundation
func hello(name:String)->String{return "Hello, \(name)!"}
print(hello(name:"World"))
"#;
