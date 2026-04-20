// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style license that can be
// found at https://opensource.org/license/MIT

//! Data-driven tool install matrix.
//!
//! Each `ToolInstallSpec` entry in `TOOL_INSTALLS` lists per-platform
//! candidate commands tried in order until one succeeds. A missing OS
//! entry means the tool is not supported on that platform; the resolver
//! returns an empty `Vec` in that case so callers can explicitly skip.

use crate::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    MacOs,
    Linux,
    Windows,
}

impl Os {
    /// The OS this binary was compiled for.
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Os::MacOs
        } else if cfg!(target_os = "windows") {
            Os::Windows
        } else {
            Os::Linux
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolRole {
    Checker,
    Formatter,
}

#[derive(Debug)]
pub struct PlatformCmds {
    pub os: Os,
    /// Candidate commands tried in order (first success wins).
    pub cmds: &'static [&'static [&'static str]],
}

#[derive(Debug)]
pub struct ToolInstallSpec {
    /// Canonical tool name (e.g. "stylua", "golangci-lint").
    pub tool: &'static str,
    pub language: Language,
    pub role: ToolRole,
    /// Only lists platforms where the tool is supported.
    /// A missing OS entry = "not supported on this platform".
    pub platforms: &'static [PlatformCmds],
    /// User-facing hint shown when auto-install is declined/disabled.
    pub hint: &'static str,
}

pub static TOOL_INSTALLS: &[ToolInstallSpec] = &[];

/// Find the spec for (lang, role); returns `None` if no entry.
fn find_spec(lang: Language, role: ToolRole) -> Option<&'static ToolInstallSpec> {
    TOOL_INSTALLS
        .iter()
        .find(|s| s.language == lang && s.role == role)
}

/// Resolve install candidate commands for (lang, role) on the current platform.
/// Returns `vec![]` if the tool is not supported on this platform.
pub fn resolve_install_cmds(lang: Language, role: ToolRole) -> Vec<Vec<String>> {
    let spec = match find_spec(lang, role) {
        Some(s) => s,
        None => return Vec::new(),
    };
    let current = Os::current();
    spec.platforms
        .iter()
        .find(|p| p.os == current)
        .map(|p| {
            p.cmds
                .iter()
                .map(|c| c.iter().map(|s| s.to_string()).collect())
                .collect()
        })
        .unwrap_or_default()
}

/// Is this tool installable on the current platform?
pub fn is_tool_supported_on_current_platform(tool: &str) -> bool {
    let current = Os::current();
    TOOL_INSTALLS
        .iter()
        .find(|s| s.tool == tool)
        .map(|s| s.platforms.iter().any(|p| p.os == current))
        .unwrap_or(false)
}

/// Which platforms does this tool support?
pub fn supported_platforms(tool: &str) -> Vec<Os> {
    TOOL_INSTALLS
        .iter()
        .find(|s| s.tool == tool)
        .map(|s| s.platforms.iter().map(|p| p.os).collect())
        .unwrap_or_default()
}

/// Hint string for a (lang, role) — used when auto-install is off/declined.
pub fn install_hint(lang: Language, role: ToolRole) -> String {
    find_spec(lang, role)
        .map(|s| s.hint.to_string())
        .unwrap_or_else(|| format!("No install hint available for {:?} {:?}", lang, role))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_table_resolves_to_empty_vec() {
        // TOOL_INSTALLS starts empty; any lookup is a miss.
        assert!(resolve_install_cmds(Language::Lua, ToolRole::Formatter).is_empty());
    }

    #[test]
    fn unsupported_tool_is_not_supported_on_current_platform() {
        assert!(!is_tool_supported_on_current_platform("nonexistent-tool"));
    }

    #[test]
    fn os_current_matches_compile_target() {
        #[cfg(target_os = "macos")]
        assert_eq!(Os::current(), Os::MacOs);
        #[cfg(target_os = "linux")]
        assert_eq!(Os::current(), Os::Linux);
        #[cfg(target_os = "windows")]
        assert_eq!(Os::current(), Os::Windows);
    }

    #[test]
    fn install_hint_for_unknown_has_fallback() {
        let hint = install_hint(Language::Dart, ToolRole::Formatter);
        assert!(hint.contains("No install hint"));
    }
}
