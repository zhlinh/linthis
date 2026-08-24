// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Complexity metrics data structures.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Metric severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricLevel {
    /// Good - within recommended limits
    Good,
    /// Warning - approaching limits
    Warning,
    /// High - exceeds recommended limits
    High,
    /// Critical - significantly exceeds limits
    Critical,
}

impl MetricLevel {
    /// Get color code for terminal output
    pub fn color_code(&self) -> &'static str {
        match self {
            MetricLevel::Good => "\x1b[32m",       // Green
            MetricLevel::Warning => "\x1b[33m",    // Yellow
            MetricLevel::High => "\x1b[31m",       // Red
            MetricLevel::Critical => "\x1b[1;31m", // Bold red
        }
    }

    /// Get emoji representation
    pub fn emoji(&self) -> &'static str {
        match self {
            MetricLevel::Good => "🟢",
            MetricLevel::Warning => "🟡",
            MetricLevel::High => "🟠",
            MetricLevel::Critical => "🔴",
        }
    }
}

/// Complexity metrics for a single entity (file or function)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Cyclomatic complexity
    pub cyclomatic: u32,
    /// Cognitive complexity (Sonar style)
    pub cognitive: u32,
    /// Total lines of code (including blanks and comments)
    pub loc: u32,
    /// Source lines of code (excluding blanks and comments)
    pub sloc: u32,
    /// Number of comments
    pub comment_lines: u32,
    /// Maximum nesting depth
    pub max_nesting: u32,
    /// Number of parameters (for functions)
    pub parameters: u32,
    /// Number of return statements (for functions)
    pub returns: u32,
    /// Halstead metrics (optional)
    pub halstead: Option<HalsteadMetrics>,
}

impl ComplexityMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get cyclomatic complexity level
    pub fn cyclomatic_level(&self) -> MetricLevel {
        match self.cyclomatic {
            0..=10 => MetricLevel::Good,
            11..=20 => MetricLevel::Warning,
            21..=50 => MetricLevel::High,
            _ => MetricLevel::Critical,
        }
    }

    /// Get cognitive complexity level
    pub fn cognitive_level(&self) -> MetricLevel {
        match self.cognitive {
            0..=15 => MetricLevel::Good,
            16..=30 => MetricLevel::Warning,
            31..=60 => MetricLevel::High,
            _ => MetricLevel::Critical,
        }
    }

    /// Get nesting depth level
    pub fn nesting_level(&self) -> MetricLevel {
        match self.max_nesting {
            0..=4 => MetricLevel::Good,
            5..=6 => MetricLevel::Warning,
            7..=8 => MetricLevel::High,
            _ => MetricLevel::Critical,
        }
    }

    /// Get the worst metric level
    pub fn overall_level(&self) -> MetricLevel {
        let levels = [
            self.cyclomatic_level(),
            self.cognitive_level(),
            self.nesting_level(),
        ];

        levels
            .into_iter()
            .max_by_key(|l| match l {
                MetricLevel::Good => 0,
                MetricLevel::Warning => 1,
                MetricLevel::High => 2,
                MetricLevel::Critical => 3,
            })
            .unwrap_or(MetricLevel::Good)
    }
}

/// Halstead complexity metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HalsteadMetrics {
    /// Number of distinct operators
    pub distinct_operators: u32,
    /// Number of distinct operands
    pub distinct_operands: u32,
    /// Total number of operators
    pub total_operators: u32,
    /// Total number of operands
    pub total_operands: u32,
    /// Program vocabulary: n1 + n2
    pub vocabulary: u32,
    /// Program length: N1 + N2
    pub length: u32,
    /// Calculated program length
    pub calculated_length: f64,
    /// Volume: N * log2(n)
    pub volume: f64,
    /// Difficulty: (n1/2) * (N2/n2)
    pub difficulty: f64,
    /// Effort: D * V
    pub effort: f64,
    /// Time to program: E / 18
    pub time: f64,
    /// Bugs estimate: V / 3000
    pub bugs: f64,
}

/// Metrics for a single function/method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionMetrics {
    /// Function name
    pub name: String,
    /// Start line number
    pub start_line: u32,
    /// End line number
    pub end_line: u32,
    /// Complexity metrics
    pub metrics: ComplexityMetrics,
    /// Function kind (function, method, closure, etc.)
    pub kind: String,
    /// Parent class/struct name (if applicable)
    pub parent: Option<String>,
}

impl FunctionMetrics {
    pub fn new(name: &str, start_line: u32, end_line: u32) -> Self {
        Self {
            name: name.to_string(),
            start_line,
            end_line,
            metrics: ComplexityMetrics::new(),
            kind: "function".to_string(),
            parent: None,
        }
    }

    /// Get the number of lines in this function
    pub fn lines(&self) -> u32 {
        self.end_line.saturating_sub(self.start_line) + 1
    }
}

/// Metrics for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetrics {
    /// File path
    pub path: PathBuf,
    /// Language
    pub language: String,
    /// Aggregate metrics for the file
    pub metrics: ComplexityMetrics,
    /// Per-function metrics
    pub functions: Vec<FunctionMetrics>,
    /// Number of classes/structs
    pub classes: u32,
    /// Number of imports/includes
    pub imports: u32,
}

impl FileMetrics {
    pub fn new(path: PathBuf, language: &str) -> Self {
        Self {
            path,
            language: language.to_string(),
            metrics: ComplexityMetrics::new(),
            functions: Vec::new(),
            classes: 0,
            imports: 0,
        }
    }

