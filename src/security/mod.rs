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
//! ## SCA (Dependency Vulnerability Detection)
//!
//! - **Rust**: cargo-audit (RustSec Advisory Database)
//! - **JavaScript/TypeScript**: npm audit
//! - **Python**: pip-audit / safety
//! - **Go**: govulncheck
//! - **Java**: dependency-check (OWASP)
//!
//! ## SAST (Source Code Security Analysis)
//!
//! - **Multi-language**: OpenGrep / Semgrep (30+ languages)
//! - **Python**: Bandit
//! - **Go**: Gosec
//! - **C/C++**: Flawfinder
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

mod advisories;
mod languages;
pub mod report;
pub mod sast;
mod scanner;
mod vulnerability;

pub use advisories::AdvisoryDatabase;
pub use report::{format_security_report, SecurityReport};
pub use sast::{SastAggregator, SastResult, SastScanOptions};
pub use scanner::{ScanOptions, ScanResult, SecurityScanner};
pub use vulnerability::{Advisory, AffectedPackage, Severity, Vulnerability};
