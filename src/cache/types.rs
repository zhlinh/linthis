// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Cache data structures for storing lint results.

use crate::utils::types::{LintIssue, Severity};
use crate::Language;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Single file cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// xxHash64 of file content
    pub content_hash: u64,
    /// File modification time (for quick staleness check)
    #[serde(with = "system_time_serde")]
    pub mtime: SystemTime,
    /// Cached lint issues for this file
    pub issues: Vec<CachedIssue>,
    /// When this entry was cached
    #[serde(with = "system_time_serde")]
    pub cached_at: SystemTime,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(content_hash: u64, mtime: SystemTime, issues: Vec<CachedIssue>) -> Self {
        Self {
            content_hash,
            mtime,
            issues,
            cached_at: SystemTime::now(),
        }
    }

    /// Convert cached issues back to LintIssue for a specific file
    pub fn to_lint_issues(&self, file_path: &Path, language: Option<Language>) -> Vec<LintIssue> {
        self.issues
            .iter()
            .map(|ci| ci.to_lint_issue(file_path.to_path_buf(), language))
            .collect()
    }
}

/// Simplified issue for caching (without file_path since it's the key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedIssue {
    pub line: usize,
    pub column: Option<usize>,
    pub message: String,
    pub severity: Severity,
    pub code: Option<String>,
    pub source: Option<String>,
    pub suggestion: Option<String>,
}

impl CachedIssue {
    /// Create from a LintIssue
    pub fn from_lint_issue(issue: &LintIssue) -> Self {
        Self {
            line: issue.line,
            column: issue.column,
            message: issue.message.clone(),
            severity: issue.severity,
            code: issue.code.clone(),
            source: issue.source.clone(),
            suggestion: issue.suggestion.clone(),
        }
    }

    /// Convert back to LintIssue
    pub fn to_lint_issue(&self, file_path: PathBuf, language: Option<Language>) -> LintIssue {
        let mut issue = LintIssue::new(file_path, self.line, self.message.clone(), self.severity);
        if let Some(col) = self.column {
            issue = issue.with_column(col);
        }
        if let Some(ref code) = self.code {
            issue = issue.with_code(code.clone());
        }
        if let Some(ref source) = self.source {
            issue = issue.with_source(source.clone());
        }
        if let Some(ref suggestion) = self.suggestion {
            issue = issue.with_suggestion(suggestion.clone());
        }
        if let Some(lang) = language {
            issue = issue.with_language(lang);
        }
        issue
    }
}

/// Statistics about cache usage
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Files skipped due to cache hit
    pub cache_hits: usize,
    /// Files that needed checking (cache miss or changed)
    pub cache_misses: usize,
    /// Files with invalidated cache (content changed)
    pub invalidated: usize,
}

impl CacheStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Total files processed
    pub fn total(&self) -> usize {
        self.cache_hits + self.cache_misses
    }

    /// Cache hit rate as percentage
    pub fn hit_rate(&self) -> f64 {
        if self.total() == 0 {
            0.0
        } else {
            (self.cache_hits as f64 / self.total() as f64) * 100.0
        }
    }
}

/// Custom serialization for SystemTime to work across platforms
mod system_time_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);
        (duration.as_secs(), duration.subsec_nanos()).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (secs, nanos): (u64, u32) = Deserialize::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::new(secs, nanos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_issue_roundtrip() {
        let original = LintIssue::new(
            PathBuf::from("test.rs"),
            10,
            "test message".to_string(),
            Severity::Warning,
        )
        .with_column(5)
        .with_code("W001".to_string())
        .with_source("clippy".to_string());

        let cached = CachedIssue::from_lint_issue(&original);
        let restored = cached.to_lint_issue(PathBuf::from("test.rs"), Some(Language::Rust));

        assert_eq!(restored.line, original.line);
        assert_eq!(restored.column, original.column);
        assert_eq!(restored.message, original.message);
        assert_eq!(restored.severity, original.severity);
        assert_eq!(restored.code, original.code);
        assert_eq!(restored.source, original.source);
    }

    #[test]
    fn test_cache_stats() {
        let mut stats = CacheStats::new();
        stats.cache_hits = 8;
        stats.cache_misses = 2;

        assert_eq!(stats.total(), 10);
        assert!((stats.hit_rate() - 80.0).abs() < 0.01);
    }

    #[test]
    fn test_cache_stats_empty() {
        let stats = CacheStats::new();
        assert_eq!(stats.total(), 0);
        assert!((stats.hit_rate() - 0.0).abs() < 0.01);
    }
}
