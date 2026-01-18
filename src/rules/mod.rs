// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Custom rules, rule filtering, and severity overrides.
//!
//! This module provides:
//! - Custom regex-based lint rules (`CustomRule`, `CustomRulesChecker`)
//! - Rule disable/enable control (`RuleFilter`)
//! - Severity level customization (`SeverityOverride`)
//!
//! # Configuration Example
//!
//! ```toml
//! [rules]
//! # Disable specific rules
//! disable = ["E501", "whitespace/*"]
//!
//! # Override severity levels
//! [rules.severity]
//! "W0612" = "error"
//! "E0001" = "info"
//!
//! # Define custom rules
//! [[rules.custom]]
//! code = "custom/no-todo"
//! pattern = "TODO|FIXME"
//! message = "Found TODO/FIXME comment"
//! severity = "warning"
//! suggestion = "Address the TODO or convert to a tracking issue"
//! ```

mod config;
mod custom_checker;
mod filter;

pub use config::{CustomRule, RulesConfig, SeverityOverride};
pub use custom_checker::{CustomRulesChecker, RuleInfo};
pub use filter::{FilterStats, RuleFilter};
