// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Complexity analyzer implementation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use super::languages::{
    GoComplexityAnalyzer, JavaComplexityAnalyzer, PythonComplexityAnalyzer,
    RustComplexityAnalyzer, TypeScriptComplexityAnalyzer,
};
use super::metrics::{FileMetrics, MetricLevel, SummaryStats};
use super::thresholds::Thresholds;

/// Language-specific complexity analyzer trait
pub trait LanguageComplexityAnalyzer: Send + Sync {
    /// Get the analyzer name
    #[allow(dead_code)]
    fn name(&self) -> &str;

    /// Get supported file extensions
    fn extensions(&self) -> &[&str];

    /// Get the language name
    fn language(&self) -> &str;

    /// Analyze a file and return metrics
    fn analyze_file(&self, path: &Path, content: &str) -> Result<FileMetrics, String>;
}

/// Options for complexity analysis
#[derive(Debug, Clone)]
pub struct AnalysisOptions {
    /// Path to analyze
    pub path: PathBuf,
    /// File patterns to include
    pub include: Vec<String>,
    /// File patterns to exclude
    pub exclude: Vec<String>,
    /// Complexity threshold for warnings
    pub threshold: Option<u32>,
    /// Output format
    pub format: String,
    /// Include trend analysis
    pub with_trends: bool,
    /// Number of historical runs for trends
    pub trend_count: usize,
    /// Verbose output
    pub verbose: bool,
    /// Parallel processing
    pub parallel: bool,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            include: Vec::new(),
            exclude: Vec::new(),
            threshold: None,
            format: "human".to_string(),
            with_trends: false,
            trend_count: 10,
            verbose: false,
            parallel: true,
        }
    }
}

impl AnalysisOptions {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            ..Default::default()
        }
    }
}

/// Result of complexity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Per-file metrics
    pub files: Vec<FileMetrics>,
    /// Summary statistics
    pub summary: SummaryStats,
    /// Metrics by language
    pub by_language: HashMap<String, SummaryStats>,
    /// Analysis duration in milliseconds
    pub duration_ms: u64,
    /// Files that couldn't be analyzed
    pub errors: Vec<String>,
    /// Thresholds used
    pub thresholds: Thresholds,
}

impl AnalysisResult {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            summary: SummaryStats::default(),
            by_language: HashMap::new(),
            duration_ms: 0,
            errors: Vec::new(),
            thresholds: Thresholds::default(),
        }
    }

    /// Get files exceeding complexity threshold
    pub fn high_complexity_files(&self, threshold: u32) -> Vec<&FileMetrics> {
        self.files
            .iter()
            .filter(|f| f.metrics.cyclomatic > threshold)
            .collect()
    }

    /// Get files sorted by complexity (descending)
    pub fn files_by_complexity(&self) -> Vec<&FileMetrics> {
        let mut files: Vec<_> = self.files.iter().collect();
        files.sort_by(|a, b| b.metrics.cyclomatic.cmp(&a.metrics.cyclomatic));
        files
    }

    /// Calculate summary statistics
    fn calculate_summary(&mut self) {
        self.summary.total_files = self.files.len();
        self.summary.total_functions = self.files.iter().map(|f| f.functions.len()).sum();
        self.summary.total_loc = self.files.iter().map(|f| f.metrics.loc as u64).sum();
        self.summary.total_sloc = self.files.iter().map(|f| f.metrics.sloc as u64).sum();

        if !self.files.is_empty() {
            let cyclo_sum: u32 = self.files.iter().map(|f| f.metrics.cyclomatic).sum();
            let cogn_sum: u32 = self.files.iter().map(|f| f.metrics.cognitive).sum();

            self.summary.avg_cyclomatic = cyclo_sum as f64 / self.files.len() as f64;
            self.summary.avg_cognitive = cogn_sum as f64 / self.files.len() as f64;
            self.summary.max_cyclomatic = self.files.iter().map(|f| f.metrics.cyclomatic).max().unwrap_or(0);
            self.summary.max_cognitive = self.files.iter().map(|f| f.metrics.cognitive).max().unwrap_or(0);
        }

        // Count high complexity
        self.summary.high_complexity_files = self.files
            .iter()
            .filter(|f| f.metrics.overall_level() == MetricLevel::High || f.metrics.overall_level() == MetricLevel::Critical)
            .count();

        self.summary.high_complexity_functions = self.files
            .iter()
            .flat_map(|f| &f.functions)
            .filter(|func| func.metrics.overall_level() == MetricLevel::High || func.metrics.overall_level() == MetricLevel::Critical)
            .count();

        // Calculate by language
        let mut by_lang: HashMap<String, Vec<&FileMetrics>> = HashMap::new();
        for file in &self.files {
            by_lang.entry(file.language.clone()).or_default().push(file);
        }

        for (lang, files) in by_lang {
            let mut stats = SummaryStats::default();
            stats.total_files = files.len();
            stats.total_functions = files.iter().map(|f| f.functions.len()).sum();
            stats.total_loc = files.iter().map(|f| f.metrics.loc as u64).sum();
            stats.total_sloc = files.iter().map(|f| f.metrics.sloc as u64).sum();

            if !files.is_empty() {
                let cyclo_sum: u32 = files.iter().map(|f| f.metrics.cyclomatic).sum();
                stats.avg_cyclomatic = cyclo_sum as f64 / files.len() as f64;
                stats.max_cyclomatic = files.iter().map(|f| f.metrics.cyclomatic).max().unwrap_or(0);
            }

            self.by_language.insert(lang, stats);
        }
    }
}

