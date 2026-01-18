// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Reports and analysis module for linthis.
//!
//! This module provides:
//! - Issue classification statistics
//! - HTML report generation
//! - Code quality trend analysis
//! - Team code style consistency analysis

mod consistency;
mod html;
mod statistics;
mod templates;
mod trends;

pub use consistency::{ConsistencyAnalysis, OutlierFile, RepeatedPattern};
pub use html::{generate_html_report, HtmlReportOptions};
pub use statistics::{FileStats, ReportStatistics, RuleStats, SeverityCounts, SummaryMetrics};
pub use trends::{
    analyze_trends, get_last_result, load_historical_results, load_result_from_file,
    TrendAnalysis, TrendDataPoint, TrendDirection,
};
