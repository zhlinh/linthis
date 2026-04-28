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
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// How linthis was installed — determines the upgrade command.
#[derive(Debug, Clone, PartialEq)]
pub enum InstallMethod {
    /// Installed via `cargo install linthis` (or `cargo install --path .`)
    Cargo,
    /// Installed via `brew install linthis`
    Homebrew,
    /// Installed via `uv tool install linthis`
    UvTool,
    /// Installed via `pipx install linthis`
    PipX,
    /// Installed via `pip install linthis` (or `uv pip install linthis`)
    Pip,
    /// Unknown installation method
    Unknown,
}

impl fmt::Display for InstallMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstallMethod::Cargo => write!(f, "cargo install"),
            InstallMethod::Homebrew => write!(f, "brew install"),
            InstallMethod::UvTool => write!(f, "uv tool install"),
            InstallMethod::PipX => write!(f, "pipx install"),
            InstallMethod::Pip => write!(f, "pip install"),
            InstallMethod::Unknown => write!(f, "unknown"),
        }
    }
}

/// Detect how linthis was installed by inspecting the current executable path.
pub fn detect_install_method() -> InstallMethod {
    let exe_path = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return InstallMethod::Unknown,
    };

    let path_str = exe_path.to_string_lossy();

    // cargo install: ~/.cargo/bin/linthis
    if path_str.contains(".cargo/bin") || path_str.contains("\\.cargo\\bin") {
        return InstallMethod::Cargo;
    }

    // brew install:
    //   macOS Apple Silicon: /opt/homebrew/Cellar/ or /opt/homebrew/bin/
    //   macOS Intel:         /usr/local/Cellar/ or /usr/local/bin/
    //   Linux (linuxbrew):   /home/linuxbrew/.linuxbrew/
    if path_str.contains("/opt/homebrew/")
        || path_str.contains("/usr/local/Cellar/")
        || path_str.contains("/home/linuxbrew/")
    {
        return InstallMethod::Homebrew;
    }

    // uv tool install:
    //   Linux: ~/.local/share/uv/tools/
    //   macOS: ~/Library/Application Support/uv/tools/
    //   Windows: %APPDATA%\uv\tools\
    if path_str.contains("/uv/tools/")
        || path_str.contains("\\uv\\tools\\")
        || path_str.contains("Application Support/uv/tools")
    {
        return InstallMethod::UvTool;
    }

    // pipx install: ~/.local/pipx/venvs/
    if path_str.contains(".local/pipx/venvs") || path_str.contains("\\pipx\\venvs") {
        return InstallMethod::PipX;
    }

    // Fallback: assume pip-based installation for any other Python-related path
    // (.local/bin, site-packages, venv, etc.)
    InstallMethod::Pip
}

/// Configuration for self-update
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelfUpdateConfig {
    /// Enable/disable self-update checks
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Update mode: "auto", "prompt", or "disabled"
    #[serde(default = "default_mode")]
    pub mode: String,

    /// Check for updates every N days (supports decimals, e.g. 0.5 = 12 hours; 0 = disabled)
    #[serde(default = "default_interval_days")]
    pub interval_days: f64,
}

fn default_enabled() -> bool {
    true
}

fn default_mode() -> String {
    "prompt".to_string()
}

