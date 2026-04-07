// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Auto-update and auto-sync runners.
//!
//! This module provides functions for automatic self-update checks
//! and plugin synchronization.

use colored::Colorize;

use super::plugin::{check_plugins_for_updates, sync_plugins};

/// Perform self-update check and optionally upgrade linthis itself
/// Returns true if update was performed, false otherwise
pub fn perform_self_update(
    self_update_config: Option<&linthis::self_update::SelfUpdateConfig>,
) -> bool {
    use linthis::self_update::{SelfUpdateConfig, SelfUpdateManager};

    // Use default config if none provided
    let default_config = SelfUpdateConfig::default();
    let config = self_update_config.unwrap_or(&default_config);

    // Validate config
    if let Err(e) = config.validate() {
        eprintln!(
            "{}: Invalid self_auto_update config: {}",
            "Warning".yellow(),
            e
        );
        return false;
    }

    // Check if disabled
    if config.is_disabled() {
        return false;
    }

    let manager = SelfUpdateManager::new();

    // Check if it's time to check for updates
    if !manager.should_check(config.interval_days) {
        return false;
    }

    // Check if update is available
    if !manager.has_update() {
        // Update timestamp even when no updates available
        let _ = manager.update_last_check_time();
        return false;
    }

    let current = manager.get_current_version();
    let latest = manager
        .get_latest_version()
        .unwrap_or_else(|| "unknown".to_string());

    // Prompt user if needed
    if config.should_prompt() && !manager.prompt_user(&current, &latest) {
        // User declined, update timestamp to avoid repeated prompts
        let _ = manager.update_last_check_time();
        return false;
    }

    // Perform upgrade
    match manager.upgrade() {
        Ok(success) => {
            if success {
                let _ = manager.update_last_check_time();
            }
            success
        }
        Err(e) => {
            eprintln!("{}: Failed to upgrade linthis: {}", "Error".red(), e);
            false
        }
    }
}

/// Handle the `linthis update` / `linthis upgrade` subcommand.
/// Returns ExitCode::SUCCESS on success, ExitCode::FAILURE on error.
pub fn handle_self_update_command(
    check: bool,
    force: bool,
    target_version: Option<String>,
) -> std::process::ExitCode {
    use linthis::self_update::{detect_install_method, SelfUpdateManager};

    let manager = SelfUpdateManager::new();
    let current = manager.get_current_version();
    let method = detect_install_method();

    println!(
        "linthis {} (installed via {})",
        current.bold(),
        method.to_string().cyan()
    );

    if let Some(ref version) = target_version {
        handle_install_specific_version(&manager, &current, version)
    } else {
        handle_update_to_latest(&manager, &current, check, force)
    }
}

/// Install a specific version of linthis.
fn handle_install_specific_version(
    manager: &linthis::self_update::SelfUpdateManager,
    current: &str,
    version: &str,
) -> std::process::ExitCode {
    use std::process::ExitCode;

    println!(
        "Installing version: {} → {}",
        current.yellow(),
        version.cyan()
    );

    match manager.install_version(version) {
        Ok(true) => {
            println!(
                "{}: auto-update may override this version. To disable: {}",
                "Note".yellow(),
                "linthis config set self_auto_update.mode disabled".cyan()
            );
            let _ = manager.update_last_check_time();
            ExitCode::SUCCESS
        }
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            ExitCode::FAILURE
        }
    }
}

/// Check for updates and optionally upgrade to the latest version.
fn handle_update_to_latest(
    manager: &linthis::self_update::SelfUpdateManager,
    current: &str,
    check: bool,
    force: bool,
) -> std::process::ExitCode {
    use std::process::ExitCode;

    println!("Checking for updates...");
    let latest = match manager.get_latest_version() {
        Some(v) => v,
        None => {
            eprintln!("{}: Failed to check for latest version", "Error".red());
            return ExitCode::FAILURE;
        }
    };

    let has_update = manager.compare_versions(current, &latest) < 0;

    if check {
        if has_update {
            println!(
                "Update available: {} → {}",
                current.yellow(),
                latest.green()
            );
        } else {
            println!("{}", "Already on the latest version.".green());
        }
        let _ = manager.update_last_check_time();
        return ExitCode::SUCCESS;
    }

    if !has_update && !force {
        println!("{}", "Already on the latest version.".green());
        let _ = manager.update_last_check_time();
        return ExitCode::SUCCESS;
    }

    if has_update {
        println!("Updating: {} → {}", current.yellow(), latest.green());
    } else {
        println!("Force reinstalling version {}...", current);
    }

    let result = if force {
        manager.force_upgrade()
    } else {
        manager.upgrade()
    };

    match result {
        Ok(true) => {
            let _ = manager.update_last_check_time();
            ExitCode::SUCCESS
        }
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            ExitCode::FAILURE
        }
    }
}

