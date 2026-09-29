// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Complexity thresholds configuration.

use serde::{Deserialize, Serialize};

/// Threshold levels for different metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfig {
    /// Good (green) threshold
    pub good: u32,
    /// Warning (yellow) threshold
    pub warning: u32,
    /// High (red) threshold
    pub high: u32,
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self {
            good: 10,
            warning: 20, // good + 10
            high: 30,    // good + 20
        }
    }
}

impl ThresholdConfig {
    /// Normalize thresholds to ensure good <= warning <= high.
    pub fn normalize(&mut self) {
        self.warning = self.warning.max(self.good);
        self.high = self.high.max(self.warning);
    }
}

/// Collection of all thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thresholds {
    /// Cyclomatic complexity thresholds
    pub cyclomatic: ThresholdConfig,
    /// Cognitive complexity thresholds
    pub cognitive: ThresholdConfig,
    /// Function length thresholds, in source lines (blank and comment lines
    /// excluded — the `sloc` metric, not the raw start..end span)
    pub function_length: ThresholdConfig,
    /// Nesting depth thresholds
    pub nesting_depth: ThresholdConfig,
    /// Parameter count thresholds
    pub parameters: ThresholdConfig,
    /// File length thresholds (lines)
    pub file_length: ThresholdConfig,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            cyclomatic: ThresholdConfig {
                good: 10,
                warning: 20,
                high: 30,
            },
            cognitive: ThresholdConfig {
                good: 15,
                warning: 30,
                high: 60,
            },
            function_length: ThresholdConfig {
                good: 400,
                warning: 450,
                high: 500,
            },
            nesting_depth: ThresholdConfig {
                good: 4,
                warning: 6,
                high: 8,
            },
            parameters: ThresholdConfig {
                good: 4,
                warning: 6,
                high: 10,
            },
            file_length: ThresholdConfig {
                good: 300,
                warning: 500,
                high: 1000,
            },
        }
    }
}

impl Thresholds {
    /// Create new thresholds with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Overlay a `[checks.complexity]` section onto these thresholds.
    ///
    /// Three call sites used to inline this — the cached-result path in
    /// `run_complexity_check`, `apply_thresholds` and `run_complexity_analysis`
    /// — so a new knob had to be wired up three times or it would silently work
    /// in one entry point and not the others.
    ///
    /// Setting only the base threshold derives the other two, matching how the
    /// defaults are spaced (cyclomatic +10/+20, function length +50/+100).
    pub fn apply_config(&mut self, config: &crate::config::ComplexityChecksConfig) {
        if let Some(t) = config.threshold {
            self.cyclomatic.good = t;
            self.cyclomatic.warning = t + 10;
            self.cyclomatic.high = t + 20;
        }
        if let Some(w) = config.warning_threshold {
            self.cyclomatic.warning = w;
        }
        if let Some(e) = config.error_threshold {
            self.cyclomatic.high = e;
        }

        if let Some(t) = config.max_function_lines {
            self.function_length.good = t;
            self.function_length.warning = t + 50;
            self.function_length.high = t + 100;
        }
        if let Some(w) = config.max_function_lines_warning {
            self.function_length.warning = w;
        }
        if let Some(e) = config.max_function_lines_error {
            self.function_length.high = e;
        }

        self.cyclomatic.normalize();
        self.function_length.normalize();
    }

    /// Create strict thresholds
    pub fn strict() -> Self {
        Self {
            cyclomatic: ThresholdConfig {
                good: 5,
                warning: 10,
                high: 20,
            },
            cognitive: ThresholdConfig {
                good: 10,
                warning: 15,
                high: 30,
            },
            function_length: ThresholdConfig {
                good: 200,
                warning: 250,
                high: 300,
            },
            nesting_depth: ThresholdConfig {
                good: 3,
                warning: 4,
                high: 5,
            },
            parameters: ThresholdConfig {
                good: 3,
                warning: 4,
                high: 6,
            },
            file_length: ThresholdConfig {
                good: 200,
                warning: 300,
                high: 500,
            },
        }
    }

    /// Create lenient thresholds
    pub fn lenient() -> Self {
        Self {
            cyclomatic: ThresholdConfig {
                good: 15,
                warning: 25,
                high: 35,
            },
            cognitive: ThresholdConfig {
                good: 20,
                warning: 40,
                high: 80,
            },
            function_length: ThresholdConfig {
                good: 600,
                warning: 700,
                high: 800,
            },
            nesting_depth: ThresholdConfig {
                good: 5,
                warning: 7,
                high: 10,
            },
            parameters: ThresholdConfig {
                good: 5,
                warning: 8,
                high: 12,
            },
            file_length: ThresholdConfig {
                good: 500,
                warning: 750,
                high: 1500,
            },
        }
    }

    /// Load from TOML string
    pub fn from_toml(content: &str) -> Result<Self, String> {
        toml::from_str(content).map_err(|e| format!("Failed to parse thresholds: {}", e))
    }

    /// Check if cyclomatic complexity exceeds threshold
    pub fn check_cyclomatic(&self, value: u32) -> &'static str {
        if value <= self.cyclomatic.good {
            "good"
        } else if value <= self.cyclomatic.warning {
            "warning"
        } else if value <= self.cyclomatic.high {
            "high"
        } else {
            "critical"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_defaults() {
        let thresholds = Thresholds::default();
        assert_eq!(thresholds.cyclomatic.good, 10);
        assert_eq!(thresholds.cyclomatic.warning, 20);
    }

    #[test]
    fn test_threshold_strict() {
        let thresholds = Thresholds::strict();
        assert!(thresholds.cyclomatic.good < Thresholds::default().cyclomatic.good);
    }

    #[test]
    fn test_check_cyclomatic() {
        let thresholds = Thresholds::default();
        // Default thresholds: good=10, warning=20, high=30
        assert_eq!(thresholds.check_cyclomatic(5), "good");
        assert_eq!(thresholds.check_cyclomatic(15), "warning");
        assert_eq!(thresholds.check_cyclomatic(25), "high");
        assert_eq!(thresholds.check_cyclomatic(35), "critical");
    }
}
