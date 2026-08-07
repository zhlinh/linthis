// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! CLI handler for security scanning commands (SCA + SAST).

use std::path::PathBuf;
use std::process::ExitCode;

use colored::Colorize;

use std::path::Path;

use linthis::cache::PerFileCache;
use linthis::config::SecurityChecksConfig;
use linthis::security::report::SecurityReportFormat;
use linthis::security::sast::{
    format_sast_report, is_skipped_scan_dir, SastAggregator, SastResult, SastScanOptions,
};
use linthis::security::{
    format_security_report, ScanOptions, ScanResult, SecurityScanner, Severity,
};

/// Parameters for the security subcommand.
pub struct SecurityCommandParams {
    pub path: PathBuf,
    pub scan_type: String,
    pub severity: Option<String>,
    pub include_dev: bool,
    pub fix: bool,
    pub ignore: Option<Vec<String>>,
    pub format: String,
    pub sbom: bool,
    pub fail_on: Option<String>,
    pub sast_config: Option<PathBuf>,
    pub verbose: bool,
}

/// Handle the security subcommand
pub fn handle_security_command(params: SecurityCommandParams) -> ExitCode {
    let SecurityCommandParams {
        path,
        scan_type,
        severity,
        include_dev,
        fix,
        ignore,
        format,
        sbom,
        fail_on,
        sast_config,
        verbose,
    } = params;
    let report_format = SecurityReportFormat::from_str(&format);
    let run_sca = scan_type == "all" || scan_type == "sca";
    let run_sast = scan_type == "all" || scan_type == "sast";

    if run_sca {
        let sca_exit = handle_sca(ScaParams {
            path: &path,
            severity: &severity,
            include_dev,
            fix,
            ignore,
            format: &format,
            sbom,
            fail_on: &fail_on,
            report_format,
            run_sast,
            verbose,
        });
        if let Some(code) = sca_exit {
            return code;
        }
    }

    if run_sast {
        let sast_exit = handle_sast(&path, &severity, sast_config, report_format, verbose);
        if let Some(code) = sast_exit {
            return code;
        }
    }

    ExitCode::SUCCESS
}

/// Parameters for SCA scanning.
struct ScaParams<'a> {
    path: &'a Path,
    severity: &'a Option<String>,
    include_dev: bool,
    fix: bool,
    ignore: Option<Vec<String>>,
    format: &'a str,
    sbom: bool,
    fail_on: &'a Option<String>,
    report_format: SecurityReportFormat,
    run_sast: bool,
    verbose: bool,
}

/// Run SCA scanning. Returns `Some(ExitCode)` for early return, `None` to continue.
fn handle_sca(params: ScaParams<'_>) -> Option<ExitCode> {
    let scanner = SecurityScanner::new();

    if params.verbose {
        print_sca_scanners(&scanner);
    }

    let languages = scanner.detect_languages(params.path);
    if languages.is_empty() && !params.run_sast {
        println!("{}", "No supported project files detected.".yellow());
        println!(
            "Supported files: Cargo.toml, package.json, requirements.txt, go.mod, pom.xml, build.gradle"
        );
        return Some(ExitCode::SUCCESS);
    }

    if languages.is_empty() {
        return None;
    }

    if params.verbose {
        println!("Detected languages: {}", languages.join(", "));
        println!();
    }

    let options = ScanOptions {
        path: params.path.to_path_buf(),
        severity_threshold: params.severity.clone(),
        include_dev: params.include_dev,
        packages: vec![],
        ignore: params.ignore.unwrap_or_default(),
        format: params.format.to_string(),
        generate_sbom: params.sbom,
        fail_on: params.fail_on.clone(),
        verbose: params.verbose,
    };

    if params.report_format == SecurityReportFormat::Human {
        println!(
            "{}",
            "🔍 SCA: Scanning dependencies for vulnerabilities...".bold()
        );
    } else {
        eprintln!("🔍 SCA: Scanning dependencies for vulnerabilities...");
    }

    match scanner.scan(&options) {
        Ok(result) => {
            let output = format_security_report(&result, params.report_format);
            println!("{}", output);

            if params.fix && !result.vulnerabilities.is_empty() {
                print_fix_suggestions(&scanner, params.path, &result);
            }
        }
        Err(e) => {
            eprintln!("{}: {}", "SCA scan failed".red().bold(), e);
            if !params.run_sast {
                return Some(ExitCode::from(1));
            }
        }
    }

    None
}

