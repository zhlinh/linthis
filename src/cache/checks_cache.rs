// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Per-file cache for security and complexity checks.
//!
//! Stores results per file keyed by content hash (xxHash64).
//! Only changed files are re-scanned; cached results are reused.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::hash::file_hash;
use crate::security::sast::SastFinding;

/// Per-file cache for checks (security, complexity).
#[derive(Serialize, Deserialize, Default)]
pub struct PerFileCache {
    /// linthis version that produced these results. The content hash only
    /// proves the input is unchanged; a fixed analyzer can report something
    /// different for the same bytes, so another version's results are dropped.
    #[serde(default)]
    pub linthis_version: String,
    /// file_path → (content_hash, serialized results JSON)
    pub entries: HashMap<String, (u64, String)>,
}

/// Result of partitioning files into cached and changed.
pub struct PartitionResult {
    /// Files that need re-scanning
    pub changed: Vec<PathBuf>,
    /// Cached findings from unchanged files
    pub cached_findings: Vec<SastFinding>,
    /// Number of cache hits
    pub cache_hits: usize,
    /// Number of cache misses
    pub cache_misses: usize,
}

impl PerFileCache {
    pub fn load(path: &Path) -> Self {
        let cache: Self = std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        if cache.linthis_version != env!("CARGO_PKG_VERSION") {
            return Self::new();
        }
        cache
    }

    /// An empty cache stamped with the running version.
    pub fn new() -> Self {
        Self {
            linthis_version: env!("CARGO_PKG_VERSION").to_string(),
            entries: HashMap::new(),
        }
    }

    pub fn save(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Stamp on the way out, so a cache built from a default value is still
        // labelled with the version that wrote it.
        #[derive(Serialize)]
        struct Stamped<'a> {
            linthis_version: &'a str,
            entries: &'a HashMap<String, (u64, String)>,
        }
        let stamped = Stamped {
            linthis_version: env!("CARGO_PKG_VERSION"),
            entries: &self.entries,
        };
        if let Ok(json) = serde_json::to_string(&stamped) {
            let _ = std::fs::write(path, json);
        }
    }

    /// Partition files into changed (need scan) and cached (reuse findings).
    pub fn partition_files(&self, files: &[PathBuf], no_cache: bool) -> PartitionResult {
        let mut result = PartitionResult {
            changed: Vec::new(),
            cached_findings: Vec::new(),
            cache_hits: 0,
            cache_misses: 0,
        };

        for file in files {
            let key = file.to_string_lossy().to_string();

            if no_cache {
                result.changed.push(file.clone());
                result.cache_misses += 1;
                continue;
            }

            let current_hash = match file_hash(file) {
                Ok(h) => h,
                Err(_) => {
                    result.changed.push(file.clone());
                    result.cache_misses += 1;
                    continue;
                }
            };

            match self.entries.get(&key) {
                Some((cached_hash, findings_json)) if *cached_hash == current_hash => {
                    result.cache_hits += 1;
                    if let Ok(findings) = serde_json::from_str::<Vec<SastFinding>>(findings_json) {
                        result.cached_findings.extend(findings);
                    }
                }
                _ => {
                    result.changed.push(file.clone());
                    result.cache_misses += 1;
                }
            }
        }

        result
    }

    /// Update cache entries from SAST scan results.
    pub fn update_from_sast(
        &mut self,
        files: &[PathBuf],
        sast_result: &crate::security::sast::SastResult,
    ) {
        for file in files {
            let key = file.to_string_lossy().to_string();
            let hash = file_hash(file).unwrap_or(0);

            let file_findings: Vec<&SastFinding> = sast_result
                .findings
                .iter()
                .filter(|f| f.file_path == *file)
                .collect();

            let json = serde_json::to_string(&file_findings).unwrap_or_else(|_| "[]".to_string());
            self.entries.insert(key, (hash, json));
        }
    }

    /// Update cache entries from complexity analysis results.
    pub fn update_from_complexity(
        &mut self,
        files: &[PathBuf],
        analysis: &crate::complexity::AnalysisResult,
    ) {
        for file in files {
            let key = file.to_string_lossy().to_string();
            let hash = file_hash(file).unwrap_or(0);

            let file_metrics = analysis.files.iter().find(|f| f.path == *file);

            let json = if let Some(metrics) = file_metrics {
                serde_json::to_string(metrics).unwrap_or_else(|_| "null".to_string())
            } else {
                "null".to_string()
            };
            self.entries.insert(key, (hash, json));
        }
    }

    /// Get cached FileMetrics for files that had cache hits.
    ///
    /// Returns FileMetrics for all cached files (used by complexity check
    /// to restore results without re-analyzing).
    pub fn get_cached_file_metrics(
        &self,
        files: &[PathBuf],
    ) -> Vec<crate::complexity::FileMetrics> {
        let mut metrics = Vec::new();
        for file in files {
            let key = file.to_string_lossy().to_string();
            let current_hash = file_hash(file).unwrap_or(0);
            if let Some((cached_hash, json)) = self.entries.get(&key) {
                if *cached_hash == current_hash {
                    if let Ok(fm) = serde_json::from_str::<crate::complexity::FileMetrics>(json) {
                        metrics.push(fm);
                    }
                }
            }
        }
        metrics
    }

    /// Format cache status message (like lint's "Running [X] check (N cached, N changed)").
    pub fn format_status(check_name: &str, partition: &PartitionResult) -> String {
        let total = partition.cache_hits + partition.cache_misses;
        if total == 0 {
            return format!("Running [{}] check...", check_name);
        }
        if partition.cache_misses == 0 {
            format!(
                "Running [{}] check ({} cached, 0 changed)",
                check_name, partition.cache_hits
            )
        } else if partition.cache_hits == 0 {
            format!("Running [{}] check...", check_name)
        } else {
            format!(
                "Running [{}] check ({} cached, {} changed)...",
                check_name, partition.cache_hits, partition.cache_misses
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_from_another_linthis_version_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("checks.json");

        let mut cache = PerFileCache::new();
        cache.entries.insert("a.rs".into(), (42, "[]".into()));
        cache.save(&path);
        // Same version → the entry survives a round trip.
        assert_eq!(PerFileCache::load(&path).entries.len(), 1);

        // A cache written by an older binary must not be served: its results
        // can be wrong in ways the content hash cannot detect.
        let stale = std::fs::read_to_string(&path)
            .unwrap()
            .replace(env!("CARGO_PKG_VERSION"), "0.0.1-old");
        std::fs::write(&path, stale).unwrap();
        assert!(PerFileCache::load(&path).entries.is_empty());
    }
}
