// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Backup and restore functions shared by fix and format subcommands.
//!
//! Provides backup creation before destructive operations (fix, format),
//! undo (restore from backup), and backup listing.

use chrono::Local;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use linthis::utils::types::LintIssue;

/// Maximum number of backups to keep
const MAX_BACKUPS: usize = 5;

/// Backup manifest containing metadata about backed up files
#[derive(Debug, Serialize, Deserialize)]
pub struct BackupManifest {
    /// Timestamp when backup was created
    pub timestamp: String,
    /// List of files that were backed up (relative paths)
    pub files: Vec<String>,
    /// Description of the backup
    pub description: String,
}

/// Get the backup directory path
pub fn get_backup_dir() -> PathBuf {
    let project_root = linthis::utils::get_project_root();
    project_root.join(".linthis").join("backup")
}

/// Create a backup of files that will be modified
pub fn create_backup(files: &[PathBuf], description: &str, quiet: bool) -> Option<String> {
    if files.is_empty() {
        return None;
    }

    let backup_dir = get_backup_dir();
    let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_path = backup_dir.join(&timestamp);

    // Create backup directory
    if let Err(e) = fs::create_dir_all(&backup_path) {
        eprintln!(
            "{}: Failed to create backup directory: {}",
            "Warning".yellow(),
            e
        );
        return None;
    }

    let project_root = linthis::utils::get_project_root();
    let mut backed_up_files = Vec::new();

    // Copy each file to backup
    for file in files {
        // Get relative path from project root
        let rel_path = match file.strip_prefix(&project_root) {
            Ok(p) => p.to_path_buf(),
            Err(_) => file.clone(),
        };

        let backup_file_path = backup_path.join(&rel_path);

        // Create parent directories
        if let Some(parent) = backup_file_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!(
                    "{}: Failed to create directory {}: {}",
                    "Warning".yellow(),
                    parent.display(),
                    e
                );
                continue;
            }
        }

        // Skip directories and .linthis paths
        if !file.is_file() {
            continue;
        }
        if rel_path.components().any(|c| c.as_os_str() == ".linthis") {
            continue;
        }

        // Copy file
        if let Err(e) = fs::copy(file, &backup_file_path) {
            eprintln!(
                "{}: Failed to backup {}: {}",
                "Warning".yellow(),
                file.display(),
                e
            );
            continue;
        }
        backed_up_files.push(rel_path.to_string_lossy().to_string());
    }

    if backed_up_files.is_empty() {
        // No files backed up, remove empty directory
        let _ = fs::remove_dir_all(&backup_path);
        return None;
    }

    // Write manifest
    let manifest = BackupManifest {
        timestamp: timestamp.clone(),
        files: backed_up_files.clone(),
        description: description.to_string(),
    };

    let manifest_path = backup_path.join("manifest.json");
    if let Err(e) = fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap_or_default(),
    ) {
        eprintln!(
            "{}: Failed to write backup manifest: {}",
            "Warning".yellow(),
            e
        );
    }

    if !quiet {
        println!("{} Backup created: {}", "✓".green(), backup_path.display());
        println!(
            "  {} file{} backed up",
            backed_up_files.len(),
            if backed_up_files.len() == 1 { "" } else { "s" }
        );
    }

    // Clean up old backups
    cleanup_old_backups();

    Some(timestamp)
}

/// Clean up old backups, keeping only the most recent MAX_BACKUPS
pub fn cleanup_old_backups() {
    let backup_dir = get_backup_dir();
    if !backup_dir.exists() {
        return;
    }

    let mut backups: Vec<_> = match fs::read_dir(&backup_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.path())
            .collect(),
        Err(_) => return,
    };

    // Sort by name (which is timestamp, so newest last)
    backups.sort();

    // Remove oldest backups if we have too many
    while backups.len() > MAX_BACKUPS {
        if let Some(oldest) = backups.first() {
            let _ = fs::remove_dir_all(oldest);
            backups.remove(0);
        }
    }
}

