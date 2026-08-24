// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Code complexity analysis module.
//!
//! This module provides complexity analysis across multiple languages:
//!
//! - **Cyclomatic Complexity**: Measures control flow complexity
//! - **Cognitive Complexity**: Measures understandability
//! - **Lines of Code (LOC/SLOC)**: Source line counts
//! - **Function Length**: Average and maximum function lengths
//! - **Nesting Depth**: Maximum nesting level
//!
//! # Example
//!
//! ```rust,no_run
//! use linthis::complexity::{ComplexityAnalyzer, AnalysisOptions};
//! use std::path::PathBuf;
//!
//! let analyzer = ComplexityAnalyzer::new();
//! let options = AnalysisOptions::new(PathBuf::from("."));
//!
//! let result = analyzer.analyze(&options).expect("Analysis failed");
//!
//! for file in result.files {
//!     println!("{}: cyclomatic={}", file.path.display(), file.metrics.cyclomatic);
//! }
//! ```

mod analyzer;
mod languages;
mod metrics;
pub mod report;
mod thresholds;

pub use analyzer::{AnalysisOptions, AnalysisResult, ComplexityAnalyzer};
pub use metrics::{
    count_cyclomatic, reportable_cyclomatic, ComplexityMetrics, CyclomaticCounts, FileMetrics,
    FunctionMetrics, MetricLevel,
};
pub use report::{format_complexity_report, ComplexityReportFormat};
pub use thresholds::{ThresholdConfig, Thresholds};
