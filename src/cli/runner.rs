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
pub fn perform_self_update(self_update_config: Option<&linthis::self_update::SelfUpdateConfig>) -> bool {
    use linthis::self_update::{SelfUpdateConfig, SelfUpdateManager};

    // Use default config if none provided
    let default_config = SelfUpdateConfig::default();
    let config = self_update_config.unwrap_or(&default_config);

    // Validate config
    if let Err(e) = config.validate() {
        eprintln!("{}: Invalid self_auto_update config: {}", "Warning".yellow(), e);
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
    let latest = manager.get_latest_version().unwrap_or_else(|| "unknown".to_string());

    // Prompt user if needed
    if config.should_prompt()
        && !manager.prompt_user(&current, &latest) {
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

/// Perform auto-sync check and optionally sync plugins
/// Returns true if sync was performed, false otherwise
pub fn perform_auto_sync(auto_sync_config: Option<&linthis::plugin::AutoSyncConfig>) -> bool {
    use linthis::plugin::{AutoSyncConfig, AutoSyncManager, PluginConfigManager};

    // Use default config if none provided
    let default_config = AutoSyncConfig::default();
    let config = auto_sync_config.unwrap_or(&default_config);

    // Validate config
    if let Err(e) = config.validate() {
        eprintln!("{}: Invalid plugin_auto_sync config: {}", "Warning".yellow(), e);
        return false;
    }

    // Skip if disabled
    if config.is_disabled() {
        return false;
    }

    // Create auto-sync manager
    let manager = match AutoSyncManager::new() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}: Failed to create auto-sync manager: {}", "Warning".yellow(), e);
            return false;
        }
    };

    // Check if sync is needed
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

    // Collect all plugins from both project and global configs
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

    // If no plugins configured, just update timestamp and return
    if all_plugins.is_empty() {
        let _ = manager.update_last_sync_time();
        return false;
    }

    // Check if any plugins have updates
    let has_updates = check_plugins_for_updates(&all_plugins);

    // If no updates available, silently update timestamp and return
    if !has_updates {
        let _ = manager.update_last_sync_time();
        return false;
    }

    // Updates are available!
    // If prompt mode, ask user
    if config.should_prompt() {
        match manager.prompt_user() {
            Ok(true) => {
                // User confirmed, proceed
            }
            Ok(false) => {
                // User declined
                println!("Skipped plugin sync.");
                // Update timestamp to avoid prompting again immediately
                let _ = manager.update_last_sync_time();
                return false;
            }
            Err(e) => {
                eprintln!("{}: Failed to prompt user: {}", "Warning".yellow(), e);
                return false;
            }
        }
    }

    // Perform sync for both project and global configs
    let mut synced = false;

    // Try project config first
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

    // Try global config
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

    // Update last sync timestamp
    if synced {
        if let Err(e) = manager.update_last_sync_time() {
            eprintln!("{}: Failed to update sync timestamp: {}", "Warning".yellow(), e);
        }
    }

    synced
}
