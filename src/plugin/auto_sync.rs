// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Auto-sync functionality for plugin updates.
//!
//! This module provides automatic synchronization of plugins similar to oh-my-zsh's
//! auto-update mechanism. Features include:
//! - Configurable sync intervals
//! - Multiple sync modes: auto, prompt, disabled
//! - Timestamp-based tracking to avoid excessive syncs

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Auto-sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSyncConfig {
    /// Whether auto-sync is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Sync mode: "auto", "prompt", or "disabled"
    #[serde(default = "default_mode")]
    pub mode: String,

    /// Sync interval in days
    #[serde(default = "default_interval_days")]
    pub interval_days: u64,
}

fn default_enabled() -> bool {
    true
}

fn default_mode() -> String {
    "prompt".to_string()
}

fn default_interval_days() -> u64 {
    7
}

impl Default for AutoSyncConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            mode: default_mode(),
            interval_days: default_interval_days(),
        }
    }
}

impl AutoSyncConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if !["auto", "prompt", "disabled"].contains(&self.mode.as_str()) {
            anyhow::bail!(
                "Invalid auto_sync.mode '{}'. Must be one of: auto, prompt, disabled",
                self.mode
            );
        }
        if self.interval_days == 0 {
            anyhow::bail!("auto_sync.interval_days must be greater than 0");
        }
        Ok(())
    }

    /// Check if auto-sync should be disabled
    pub fn is_disabled(&self) -> bool {
        !self.enabled || self.mode == "disabled"
    }

    /// Check if auto-sync should prompt the user
    pub fn should_prompt(&self) -> bool {
        self.mode == "prompt"
    }
}

/// Manages auto-sync state and timestamp tracking
pub struct AutoSyncManager {
    timestamp_file: PathBuf,
}

impl AutoSyncManager {
    /// Create a new AutoSyncManager with default timestamp file location
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(PathBuf::from)
            .context("Cannot determine home directory")?;

        let linthis_dir = home.join(".linthis");
        let timestamp_file = linthis_dir.join(".plugin_sync_last_check");

        Ok(Self { timestamp_file })
    }

    /// Get the path to the timestamp file
    pub fn timestamp_file_path(&self) -> &PathBuf {
        &self.timestamp_file
    }

    /// Get the last sync timestamp in seconds since UNIX epoch
    pub fn get_last_sync_time(&self) -> Result<Option<u64>> {
        if !self.timestamp_file.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&self.timestamp_file)
            .context("Failed to read last sync timestamp")?;

        let timestamp = content
            .trim()
            .parse::<u64>()
            .context("Invalid timestamp format in .plugin_sync_last_check file")?;

        Ok(Some(timestamp))
    }

    /// Update the last sync timestamp to current time
    pub fn update_last_sync_time(&self) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.timestamp_file.parent() {
            fs::create_dir_all(parent).context("Failed to create .linthis directory")?;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("System time is before UNIX epoch")?
            .as_secs();

        fs::write(&self.timestamp_file, now.to_string())
            .context("Failed to write last sync timestamp")?;

        Ok(())
    }

    /// Get current time in seconds since UNIX epoch
    fn current_time() -> Result<u64> {
        Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("System time is before UNIX epoch")?
            .as_secs())
    }

    /// Check if sync is needed based on interval
    pub fn should_sync(&self, config: &AutoSyncConfig) -> Result<bool> {
        if config.is_disabled() {
            return Ok(false);
        }

        let last_sync = match self.get_last_sync_time()? {
            Some(time) => time,
            None => {
                // Never synced before, should sync
                return Ok(true);
            }
        };

        let now = Self::current_time()?;
        let interval_seconds = config.interval_days * 24 * 60 * 60;
        let elapsed = now.saturating_sub(last_sync);

        Ok(elapsed >= interval_seconds)
    }

    /// Prompt user for confirmation to sync
    pub fn prompt_user(&self) -> Result<bool> {
        print!("Updates available for plugins. Update now? [Y/n]: ");
        io::stdout().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        let response = response.trim().to_lowercase();

        // Default to yes if empty or "y"/"yes"
        Ok(response.is_empty() || response == "y" || response == "yes")
    }

    /// Get human-readable time since last sync
    pub fn time_since_last_sync(&self) -> Result<Option<String>> {
        let last_sync = match self.get_last_sync_time()? {
            Some(time) => time,
            None => return Ok(None),
        };

        let now = Self::current_time()?;
        let elapsed = now.saturating_sub(last_sync);

        let days = elapsed / (24 * 60 * 60);
        let hours = (elapsed % (24 * 60 * 60)) / (60 * 60);

        let description = if days > 0 {
            format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
        } else if hours > 0 {
            format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
        } else {
            "less than an hour ago".to_string()
        };

        Ok(Some(description))
    }
}

