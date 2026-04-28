//! Detect the user's current shell from environment / explicit flag.

#![allow(dead_code)]

use super::state::Shell;

/// Detection error. Distinct variants so the caller can craft an actionable
/// message ("set --shell" vs "we don't support that one").
#[derive(Debug, PartialEq, Eq)]
pub enum DetectError {
    /// `$SHELL` is empty/unset and no override given.
    Unset,
    /// Couldn't map the value to a supported shell.
    Unrecognized(String),
}

impl std::fmt::Display for DetectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectError::Unset => {
                write!(f, "could not detect shell (set $SHELL or pass --shell <name>)")
            }
            DetectError::Unrecognized(s) => {
                write!(
                    f,
                    "unrecognized shell `{s}` (try --shell bash|zsh|fish|powershell)"
                )
            }
        }
    }
}
impl std::error::Error for DetectError {}

/// Parse a shell name (the value users pass via `--shell`). Case-insensitive.
pub fn parse_shell_name(name: &str) -> Result<Shell, DetectError> {
    match name.trim().to_ascii_lowercase().as_str() {
        "bash" => Ok(Shell::Bash),
        "zsh" => Ok(Shell::Zsh),
        "fish" => Ok(Shell::Fish),
        "powershell" | "pwsh" | "ps" => Ok(Shell::PowerShell),
        other => Err(DetectError::Unrecognized(other.to_string())),
    }
}

/// Detect from a `$SHELL` value. The env var typically holds an absolute path
/// like `/bin/zsh`; we extract the file stem and parse it.
pub fn detect_from_env_shell(value: &str) -> Result<Shell, DetectError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DetectError::Unset);
    }
    let stem = std::path::Path::new(trimmed)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(trimmed);
    parse_shell_name(stem)
}

/// High-level: explicit override wins, otherwise read `$SHELL`.
pub fn detect(explicit: Option<&str>) -> Result<Shell, DetectError> {
    if let Some(name) = explicit {
        return parse_shell_name(name);
    }
    let value = std::env::var("SHELL").unwrap_or_default();
    detect_from_env_shell(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bash_zsh_fish() {
        assert_eq!(parse_shell_name("bash").unwrap(), Shell::Bash);
        assert_eq!(parse_shell_name("Zsh").unwrap(), Shell::Zsh);
        assert_eq!(parse_shell_name("fish").unwrap(), Shell::Fish);
    }

    #[test]
    fn parses_powershell_aliases() {
        assert_eq!(parse_shell_name("powershell").unwrap(), Shell::PowerShell);
        assert_eq!(parse_shell_name("pwsh").unwrap(), Shell::PowerShell);
        assert_eq!(parse_shell_name("PS").unwrap(), Shell::PowerShell);
    }

    #[test]
    fn rejects_unknown_name() {
        let e = parse_shell_name("ksh").unwrap_err();
        assert_eq!(e, DetectError::Unrecognized("ksh".into()));
    }

    #[test]
    fn detects_from_env_shell_path() {
        assert_eq!(detect_from_env_shell("/bin/zsh").unwrap(), Shell::Zsh);
        assert_eq!(
            detect_from_env_shell("/usr/local/bin/fish").unwrap(),
            Shell::Fish
        );
        assert_eq!(detect_from_env_shell("bash").unwrap(), Shell::Bash);
    }

    #[test]
    fn empty_env_shell_is_unset_error() {
        assert_eq!(detect_from_env_shell("").unwrap_err(), DetectError::Unset);
        assert_eq!(
            detect_from_env_shell("   ").unwrap_err(),
            DetectError::Unset
        );
    }

    #[test]
    fn explicit_override_beats_env() {
        // We can't unset $SHELL safely in tests (parallel-test pollution), so
        // we just assert that `explicit` short-circuits before any env read.
        assert_eq!(detect(Some("fish")).unwrap(), Shell::Fish);
    }
}
