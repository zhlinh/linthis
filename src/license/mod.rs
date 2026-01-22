// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! License compliance checking module.
//!
//! This module provides license scanning and compliance checking across
//! multiple language ecosystems:
//!
//! - **Rust**: Cargo.toml license field, cargo-license
//! - **JavaScript/TypeScript**: package.json license field
//! - **Python**: setup.py, pyproject.toml, PKG-INFO
//! - **Go**: go.mod with license detection
//! - **Java**: pom.xml licenses section
//!
//! # Example
//!
//! ```rust,no_run
//! use linthis::license::{LicenseScanner, LicensePolicy, ScanOptions};
//! use std::path::PathBuf;
//!
//! let scanner = LicenseScanner::new();
//! let policy = LicensePolicy::default();
//! let options = ScanOptions::new(PathBuf::from("."));
//!
//! let result = scanner.scan(&options).expect("Scan failed");
//! let violations = policy.check(&result);
//!
//! for violation in violations {
//!     println!("{}: {}", violation.package, violation.reason);
//! }
//! ```

mod scanner;
pub mod policy;
mod spdx;
pub mod report;
mod languages;

pub use scanner::{LicenseScanner, ScanOptions, ScanResult, PackageLicense};
pub use policy::{LicensePolicy, PolicyViolation, LicenseCategory};
pub use spdx::{SpdxLicense, parse_spdx_expression};
pub use report::{format_license_report, LicenseReportFormat};