/// List available backups
pub fn handle_list_backups(restore_cmd: &str) -> ExitCode {
    let backup_dir = get_backup_dir();

    if !backup_dir.exists() {
        println!("{} No backups found.", "→".cyan());
        println!("  Backups are created automatically when running fix/format commands.");
        return ExitCode::SUCCESS;
    }

    let mut backups: Vec<_> = match fs::read_dir(&backup_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.path())
            .collect(),
        Err(e) => {
            eprintln!("{}: Failed to read backup directory: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    if backups.is_empty() {
        println!("{} No backups found.", "→".cyan());
        return ExitCode::SUCCESS;
    }

    // Sort by name (newest last)
    backups.sort();
    backups.reverse(); // Show newest first

    println!("{} Available backups:", "→".cyan());
    println!();

    for (idx, backup_path) in backups.iter().enumerate() {
        let backup_name = backup_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let manifest_path = backup_path.join("manifest.json");

        let (file_count, description) = if manifest_path.exists() {
            match fs::read_to_string(&manifest_path) {
                Ok(content) => match serde_json::from_str::<BackupManifest>(&content) {
                    Ok(m) => (m.files.len(), m.description),
                    Err(_) => (0, String::new()),
                },
                Err(_) => (0, String::new()),
            }
        } else {
            (0, String::new())
        };

        let marker = if idx == 0 { "(latest)" } else { "" };
        println!(
            "  {} {} {} - {} file{}",
            format!("[{}]", idx + 1).cyan(),
            backup_name,
            marker.green(),
            file_count,
            if file_count == 1 { "" } else { "s" }
        );
        if !description.is_empty() {
            println!("      {}", description.dimmed());
        }
    }

    println!();
    println!(
        "To restore: {} or {} <backup-name>",
        format!("{} --undo", restore_cmd).cyan(),
        format!("{} --undo", restore_cmd).cyan()
    );

    ExitCode::SUCCESS
}

/// Resolve the backup path from a source identifier ("last" or a backup name).
fn resolve_backup_path(
    backup_dir: &std::path::Path,
    source: &str,
    list_cmd: &str,
) -> Result<PathBuf, ExitCode> {
    if source == "last" {
        let mut backups: Vec<_> = match fs::read_dir(backup_dir) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.path())
                .collect(),
            Err(e) => {
                eprintln!("{}: Failed to read backup directory: {}", "Error".red(), e);
                return Err(ExitCode::from(1));
            }
        };

        if backups.is_empty() {
            eprintln!("{}: No backups found.", "Error".red());
            return Err(ExitCode::from(1));
        }

        backups.sort();
        Ok(backups.pop().unwrap())
    } else {
        let path = backup_dir.join(source);
        if !path.exists() {
            eprintln!("{}: Backup not found: {}", "Error".red(), source);
            eprintln!("  Run {} to see available backups.", list_cmd.cyan());
            return Err(ExitCode::from(1));
        }
        Ok(path)
    }
}

/// Read and parse a backup manifest from the given backup path.
fn read_backup_manifest(backup_path: &std::path::Path) -> Result<BackupManifest, ExitCode> {
    let manifest_path = backup_path.join("manifest.json");
    if !manifest_path.exists() {
        eprintln!("{}: Backup manifest not found.", "Error".red());
        return Err(ExitCode::from(1));
    }
    let content = fs::read_to_string(&manifest_path).map_err(|e| {
        eprintln!("{}: Failed to read manifest: {}", "Error".red(), e);
        ExitCode::from(1)
    })?;
    serde_json::from_str(&content).map_err(|e| {
        eprintln!("{}: Failed to parse manifest: {}", "Error".red(), e);
        ExitCode::from(1)
    })
}

/// Restore individual files from backup, returning (restored_count, failed_count).
fn restore_backup_files(
    manifest: &BackupManifest,
    backup_path: &std::path::Path,
    project_root: &std::path::Path,
) -> (usize, usize) {
    let mut restored_count = 0;
    let mut failed_count = 0;

    for rel_path in &manifest.files {
        let backup_file = backup_path.join(rel_path);
        let target_file = project_root.join(rel_path);

        if !backup_file.exists() {
            eprintln!("  {} Missing backup file: {}", "⚠".yellow(), rel_path);
            failed_count += 1;
            continue;
        }

        if let Some(parent) = target_file.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!(
                    "  {} Failed to create directory for {}: {}",
                    "✗".red(),
                    rel_path,
                    e
                );
                failed_count += 1;
                continue;
            }
        }

        match fs::copy(&backup_file, &target_file) {
            Ok(_) => {
                println!("  {} Restored: {}", "✓".green(), rel_path);
                restored_count += 1;
            }
            Err(e) => {
                eprintln!("  {} Failed to restore {}: {}", "✗".red(), rel_path, e);
                failed_count += 1;
            }
        }
    }

    (restored_count, failed_count)
}

/// Restore files from a backup
pub fn handle_undo(source: &str, list_cmd: &str) -> ExitCode {
    let backup_dir = get_backup_dir();

    if !backup_dir.exists() {
        eprintln!("{}: No backups found.", "Error".red());
        eprintln!("  Run a fix or format command first to create a backup.");
        return ExitCode::from(1);
    }

    let backup_path = match resolve_backup_path(&backup_dir, source, list_cmd) {
        Ok(p) => p,
        Err(code) => return code,
    };

    let backup_name = backup_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    println!("{} Restoring from backup: {}", "→".cyan(), backup_name);

    let manifest = match read_backup_manifest(&backup_path) {
        Ok(m) => m,
        Err(code) => return code,
    };

    let project_root = linthis::utils::get_project_root();
    let (restored_count, failed_count) =
        restore_backup_files(&manifest, &backup_path, &project_root);

    println!();
    if failed_count == 0 {
        println!(
            "{} Restored {} file{} from backup {}",
            "✓".green().bold(),
            restored_count,
            if restored_count == 1 { "" } else { "s" },
            backup_name
        );
        ExitCode::SUCCESS
    } else {
        println!(
            "{} Restored {} file{}, {} failed",
            "⚠".yellow(),
            restored_count,
            if restored_count == 1 { "" } else { "s" },
            failed_count
        );
        ExitCode::from(1)
    }
}

/// Collect unique files from lint issues
pub fn collect_files_from_issues(issues: &[LintIssue]) -> Vec<PathBuf> {
    let mut files: HashSet<PathBuf> = HashSet::new();
    for issue in issues {
        files.insert(issue.file_path.clone());
    }
    files.into_iter().collect()
}
