// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

//! Persisted shell-integration state at `~/.linthis/shell-state.toml`.
//!
//! Single source of truth for which features are enabled per shell.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Supported shells. Stable across versions — used as TOML section names.
//
// `PowerShell` ends with the enum name `Shell`, which trips
// `clippy::enum_variant_names`. The product name is canonical and is referenced
// across later tasks as `Shell::PowerShell`, so we suppress the lint locally.
#[allow(clippy::enum_variant_names)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}

impl Shell {
    /// Stable lowercase name used in the TOML section header.
    pub fn key(self) -> &'static str {
        match self {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
            Shell::PowerShell => "powershell",
        }
    }

    /// All supported shells, in display/iteration order.
    pub const ALL: [Shell; 4] = [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell];
}

/// Per-shell flags. Default = both off.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellFlags {
    #[serde(default)]
    pub ac: bool,
    #[serde(default)]
    pub alias: bool,
}

impl ShellFlags {
    pub fn is_empty(self) -> bool {
        !self.ac && !self.alias
    }
}

/// Full state. Fields are public so callers can flip them and pass back to `save`.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellState {
    #[serde(default)]
    pub bash: ShellFlags,
    #[serde(default)]
    pub zsh: ShellFlags,
    #[serde(default)]
    pub fish: ShellFlags,
    #[serde(default)]
    pub powershell: ShellFlags,
}

impl ShellState {
    pub fn flags(&self, shell: Shell) -> ShellFlags {
        match shell {
            Shell::Bash => self.bash,
            Shell::Zsh => self.zsh,
            Shell::Fish => self.fish,
            Shell::PowerShell => self.powershell,
        }
    }

    pub fn flags_mut(&mut self, shell: Shell) -> &mut ShellFlags {
        match shell {
            Shell::Bash => &mut self.bash,
            Shell::Zsh => &mut self.zsh,
            Shell::Fish => &mut self.fish,
            Shell::PowerShell => &mut self.powershell,
        }
    }
}

/// Errors load/save can produce. `Corrupt` is distinct from `Io` so the caller
/// can tell "the user's TOML is wrong" from "I couldn't read the file at all".
#[derive(Debug)]
pub enum StateError {
    Io(std::io::Error),
    Corrupt(String),
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateError::Io(e) => write!(f, "shell-state I/O error: {e}"),
            StateError::Corrupt(s) => write!(f, "shell-state.toml is corrupt: {s}"),
        }
    }
}
impl std::error::Error for StateError {}

/// Default path resolution: `~/.linthis/shell-state.toml`.
pub fn default_state_path(home: &Path) -> PathBuf {
    home.join(".linthis").join("shell-state.toml")
}

/// Load state from `path`. Missing file → `Ok(default)`. Unreadable → `Err(Io)`.
/// Malformed TOML → `Err(Corrupt)`.
pub fn load(path: &Path) -> Result<ShellState, StateError> {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(ShellState::default()),
        Err(e) => return Err(StateError::Io(e)),
    };
    toml::from_str(&raw).map_err(|e| StateError::Corrupt(e.to_string()))
}

/// Save state to `path`. Creates parent dirs as needed. Atomic temp+rename.
pub fn save(path: &Path, state: &ShellState) -> Result<(), StateError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(StateError::Io)?;
    }
    let body = toml::to_string_pretty(state)
        .map_err(|e| StateError::Corrupt(format!("failed to serialize: {e}")))?;
    // Atomic save: write to .toml.tmp first, then rename over the real file.
    // If the process dies between write and rename, the orphaned .tmp file is
    // harmless — `load` only reads `.toml`, and the next `save` will overwrite
    // the temp before renaming. Same-directory write guarantees same-device
    // rename, so cross-device rename failures are not possible here.
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, body.as_bytes()).map_err(StateError::Io)?;
    std::fs::rename(&tmp, path).map_err(StateError::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_file_loads_as_empty_state() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("absent.toml");
        let state = load(&path).expect("missing file is not an error");
        assert_eq!(state, ShellState::default());
    }

    #[test]
    fn missing_section_or_key_reads_as_false() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("partial.toml");
        std::fs::write(
            &path,
            "[bash]\nac = true\n\n[zsh]\n", // no alias key, no fish/powershell sections
        )
        .unwrap();
        let state = load(&path).unwrap();
        assert!(state.bash.ac);
        assert!(!state.bash.alias);
        assert!(!state.zsh.ac && !state.zsh.alias);
        assert!(state.fish.is_empty());
        assert!(state.powershell.is_empty());
    }

    #[test]
    fn round_trip_preserves_state() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.toml");
        let mut want = ShellState::default();
        want.bash = ShellFlags {
            ac: true,
            alias: true,
        };
        want.zsh = ShellFlags {
            ac: true,
            alias: false,
        };
        save(&path, &want).unwrap();
        let got = load(&path).unwrap();
        assert_eq!(got, want);
    }

    #[test]
    fn corrupt_toml_returns_typed_error_not_panic() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, "this is not = = valid toml [[[").unwrap();
        match load(&path) {
            Err(StateError::Corrupt(_)) => {}
            other => panic!("expected Corrupt error, got {:?}", other),
        }
    }

    #[test]
    fn save_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested").join("dirs").join("state.toml");
        save(&path, &ShellState::default()).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn flags_mut_updates_correct_section() {
        let mut s = ShellState::default();
        s.flags_mut(Shell::Fish).alias = true;
        assert!(s.fish.alias);
        assert!(!s.bash.alias);
    }
}
