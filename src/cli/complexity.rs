// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! CLI handler for complexity analysis command.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use linthis::cache::PerFileCache;
use linthis::complexity::{
    format_complexity_report, AnalysisOptions, AnalysisResult, ComplexityAnalyzer,
    ComplexityReportFormat, MetricLevel, Thresholds,
};
use linthis::config::ComplexityChecksConfig;
use linthis::utils::{get_staged_files, get_uncommitted_files};

/// Options for complexity command
pub struct ComplexityCommandOptions {
    pub path: PathBuf,
    pub staged: bool,
    pub modified: bool,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub threshold: Option<u32>,
    pub preset: String,
    pub format: String,
    pub with_trends: bool,
    pub trend_count: usize,
    pub only_high: bool,
    pub sort: String,
    pub no_parallel: bool,
    pub verbose: bool,
}

/// Handle the complexity analysis command
pub fn handle_complexity_command(options: ComplexityCommandOptions) -> ExitCode {
    let target_files = match resolve_target_files(&options) {
        Ok(files) => files,
        Err(code) => return code,
    };

    let analysis_options = build_analysis_options(&options, &target_files);

    show_cache_status(&analysis_options, &options);

    let mut result = match ComplexityAnalyzer::new().analyze(&analysis_options) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error during analysis: {}", e);
            return ExitCode::FAILURE;
        }
    };

    update_cache_after_analysis(&result);
    apply_thresholds(&mut result, &options);

    let format = options
        .format
        .parse::<ComplexityReportFormat>()
        .unwrap_or_default();

    filter_and_sort(&mut result, &options);

    let report = format_complexity_report(&result, format);
    println!("{}", report);

    save_complexity_result(&result, options.verbose);

    compute_complexity_exit_code(&result)
}

/// Resolve target files from --staged, --modified, or default path.
/// Returns `Err(ExitCode)` for early exit, `Ok(None)` for full-directory mode.
fn resolve_target_files(
    options: &ComplexityCommandOptions,
) -> Result<Option<Vec<PathBuf>>, ExitCode> {
    if options.staged {
        match get_staged_files() {
            Ok(files) => {
                if files.is_empty() {
                    eprintln!("No staged files found.");
                    return Err(ExitCode::SUCCESS);
                }
                if options.verbose {
                    println!("Analyzing {} staged file(s)", files.len());
                }
                Ok(Some(files))
            }
            Err(e) => {
                eprintln!("Failed to get staged files: {}", e);
                Err(ExitCode::FAILURE)
            }
        }
    } else if options.modified {
        match get_uncommitted_files() {
            Ok(files) => {
                if files.is_empty() {
                    eprintln!("No modified files found.");
                    return Err(ExitCode::SUCCESS);
                }
                if options.verbose {
                    println!("Analyzing {} modified file(s)", files.len());
                }
                Ok(Some(files))
            }
            Err(e) => {
                eprintln!("Failed to get modified files: {}", e);
                Err(ExitCode::FAILURE)
            }
        }
    } else {
        if options.verbose {
            println!("Analyzing code complexity in: {}", options.path.display());
        }
        Ok(None)
    }
}

/// Build `AnalysisOptions` from command options and resolved files.
fn build_analysis_options(
    options: &ComplexityCommandOptions,
    target_files: &Option<Vec<PathBuf>>,
) -> AnalysisOptions {
    let mut analysis_options = AnalysisOptions::new(options.path.clone());

    if let Some(ref files) = target_files {
        analysis_options.files = files.clone();
    }

    analysis_options.include = options.include.clone().unwrap_or_default();
    analysis_options.exclude = options.exclude.clone().unwrap_or_default();
    analysis_options.threshold = options.threshold;
    analysis_options.format = options.format.clone();
    analysis_options.with_trends = options.with_trends;
    analysis_options.trend_count = options.trend_count;
    analysis_options.verbose = options.verbose;
    analysis_options.parallel = !options.no_parallel;

    analysis_options
}

/// Display per-file cache status.
fn show_cache_status(analysis_options: &AnalysisOptions, options: &ComplexityCommandOptions) {
    let cache_path = linthis::utils::get_cache_dir().join("complexity-cache.json");
    let cache = PerFileCache::load(&cache_path);

    let cache_target = if !analysis_options.files.is_empty() {
        analysis_options.files.clone()
    } else {
        collect_complexity_files(&analysis_options.path)
    };

    let partition = cache.partition_files(&cache_target, false);
    if options.verbose || !partition.changed.is_empty() || partition.cache_hits > 0 {
        eprintln!("{}", PerFileCache::format_status("complexity", &partition));
    }
}

/// Collect source files eligible for complexity analysis from a directory.
fn collect_complexity_files(path: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(path)
        .max_depth(10)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    matches!(
                        ext,
                        "py" | "js"
                            | "jsx"
                            | "ts"
                            | "tsx"
                            | "go"
                            | "rs"
                            | "java"
                            | "kt"
                            | "c"
                            | "h"
                            | "cpp"
                            | "cc"
                            | "rb"
                            | "php"
                            | "swift"
                            | "scala"
                            | "mm"
                            | "m"
                            | "lua"
                            | "sh"
                    )
                })
                .unwrap_or(false)
        })
        .map(|e| e.into_path())
        .collect()
}

