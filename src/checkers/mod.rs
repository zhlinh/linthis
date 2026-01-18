// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Language-specific linter implementations.
//!
//! This module provides checker implementations for each supported language.
//! Each checker wraps one or more external linting tools and normalizes their
//! output to the common [`LintIssue`](crate::utils::types::LintIssue) format.
//!
//! ## Supported Languages and Tools
//!
//! | Language | Checker | Primary Tool | Fallback |
//! |----------|---------|--------------|----------|
//! | Rust | [`RustChecker`] | clippy | rustc |
//! | Python | [`PythonChecker`] | ruff | - |
//! | TypeScript/JS | [`TypeScriptChecker`] | eslint | - |
//! | Go | [`GoChecker`] | golangci-lint | go vet |
//! | Java | [`JavaChecker`] | checkstyle | - |
//! | C++/C | [`CppChecker`] | cpplint, clang-tidy | - |
//! | Objective-C | [`CppChecker`] | cpplint, clang-tidy | - |
//! | Swift | [`SwiftChecker`] | swiftlint | - |
//! | Kotlin | [`KotlinChecker`] | ktlint, detekt | - |
//! | Dart | [`DartChecker`] | dart analyze | - |
//! | Lua | [`LuaChecker`] | luacheck | - |
//! | Shell/Bash | [`ShellChecker`] | shellcheck | - |
//! | Ruby | [`RubyChecker`] | rubocop | - |
//! | PHP | [`PhpChecker`] | phpcs | - |
//! | Scala | [`ScalaChecker`] | scalafix | - |
//! | C# | [`CSharpChecker`] | dotnet-format | - |
//!
//! ## Checker Trait
//!
//! All checkers implement the [`Checker`] trait:
//!
//! ```rust,ignore
//! pub trait Checker: Send + Sync {
//!     /// Returns the checker name (e.g., "rust", "python")
//!     fn name(&self) -> &str;
//!
//!     /// Check if the underlying tool is available
//!     fn is_available(&self) -> bool;
//!
//!     /// Run the checker on a file
//!     fn check(&self, path: &Path) -> Result<Vec<LintIssue>>;
//! }
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use linthis::checkers::{Checker, RustChecker};
//! use std::path::Path;
//!
//! let checker = RustChecker::new();
//!
//! if checker.is_available() {
//!     let issues = checker.check(Path::new("src/main.rs")).unwrap();
//!     for issue in issues {
//!         println!("{}:{}: {}", issue.file_path.display(), issue.line, issue.message);
//!     }
//! } else {
//!     eprintln!("Clippy not available. Install with: rustup component add clippy");
//! }
//! ```

pub mod cpp;
pub mod csharp;
pub mod dart;
pub mod go;
pub mod java;
pub mod kotlin;
pub mod lua;
pub mod php;
pub mod python;
pub mod ruby;
pub mod rust;
pub mod scala;
pub mod shell;
pub mod swift;
pub mod traits;
pub mod typescript;

pub use cpp::CppChecker;
pub use csharp::CSharpChecker;
pub use dart::DartChecker;
pub use go::GoChecker;
pub use java::JavaChecker;
pub use kotlin::KotlinChecker;
pub use lua::LuaChecker;
pub use php::PhpChecker;
pub use python::PythonChecker;
pub use ruby::RubyChecker;
pub use rust::RustChecker;
pub use scala::ScalaChecker;
pub use shell::ShellChecker;
pub use swift::SwiftChecker;
pub use traits::Checker;
pub use typescript::TypeScriptChecker;