impl Default for AnalysisResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Main complexity analyzer
pub struct ComplexityAnalyzer {
    analyzers: Vec<Box<dyn LanguageComplexityAnalyzer>>,
}

impl Default for ComplexityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ComplexityAnalyzer {
    /// Create a new complexity analyzer with all language analyzers
    pub fn new() -> Self {
        let analyzers: Vec<Box<dyn LanguageComplexityAnalyzer>> = vec![
            Box::new(RustComplexityAnalyzer::new()),
            Box::new(TypeScriptComplexityAnalyzer::new()),
            Box::new(PythonComplexityAnalyzer::new()),
            Box::new(GoComplexityAnalyzer::new()),
            Box::new(JavaComplexityAnalyzer::new()),
        ];

        Self { analyzers }
    }

    /// Get analyzer for a file based on extension
    fn get_analyzer(&self, path: &Path) -> Option<&dyn LanguageComplexityAnalyzer> {
        let ext = path.extension()?.to_str()?;
        self.analyzers
            .iter()
            .find(|a| a.extensions().contains(&ext))
            .map(|a| a.as_ref())
    }

    /// Analyze a directory or file
    pub fn analyze(&self, options: &AnalysisOptions) -> Result<AnalysisResult, String> {
        let start = Instant::now();
        let mut result = AnalysisResult::new();

        // Collect files to analyze
        let files = self.collect_files(&options.path, &options.include, &options.exclude)?;

        // Analyze files
        if options.parallel {
            let results: Vec<_> = files
                .par_iter()
                .filter_map(|path| self.analyze_single_file(path))
                .collect();

            for file_result in results {
                match file_result {
                    Ok(metrics) => result.files.push(metrics),
                    Err(e) => result.errors.push(e),
                }
            }
        } else {
            for path in &files {
                match self.analyze_single_file(path) {
                    Some(Ok(metrics)) => result.files.push(metrics),
                    Some(Err(e)) => result.errors.push(e),
                    None => {}
                }
            }
        }

        result.calculate_summary();
        result.duration_ms = start.elapsed().as_millis() as u64;

        Ok(result)
    }

    fn analyze_single_file(&self, path: &Path) -> Option<Result<FileMetrics, String>> {
        let analyzer = self.get_analyzer(path)?;

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return Some(Err(format!("{}: {}", path.display(), e))),
        };

        Some(analyzer.analyze_file(path, &content))
    }

    fn collect_files(
        &self,
        path: &Path,
        include: &[String],
        exclude: &[String],
    ) -> Result<Vec<PathBuf>, String> {
        let mut files = Vec::new();

        if path.is_file() {
            files.push(path.to_path_buf());
            return Ok(files);
        }

        let walker = walkdir::WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let path = e.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                // Skip hidden files and common non-source directories
                if name.starts_with('.') {
                    return false;
                }

                if path.is_dir() {
                    let skip_dirs = ["node_modules", "target", "build", "dist", "__pycache__", ".git", "vendor"];
                    if skip_dirs.contains(&name) {
                        return false;
                    }
                }

                true
            });

        for entry in walker.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Check if we have an analyzer for this file
            if self.get_analyzer(path).is_none() {
                continue;
            }

            // Check exclude patterns
            let path_str = path.to_string_lossy();
            let should_exclude = exclude.iter().any(|pattern| {
                globset::Glob::new(pattern)
                    .ok()
                    .and_then(|g| g.compile_matcher().is_match(&*path_str).then_some(()))
                    .is_some()
            });

            if should_exclude {
                continue;
            }

            // Check include patterns (if any)
            if !include.is_empty() {
                let should_include = include.iter().any(|pattern| {
                    globset::Glob::new(pattern)
                        .ok()
                        .and_then(|g| g.compile_matcher().is_match(&*path_str).then_some(()))
                        .is_some()
                });

                if !should_include {
                    continue;
                }
            }

            files.push(path.to_path_buf());
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_options_default() {
        let options = AnalysisOptions::default();
        assert!(options.include.is_empty());
        assert!(options.exclude.is_empty());
        assert!(options.parallel);
    }

    #[test]
    fn test_analysis_result_default() {
        let result = AnalysisResult::new();
        assert!(result.files.is_empty());
        assert_eq!(result.duration_ms, 0);
    }

    #[test]
    fn test_complexity_analyzer_creation() {
        let analyzer = ComplexityAnalyzer::new();
        let temp_dir = tempfile::tempdir().unwrap();

        let options = AnalysisOptions::new(temp_dir.path().to_path_buf());
        let result = analyzer.analyze(&options);
        assert!(result.is_ok());
    }
}
