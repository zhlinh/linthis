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
use linthis::security::sast::{format_sast_report, SastAggregator, SastResult, SastScanOptions};
use linthis::security::{format_security_report, ScanOptions, SecurityScanner, Severity};

/// Handle the security subcommand
pub fn handle_security_command(
    path: PathBuf,
    scan_type: String,
    severity: Option<String>,
    include_dev: bool,
    fix: bool,
    ignore: Option<Vec<String>>,
    format: String,
    sbom: bool,
    fail_on: Option<String>,
    sast_config: Option<PathBuf>,
    verbose: bool,
) -> ExitCode {
    let report_format = SecurityReportFormat::from_str(&format);
    let run_sca = scan_type == "all" || scan_type == "sca";
    let run_sast = scan_type == "all" || scan_type == "sast";
    let mut has_critical_high = false;

    // --- SCA (Dependency Vulnerability Scanning) ---
    if run_sca {
        let scanner = SecurityScanner::new();

        if verbose {
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

        let languages = scanner.detect_languages(&path);
        if languages.is_empty() && !run_sast {
            println!("{}", "No supported project files detected.".yellow());
            println!("Supported files: Cargo.toml, package.json, requirements.txt, go.mod, pom.xml, build.gradle");
            return ExitCode::SUCCESS;
        }

        if !languages.is_empty() {
            if verbose {
                println!("Detected languages: {}", languages.join(", "));
                println!();
            }

            let options = ScanOptions {
                path: path.clone(),
                severity_threshold: severity.clone(),
                include_dev,
                packages: vec![],
                ignore: ignore.unwrap_or_default(),
                format: format.clone(),
                generate_sbom: sbom,
                fail_on: fail_on.clone(),
                verbose,
            };

            if report_format == SecurityReportFormat::Human {
                println!("{}", "🔍 SCA: Scanning dependencies for vulnerabilities...".bold());
            } else {
                eprintln!("🔍 SCA: Scanning dependencies for vulnerabilities...");
            }

            match scanner.scan(&options) {
                Ok(result) => {
                    let output = format_security_report(&result, report_format);
                    println!("{}", output);

                    if fix && !result.vulnerabilities.is_empty() {
                        println!("{}", "\n📋 Fix Suggestions:".bold());
                        println!("{}", "-".repeat(50));

                        match scanner.fix(&path, &result) {
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
                                eprintln!(
                                    "{}: {}",
                                    "Failed to generate fix suggestions".red(),
                                    e
                                );
                            }
                        }
                    }

                    if result.critical_high_count() > 0 {
                        has_critical_high = true;
                    }
                }
                Err(e) => {
                    eprintln!("{}: {}", "SCA scan failed".red().bold(), e);
                    if !run_sast {
                        return ExitCode::from(1);
                    }
                }
            }
        }
    }

    // --- SAST (Source Code Security Analysis) ---
    if run_sast {
        let sast = SastAggregator::with_config(sast_config.as_deref());

        if verbose {
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

        let sast_options = SastScanOptions {
            severity_threshold: severity.as_ref().map(|s| Severity::from_str(s)),
            config_path: sast_config,
            rules: vec![],
            exclude: vec![],
            verbose,
        };

        // Per-file cache for SAST
        let project_root = linthis::utils::get_project_root();
        let cache_path = project_root.join(".linthis").join("security-cache.json");
        let mut cache = PerFileCache::load(&cache_path);

        // Collect target files for cache partitioning
        let target_files: Vec<PathBuf> = if path.is_file() {
            vec![path.clone()]
        } else {
            // Walk directory to collect source files
            walkdir::WalkDir::new(&path)
                .max_depth(10)
                .into_iter()
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
                                "py" | "js" | "jsx" | "ts" | "tsx" | "go" | "rs" | "java"
                                    | "kt" | "c" | "h" | "cpp" | "cc" | "rb" | "php"
                                    | "swift" | "scala" | "cs" | "yaml" | "yml" | "toml"
                                    | "json" | "env" | "cfg" | "ini" | "conf" | "sh"
                                    | "mm" | "m"
                            )
                        })
                        .unwrap_or(false)
                })
                .map(|e| e.into_path())
                .collect()
        };

        let partition = cache.partition_files(&target_files, false);

        // Show cache status
        if report_format == SecurityReportFormat::Human {
            eprintln!("{}", PerFileCache::format_status("security", &partition));
        }

        // Only scan changed files
        let result = if !partition.changed.is_empty() {
            let r = sast.scan(&path, &partition.changed, &sast_options);
            cache.update_from_sast(&partition.changed, &r);
            cache.save(&cache_path);

            // Merge cached + fresh
            let mut merged = r;
            let mut all_findings = partition.cached_findings;
            all_findings.append(&mut merged.findings);
            merged.findings = all_findings;
            // Rebuild counts
            merged.by_severity.clear();
            for f in &merged.findings {
                *merged.by_severity.entry(f.severity.to_string()).or_insert(0) += 1;
            }
            merged.by_tool.clear();
            for f in &merged.findings {
                *merged.by_tool.entry(f.source.clone()).or_insert(0) += 1;
            }
            merged
        } else {
            // All cached — build result from cached findings only
            let mut merged = linthis::security::sast::SastResult {
                findings: partition.cached_findings,
                by_severity: std::collections::HashMap::new(),
                by_tool: std::collections::HashMap::new(),
                scanner_status: sast.available_scanners().iter().map(|(n, a, _)| (n.to_string(), *a)).collect(),
                unavailable_tools: vec![],
                duration_ms: 0,
                errors: vec![],
            };
            for f in &merged.findings {
                *merged.by_severity.entry(f.severity.to_string()).or_insert(0) += 1;
            }
            for f in &merged.findings {
                *merged.by_tool.entry(f.source.clone()).or_insert(0) += 1;
            }
            merged
        };

        let output = format_sast_report(&result, report_format);
        println!("{}", output);

        if !result.errors.is_empty() {
            for err in &result.errors {
                eprintln!("  {}: {}", "Error".red(), err);
            }
        }

        // Calculate exit code using unified FailOn (default: warning)
        let sec_errors = result.findings.iter().filter(|f| {
            matches!(f.severity, linthis::security::Severity::Critical | linthis::security::Severity::High)
        }).count();
        let sec_warnings = result.findings.iter().filter(|f| {
            f.severity == linthis::security::Severity::Medium
        }).count();
        let sec_infos = result.findings.iter().filter(|f| {
            matches!(f.severity, linthis::security::Severity::Low | linthis::security::Severity::None | linthis::security::Severity::Unknown)
        }).count();
        let fail_on_level = linthis::config::FailOn::default(); // warning
        let exit_code = fail_on_level.exit_code(sec_errors, sec_warnings, sec_infos);
        if exit_code != 0 {
            return ExitCode::from(exit_code as u8);
        }
    }

    ExitCode::SUCCESS
}

/// Run SAST scan and return results (for integration with main lint flow via --checks).
///
/// When `files` is non-empty, only those files are scanned.
/// When empty, scans the entire `path` directory.
pub fn run_sast_scan(
    path: &Path,
    files: &[PathBuf],
    config: &SecurityChecksConfig,
) -> SastResult {
    let sast = SastAggregator::with_config(config.sast_config.as_deref());
    let sast_options = SastScanOptions {
        severity_threshold: None, // report all findings, fail_on controls exit code
        config_path: config.sast_config.clone(),
        ..Default::default()
    };
    sast.scan(path, files, &sast_options)
}
