// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Source resolution for the three-tier hook/plugin override system.
//!
//! Resolves a [`HookSource`] entry to either file content (string) or a
//! local directory path, depending on the variant:
//!
//! | Variant | `resolve_to_string` | `resolve_to_dir` |
//! |---------|---------------------|-----------------|
//! | File | ✓ | ✓ |
//! | Plugin | ✓ | ✓ |
//! | Marketplace | ✓ | ✓ |
//! | Url | ✓ | ✗ (error) |
//! | Git | ✓ | ✓ |
//!
//! **Error semantics:** If a TOML source entry exists but resolution fails,
//! the error is returned to the caller as a hard error — do NOT fall through
//! to the built-in generator.  Fall-through only happens when no entry exists.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::HookSource;

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Resolve a plugin alias to its cached directory.
///
/// Loads the project config to find the plugin URL, then maps it through
/// `PluginCache::url_to_cache_path`.  Returns an error if the alias is not
/// configured or if its cache directory does not exist.
fn plugin_cache_dir(alias: &str) -> Result<PathBuf, String> {
    use crate::plugin::{PluginCache, PluginConfigManager};

    // Search both project and global configs for the alias.
    let project_url = PluginConfigManager::project()
        .ok()
        .and_then(|m| m.get_plugin_by_alias(alias).ok().flatten())
        .map(|(url, _)| url);
    let global_url = PluginConfigManager::global()
        .ok()
        .and_then(|m| m.get_plugin_by_alias(alias).ok().flatten())
        .map(|(url, _)| url);

    let url = project_url.or(global_url).ok_or_else(|| {
        format!(
            "Plugin '{}' is not configured. Run `linthis plugin add {} <url>` first.",
            alias, alias
        )
    })?;

    let cache = PluginCache::new().map_err(|e| e.to_string())?;
    let cache_path = cache.url_to_cache_path(&url);

    if !cache_path.exists() {
        return Err(format!(
            "Plugin '{}' is configured but not cached. Run `linthis plugin sync` to download it.",
            alias
        ));
    }

    Ok(cache_path)
}

/// Fetch a marketplace repo on-demand and return the path to the named plugin subdir.
///
/// Clones to the plugin cache directory (identified by marketplace URL), or
/// updates an existing clone.  The marketplace git URL is taken directly from
/// the caller (already resolved from `HookConfig.marketplaces`).
fn marketplace_plugin_dir(marketplace_url: &str, plugin_name: &str) -> Result<PathBuf, String> {
    use crate::plugin::{fetcher::PluginFetcher, PluginCache};

    let cache = PluginCache::new().map_err(|e| e.to_string())?;
    let marketplace_cache = cache.url_to_cache_path(marketplace_url);
    let fetcher = PluginFetcher::new();

    if marketplace_cache.exists() {
        fetcher
            .update_plugin(marketplace_url, &marketplace_cache, None)
            .map_err(|e| format!("Failed to update marketplace '{}': {}", marketplace_url, e))?;
    } else {
        fetcher
            .clone_plugin(marketplace_url, &marketplace_cache, None)
            .map_err(|e| format!("Failed to clone marketplace '{}': {}", marketplace_url, e))?;
    }

    let plugin_dir = marketplace_cache.join(plugin_name);
    if !plugin_dir.exists() {
        return Err(format!(
            "Plugin '{}' not found in marketplace '{}'",
            plugin_name, marketplace_url
        ));
    }

    Ok(plugin_dir)
}

/// Clone a git repo to a temporary directory and return `(temp_dir, resolved_path)`.
///
/// The returned `TempDir` must be kept alive for as long as the path is used;
/// dropping it deletes the clone.
fn git_clone_temp(
    git_url: &str,
    git_ref: Option<&str>,
    inner_path: &str,
) -> Result<(tempfile::TempDir, PathBuf), String> {
    use crate::plugin::fetcher::PluginFetcher;

    let tmp =
        tempfile::TempDir::new().map_err(|e| format!("Failed to create temp directory: {}", e))?;

    let fetcher = PluginFetcher::new();
    fetcher
        .clone_plugin(git_url, tmp.path(), git_ref)
        .map_err(|e| format!("Failed to clone '{}': {}", git_url, e))?;

    let resolved = tmp.path().join(inner_path);
    if !resolved.exists() {
        return Err(format!(
            "Path '{}' not found in git repo '{}'",
            inner_path, git_url
        ));
    }

    Ok((tmp, resolved))
}

