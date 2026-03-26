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

        // Use stderr for status messages when outputting structured formats
        if report_format == SecurityReportFormat::Human {
            println!(
                "{}",
                "🔍 SAST: Scanning source code for security issues...".bold()
            );
        } else {
            eprintln!("🔍 SAST: Scanning source code for security issues...");
        }

        let result = sast.scan(&path, &[], &sast_options);

        let output = format_sast_report(&result, report_format);
        println!("{}", output);

        if !result.errors.is_empty() {
            for err in &result.errors {
                eprintln!("  {}: {}", "Error".red(), err);
            }
        }

        if result.critical_high_count() > 0 {
            has_critical_high = true;
        }
    }

    // Check fail condition
    if let Some(ref threshold_str) = fail_on {
        if has_critical_high {
            eprintln!(
                "\n{}: Found security issues with severity >= {}",
                "Error".red().bold(),
                threshold_str
            );
            return ExitCode::from(1);
        }
    }

    if has_critical_high && fail_on.is_none() {
        eprintln!(
            "\n{}: Critical/high security issues found",
            "Warning".yellow().bold(),
        );
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
        severity_threshold: config.fail_on.as_ref().map(|s| Severity::from_str(s)),
        config_path: config.sast_config.clone(),
        ..Default::default()
    };
    sast.scan(path, files, &sast_options)
}
