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

/// Default maximum number of backups to keep (used when config is unavailable)
const DEFAULT_MAX_BACKUPS: usize = 5;

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

/// Clean up old backups, keeping only the most recent `max_backups`.
/// If `max_backups` is 0, no cleanup is performed (unlimited).
/// Always keeps at least 1 backup.
///
/// When over the limit, first removes backups with no actual file differences
/// (no-diff backups), then falls back to removing the oldest.
pub fn cleanup_old_backups_with_limit(max_backups: usize) {
    if max_backups == 0 {
        return; // unlimited
    }
    let keep = max_backups.max(1); // always keep at least 1

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

    if backups.len() <= keep {
        return;
    }

    // Sort by name (which is timestamp, so newest last)
    backups.sort();

    let project_root = linthis::utils::get_project_root();

    // First pass: remove no-diff backups (oldest first, never remove the newest)
    let mut i = 0;
    while backups.len() > keep && i < backups.len().saturating_sub(1) {
        if !backup_has_diff(&backups[i], &project_root) {
            let _ = fs::remove_dir_all(&backups[i]);
            backups.remove(i);
        } else {
            i += 1;
        }
    }

    // Second pass: if still over limit, remove oldest regardless
    while backups.len() > keep {
        if let Some(oldest) = backups.first() {
            let _ = fs::remove_dir_all(oldest);
            backups.remove(0);
        }
    }
}

