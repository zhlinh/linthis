// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Cache storage and persistence.

use super::hash::{file_hash, is_file_changed};
use super::types::{CacheEntry, CacheStats, CachedIssue};
use crate::utils::types::LintIssue;
use crate::{Language, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Current cache format version (increment to invalidate old caches)
const CACHE_VERSION: u32 = 1;

/// Default cache max age in days
const DEFAULT_CACHE_MAX_AGE_DAYS: u32 = 7;

/// Cache directory name
const CACHE_DIR: &str = ".linthis";

/// Cache file name
const CACHE_FILE: &str = "cache.json";

/// The main lint cache structure
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LintCache {
    /// Cache format version
    pub version: u32,
    /// Checker name -> (relative file path -> cache entry)
    pub entries: HashMap<String, HashMap<PathBuf, CacheEntry>>,
    /// Runtime statistics (not persisted)
    #[serde(skip)]
    stats: CacheStats,
}

impl LintCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            version: CACHE_VERSION,
            entries: HashMap::new(),
            stats: CacheStats::new(),
        }
    }

    /// Load cache from the project's .linthis/cache.json
    pub fn load(project_root: &Path) -> Result<Self> {
        let cache_path = Self::cache_path(project_root);

        if !cache_path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(&cache_path)?;
        let cache: Self = match serde_json::from_str(&content) {
            Ok(c) => c,
            Err(_) => {
                // Cache corrupted, start fresh
                log::warn!("Cache corrupted, creating new cache");
                return Ok(Self::new());
            }
        };

        // Check version compatibility
        if cache.version != CACHE_VERSION {
            log::info!("Cache version mismatch, creating new cache");
            return Ok(Self::new());
        }

        Ok(cache)
    }

    /// Save cache to disk
    pub fn save(&self, project_root: &Path) -> Result<()> {
        let cache_dir = project_root.join(CACHE_DIR);
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
        }

        let cache_path = Self::cache_path(project_root);
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&cache_path, content)?;

        Ok(())
    }

    /// Get the cache file path
    fn cache_path(project_root: &Path) -> PathBuf {
        project_root.join(CACHE_DIR).join(CACHE_FILE)
    }

    /// Clear the cache file
    pub fn clear(project_root: &Path) -> Result<()> {
        let cache_path = Self::cache_path(project_root);
        if cache_path.exists() {
            fs::remove_file(&cache_path)?;
        }
        Ok(())
    }

    /// Check if a file needs checking (cache miss or content changed)
    ///
    /// Returns `Some(cached_issues)` if cache is valid, `None` if needs checking.
    pub fn check_file(
        &mut self,
        checker_name: &str,
        file_path: &Path,
        project_root: &Path,
    ) -> Option<Vec<LintIssue>> {
        let relative_path = file_path
            .strip_prefix(project_root)
            .unwrap_or(file_path)
            .to_path_buf();

        let checker_cache = match self.entries.get(checker_name) {
            Some(cache) => cache,
            None => {
                // No cache for this checker yet
                self.stats.cache_misses += 1;
                return None;
            }
        };

        let entry = match checker_cache.get(&relative_path) {
            Some(e) => e,
            None => {
                // File not in cache
                self.stats.cache_misses += 1;
                return None;
            }
        };

        // Check if file has changed
        match is_file_changed(file_path, entry.mtime, entry.content_hash) {
            Ok(true) => {
                // File changed, cache invalid
                self.stats.cache_misses += 1;
                self.stats.invalidated += 1;
                None
            }
            Ok(false) => {
                // Cache hit!
                self.stats.cache_hits += 1;
                let language = Language::from_path(file_path);
                Some(entry.to_lint_issues(file_path, language))
            }
            Err(_) => {
                // Can't check file, assume changed
                self.stats.cache_misses += 1;
                None
            }
        }
    }

    /// Update cache with new check results
    pub fn update_file(
        &mut self,
        checker_name: &str,
        file_path: &Path,
        project_root: &Path,
        issues: &[LintIssue],
    ) -> Result<()> {
        let relative_path = file_path
            .strip_prefix(project_root)
            .unwrap_or(file_path)
            .to_path_buf();

        let content_hash = file_hash(file_path)?;
        let mtime = fs::metadata(file_path)?.modified()?;
        let cached_issues: Vec<CachedIssue> =
            issues.iter().map(CachedIssue::from_lint_issue).collect();

        let entry = CacheEntry::new(content_hash, mtime, cached_issues);

        self.entries
            .entry(checker_name.to_string())
            .or_default()
            .insert(relative_path, entry);

        Ok(())
    }

    /// Remove stale entries older than max_age_days
    pub fn prune(&mut self, max_age_days: Option<u32>) {
        let max_age = Duration::from_secs(
            max_age_days.unwrap_or(DEFAULT_CACHE_MAX_AGE_DAYS) as u64 * 24 * 60 * 60,
        );
        let now = SystemTime::now();

        for checker_entries in self.entries.values_mut() {
            checker_entries.retain(|_, entry| {
                now.duration_since(entry.cached_at)
                    .map(|age| age < max_age)
                    .unwrap_or(false)
            });
        }

        // Remove empty checker entries
        self.entries.retain(|_, entries| !entries.is_empty());
    }

    /// Get cache statistics
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Reset statistics (call at start of run)
    pub fn reset_stats(&mut self) {
        self.stats = CacheStats::new();
    }

    /// Check if cache has any entries for a checker
    pub fn has_entries(&self, checker_name: &str) -> bool {
        self.entries
            .get(checker_name)
            .map(|e| !e.is_empty())
            .unwrap_or(false)
    }

    /// Get total number of cached entries across all checkers
    pub fn total_entries(&self) -> usize {
        self.entries.values().map(|e| e.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::types::Severity;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_cache_new() {
        let cache = LintCache::new();
        assert_eq!(cache.version, CACHE_VERSION);
        assert!(cache.entries.is_empty());
    }

    #[test]
    fn test_cache_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        let mut cache = LintCache::new();
        cache
            .entries
            .insert("test_checker".to_string(), HashMap::new());

        cache.save(project_root).unwrap();

        let loaded = LintCache::load(project_root).unwrap();
        assert_eq!(loaded.version, CACHE_VERSION);
        assert!(loaded.entries.contains_key("test_checker"));
    }

    #[test]
    fn test_cache_clear() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        let cache = LintCache::new();
        cache.save(project_root).unwrap();

        let cache_path = project_root.join(CACHE_DIR).join(CACHE_FILE);
        assert!(cache_path.exists());

        LintCache::clear(project_root).unwrap();
        assert!(!cache_path.exists());
    }

    #[test]
    fn test_cache_update_and_check() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create a test file
        let test_file = project_root.join("test.rs");
        let mut file = fs::File::create(&test_file).unwrap();
        writeln!(file, "fn main() {{}}").unwrap();

        let mut cache = LintCache::new();

        // Initially no cache
        assert!(cache
            .check_file("clippy", &test_file, project_root)
            .is_none());

        // Add some issues
        let issues = vec![LintIssue::new(
            test_file.clone(),
            1,
            "test issue".to_string(),
            Severity::Warning,
        )];

        cache
            .update_file("clippy", &test_file, project_root, &issues)
            .unwrap();

        // Now cache should hit
        let cached = cache.check_file("clippy", &test_file, project_root);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);
    }

    #[test]
    fn test_cache_stats() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create a test file
        let test_file = project_root.join("test.rs");
        let mut file = fs::File::create(&test_file).unwrap();
        writeln!(file, "fn main() {{}}").unwrap();

        let mut cache = LintCache::new();

        // Miss
        cache.check_file("clippy", &test_file, project_root);
        assert_eq!(cache.stats().cache_misses, 1);

        // Update cache
        cache
            .update_file("clippy", &test_file, project_root, &[])
            .unwrap();

        // Hit
        cache.check_file("clippy", &test_file, project_root);
        assert_eq!(cache.stats().cache_hits, 1);
    }

    #[test]
    fn test_cache_prune() {
        let mut cache = LintCache::new();

        // Add an old entry
        let mut checker_entries = HashMap::new();
        let old_entry = CacheEntry {
            content_hash: 12345,
            mtime: SystemTime::now() - Duration::from_secs(30 * 24 * 60 * 60), // 30 days old
            issues: vec![],
            cached_at: SystemTime::now() - Duration::from_secs(30 * 24 * 60 * 60),
        };
        checker_entries.insert(PathBuf::from("old.rs"), old_entry);
        cache.entries.insert("clippy".to_string(), checker_entries);

        assert_eq!(cache.total_entries(), 1);

        cache.prune(Some(7)); // 7 day max age

        assert_eq!(cache.total_entries(), 0);
    }
}
