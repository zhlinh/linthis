// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

//! SVN VCS provider implementation.

use super::{VcsCapabilities, VcsKind, VcsProvider};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Check if the current directory is inside a SVN working copy.
pub(crate) fn is_svn_repo() -> bool {
    Command::new("svn")
        .args(["info"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Detect SVN working copy root.
fn detect_svn_root() -> PathBuf {
    Command::new("svn")
        .args(["info", "--show-item", "wc-root"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

/// SVN VCS provider.
pub struct SvnProvider {
    root: PathBuf,
}

impl Default for SvnProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SvnProvider {
    pub fn new() -> Self {
        Self {
            root: detect_svn_root(),
        }
    }

    /// Parse `svn status` output into file paths.
    /// SVN status format: "M       path/to/file"
    /// Status codes: M=modified, A=added, R=replaced, ?=untracked
    fn parse_status_files(&self, include_untracked: bool) -> Vec<PathBuf> {
        let output = match Command::new("svn").arg("status").output() {
            Ok(o) if o.status.success() => o,
            _ => return Vec::new(),
        };

        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                if line.len() < 2 {
                    return None;
                }
                let status = line.chars().next()?;
                let path_str = line[1..].trim();
                match status {
                    'M' | 'A' | 'R' => Some(self.root.join(path_str)),
                    '?' if include_untracked => Some(self.root.join(path_str)),
                    _ => None,
                }
            })
            .filter(|p| p.exists())
            .collect()
    }
}

impl VcsProvider for SvnProvider {
    fn kind(&self) -> VcsKind {
        VcsKind::Svn
    }

    fn capabilities(&self) -> VcsCapabilities {
        VcsCapabilities {
            has_staging_area: false,
            has_client_hooks: false,
            has_worktree: false,
            has_branches: true,
        }
    }

    fn project_root(&self) -> &Path {
        &self.root
    }

    fn get_pending_files(&self) -> crate::Result<Vec<PathBuf>> {
        // SVN has no staging area — pending = all modified/added files
        Ok(self.parse_status_files(false))
    }

    fn get_modified_files(&self) -> crate::Result<Vec<PathBuf>> {
        Ok(self.parse_status_files(true))
    }

    fn get_changed_files(&self, base: Option<&str>) -> crate::Result<Vec<PathBuf>> {
        let args = if let Some(rev) = base {
            vec!["diff", "-r", rev, "--summarize"]
        } else {
            vec!["diff", "--summarize"]
        };

        let output = Command::new("svn")
            .args(&args)
            .output()
            .map_err(crate::LintisError::Io)?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let files = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                // Format: "M       path/to/file"
                let path_str = line.get(1..)?.trim();
                if path_str.is_empty() {
                    return None;
                }
                let p = self.root.join(path_str);
                if p.exists() {
                    Some(p)
                } else {
                    None
                }
            })
            .collect();

        Ok(files)
    }

    fn get_diff(&self, base: Option<&str>) -> crate::Result<String> {
        let mut args = vec!["diff"];
        if let Some(rev) = base {
            args.extend(["-r", rev]);
        }

        let output = Command::new("svn")
            .args(&args)
            .output()
            .map_err(crate::LintisError::Io)?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn stage_files(&self, files: &[PathBuf]) -> crate::Result<()> {
        // SVN: only need to `svn add` untracked files; tracked modified files
        // are automatically included in the next commit.
        for file in files {
            // Check if file is untracked (status '?')
            if let Ok(output) = Command::new("svn")
                .args(["status", &file.to_string_lossy()])
                .output()
            {
                let status = String::from_utf8_lossy(&output.stdout);
                if status.starts_with('?') {
                    let _ = Command::new("svn")
                        .args(["add", &file.to_string_lossy()])
                        .output();
                }
            }
        }
        Ok(())
    }

    fn get_user_name(&self) -> Option<String> {
        // Try svn info for last-changed-author, or fall back to env
        Command::new("svn")
            .args(["info", "--show-item", "last-changed-author"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| std::env::var("USER").ok())
    }

    fn get_remote_url(&self) -> Option<String> {
        Command::new("svn")
            .args(["info", "--show-item", "repos-root-url"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    }

    fn current_branch(&self) -> Option<String> {
        // SVN doesn't have native branches the same way; try to extract from URL
        Command::new("svn")
            .args(["info", "--show-item", "relative-url"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    }

    fn file_contributors(&self, file: &Path) -> Vec<String> {
        Command::new("svn")
            .args(["log", "--quiet", &file.to_string_lossy()])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter(|l| l.starts_with('r'))
                    .filter_map(|l| l.split('|').nth(1).map(|s| s.trim().to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn hooks_dir(&self) -> Option<PathBuf> {
        None // SVN hooks are server-side only
    }

    fn global_hooks_dir(&self) -> Option<PathBuf> {
        None
    }

    fn create_worktree(&self, _path: &Path, _branch: &str) -> crate::Result<()> {
        Err(crate::LintisError::Generic(
            "SVN does not support worktrees".to_string(),
        ))
    }

    fn remove_worktree(&self, _path: &Path) -> crate::Result<()> {
        Err(crate::LintisError::Generic(
            "SVN does not support worktrees".to_string(),
        ))
    }

    fn apply_diff_to(&self, diff: &str, target: &Path) -> crate::Result<()> {
        use std::io::Write;
        use std::process::Stdio;

        let mut child = Command::new("patch")
            .args(["-p0", "-d", &target.to_string_lossy()])
            .stdin(Stdio::piped())
            .spawn()
            .map_err(crate::LintisError::Io)?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(diff.as_bytes())
                .map_err(crate::LintisError::Io)?;
        }
        child.wait().map_err(crate::LintisError::Io)?;
        Ok(())
    }
}