/// Clean up old backups using config or default limit.
pub fn cleanup_old_backups() {
    let project_root = linthis::utils::get_project_root();
    let max = linthis::config::Config::load_project_config(&project_root)
        .map(|c| c.retention.backups)
        .unwrap_or(DEFAULT_MAX_BACKUPS);
    cleanup_old_backups_with_limit(max);
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

/// Restore files from a backup (legacy, use handle_undo_filtered instead)
#[allow(dead_code)]
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

// ═══════════════════════════════════════════════════════════════
// Unified backup / undo / redo commands
// ═══════════════════════════════════════════════════════════════

/// Handle `linthis backup create <files> -d <description>`
pub fn handle_backup_create(files: &[PathBuf], description: &str) -> ExitCode {
    if files.is_empty() {
        eprintln!("{}: No files specified.", "Error".red());
        return ExitCode::from(1);
    }
    match create_backup(files, description, false) {
        Some(_) => ExitCode::SUCCESS,
        None => {
            eprintln!("{}: No files were backed up.", "Warning".yellow());
            ExitCode::SUCCESS
        }
    }
}

/// Handle `linthis backup show <id>`
pub fn handle_backup_show(id: &str) -> ExitCode {
    let backup_dir = get_backup_dir();
    if !backup_dir.exists() {
        println!("No backups found.");
        return ExitCode::SUCCESS;
    }

    let backup_path = match resolve_backup_path(&backup_dir, id, "linthis backup list") {
        Ok(p) => p,
        Err(code) => return code,
    };

    let manifest = match read_backup_manifest(&backup_path) {
        Ok(m) => m,
        Err(code) => return code,
    };

    println!("{}", "Backup Details".bold());
    println!("  Timestamp:   {}", manifest.timestamp.cyan());
    println!("  Description: {}", manifest.description);
    println!("  Files ({}):", manifest.files.len());
    for f in &manifest.files {
        println!("    {}", f);
    }

    ExitCode::SUCCESS
}

/// Handle `linthis undo [filter]` — restore from matching backup with redo support.
pub fn handle_undo_filtered(filter: &str) -> ExitCode {
    let backup_dir = get_backup_dir();
    if !backup_dir.exists() {
        eprintln!("{}: No backups found.", "Error".red());
        return ExitCode::from(1);
    }

    // Find matching backup
    let backup_path = if matches!(filter, "format" | "fix" | "hook") {
        match find_latest_backup_by_type(&backup_dir, filter) {
            Some(p) => p,
            None => {
                eprintln!(
                    "{}: No '{}' backup found. Run {} to see available backups.",
                    "Error".red(),
                    filter,
                    "linthis backup list".cyan()
                );
                return ExitCode::from(1);
            }
        }
    } else {
        // "last" or specific timestamp
        match resolve_backup_path(&backup_dir, filter, "linthis backup list") {
            Ok(p) => p,
            Err(code) => return code,
        }
    };

    let manifest = match read_backup_manifest(&backup_path) {
        Ok(m) => m,
        Err(code) => return code,
    };

    println!(
        "{} Undoing: {} ({})",
        "←".cyan().bold(),
        manifest.description,
        manifest.timestamp
    );

    let project_root = linthis::utils::get_project_root();

    // Save current state to redo directory before restoring
    save_redo_state(&manifest, &project_root);

    // Restore from backup
    let (restored, failed) = restore_backup_files(&manifest, &backup_path, &project_root);

    if failed == 0 {
        println!(
            "{} Undone: {} file{} restored. Use {} to re-apply.",
            "✓".green(),
            restored,
            if restored == 1 { "" } else { "s" },
            "linthis redo".cyan()
        );
        ExitCode::SUCCESS
    } else {
        println!(
            "{} Restored {} file{}, {} failed",
            "⚠".yellow(),
            restored,
            if restored == 1 { "" } else { "s" },
            failed
        );
        ExitCode::from(1)
    }
}

/// Handle `linthis redo` — re-apply changes that were undone.
pub fn handle_redo() -> ExitCode {
    let redo_dir = get_redo_dir();
    if !redo_dir.exists() {
        eprintln!("{}: Nothing to redo.", "Error".red());
        return ExitCode::from(1);
    }

    let manifest = match read_backup_manifest(&redo_dir) {
        Ok(m) => m,
        Err(_) => {
            eprintln!("{}: No redo state found.", "Error".red());
            return ExitCode::from(1);
        }
    };

    println!(
        "{} Redoing: {} file{}",
        "→".cyan().bold(),
        manifest.files.len(),
        if manifest.files.len() == 1 { "" } else { "s" }
    );

    let project_root = linthis::utils::get_project_root();
    let (restored, failed) = restore_backup_files(&manifest, &redo_dir, &project_root);

    // Clear redo directory after restore
    let _ = fs::remove_dir_all(&redo_dir);

    if failed == 0 {
        println!(
            "{} Redone: {} file{} restored",
            "✓".green(),
            restored,
            if restored == 1 { "" } else { "s" }
        );
        ExitCode::SUCCESS
    } else {
        println!(
            "{} Restored {} file{}, {} failed",
            "⚠".yellow(),
            restored,
            if restored == 1 { "" } else { "s" },
            failed
        );
        ExitCode::from(1)
    }
}

/// Get the redo directory path.
fn get_redo_dir() -> PathBuf {
    let project_root = linthis::utils::get_project_root();
    project_root.join(".linthis").join("redo")
}

/// Save current file state to redo directory before undo.
fn save_redo_state(manifest: &BackupManifest, project_root: &std::path::Path) {
    let redo_dir = get_redo_dir();

    // Clear previous redo state
    let _ = fs::remove_dir_all(&redo_dir);
    if fs::create_dir_all(&redo_dir).is_err() {
        return;
    }

    let mut saved_files = Vec::new();

    for rel_path in &manifest.files {
        let source = project_root.join(rel_path);
        if !source.is_file() {
            continue;
        }

        let dest = redo_dir.join(rel_path);
        if let Some(parent) = dest.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::copy(&source, &dest).is_ok() {
            saved_files.push(rel_path.clone());
        }
    }

    // Write redo manifest
    let redo_manifest = BackupManifest {
        timestamp: chrono::Local::now().format("%Y%m%d-%H%M%S").to_string(),
        files: saved_files,
        description: format!("redo state for undo of '{}'", manifest.description),
    };
    let _ = fs::write(
        redo_dir.join("manifest.json"),
        serde_json::to_string_pretty(&redo_manifest).unwrap_or_default(),
    );
}

/// Find the latest backup matching a type filter (by description).
fn find_latest_backup_by_type(backup_dir: &std::path::Path, filter: &str) -> Option<PathBuf> {
    let mut backups: Vec<PathBuf> = fs::read_dir(backup_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .collect();

    // Sort newest first
    backups.sort();
    backups.reverse();

    for backup_path in backups {
        let manifest_path = backup_path.join("manifest.json");
        if let Ok(content) = fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = serde_json::from_str::<BackupManifest>(&content) {
                let desc_lower = manifest.description.to_lowercase();
                let matches = match filter {
                    "format" => desc_lower.contains("format"),
                    "fix" => desc_lower.contains("fix"),
                    "hook" => desc_lower.contains("hook"),
                    _ => false,
                };
                if matches {
                    return Some(backup_path);
                }
            }
        }
    }

    None
}

/// Check if a backup has any actual differences compared to current files.
fn backup_has_diff(backup_path: &std::path::Path, project_root: &std::path::Path) -> bool {
    let manifest = match read_backup_manifest(backup_path) {
        Ok(m) => m,
        Err(_) => return false,
    };

    for rel_path in &manifest.files {
        let backup_file = backup_path.join(rel_path);
        let current_file = project_root.join(rel_path);

        let backup_content = fs::read_to_string(&backup_file).unwrap_or_default();
        let current_content = fs::read_to_string(&current_file).unwrap_or_default();

        if backup_content != current_content {
            return true;
        }
    }

    false
}

/// Handle `linthis backup diff [id]` — show diff between backup and current files.
/// When id is "last", automatically skips backups with no changes and finds
/// the most recent backup that has actual differences.
pub fn handle_backup_diff(id: &str) -> ExitCode {
    let backup_dir = get_backup_dir();
    if !backup_dir.exists() {
        eprintln!("{}: No backups found.", "Error".red());
        return ExitCode::from(1);
    }

    let project_root = linthis::utils::get_project_root();

    // When "last", find the most recent backup with actual changes
    let backup_path = if id == "last" {
        match find_latest_backup_with_diff(&backup_dir, &project_root) {
            Some(p) => p,
            None => {
                println!(
                    "  {}",
                    "No backups with differences found.".green()
                );
                return ExitCode::SUCCESS;
            }
        }
    } else {
        match resolve_backup_path(&backup_dir, id, "linthis backup list") {
            Ok(p) => p,
            Err(code) => return code,
        }
    };

    let manifest = match read_backup_manifest(&backup_path) {
        Ok(m) => m,
        Err(code) => return code,
    };

    println!(
        "📊 Diff: {} ({})",
        manifest.description,
        manifest.timestamp.cyan()
    );
    println!();

    let mut has_diff = false;

    for rel_path in &manifest.files {
        let backup_file = backup_path.join(rel_path);
        let current_file = project_root.join(rel_path);

        let backup_content = fs::read_to_string(&backup_file).unwrap_or_default();
        let current_content = fs::read_to_string(&current_file).unwrap_or_default();

        if backup_content == current_content {
            continue;
        }

        has_diff = true;
        print_unified_diff(
            &backup_content,
            &current_content,
            &format!("a/{} (backup)", rel_path),
            &format!("b/{} (current)", rel_path),
        );
        println!();
    }

    if !has_diff {
        println!(
            "  {}",
            "No differences — current files match backup.".green()
        );
    }

    ExitCode::SUCCESS
}

/// Find the most recent backup that has actual file differences.
/// Walks backwards through sorted backups until one with a diff is found.
fn find_latest_backup_with_diff(
    backup_dir: &std::path::Path,
    project_root: &std::path::Path,
) -> Option<PathBuf> {
    let mut backups: Vec<PathBuf> = fs::read_dir(backup_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .collect();

    if backups.is_empty() {
        return None;
    }

    // Sort ascending by name (timestamp-based), then iterate from newest
    backups.sort();

    for backup_path in backups.iter().rev() {
        if backup_has_diff(backup_path, project_root) {
            return Some(backup_path.clone());
        }
    }

    None
}

/// Print a unified diff with context lines (like `git diff`).
fn print_unified_diff(old_content: &str, new_content: &str, old_label: &str, new_label: &str) {
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(old_content, new_content);
    let mut has_output = false;

    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        if !has_output {
            println!("{}", format!("--- {}", old_label).red());
            println!("{}", format!("+++ {}", new_label).green());
            has_output = true;
        }
        println!("{}", format!("{}", hunk.header()).cyan());
        for change in hunk.iter_changes() {
            match change.tag() {
                ChangeTag::Delete => {
                    print!("{}", format!("-{}", change).red());
                }
                ChangeTag::Insert => {
                    print!("{}", format!("+{}", change).green());
                }
                ChangeTag::Equal => {
                    print!(" {}", change);
                }
            }
        }
    }
}

/// Handle `linthis backup` subcommand dispatch.
pub fn handle_backup_command(action: super::commands::BackupCommands) -> ExitCode {
    use super::commands::BackupCommands;
    match action {
        BackupCommands::Create { files, description } => handle_backup_create(&files, &description),
        BackupCommands::List => handle_list_backups("linthis backup list"),
        BackupCommands::Show { id } => handle_backup_show(&id),
        BackupCommands::Diff { id } => handle_backup_diff(&id),
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