/// Print available SCA scanners.
fn print_sca_scanners(scanner: &SecurityScanner) {
    println!("{}", "Available SCA scanners:".bold());
    for (name, lang, available) in scanner.available_scanners() {
        let status = if available {
            "✓".green()
        } else {
            "✗".red()
        };
        println!("  {} {} ({})", status, name, lang);
    }
    println!();
}

/// Print fix suggestions for SCA vulnerabilities.
fn print_fix_suggestions(scanner: &SecurityScanner, path: &Path, result: &ScanResult) {
    println!("{}", "\n📋 Fix Suggestions:".bold());
    println!("{}", "-".repeat(50));

    match scanner.fix(path, result) {
        Ok(fix_result) => {
            if !fix_result.commands.is_empty() {
                println!("\nRecommended commands:");
                for cmd in &fix_result.commands {
                    println!("  $ {}", cmd.cyan());
                }
            }
            if !fix_result.messages.is_empty() {
                println!("\nNotes:");
                for msg in &fix_result.messages {
                    println!("  • {}", msg);
                }
            }
            if fix_result.needs_review {
                println!(
                    "\n{}",
                    "⚠️  Some vulnerabilities require manual review".yellow()
                );
            }
        }
        Err(e) => {
            eprintln!("{}: {}", "Failed to generate fix suggestions".red(), e);
        }
    }
}

/// Run SAST scanning. Returns `Some(ExitCode)` for early return, `None` to continue.
fn handle_sast(
    path: &Path,
    severity: &Option<String>,
    sast_config: Option<PathBuf>,
    report_format: SecurityReportFormat,
    verbose: bool,
) -> Option<ExitCode> {
    let sast = SastAggregator::with_config(sast_config.as_deref());

    if verbose {
        print_sast_scanners(&sast);
    }

    let sast_options = SastScanOptions {
        severity_threshold: severity.as_ref().map(|s| Severity::from_str(s)),
        config_path: sast_config,
        rules: vec![],
        exclude: vec![],
        verbose,
    };

    let result = run_cached_sast_scan(&sast, path, &sast_options, report_format);

    let output = format_sast_report(&result, report_format);
    println!("{}", output);

    if !result.errors.is_empty() {
        for err in &result.errors {
            eprintln!("  {}: {}", "Error".red(), err);
        }
    }

    let exit_code = compute_sast_exit_code(&result);
    if exit_code != 0 {
        return Some(ExitCode::from(exit_code as u8));
    }

    None
}

/// Print available SAST scanners.
fn print_sast_scanners(sast: &SastAggregator) {
    println!("{}", "Available SAST scanners:".bold());
    for (name, available, langs) in sast.available_scanners() {
        let status = if available {
            "✓".green()
        } else {
            "✗".red()
        };
        println!("  {} {} ({})", status, name, langs.join(", "));
    }
    println!();
}

/// Collect source files for SAST scanning from a directory.
///
/// Prunes linthis' own working directory, VCS metadata, dependency trees, and
/// build outputs (see `is_skipped_scan_dir`). Without this, `.linthis/secrets.toml`
/// is scanned by the very patterns it defines and reports itself as a finding.
fn collect_sast_target_files(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }

    walkdir::WalkDir::new(path)
        .max_depth(10)
        .into_iter()
        .filter_entry(|e| {
            // Depth 0 is the scan root itself and must not be pruned, or the walk
            // yields nothing at all.
            if e.depth() == 0 || !e.file_type().is_dir() {
                return true;
            }
            !is_skipped_scan_dir(&e.file_name().to_string_lossy())
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.')
        })
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
                            | "cs"
                            | "yaml"
                            | "yml"
                            | "toml"
                            | "json"
                            | "env"
                            | "cfg"
                            | "ini"
                            | "conf"
                            | "sh"
                            | "mm"
                            | "m"
                    )
                })
                .unwrap_or(false)
        })
        .map(|e| e.into_path())
        .collect()
}