/// Collect all plugins from both project and global configs.
fn collect_all_plugins() -> Vec<(String, String, Option<String>)> {
    use linthis::plugin::PluginConfigManager;

    let mut all_plugins = Vec::new();

    if let Ok(project_manager) = PluginConfigManager::project() {
        if let Ok(plugins) = project_manager.list_plugins() {
            all_plugins.extend(plugins);
        }
    }

    if let Ok(global_manager) = PluginConfigManager::global() {
        if let Ok(plugins) = global_manager.list_plugins() {
            all_plugins.extend(plugins);
        }
    }

    all_plugins
}

/// Sync plugins from both project and global configs.
/// Returns true if any sync succeeded.
fn sync_all_plugin_configs() -> bool {
    use linthis::plugin::PluginConfigManager;

    let mut synced = false;

    if let Ok(project_manager) = PluginConfigManager::project() {
        if let Ok(plugins) = project_manager.list_plugins() {
            if !plugins.is_empty() {
                println!("{} Syncing project plugins...", "↓".cyan());
                if sync_plugins(&plugins).is_ok() {
                    synced = true;
                }
            }
        }
    }

    if let Ok(global_manager) = PluginConfigManager::global() {
        if let Ok(plugins) = global_manager.list_plugins() {
            if !plugins.is_empty() {
                println!("{} Syncing global plugins...", "↓".cyan());
                if sync_plugins(&plugins).is_ok() {
                    synced = true;
                }
            }
        }
    }

    synced
}

/// Prompt the user for sync confirmation if required by config.
/// Returns false if user declined or prompt failed.
fn confirm_sync_prompt(
    config: &linthis::plugin::AutoSyncConfig,
    manager: &linthis::plugin::AutoSyncManager,
) -> bool {
    if !config.should_prompt() {
        return true;
    }
    match manager.prompt_user() {
        Ok(true) => true,
        Ok(false) => {
            println!("Skipped plugin sync.");
            let _ = manager.update_last_sync_time();
            false
        }
        Err(e) => {
            eprintln!("{}: Failed to prompt user: {}", "Warning".yellow(), e);
            false
        }
    }
}

/// Perform auto-sync check and optionally sync plugins
/// Returns true if sync was performed, false otherwise
pub fn perform_auto_sync(auto_sync_config: Option<&linthis::plugin::AutoSyncConfig>) -> bool {
    use linthis::plugin::{AutoSyncConfig, AutoSyncManager};

    let default_config = AutoSyncConfig::default();
    let config = auto_sync_config.unwrap_or(&default_config);

    if let Err(e) = config.validate() {
        eprintln!(
            "{}: Invalid plugin_auto_sync config: {}",
            "Warning".yellow(),
            e
        );
        return false;
    }

    if config.is_disabled() {
        return false;
    }

    let manager = match AutoSyncManager::new() {
        Ok(m) => m,
        Err(e) => {
            eprintln!(
                "{}: Failed to create auto-sync manager: {}",
                "Warning".yellow(),
                e
            );
            return false;
        }
    };

    let should_sync = match manager.should_sync(config) {
        Ok(should) => should,
        Err(e) => {
            eprintln!("{}: Failed to check sync status: {}", "Warning".yellow(), e);
            return false;
        }
    };

    if !should_sync {
        return false;
    }

    let all_plugins = collect_all_plugins();
    if all_plugins.is_empty() {
        let _ = manager.update_last_sync_time();
        return false;
    }

    if !check_plugins_for_updates(&all_plugins) {
        let _ = manager.update_last_sync_time();
        return false;
    }

    if !confirm_sync_prompt(config, &manager) {
        return false;
    }

    let synced = sync_all_plugin_configs();

    if synced {
        if let Err(e) = manager.update_last_sync_time() {
            eprintln!(
                "{}: Failed to update sync timestamp: {}",
                "Warning".yellow(),
                e
            );
        }
    }

    synced
}