    /// Get the most complex function
    pub fn most_complex_function(&self) -> Option<&FunctionMetrics> {
        self.functions.iter().max_by_key(|f| f.metrics.cyclomatic)
    }

    /// Calculate average cyclomatic complexity
    pub fn average_cyclomatic(&self) -> f64 {
        if self.functions.is_empty() {
            return 0.0;
        }
        let sum: u32 = self.functions.iter().map(|f| f.metrics.cyclomatic).sum();
        sum as f64 / self.functions.len() as f64
    }

    /// Get functions exceeding a complexity threshold
    pub fn functions_above_threshold(&self, threshold: u32) -> Vec<&FunctionMetrics> {
        self.functions
            .iter()
            .filter(|f| f.metrics.cyclomatic > threshold)
            .collect()
    }
}

/// Summary statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SummaryStats {
    /// Total files analyzed
    pub total_files: usize,
    /// Total functions analyzed
    pub total_functions: usize,
    /// Total lines of code
    pub total_loc: u64,
    /// Total source lines of code
    pub total_sloc: u64,
    /// Average cyclomatic complexity
    pub avg_cyclomatic: f64,
    /// Average cognitive complexity
    pub avg_cognitive: f64,
    /// Maximum cyclomatic complexity
    pub max_cyclomatic: u32,
    /// Maximum cognitive complexity
    pub max_cognitive: u32,
    /// Files with high complexity
    pub high_complexity_files: usize,
    /// Functions with high complexity
    pub high_complexity_functions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complexity_metrics_default() {
        let metrics = ComplexityMetrics::new();
        assert_eq!(metrics.cyclomatic, 0);
        assert_eq!(metrics.cognitive, 0);
        assert_eq!(metrics.loc, 0);
    }

    #[test]
    fn test_cyclomatic_level() {
        let mut metrics = ComplexityMetrics::new();

        metrics.cyclomatic = 5;
        assert_eq!(metrics.cyclomatic_level(), MetricLevel::Good);

        metrics.cyclomatic = 15;
        assert_eq!(metrics.cyclomatic_level(), MetricLevel::Warning);

        metrics.cyclomatic = 35;
        assert_eq!(metrics.cyclomatic_level(), MetricLevel::High);

        metrics.cyclomatic = 60;
        assert_eq!(metrics.cyclomatic_level(), MetricLevel::Critical);
    }

    #[test]
    fn test_function_metrics() {
        let func = FunctionMetrics::new("test_func", 10, 20);
        assert_eq!(func.lines(), 11);
    }

    #[test]
    fn test_file_metrics() {
        let file = FileMetrics::new(PathBuf::from("test.rs"), "rust");
        assert!(file.functions.is_empty());
        assert_eq!(file.average_cyclomatic(), 0.0);
    }
}

/// Cognitive complexity at or below this marks a *flat dispatch*: a match
/// table, a chain of independent guards, one line per case.
///
/// Cyclomatic complexity counts branches, so such a function scores as high as
/// its number of cases — a fifteen-language lookup table reads as 16. Cognitive
/// complexity is the one that models comprehension: no nesting, no interleaved
/// conditions, nothing to hold in your head, so it stays near zero. When the
/// two disagree that sharply, the cyclomatic number is measuring how many
/// cases exist, not how hard the code is.
pub const FLAT_DISPATCH_COGNITIVE: u32 = 5;

/// Whether a function's cyclomatic complexity is worth reporting as an issue.
///
/// Flat dispatch is exempt: splitting a lookup table into a const array to
/// bring the count down makes it harder to read, not easier. `linthis
/// complexity` still shows the raw numbers — this only governs what becomes a
/// finding that can block a commit.
pub fn reportable_cyclomatic(metrics: &ComplexityMetrics, threshold: u32) -> bool {
    metrics.cyclomatic > threshold && metrics.cognitive > FLAT_DISPATCH_COGNITIVE
}

#[cfg(test)]
mod reportable_tests {
    use super::*;

    fn metrics(cyclomatic: u32, cognitive: u32) -> ComplexityMetrics {
        ComplexityMetrics {
            cyclomatic,
            cognitive,
            ..Default::default()
        }
    }

    #[test]
    fn a_flat_lookup_table_is_not_an_issue() {
        // get_config_names: 21 arms, one line each, nothing nested.
        assert!(!reportable_cyclomatic(&metrics(21, 2), 10));
    }

    #[test]
    fn branchy_code_still_is() {
        assert!(reportable_cyclomatic(&metrics(21, 30), 10));
        // Below the threshold, cognitive load does not matter.
        assert!(!reportable_cyclomatic(&metrics(8, 30), 10));
    }
}

/// How many functions fall in each severity band, counting only what
/// [`reportable_cyclomatic`] considers worth reporting.
pub struct CyclomaticCounts {
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
}

/// Tally reportable cyclomatic complexity across analyzed files.
///
/// Three callers used to inline this — two exit-code paths and the check
/// summary — which is how a function could be exempt from the issue list and
/// still fail the build.
pub fn count_cyclomatic(
    files: &[FileMetrics],
    thresholds: &crate::complexity::ThresholdConfig,
) -> CyclomaticCounts {
    let mut counts = CyclomaticCounts {
        errors: 0,
        warnings: 0,
        infos: 0,
    };

    for func in files.iter().flat_map(|f| &f.functions) {
        if !reportable_cyclomatic(&func.metrics, thresholds.good) {
            continue;
        }
        let c = func.metrics.cyclomatic;
        if c > thresholds.high {
            counts.errors += 1;
        } else if c > thresholds.warning {
            counts.warnings += 1;
        } else {
            counts.infos += 1;
        }
    }

    counts
}
