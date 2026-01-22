// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! CLI handler for license compliance commands.

use std::path::PathBuf;
use std::process::ExitCode;

use colored::Colorize;

use linthis::license::{
    format_license_report, LicensePolicy, LicenseReportFormat, LicenseScanner, ScanOptions,
};

/// Handle the license subcommand
pub fn handle_license_command(
    path: PathBuf,
    policy: String,
    policy_file: Option<PathBuf>,
    include_dev: bool,
    format: String,
    sbom: bool,
    fail_on_violation: bool,
    verbose: bool,
) -> ExitCode {
    let scanner = LicenseScanner::new();

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
        include_dev,
        format: format.clone(),
        generate_sbom: sbom,
        verbose,
    };

    // Load policy
    let license_policy = if let Some(ref policy_path) = policy_file {
        match std::fs::read_to_string(policy_path) {
            Ok(content) => match LicensePolicy::from_toml(&content) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{}: {}", "Failed to load policy file".red(), e);
                    return ExitCode::from(1);
                }
            },
            Err(e) => {
                eprintln!("{}: {}", "Failed to read policy file".red(), e);
                return ExitCode::from(1);
            }
        }
    } else {
        match policy.as_str() {
            "strict" => LicensePolicy::strict(),
            "permissive" => LicensePolicy::permissive(),
            _ => LicensePolicy::default(),
        }
    };

    // Run scan
    println!("{}", "📜 Scanning for licenses...".bold());

    match scanner.scan(&options) {
        Ok(result) => {
            // Check policy
            let violations = license_policy.check(&result);

            // Format and print results
            let report_format = LicenseReportFormat::from_str(&format);
            let output = format_license_report(&result, &violations, report_format);
            println!("{}", output);

            // Generate SBOM if requested
            if sbom && format != "spdx" {
                println!("\n{}", "📋 SBOM (SPDX format):".bold());
                let sbom_output = format_license_report(
                    &result,
                    &violations,
                    LicenseReportFormat::Spdx,
                );
                println!("{}", sbom_output);
            }

            // Check for violations
            let error_violations: Vec<_> = violations
                .iter()
                .filter(|v| {
                    v.violation_type != linthis::license::policy::ViolationType::Warning
                })
                .collect();

            if fail_on_violation && !error_violations.is_empty() {
                eprintln!(
                    "\n{}: {} license policy violation(s) found",
                    "Error".red().bold(),
                    error_violations.len()
                );
                return ExitCode::from(1);
            }

            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}: {}", "License scan failed".red().bold(), e);
            ExitCode::from(1)
        }
    }
}
