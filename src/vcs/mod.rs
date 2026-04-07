// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

//! Version Control System abstraction layer.
//!
//! Provides a unified interface for Git, SVN, and non-VCS directories.
//! Use `detect_vcs()` to auto-detect the VCS in the current directory.

mod git;
mod none;
mod svn;

pub use git::GitProvider;
pub use none::NoneProvider;
pub use svn::SvnProvider;

use std::path::{Path, PathBuf};

/// Supported VCS types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsKind {
    Git,
    Svn,
    None,
}

impl std::fmt::Display for VcsKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Git => write!(f, "git"),
            Self::Svn => write!(f, "svn"),
            Self::None => write!(f, "none"),
        }
    }
}

/// VCS capability flags — what operations are supported.
#[derive(Debug, Clone)]
pub struct VcsCapabilities {
    /// Whether the VCS has a staging area (Git: yes, SVN: no)
    pub has_staging_area: bool,
    /// Whether client-side hooks are supported (Git: yes, SVN: no)
    pub has_client_hooks: bool,
    /// Whether worktree isolation is supported (Git: yes, SVN: no)
    pub has_worktree: bool,
    /// Whether branching is supported
    pub has_branches: bool,
}

/// Unified VCS provider interface.
pub trait VcsProvider: Send + Sync {
    /// VCS type identifier.
    fn kind(&self) -> VcsKind;

    /// Query supported capabilities.
    fn capabilities(&self) -> VcsCapabilities;

    /// Project root directory.
    fn project_root(&self) -> &Path;

    /// Get files pending commit (Git: staged, SVN: modified/added).
    fn get_pending_files(&self) -> crate::Result<Vec<PathBuf>>;

    /// Get all modified files (Git: staged+unstaged+untracked, SVN: modified).
    fn get_modified_files(&self) -> crate::Result<Vec<PathBuf>>;

    /// Get files changed relative to a base ref.
    fn get_changed_files(&self, base: Option<&str>) -> crate::Result<Vec<PathBuf>>;

    /// Get unified diff output.
    fn get_diff(&self, base: Option<&str>) -> crate::Result<String>;

    /// Mark files for commit (Git: git add, SVN: svn add for new files).
    fn stage_files(&self, files: &[PathBuf]) -> crate::Result<()>;

    /// Get the current user name.
    fn get_user_name(&self) -> Option<String>;

    /// Get the remote repository URL.
    fn get_remote_url(&self) -> Option<String>;

    /// Get the current branch name.
    fn current_branch(&self) -> Option<String>;

    /// Get file contributor history.
    fn file_contributors(&self, file: &Path) -> Vec<String>;

    /// Get the client-side hooks directory (None if not supported).
    fn hooks_dir(&self) -> Option<PathBuf>;

    /// Get the global hooks directory.
    fn global_hooks_dir(&self) -> Option<PathBuf>;

    /// Create an isolated worktree for safe testing.
    fn create_worktree(&self, path: &Path, branch: &str) -> crate::Result<()>;

    /// Remove a worktree.
    fn remove_worktree(&self, path: &Path) -> crate::Result<()>;

    /// Apply a diff to a target directory.
    fn apply_diff_to(&self, diff: &str, target: &Path) -> crate::Result<()>;
}

/// Auto-detect VCS type and return the appropriate provider.
pub fn detect_vcs() -> Box<dyn VcsProvider> {
    if git::is_git_repo() {
        Box::new(GitProvider::new())
    } else if svn::is_svn_repo() {
        Box::new(SvnProvider::new())
    } else {
        Box::new(NoneProvider::new())
    }
}
