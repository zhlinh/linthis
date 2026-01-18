// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Cache management CLI handlers.

use super::commands::CacheCommands;
use colored::Colorize;
use linthis::cache::LintCache;
use linthis::utils::get_project_root;
use std::process::ExitCode;

/// Handle cache subcommands
pub fn handle_cache_command(action: CacheCommands) -> ExitCode {
    match action {
        CacheCommands::Clear => handle_cache_clear(),
        CacheCommands::Status => handle_cache_status(),
    }
}

/// Clear the lint cache
fn handle_cache_clear() -> ExitCode {
    let project_root = get_project_root();

    match LintCache::clear(&project_root) {
        Ok(()) => {
            println!("{} Cache cleared successfully", "✓".green());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}: {}", "Error clearing cache".red(), e);
            ExitCode::from(1)
        }
    }
}

/// Show cache status and statistics
fn handle_cache_status() -> ExitCode {
    let project_root = get_project_root();
    let cache_path = project_root.join(".linthis").join("cache.json");

    if !cache_path.exists() {
        println!("{} No cache found", "ℹ".blue());
        println!("  Cache will be created on first run");
        return ExitCode::SUCCESS;
    }

    match LintCache::load(&project_root) {
        Ok(cache) => {
            println!("{}", "Cache Status".bold());
            println!("{}", "─".repeat(40));
            println!("  Location: {}", cache_path.display());

            // Get file size
            if let Ok(metadata) = std::fs::metadata(&cache_path) {
                let size_kb = metadata.len() as f64 / 1024.0;
                println!("  Size: {:.1} KB", size_kb);
            }

            println!("  Entries: {}", cache.total_entries());

            // Show per-checker breakdown
            if cache.total_entries() > 0 {
                println!();
                println!("  {}", "Cached files per checker:".dimmed());
                for (checker, entries) in cache.entries.iter() {
                    println!("    {}: {} files", checker, entries.len());
                }
            }

            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}: {}", "Error reading cache".red(), e);
            ExitCode::from(1)
        }
    }
}