impl Default for AutoSyncManager {
    fn default() -> Self {
        Self::new().expect("Failed to create AutoSyncManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_temp_manager() -> (AutoSyncManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let timestamp_file = temp_dir.path().join(".plugin_sync_last_check");
        let manager = AutoSyncManager { timestamp_file };
        (manager, temp_dir)
    }

    #[test]
    fn test_auto_sync_config_default() {
        let config = AutoSyncConfig::default();
        assert!(config.enabled);
        assert_eq!(config.mode, "prompt");
        assert_eq!(config.interval_days, 7);
    }

    #[test]
    fn test_auto_sync_config_validate() {
        let mut config = AutoSyncConfig::default();
        assert!(config.validate().is_ok());

        config.mode = "invalid".to_string();
        assert!(config.validate().is_err());

        config.mode = "auto".to_string();
        assert!(config.validate().is_ok());

        config.interval_days = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_auto_sync_config_is_disabled() {
        let mut config = AutoSyncConfig::default();
        assert!(!config.is_disabled());

        config.enabled = false;
        assert!(config.is_disabled());

        config.enabled = true;
        config.mode = "disabled".to_string();
        assert!(config.is_disabled());
    }

    #[test]
    fn test_auto_sync_config_should_prompt() {
        let mut config = AutoSyncConfig::default();
        assert!(config.should_prompt());

        config.mode = "auto".to_string();
        assert!(!config.should_prompt());
    }

    #[test]
    fn test_get_last_sync_time_none() {
        let (manager, _temp) = create_temp_manager();
        let result = manager.get_last_sync_time().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_update_and_get_last_sync_time() {
        let (manager, _temp) = create_temp_manager();

        // Update timestamp
        manager.update_last_sync_time().unwrap();

        // Verify it was written
        let result = manager.get_last_sync_time().unwrap();
        assert!(result.is_some());

        let timestamp = result.unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Should be very close to current time (within 1 second)
        assert!((now - timestamp) < 1);
    }

    #[test]
    fn test_should_sync_never_synced() {
        let (manager, _temp) = create_temp_manager();
        let config = AutoSyncConfig::default();

        // Should sync if never synced before
        assert!(manager.should_sync(&config).unwrap());
    }

    #[test]
    fn test_should_sync_disabled() {
        let (manager, _temp) = create_temp_manager();
        let mut config = AutoSyncConfig::default();
        config.enabled = false;

        // Should not sync if disabled
        assert!(!manager.should_sync(&config).unwrap());
    }

    #[test]
    fn test_should_sync_interval() {
        let (manager, _temp) = create_temp_manager();
        let config = AutoSyncConfig {
            enabled: true,
            mode: "auto".to_string(),
            interval_days: 7,
        };

        // Update timestamp to now
        manager.update_last_sync_time().unwrap();

        // Should not sync immediately after
        assert!(!manager.should_sync(&config).unwrap());

        // Manually set old timestamp
        let old_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - (8 * 24 * 60 * 60); // 8 days ago

        fs::write(&manager.timestamp_file, old_time.to_string()).unwrap();

        // Should sync now
        assert!(manager.should_sync(&config).unwrap());
    }

    #[test]
    fn test_time_since_last_sync() {
        let (manager, _temp) = create_temp_manager();

        // No last sync
        assert!(manager.time_since_last_sync().unwrap().is_none());

        // Set timestamp to 2 days ago
        let two_days_ago = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - (2 * 24 * 60 * 60);

        fs::write(&manager.timestamp_file, two_days_ago.to_string()).unwrap();

        let time_str = manager.time_since_last_sync().unwrap().unwrap();
        assert!(time_str.contains("2 days"));
    }
}