/// Update the per-file cache after analysis.
fn update_cache_after_analysis(result: &AnalysisResult) {
    let cache_path = linthis::utils::get_cache_dir().join("complexity-cache.json");
    let mut cache = PerFileCache::load(&cache_path);
    let analyzed_files: Vec<PathBuf> = result.files.iter().map(|f| f.path.clone()).collect();
    cache.update_from_complexity(&analyzed_files, result);
    cache.save(&cache_path);
}

/// Apply threshold preset and custom overrides.
fn apply_thresholds(result: &mut AnalysisResult, options: &ComplexityCommandOptions) {
    result.thresholds = match options.preset.as_str() {
        "strict" => Thresholds::strict(),
        "lenient" => Thresholds::lenient(),
        _ => Thresholds::default(),
    };

    if let Some(threshold) = options.threshold {
        result.thresholds.cyclomatic.good = threshold;
        result.thresholds.cyclomatic.warning = threshold + 10;
        result.thresholds.cyclomatic.high = threshold + 20;
    }
    result.thresholds.cyclomatic.normalize();
}

/// Filter to high-complexity files and sort results.
fn filter_and_sort(result: &mut AnalysisResult, options: &ComplexityCommandOptions) {
    if options.only_high {
        result.files.retain(|f| {
            f.metrics.overall_level() == MetricLevel::High
                || f.metrics.overall_level() == MetricLevel::Critical
        });
    }

    match options.sort.as_str() {
        "cognitive" => {
            result
                .files
                .sort_by(|a, b| b.metrics.cognitive.cmp(&a.metrics.cognitive));
        }
        "lines" | "loc" => {
            result
                .files
                .sort_by(|a, b| b.metrics.loc.cmp(&a.metrics.loc));
        }
        "name" => {
            result.files.sort_by(|a, b| a.path.cmp(&b.path));
        }
        _ => {
            result
                .files
                .sort_by(|a, b| b.metrics.cyclomatic.cmp(&a.metrics.cyclomatic));
        }
    }
}

/// Save complexity result as JSON to the global check/result/ directory.
fn save_complexity_result(result: &AnalysisResult, verbose: bool) {
    use chrono::Local;
    use std::fs::{self, File};
    use std::io::Write;

    let result_dir = linthis::utils::get_result_dir();
    if let Err(e) = fs::create_dir_all(&result_dir) {
        eprintln!("Warning: Failed to create {}: {}", result_dir.display(), e);
        return;
    }

    let timestamp = Local::now().format("%Y%m%d-%H%M%S");
    let result_file = result_dir.join(format!("complexity-{}.json", timestamp));
    match serde_json::to_string_pretty(result) {
        Ok(json) => match File::create(&result_file) {
            Ok(mut f) => {
                let _ = writeln!(f, "{}", json);
                if !verbose {
                    eprintln!(
                        "\x1b[32m✓\x1b[0m Results saved to {}",
                        result_file.display()
                    );
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to write {}: {}", result_file.display(), e);
            }
        },
        Err(e) => eprintln!("Warning: Failed to serialize result: {}", e),
    }
}

/// Calculate exit code based on complexity thresholds and FailOn level.
fn compute_complexity_exit_code(result: &AnalysisResult) -> ExitCode {
    let counts =
        linthis::complexity::count_cyclomatic(&result.files, &result.thresholds.cyclomatic);
    let fail_on = linthis::config::FailOn::default();
    let code = fail_on.exit_code(counts.errors, counts.warnings, counts.infos);
    ExitCode::from(code as u8)
}

/// Run complexity analysis and return results (for integration with main lint flow via --checks).
pub fn run_complexity_analysis(
    path: &Path,
    files: &[PathBuf],
    config: &ComplexityChecksConfig,
) -> Result<AnalysisResult, String> {
    let analyzer = ComplexityAnalyzer::new();
    let mut options = AnalysisOptions::new(path.to_path_buf());
    if !files.is_empty() {
        options.files = files.to_vec();
    }
    options.threshold = config.threshold;
    let mut result = analyzer.analyze(&options)?;

    // Apply threshold overrides from config
    if let Some(t) = config.threshold {
        result.thresholds.cyclomatic.good = t;
        result.thresholds.cyclomatic.warning = t + 10;
        result.thresholds.cyclomatic.high = t + 20;
    }
    if let Some(w) = config.warning_threshold {
        result.thresholds.cyclomatic.warning = w;
    }
    if let Some(e) = config.error_threshold {
        result.thresholds.cyclomatic.high = e;
    }
    // Normalize: ensure good <= warning <= high
    result.thresholds.cyclomatic.normalize();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complexity_command_options() {
        let options = ComplexityCommandOptions {
            path: PathBuf::from("."),
            staged: false,
            modified: false,
            include: None,
            exclude: None,
            threshold: Some(10),
            preset: "default".to_string(),
            format: "human".to_string(),
            with_trends: false,
            trend_count: 10,
            only_high: false,
            sort: "cyclomatic".to_string(),
            no_parallel: false,
            verbose: false,
        };

        assert_eq!(options.threshold, Some(10));
        assert_eq!(options.preset, "default");
    }
}
