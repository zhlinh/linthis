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
            warning: 20,
            high: 50,
        }
    }
}

/// Collection of all thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thresholds {
    /// Cyclomatic complexity thresholds
    pub cyclomatic: ThresholdConfig,
    /// Cognitive complexity thresholds
    pub cognitive: ThresholdConfig,
    /// Function length thresholds (lines)
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
                high: 50,
            },
            cognitive: ThresholdConfig {
                good: 15,
                warning: 30,
                high: 60,
            },
            function_length: ThresholdConfig {
                good: 50,
                warning: 100,
                high: 200,
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
                good: 25,
                warning: 50,
                high: 100,
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
                warning: 30,
                high: 75,
            },
            cognitive: ThresholdConfig {
                good: 20,
                warning: 40,
                high: 80,
            },
            function_length: ThresholdConfig {
                good: 75,
                warning: 150,
                high: 300,
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
        toml::from_str(content)
            .map_err(|e| format!("Failed to parse thresholds: {}", e))
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
        assert_eq!(thresholds.check_cyclomatic(5), "good");
        assert_eq!(thresholds.check_cyclomatic(15), "warning");
        assert_eq!(thresholds.check_cyclomatic(35), "high");
        assert_eq!(thresholds.check_cyclomatic(60), "critical");
    }
}
