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
//! This module provides mechanisms for customizing lint behavior:
//!
//! - **Custom rules**: Define project-specific regex-based lint rules
//! - **Rule filtering**: Disable specific rules or rule prefixes
//! - **Severity overrides**: Change severity levels for any rule
//!
//! # Main Types
//!
//! - [`RulesConfig`] - Configuration container for all rule settings
//! - [`CustomRule`] - Definition of a custom regex-based rule
//! - [`CustomRulesChecker`] - Checker that runs custom rules against files
//! - [`RuleFilter`] - Filters issues based on disable/severity settings
//! - [`SeverityOverride`] - Severity level override values
//!
//! # Configuration Example
//!
//! ```toml
//! [rules]
//! # Disable specific rules (exact match or prefix with /*)
//! disable = ["E501", "whitespace/*", "clippy::needless_*"]
//!
//! # Override severity levels (error, warning, info, off)
//! [rules.severity]
//! "W0612" = "error"    # Upgrade warning to error
//! "E0001" = "info"     # Downgrade error to info
//! "todo" = "off"       # Disable entirely
//!
//! # Define custom regex-based rules
//! [[rules.custom]]
//! code = "custom/no-todo"
//! pattern = "TODO|FIXME"
//! message = "Found TODO/FIXME comment"
//! severity = "warning"
//! suggestion = "Address the TODO or convert to a tracking issue"
//! languages = ["rust", "python"]  # Optional: limit to specific languages
//! extensions = ["rs", "py"]        # Optional: limit to specific extensions
//! ```
//!
//! # Usage Example
//!
//! ```rust,no_run
//! use linthis::rules::{RulesConfig, RuleFilter, CustomRulesChecker, CustomRule};
//! use linthis::Severity;
//!
//! // Create custom rule programmatically
//! let rule = CustomRule::new("custom/no-print", r"println!\(", "No println! in production")
//!     .with_severity(Severity::Warning)
//!     .with_suggestion("Use log::info! instead")
//!     .with_languages(vec!["rust".to_string()]);
//!
//! // Create checker from rules
//! let mut config = RulesConfig::default();
//! config.custom.push(rule);
//! let checker = CustomRulesChecker::new(&config.custom).unwrap();
//!
//! // Create filter for issue processing
//! let filter = RuleFilter::from_config(&config);
//! ```

mod config;
mod custom_checker;
mod filter;

pub use config::{CustomRule, RulesConfig, SeverityOverride};
pub use custom_checker::{CustomRulesChecker, RuleInfo};
pub use filter::{FilterStats, RuleFilter};
