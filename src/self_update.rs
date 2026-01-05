// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Self-update functionality for linthis itself.
//!
//! Provides automatic update checking and installation via pip,
//! inspired by oh-my-zsh's auto-update mechanism.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Configuration for self-update
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelfUpdateConfig {
    /// Enable/disable self-update checks
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Update mode: "auto", "prompt", or "disabled"
    #[serde(default = "default_mode")]
    pub mode: String,

    /// Check for updates every N days
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

impl Default for SelfUpdateConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            mode: default_mode(),
            interval_days: default_interval_days(),
        }
    }
}

impl SelfUpdateConfig {
    /// Check if auto-update is disabled
    pub fn is_disabled(&self) -> bool {
        !self.enabled || self.mode == "disabled"
    }

    /// Check if should prompt user before updating
    pub fn should_prompt(&self) -> bool {
        self.mode == "prompt"
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if !["auto", "prompt", "disabled"].contains(&self.mode.as_str()) {
            return Err(format!(
                "Invalid mode '{}'. Must be 'auto', 'prompt', or 'disabled'",
                self.mode
            ));
        }

        if self.interval_days == 0 {
            return Err("interval_days must be greater than 0".to_string());
        }

        Ok(())
    }
}

/// Manages self-update timing and execution
#[derive(Debug)]
pub struct SelfUpdateManager {
    timestamp_file: PathBuf,
}

impl Default for SelfUpdateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SelfUpdateManager {
    /// Create a new self-update manager
    pub fn new() -> Self {
        let home_dir = Self::get_home_dir().expect("Failed to get home directory");
        let linthis_dir = home_dir.join(".linthis");
        let timestamp_file = linthis_dir.join(".self_update_last_check");

        Self { timestamp_file }
    }

    /// Get home directory
    fn get_home_dir() -> Option<PathBuf> {
        std::env::var("HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
    }

    /// Check if it's time to check for updates
    pub fn should_check(&self, interval_days: u64) -> bool {
        match self.get_last_check_time() {
            Some(last_check) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let days_since_check = (now - last_check) / 86400; // 86400 seconds in a day
                days_since_check >= interval_days
            }
            None => true, // Never checked before
        }
    }

    /// Get the last check timestamp
    pub fn get_last_check_time(&self) -> Option<u64> {
        fs::read_to_string(&self.timestamp_file)
            .ok()
            .and_then(|content| content.trim().parse::<u64>().ok())
    }

    /// Update the last check timestamp to current time
    pub fn update_last_check_time(&self) -> io::Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.timestamp_file.parent() {
            fs::create_dir_all(parent)?;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        fs::write(&self.timestamp_file, now.to_string())
    }