fn resolve_marketplace_url(
    name: &str,
    marketplaces: &HashMap<String, String>,
) -> Result<String, String> {
    marketplaces.get(name).cloned().ok_or_else(|| {
        format!(
            "Marketplace '{}' is not defined in [hook.marketplaces]",
            name
        )
    })
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Resolve a `HookSource` to its file contents as a string.
///
/// Used for git hook scripts, agent-hook JSON, and memory markdown content.
///
/// `project_root` is the root of the current project (git worktree root or CWD).
/// `marketplaces` maps marketplace names to their git URLs (from `HookConfig.marketplaces`).
pub fn resolve_to_string(
    source: &HookSource,
    project_root: &Path,
    marketplaces: &HashMap<String, String>,
) -> Result<String, String> {
    match source {
        HookSource::File { file } => {
            let path = project_root.join(file);
            std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read local file '{}': {}", path.display(), e))
        }

        HookSource::Plugin { plugin, file } => {
            let cache_dir = plugin_cache_dir(plugin)?;
            let path = cache_dir.join(file);
            std::fs::read_to_string(&path)
                .map_err(|e| format!("File '{}' not found in plugin '{}': {}", file, plugin, e))
        }

        HookSource::Marketplace {
            marketplace,
            plugin,
            file,
        } => {
            let mkt_url = resolve_marketplace_url(marketplace, marketplaces)?;
            let plugin_dir = marketplace_plugin_dir(&mkt_url, plugin)?;
            let path = plugin_dir.join(file);
            std::fs::read_to_string(&path).map_err(|e| {
                format!(
                    "File '{}' not found in marketplace plugin '{}/{}': {}",
                    file, marketplace, plugin, e
                )
            })
        }

        HookSource::Url { url } => reqwest::blocking::get(url)
            .map_err(|e| format!("HTTP GET '{}' failed: {}", url, e))?
            .text()
            .map_err(|e| format!("Failed to read HTTP response from '{}': {}", url, e)),

        HookSource::Git { git, git_ref, path } => {
            let (_tmp, resolved) = git_clone_temp(git, git_ref.as_deref(), path)?;
            std::fs::read_to_string(&resolved)
                .map_err(|e| format!("Failed to read '{}' from git repo '{}': {}", path, git, e))
            // _tmp dropped here → temp dir deleted
        }
    }
}

/// A resolved directory — either a persistent path or a temporary clone.
///
/// For `Temporary`, the inner `TempDir` keeps the clone alive.  Drop this
/// value to delete the temporary clone.
pub enum ResolvedDir {
    Persistent(PathBuf),
    Temporary {
        _tmp: tempfile::TempDir,
        path: PathBuf,
    },
}

impl ResolvedDir {
    /// Borrow the resolved directory path.
    pub fn path(&self) -> &Path {
        match self {
            ResolvedDir::Persistent(p) => p,
            ResolvedDir::Temporary { path, .. } => path,
        }
    }
}

/// Resolve a `HookSource` to a local directory path.
///
/// Used for agent plugin directory sources (containing `skill/`, `command/`, `memory/`).
/// The `Url` variant is not supported for directory sources.
///
/// For the `Git` variant, a `TempDir` is kept alive inside the returned `ResolvedDir`.
pub fn resolve_to_dir(
    source: &HookSource,
    project_root: &Path,
    marketplaces: &HashMap<String, String>,
) -> Result<ResolvedDir, String> {
    match source {
        HookSource::File { file } => {
            let path = project_root.join(file);
            if !path.is_dir() {
                return Err(format!(
                    "Expected a directory at '{}' but it does not exist or is a file",
                    path.display()
                ));
            }
            Ok(ResolvedDir::Persistent(path))
        }

        HookSource::Plugin { plugin, file } => {
            let cache_dir = plugin_cache_dir(plugin)?;
            let path = cache_dir.join(file);
            if !path.is_dir() {
                return Err(format!(
                    "Directory '{}' not found in plugin '{}'",
                    file, plugin
                ));
            }
            Ok(ResolvedDir::Persistent(path))
        }

        HookSource::Marketplace {
            marketplace,
            plugin,
            file,
        } => {
            let mkt_url = resolve_marketplace_url(marketplace, marketplaces)?;
            let plugin_dir = marketplace_plugin_dir(&mkt_url, plugin)?;
            let path = plugin_dir.join(file);
            if !path.is_dir() {
                return Err(format!(
                    "Directory '{}' not found in marketplace plugin '{}/{}'",
                    file, marketplace, plugin
                ));
            }
            Ok(ResolvedDir::Persistent(path))
        }

        HookSource::Url { url } => Err(format!(
            "The `url` source variant does not support directory resolution (source: {})",
            url
        )),

        HookSource::Git { git, git_ref, path } => {
            let (tmp, resolved) = git_clone_temp(git, git_ref.as_deref(), path)?;
            if !resolved.is_dir() {
                return Err(format!(
                    "'{}' in git repo '{}' is not a directory",
                    path, git
                ));
            }
            Ok(ResolvedDir::Temporary {
                _tmp: tmp,
                path: resolved,
            })
        }
    }
}

// ── Fixed-path auto-discovery helpers ────────────────────────────────────────

/// Tier-1: Check whether a fixed-path git hook override file exists.
///
/// Returns the override file path if it exists, or `None` to fall through.
///
/// `tool_type_dir` is the directory name under `hooks/` (e.g., `"git"`, `"prek-with-agent"`).
/// `event_filename` is the git hook filename (e.g., `"pre-commit"`, `"commit-msg"`).
pub fn fixed_git_hook_path(
    project_root: &Path,
    tool_type_dir: &str,
    event_filename: &str,
) -> Option<PathBuf> {
    let path = project_root
        .join("hooks")
        .join(tool_type_dir)
        .join(event_filename);
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

/// Tier-1: Check whether a fixed-path agent plugin directory exists.
///
/// Two-tier lookup with provider override:
/// 1. `hooks/agent/plugins/<provider>/<plugin>/` — provider-specific override
/// 2. `hooks/agent/plugins/_default/<plugin>/`   — default fallback
///
/// Returns the first directory that exists, or `None` to fall through to Tier 2/3.
pub fn fixed_agent_plugin_dir(
    project_root: &Path,
    provider_name: &str,
    plugin_id: &str,
) -> Option<PathBuf> {
    // Tier 1a: provider-specific override
    let provider_path = project_root
        .join("hooks")
        .join("agent")
        .join("plugins")
        .join(provider_name)
        .join(plugin_id);
    if provider_path.is_dir() {
        return Some(provider_path);
    }
    // Tier 1b: _default fallback
    let default_path = project_root
        .join("hooks")
        .join("agent")
        .join("plugins")
        .join("_default")
        .join(plugin_id);
    if default_path.is_dir() {
        Some(default_path)
    } else {
        None
    }
}
