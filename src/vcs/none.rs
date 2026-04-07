// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

//! No-VCS provider for directories not under version control.
//!
//! Provides sensible defaults: project root is current directory,
//! file discovery methods return empty lists.
//! `linthis -i <file>` still works since explicit paths bypass VCS.

use super::{VcsCapabilities, VcsKind, VcsProvider};
use std::path::{Path, PathBuf};

/// Provider for directories not under any VCS.
pub struct NoneProvider {
    root: PathBuf,
}

impl Default for NoneProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl NoneProvider {
    pub fn new() -> Self {
        Self {
            root: std::env::current_dir().unwrap_or_default(),
        }
    }
}

impl VcsProvider for NoneProvider {
    fn kind(&self) -> VcsKind {
        VcsKind::None
    }

    fn capabilities(&self) -> VcsCapabilities {
        VcsCapabilities {
            has_staging_area: false,
            has_client_hooks: false,
            has_worktree: false,
            has_branches: false,
        }
    }

    fn project_root(&self) -> &Path {
        &self.root
    }

    fn get_pending_files(&self) -> crate::Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }

    fn get_modified_files(&self) -> crate::Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }

    fn get_changed_files(&self, _base: Option<&str>) -> crate::Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }

    fn get_diff(&self, _base: Option<&str>) -> crate::Result<String> {
        Ok(String::new())
    }

    fn stage_files(&self, _files: &[PathBuf]) -> crate::Result<()> {
        Ok(()) // no-op
    }

    fn get_user_name(&self) -> Option<String> {
        std::env::var("USER")
            .ok()
            .or_else(|| std::env::var("USERNAME").ok())
    }

    fn get_remote_url(&self) -> Option<String> {
        None
    }

    fn current_branch(&self) -> Option<String> {
        None
    }

    fn file_contributors(&self, _file: &Path) -> Vec<String> {
        Vec::new()
    }

    fn hooks_dir(&self) -> Option<PathBuf> {
        None
    }

    fn global_hooks_dir(&self) -> Option<PathBuf> {
        None
    }

    fn create_worktree(&self, _path: &Path, _branch: &str) -> crate::Result<()> {
        Err(crate::LintisError::Generic(
            "No VCS available for worktree support".to_string(),
        ))
    }

    fn remove_worktree(&self, _path: &Path) -> crate::Result<()> {
        Ok(())
    }

    fn apply_diff_to(&self, _diff: &str, _target: &Path) -> crate::Result<()> {
        Err(crate::LintisError::Generic(
            "No VCS available for diff apply".to_string(),
        ))
    }
}