fn default_interval_days() -> f64 {
    7.0
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

        if self.interval_days < 0.0 {
            return Err("interval_days must be >= 0 (0 means disabled)".to_string());
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
        let home_dir = crate::utils::home_dir().expect("Failed to get home directory");
        let linthis_dir = home_dir.join(".linthis");
        let timestamp_file = linthis_dir.join(".self_update_last_check");

        Self { timestamp_file }
    }

    /// Check if it's time to check for updates.
    /// Returns false if interval_days is 0 (disabled).
    pub fn should_check(&self, interval_days: f64) -> bool {
        if interval_days <= 0.0 {
            return false; // 0 = disabled
        }
        match self.get_last_check_time() {
            Some(last_check) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let elapsed_secs = now.saturating_sub(last_check);
                let interval_secs = (interval_days * 86400.0) as u64;
                elapsed_secs >= interval_secs
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

    /// Get the latest version, dispatching to the appropriate source
    /// based on the install method.
    pub fn get_latest_version(&self) -> Option<String> {
        let method = detect_install_method();
        match method {
            InstallMethod::Cargo | InstallMethod::Homebrew => self.get_latest_version_crates_io(),
            _ => self.get_latest_version_pypi(),
        }
    }

    /// Check PyPI for the latest version via pip.
    pub fn get_latest_version_pypi(&self) -> Option<String> {
        let output = Command::new("pip")
            .args(["index", "versions", "linthis"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse output: "Available versions: 0.17.1, 0.17.0, ..."
        for line in stdout.lines() {
            if line.contains("Available versions:") {
                if let Some(versions_str) = line.split(':').nth(1) {
                    if let Some(latest) = versions_str.split(',').next() {
                        return Some(latest.trim().to_string());
                    }
                }
            }
        }

        None
    }

    /// Check crates.io for the latest version via API.
    pub fn get_latest_version_crates_io(&self) -> Option<String> {
        #[derive(Deserialize)]
        struct CrateResponse {
            #[serde(rename = "crate")]
            krate: CrateInfo,
        }
        #[derive(Deserialize)]
        struct CrateInfo {
            max_version: String,
        }

        let client = reqwest::blocking::Client::builder()
            .user_agent("linthis-self-update")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .ok()?;

        let resp: CrateResponse = client
            .get("https://crates.io/api/v1/crates/linthis")
            .send()
            .ok()?
            .json()
            .ok()?;

        Some(resp.krate.max_version)
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
    pub fn compare_versions(&self, v1: &str, v2: &str) -> i32 {
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

    /// Execute upgrade using the detected install method.
    pub fn upgrade(&self) -> io::Result<bool> {
        self.run_upgrade(false)
    }

    /// Execute force upgrade using the detected install method.
    pub fn force_upgrade(&self) -> io::Result<bool> {
        self.run_upgrade(true)
    }

    /// Run the upgrade command, optionally forcing reinstall.
    fn run_upgrade(&self, force: bool) -> io::Result<bool> {
        let method = detect_install_method();
        let (cmd, args, label) = match method {
            InstallMethod::Cargo => {
                let mut args = vec!["install", "linthis", "--force"];
                if force {
                    args.push("--force");
                }
                ("cargo", args, "cargo")
            }
            InstallMethod::Homebrew => {
                let args = if force {
                    vec!["reinstall", "linthis"]
                } else {
                    vec!["upgrade", "linthis"]
                };
                ("brew", args, "brew")
            }
            InstallMethod::UvTool => {
                let args = if force {
                    vec!["tool", "install", "--force", "linthis"]
                } else {
                    vec!["tool", "upgrade", "linthis"]
                };
                ("uv", args, "uv tool")
            }
            InstallMethod::PipX => {
                let args = if force {
                    vec!["install", "--force", "linthis"]
                } else {
                    vec!["upgrade", "linthis"]
                };
                ("pipx", args, "pipx")
            }
            InstallMethod::Pip | InstallMethod::Unknown => {
                let mut args = vec!["install", "--upgrade", "linthis"];
                if force {
                    args.push("--force-reinstall");
                }
                ("pip", args, "pip")
            }
        };

        println!("↓ Upgrading linthis via {}...", label);

        let output = Command::new(cmd).args(&args).output()?;

        if output.status.success() {
            println!("✓ linthis upgraded successfully via {}", label);
            Ok(true)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("✗ Failed to upgrade linthis via {}: {}", label, stderr);
            Ok(false)
        }
    }

    /// Install a specific version of linthis.
    pub fn install_version(&self, version: &str) -> io::Result<bool> {
        let method = detect_install_method();

        if method == InstallMethod::Homebrew {
            eprintln!(
                "Homebrew does not support installing a specific version directly.\n\
                 To pin a version, use: brew install linthis@{version}\n\
                 Or switch to cargo: cargo install linthis@{version} --force"
            );
            return Ok(false);
        }

        let version_spec = format!("linthis=={}", version);
        let cargo_version_spec = format!("linthis@{}", version);

        let (cmd, args, label) = match method {
            InstallMethod::Cargo => (
                "cargo",
                vec!["install", &cargo_version_spec, "--force"],
                "cargo",
            ),
            InstallMethod::UvTool => (
                "uv",
                vec!["tool", "install", &version_spec, "--force"],
                "uv tool",
            ),
            InstallMethod::PipX => ("pipx", vec!["install", &version_spec, "--force"], "pipx"),
            InstallMethod::Homebrew => unreachable!(),
            InstallMethod::Pip | InstallMethod::Unknown => {
                ("pip", vec!["install", &version_spec], "pip")
            }
        };

        println!("↓ Installing linthis {} via {}...", version, label);

        let output = Command::new(cmd).args(&args).output()?;

        if output.status.success() {
            println!("✓ linthis {} installed successfully via {}", version, label);
            Ok(true)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!(
                "✗ Failed to install linthis {} via {}: {}",
                version, label, stderr
            );
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
        assert!((config.interval_days - 7.0).abs() < f64::EPSILON);
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
        bad_config2.interval_days = -1.0;
        assert!(bad_config2.validate().is_err());

        // interval_days = 0 is valid (means 12 hours)
        let mut zero_config = config.clone();
        zero_config.interval_days = 0.0;
        assert!(zero_config.validate().is_ok());
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
        assert!(manager.should_check(7.0));
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