    /// Get current linthis version
    pub fn get_current_version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// Check PyPI for the latest version
    pub fn get_latest_version(&self) -> Option<String> {
        // Use pip to check the latest version
        let output = Command::new("pip")
            .args(["index", "versions", "linthis"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse output to find available versions
        // Expected format: "linthis (0.0.4)"
        // Available versions: 0.0.4, 0.0.3, ...
        for line in stdout.lines() {
            if line.contains("Available versions:") {
                // Extract first version (latest)
                if let Some(versions_str) = line.split(':').nth(1) {
                    if let Some(latest) = versions_str.split(',').next() {
                        return Some(latest.trim().to_string());
                    }
                }
            }
        }

        None
    }

    /// Check if an update is available
    pub fn has_update(&self) -> bool {
        let current = self.get_current_version();

        match self.get_latest_version() {
            Some(latest) => {
                // Simple version comparison
                self.compare_versions(&current, &latest) < 0
            }
            None => false,
        }
    }

    /// Compare two version strings (simple lexicographic comparison)
    /// Returns: -1 if v1 < v2, 0 if equal, 1 if v1 > v2
    fn compare_versions(&self, v1: &str, v2: &str) -> i32 {
        let parts1: Vec<u32> = v1.split('.').filter_map(|s| s.parse().ok()).collect();
        let parts2: Vec<u32> = v2.split('.').filter_map(|s| s.parse().ok()).collect();

        for i in 0..parts1.len().max(parts2.len()) {
            let p1 = parts1.get(i).unwrap_or(&0);
            let p2 = parts2.get(i).unwrap_or(&0);

            if p1 < p2 {
                return -1;
            } else if p1 > p2 {
                return 1;
            }
        }

        0
    }

    /// Prompt user for confirmation
    pub fn prompt_user(&self, current: &str, latest: &str) -> bool {
        print!(
            "A new version of linthis is available: {} → {}. Update now? [Y/n]: ",
            current, latest
        );
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let response = input.trim().to_lowercase();
        response.is_empty() || response == "y" || response == "yes"
    }

    /// Execute pip upgrade
    pub fn upgrade(&self) -> io::Result<bool> {
        println!("↓ Upgrading linthis via pip...");

        let output = Command::new("pip")
            .args(["install", "--upgrade", "linthis"])
            .output()?;

        if output.status.success() {
            println!("✓ linthis upgraded successfully");
            Ok(true)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("✗ Failed to upgrade linthis: {}", stderr);
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_update_config_default() {
        let config = SelfUpdateConfig::default();
        assert!(config.enabled);
        assert_eq!(config.mode, "prompt");
        assert_eq!(config.interval_days, 7);
    }

    #[test]
    fn test_self_update_config_is_disabled() {
        let mut config = SelfUpdateConfig::default();
        assert!(!config.is_disabled());

        config.enabled = false;
        assert!(config.is_disabled());

        config.enabled = true;
        config.mode = "disabled".to_string();
        assert!(config.is_disabled());
    }

    #[test]
    fn test_self_update_config_should_prompt() {
        let mut config = SelfUpdateConfig::default();
        assert!(config.should_prompt());

        config.mode = "auto".to_string();
        assert!(!config.should_prompt());

        config.mode = "disabled".to_string();
        assert!(!config.should_prompt());
    }

    #[test]
    fn test_self_update_config_validate() {
        let config = SelfUpdateConfig::default();
        assert!(config.validate().is_ok());

        let mut bad_config = config.clone();
        bad_config.mode = "invalid".to_string();
        assert!(bad_config.validate().is_err());

        let mut bad_config2 = config.clone();
        bad_config2.interval_days = 0;
        assert!(bad_config2.validate().is_err());
    }

    #[test]
    fn test_version_comparison() {
        let manager = SelfUpdateManager::new();

        assert_eq!(manager.compare_versions("0.0.1", "0.0.2"), -1);
        assert_eq!(manager.compare_versions("0.0.2", "0.0.1"), 1);
        assert_eq!(manager.compare_versions("0.0.1", "0.0.1"), 0);
        assert_eq!(manager.compare_versions("1.0.0", "0.9.9"), 1);
        assert_eq!(manager.compare_versions("0.0.10", "0.0.9"), 1);
    }

    #[test]
    fn test_get_current_version() {
        let manager = SelfUpdateManager::new();
        let version = manager.get_current_version();
        assert!(!version.is_empty());
        // Should match CARGO_PKG_VERSION
        assert_eq!(version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_should_check_never_checked() {
        let manager = SelfUpdateManager::new();
        // Clean up any existing timestamp file from previous runs
        let _ = fs::remove_file(&manager.timestamp_file);

        // If never checked, should always return true
        assert!(manager.should_check(7));
    }

    #[test]
    fn test_update_and_get_last_check_time() {
        let manager = SelfUpdateManager::new();

        // Clean up any existing timestamp file from previous runs
        let _ = fs::remove_file(&manager.timestamp_file);

        // Update timestamp
        let result = manager.update_last_check_time();
        assert!(result.is_ok());

        // Should be able to read it back
        let timestamp = manager.get_last_check_time();
        assert!(timestamp.is_some());

        // Should be a recent timestamp (within last minute)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let last_check = timestamp.unwrap();
        assert!(now - last_check < 60);

        // Clean up after test
        let _ = fs::remove_file(&manager.timestamp_file);
    }
}
