// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! CLI handler for security scanning commands.

use std::path::PathBuf;
use std::process::ExitCode;

use colored::Colorize;

use linthis::security::{
    format_security_report, ScanOptions, SecurityScanner, Severity,
};
use linthis::security::report::SecurityReportFormat;

/// Handle the security subcommand
pub fn handle_security_command(
    path: PathBuf,
    severity: Option<String>,
    include_dev: bool,
    fix: bool,
    ignore: Option<Vec<String>>,
    format: String,
    sbom: bool,
    fail_on: Option<String>,
    verbose: bool,
) -> ExitCode {
    let scanner = SecurityScanner::new();

    // Show available scanners in verbose mode
    if verbose {
        println!("{}", "Available security scanners:".bold());
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

    // Detect languages
    let languages = scanner.detect_languages(&path);
    if languages.is_empty() {
        println!("{}", "No supported project files detected.".yellow());
        println!("Supported files: Cargo.toml, package.json, requirements.txt, go.mod, pom.xml, build.gradle");
        return ExitCode::SUCCESS;
    }

    if verbose {
        println!("Detected languages: {}", languages.join(", "));
        println!();
    }

    // Build scan options
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

    // Run scan
    println!("{}", "🔍 Scanning for vulnerabilities...".bold());

    match scanner.scan(&options) {
        Ok(result) => {
            // Format and print results
            let report_format = SecurityReportFormat::from_str(&format);
            let output = format_security_report(&result, report_format);
            println!("{}", output);

            // Show fix suggestions if requested
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
                            println!("\n{}", "⚠️  Some vulnerabilities require manual review".yellow());
                        }
                    }
                    Err(e) => {
                        eprintln!("{}: {}", "Failed to generate fix suggestions".red(), e);
                    }
                }
            }

            // Check fail condition
            if let Some(ref threshold_str) = fail_on {
                let threshold = Severity::from_str(threshold_str);
                if result.has_vulnerabilities_above(threshold) {
                    eprintln!(
                        "\n{}: Found vulnerabilities with severity >= {}",
                        "Error".red().bold(),
                        threshold_str
                    );
                    return ExitCode::from(1);
                }
            }

            // Return success unless critical/high vulnerabilities found
            if result.critical_high_count() > 0 && fail_on.is_none() {
                // Only warn, don't fail by default
                eprintln!(
                    "\n{}: {} critical/high vulnerabilities found",
                    "Warning".yellow().bold(),
                    result.critical_high_count()
                );
            }

            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}: {}", "Security scan failed".red().bold(), e);
            ExitCode::from(1)
        }
    }
}