/// Run SAST scan with per-file caching, returning the merged result.
fn run_cached_sast_scan(
    sast: &SastAggregator,
    path: &Path,
    sast_options: &SastScanOptions,
    report_format: SecurityReportFormat,
) -> SastResult {
    let cache_path = linthis::utils::get_cache_dir().join("security-cache.json");
    let mut cache = PerFileCache::load(&cache_path);

    let target_files = collect_sast_target_files(path);
    let partition = cache.partition_files(&target_files, false);

    if report_format == SecurityReportFormat::Human {
        eprintln!("{}", PerFileCache::format_status("security", &partition));
    }

    if !partition.changed.is_empty() {
        let r = sast.scan(path, &partition.changed, sast_options);
        cache.update_from_sast(&partition.changed, &r);
        cache.save(&cache_path);

        let mut merged = r;
        let mut all_findings = partition.cached_findings;
        all_findings.append(&mut merged.findings);
        merged.findings = all_findings;
        rebuild_sast_counts(&mut merged);
        merged
    } else {
        let mut merged = SastResult {
            findings: partition.cached_findings,
            by_severity: std::collections::HashMap::new(),
            by_tool: std::collections::HashMap::new(),
            scanner_status: sast
                .available_scanners()
                .iter()
                .map(|(n, a, _)| (n.to_string(), *a))
                .collect(),
            unavailable_tools: vec![],
            duration_ms: 0,
            errors: vec![],
        };
        rebuild_sast_counts(&mut merged);
        merged
    }
}

/// Rebuild severity and tool count maps from findings.
fn rebuild_sast_counts(result: &mut SastResult) {
    result.by_severity.clear();
    for f in &result.findings {
        *result
            .by_severity
            .entry(f.severity.to_string())
            .or_insert(0) += 1;
    }
    result.by_tool.clear();
    for f in &result.findings {
        *result.by_tool.entry(f.source.clone()).or_insert(0) += 1;
    }
}

/// Compute exit code from SAST findings using unified FailOn.
fn compute_sast_exit_code(result: &SastResult) -> i32 {
    let sec_errors = result
        .findings
        .iter()
        .filter(|f| {
            matches!(
                f.severity,
                linthis::security::Severity::Critical | linthis::security::Severity::High
            )
        })
        .count();
    let sec_warnings = result
        .findings
        .iter()
        .filter(|f| f.severity == linthis::security::Severity::Medium)
        .count();
    let sec_infos = result
        .findings
        .iter()
        .filter(|f| {
            matches!(
                f.severity,
                linthis::security::Severity::Low
                    | linthis::security::Severity::None
                    | linthis::security::Severity::Unknown
            )
        })
        .count();
    let fail_on_level = linthis::config::FailOn::default();
    fail_on_level.exit_code(sec_errors, sec_warnings, sec_infos)
}

/// Run SAST scan and return results (for integration with main lint flow via --checks).
///
/// When `files` is non-empty, only those files are scanned.
/// When empty, scans the entire `path` directory.
pub fn run_sast_scan(path: &Path, files: &[PathBuf], config: &SecurityChecksConfig) -> SastResult {
    let sast = SastAggregator::with_config(config.sast_config.as_deref());
    let sast_options = SastScanOptions {
        severity_threshold: None, // report all findings, fail_on controls exit code
        config_path: config.sast_config.clone(),
        ..Default::default()
    };
    sast.scan(path, files, &sast_options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn file_names(paths: &[PathBuf]) -> Vec<String> {
        let mut names: Vec<String> = paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn collect_prunes_linthis_and_build_dirs() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        fs::create_dir_all(root.join(".linthis")).unwrap();
        fs::write(root.join(".linthis/secrets.toml"), "id = \"x\"\n").unwrap();
        fs::create_dir_all(root.join("target/debug")).unwrap();
        fs::write(root.join("target/debug/meta.json"), "{}\n").unwrap();
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::write(root.join("node_modules/pkg/index.js"), "\n").unwrap();
        fs::write(root.join("app.py"), "\n").unwrap();

        assert_eq!(file_names(&collect_sast_target_files(root)), ["app.py"]);
    }

    #[test]
    fn collect_keeps_ci_workflow_files() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        fs::create_dir_all(root.join(".github/workflows")).unwrap();
        fs::write(root.join(".github/workflows/ci.yml"), "on: push\n").unwrap();

        assert_eq!(file_names(&collect_sast_target_files(root)), ["ci.yml"]);
    }

    #[test]
    fn collect_does_not_prune_the_scan_root_itself() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join(".linthis");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("secrets.toml"), "id = \"x\"\n").unwrap();

        // Pruning depth 0 would make the walk yield nothing. `SastAggregator::scan`
        // is what ultimately drops files under a skipped directory.
        assert_eq!(
            file_names(&collect_sast_target_files(&root)),
            ["secrets.toml"]
        );
    }
}
