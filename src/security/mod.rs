// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Security scanning module for dependency vulnerability detection.
//!
//! This module provides unified security scanning across multiple languages
//! by integrating with language-specific security tools:
//!
//! - **Rust**: cargo-audit (RustSec Advisory Database)
//! - **JavaScript/TypeScript**: npm audit
//! - **Python**: pip-audit / safety
//! - **Go**: govulncheck
//! - **Java**: dependency-check (OWASP)
//!
//! # Example
//!
//! ```rust,no_run
//! use linthis::security::{SecurityScanner, ScanOptions};
//! use std::path::PathBuf;
//!
//! let scanner = SecurityScanner::new();
//! let options = ScanOptions {
//!     path: PathBuf::from("."),
//!     severity_threshold: Some("high".to_string()),
//!     ..Default::default()
//! };
//!
//! let result = scanner.scan(&options).expect("Scan failed");
//! println!("Found {} vulnerabilities", result.vulnerabilities.len());
//! ```

mod scanner;
mod vulnerability;
mod advisories;
mod languages;
pub mod report;

pub use scanner::{SecurityScanner, ScanOptions, ScanResult};
pub use vulnerability::{Vulnerability, Severity, AffectedPackage, Advisory};
pub use advisories::AdvisoryDatabase;
pub use report::{SecurityReport, format_security_report};
