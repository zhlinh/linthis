// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Plugin fetcher for cloning and updating Git repositories.
//!
//! Uses shell Git commands via std::process::Command to:
//! - Clone plugin repositories with shallow clone (--depth 1)
//! - Update existing cached plugins (git pull)
//! - Checkout specific refs (tags, branches, commits)

use chrono::Utc;
use std::path::Path;
use std::process::Command;

use super::cache::{CachedPlugin, PluginCache};
use super::{log_plugin_operation, PluginError, PluginSource, Result};

/// Plugin fetcher handles Git operations
#[derive(Debug)]
pub struct PluginFetcher {
    verbose: bool,
}

impl Default for PluginFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginFetcher {
    /// Create a new plugin fetcher
    pub fn new() -> Self {
        Self { verbose: false }
    }

    /// Create a fetcher with verbose logging
    pub fn with_verbose(verbose: bool) -> Self {
        Self { verbose }
    }

    /// Check if Git is available on the system
    pub fn check_git_available() -> Result<()> {
        let output = Command::new("git").arg("--version").output();

        match output {
            Ok(output) if output.status.success() => Ok(()),
            _ => Err(PluginError::GitNotInstalled),
        }
    }

    /// Fetch a plugin from Git repository
    ///
    /// If already cached, returns the cached version unless force_update is true.
    pub fn fetch(
        &self,
        source: &PluginSource,
        cache: &PluginCache,
        force_update: bool,
    ) -> Result<CachedPlugin> {
        // Check Git availability first
        Self::check_git_available()?;

        let url = source
            .url
            .as_ref()
            .ok_or_else(|| PluginError::CloneFailed {
                url: source.name.clone(),
                message: "No URL provided for plugin".to_string(),
            })?;

        let cache_path = cache.url_to_cache_path(url);

        // Check if already cached
        if cache_path.exists() {
            if force_update {
                log_plugin_operation("update", &format!("Updating {}", source.name), self.verbose);
                self.update_plugin(url, &cache_path, source.git_ref.as_deref())?;
            } else {
                log_plugin_operation(
                    "cache hit",
                    &format!("Using cached {}", source.name),
                    self.verbose,
                );
            }
        } else {
            log_plugin_operation("clone", &format!("Cloning {}", url), self.verbose);
            self.clone_plugin(url, &cache_path, source.git_ref.as_deref())?;
        }

        // Create/update cache metadata
        let now = Utc::now();
        let plugin = CachedPlugin {
            name: source.name.clone(),
            url: url.clone(),
            git_ref: source.git_ref.clone(),
            cached_at: now,
            last_updated: now,
            cache_path,
        };

        cache.save_cache_metadata(&plugin)?;

        Ok(plugin)
    }

    /// Clone a plugin repository with shallow clone
    fn clone_plugin(&self, url: &str, target_path: &Path, git_ref: Option<&str>) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Build clone command
        let mut cmd = Command::new("git");
        cmd.arg("clone")
            .arg("--depth")
            .arg("1")
            .arg("--single-branch");

        // Add branch if specified
        if let Some(ref_name) = git_ref {
            cmd.arg("--branch").arg(ref_name);
        }

        cmd.arg(url).arg(target_path);

        log_plugin_operation(
            "git",
            &format!("git clone --depth 1 {} {:?}", url, target_path),
            self.verbose,
        );

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PluginError::CloneFailed {
                url: url.to_string(),
                message: stderr.to_string(),
            });
        }

        // If git_ref is a commit hash, we need to fetch and checkout it
        if let Some(ref_name) = git_ref {
            if self.looks_like_commit_hash(ref_name) {
                self.checkout_commit(target_path, ref_name)?;
            }
        }

        Ok(())
    }

    /// Update an existing cached plugin
    fn update_plugin(&self, url: &str, cache_path: &Path, git_ref: Option<&str>) -> Result<()> {
        // Fetch latest changes
        let mut cmd = Command::new("git");
        cmd.current_dir(cache_path)
            .arg("fetch")
            .arg("--depth")
            .arg("1");

        if let Some(ref_name) = git_ref {
            cmd.arg("origin").arg(ref_name);
        }

        log_plugin_operation("git", "git fetch --depth 1", self.verbose);

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PluginError::UpdateFailed {
                name: url.to_string(),
                message: stderr.to_string(),
            });
        }

        // Reset to origin/HEAD or specified ref
        let mut reset_cmd = Command::new("git");
        reset_cmd.current_dir(cache_path).arg("reset").arg("--hard");

        if let Some(ref_name) = git_ref {
            if self.looks_like_commit_hash(ref_name) {
                reset_cmd.arg(ref_name);
            } else {
                reset_cmd.arg(format!("origin/{}", ref_name));
            }
        } else {
            reset_cmd.arg("origin/HEAD");
        }

        log_plugin_operation("git", "git reset --hard", self.verbose);

        let reset_output = reset_cmd.output()?;

        if !reset_output.status.success() {
            let stderr = String::from_utf8_lossy(&reset_output.stderr);
            return Err(PluginError::UpdateFailed {
                name: url.to_string(),
                message: stderr.to_string(),
            });
        }

        Ok(())
    }

    /// Checkout a specific commit (for commit hash refs)
    fn checkout_commit(&self, repo_path: &Path, commit: &str) -> Result<()> {
        // Fetch the specific commit
        let fetch_output = Command::new("git")
            .current_dir(repo_path)
            .arg("fetch")
            .arg("--depth")
            .arg("1")
            .arg("origin")
            .arg(commit)
            .output()?;

        if !fetch_output.status.success() {
            // Commit might already be fetched, continue
            log_plugin_operation(
                "git",
                "Commit fetch failed, trying checkout anyway",
                self.verbose,
            );
        }

        // Checkout the commit
        let checkout_output = Command::new("git")
            .current_dir(repo_path)
            .arg("checkout")
            .arg(commit)
            .output()?;

        if !checkout_output.status.success() {
            let stderr = String::from_utf8_lossy(&checkout_output.stderr);
            return Err(PluginError::CloneFailed {
                url: commit.to_string(),
                message: format!("Failed to checkout commit: {}", stderr),
            });
        }

        Ok(())
    }

    /// Check if a string looks like a Git commit hash
    fn looks_like_commit_hash(&self, s: &str) -> bool {
        s.len() >= 7 && s.len() <= 40 && s.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Check if we're likely offline (network unavailable)
    pub fn check_network_available(&self, url: &str) -> bool {
        // Try a quick git ls-remote to check connectivity
        let output = Command::new("git")
            .arg("ls-remote")
            .arg("--exit-code")
            .arg("--heads")
            .arg(url)
            .arg("HEAD")
            .output();

        match output {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looks_like_commit_hash() {
        let fetcher = PluginFetcher::new();

        assert!(fetcher.looks_like_commit_hash("abc1234"));
        assert!(fetcher.looks_like_commit_hash("abc1234567890abcdef1234567890abcdef1234")); // 40 chars
        assert!(!fetcher.looks_like_commit_hash("main"));
        assert!(!fetcher.looks_like_commit_hash("v1.0.0"));
        assert!(!fetcher.looks_like_commit_hash("abc")); // too short
    }

    #[test]
    fn test_git_available() {
        // This test will pass if git is installed, skip otherwise
        let result = PluginFetcher::check_git_available();
        // Just check it doesn't panic; availability depends on system
        let _ = result;
    }
}
