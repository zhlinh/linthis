// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

//! Git VCS provider implementation.

use super::{VcsCapabilities, VcsKind, VcsProvider};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Check if the current directory is inside a git repository.
pub(crate) fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Detect the git project root directory.
fn detect_git_root() -> PathBuf {
    Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

/// Parse git command output into absolute file paths.
fn parse_file_list(output: &[u8], root: &Path) -> Vec<PathBuf> {
    String::from_utf8_lossy(output)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| root.join(line.trim()))
        .filter(|p| p.exists())
        .collect()
}

/// Git VCS provider.
pub struct GitProvider {
    root: PathBuf,
}

impl Default for GitProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GitProvider {
    pub fn new() -> Self {
        Self {
            root: detect_git_root(),
        }
    }
}

impl VcsProvider for GitProvider {
    fn kind(&self) -> VcsKind {
        VcsKind::Git
    }

    fn capabilities(&self) -> VcsCapabilities {
        VcsCapabilities {
            has_staging_area: true,
            has_client_hooks: true,
            has_worktree: true,
            has_branches: true,
        }
    }

    fn project_root(&self) -> &Path {
        &self.root
    }

    fn get_pending_files(&self) -> crate::Result<Vec<PathBuf>> {
        let output = Command::new("git")
            .args(["diff", "--cached", "--name-only", "--diff-filter=d"])
            .output()
            .map_err(crate::LintisError::Io)?;

        if !output.status.success() {
            return Ok(Vec::new());
        }
        Ok(parse_file_list(&output.stdout, &self.root))
    }

    fn get_modified_files(&self) -> crate::Result<Vec<PathBuf>> {
        use std::collections::HashSet;
        let mut files: HashSet<PathBuf> = HashSet::new();

        // Staged
        files.extend(self.get_pending_files()?);

        // Unstaged modified
        if let Ok(output) = Command::new("git")
            .args(["diff", "--name-only", "--diff-filter=d"])
            .output()
        {
            if output.status.success() {
                files.extend(parse_file_list(&output.stdout, &self.root));
            }
        }

        // Untracked
        if let Ok(output) = Command::new("git")
            .args(["ls-files", "--others", "--exclude-standard"])
            .output()
        {
            if output.status.success() {
                files.extend(parse_file_list(&output.stdout, &self.root));
            }
        }

        Ok(files.into_iter().collect())
    }

    fn get_changed_files(&self, base: Option<&str>) -> crate::Result<Vec<PathBuf>> {
        let base_ref = base.unwrap_or("HEAD");

        let output = Command::new("git")
            .args(["diff", "--name-only", "--diff-filter=d", base_ref])
            .output()
            .map_err(crate::LintisError::Io)?;

        if output.status.success() {
            return Ok(parse_file_list(&output.stdout, &self.root));
        }

        // Fallback: try merge-base
        if let Ok(mb) = Command::new("git")
            .args(["merge-base", "HEAD", base_ref])
            .output()
        {
            if mb.status.success() {
                let base_commit = String::from_utf8_lossy(&mb.stdout).trim().to_string();
                if let Ok(output) = Command::new("git")
                    .args(["diff", "--name-only", "--diff-filter=d", &base_commit])
                    .output()
                {
                    if output.status.success() {
                        return Ok(parse_file_list(&output.stdout, &self.root));
                    }
                }
            }
        }

        Ok(Vec::new())
    }

    fn get_diff(&self, base: Option<&str>) -> crate::Result<String> {
        let args = if let Some(b) = base {
            vec!["diff", b]
        } else {
            vec!["diff", "--cached"]
        };

        let output = Command::new("git")
            .args(&args)
            .output()
            .map_err(crate::LintisError::Io)?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn stage_files(&self, files: &[PathBuf]) -> crate::Result<()> {
        if files.is_empty() {
            return Ok(());
        }
        let mut cmd = Command::new("git");
        cmd.arg("add");
        for f in files {
            cmd.arg(f);
        }
        cmd.output().map_err(crate::LintisError::Io)?;
        Ok(())
    }

    fn get_user_name(&self) -> Option<String> {
        Command::new("git")
            .args(["config", "user.name"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    }

    fn get_remote_url(&self) -> Option<String> {
        Command::new("git")
            .args(["remote", "get-url", "origin"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    }

    fn current_branch(&self) -> Option<String> {
        Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    }

    fn file_contributors(&self, file: &Path) -> Vec<String> {
        Command::new("git")
            .args(["log", "--format=%aN", "--", &file.to_string_lossy()])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn hooks_dir(&self) -> Option<PathBuf> {
        Some(self.root.join(".git").join("hooks"))
    }

    fn global_hooks_dir(&self) -> Option<PathBuf> {
        Command::new("git")
            .args(["config", "--global", "core.hooksPath"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
    }

    fn create_worktree(&self, path: &Path, branch: &str) -> crate::Result<()> {
        let output = Command::new("git")
            .args([
                "worktree",
                "add",
                &path.to_string_lossy(),
                "-b",
                branch,
                "HEAD",
            ])
            .output()
            .map_err(crate::LintisError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::LintisError::Generic(format!(
                "Failed to create worktree: {}",
                stderr.trim()
            )));
        }
        Ok(())
    }

    fn remove_worktree(&self, path: &Path) -> crate::Result<()> {
        let _ = Command::new("git")
            .args(["worktree", "remove", "--force", &path.to_string_lossy()])
            .output();
        Ok(())
    }

    fn apply_diff_to(&self, diff: &str, target: &Path) -> crate::Result<()> {
        use std::io::Write;
        use std::process::Stdio;

        let mut child = Command::new("git")
            .args(["-C", &target.to_string_lossy(), "apply"])
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
