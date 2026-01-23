// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Advisory database management for security scanning.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::vulnerability::{Advisory, Severity};

/// Advisory database for caching and querying security advisories
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AdvisoryDatabase {
    /// Cached advisories by ID
    advisories: HashMap<String, Advisory>,
    /// Last update timestamp
    last_updated: Option<String>,
    /// Database source URLs
    sources: Vec<String>,
}

impl AdvisoryDatabase {
    /// Create a new empty advisory database
    pub fn new() -> Self {
        Self::default()
    }

    /// Load database from cache
    pub fn load_from_cache() -> Result<Self, String> {
        let cache_path = Self::cache_path()?;

        if !cache_path.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(&cache_path)
            .map_err(|e| format!("Failed to read advisory cache: {}", e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse advisory cache: {}", e))
    }

    /// Save database to cache
    pub fn save_to_cache(&self) -> Result<(), String> {
        let cache_path = Self::cache_path()?;

        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create cache directory: {}", e))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize advisory cache: {}", e))?;

        std::fs::write(&cache_path, content)
            .map_err(|e| format!("Failed to write advisory cache: {}", e))
    }

    /// Get the cache file path
    fn cache_path() -> Result<PathBuf, String> {
        let dirs = directories::ProjectDirs::from("io", "linthis", "linthis")
            .ok_or_else(|| "Failed to get project directories".to_string())?;

        Ok(dirs.cache_dir().join("security").join("advisories.json"))
    }

    /// Add an advisory to the database
    pub fn add_advisory(&mut self, advisory: Advisory) {
        self.advisories.insert(advisory.id.clone(), advisory);
    }

    /// Get an advisory by ID
    pub fn get_advisory(&self, id: &str) -> Option<&Advisory> {
        self.advisories.get(id)
    }

    /// Search advisories by keyword
    pub fn search(&self, keyword: &str) -> Vec<&Advisory> {
        let keyword_lower = keyword.to_lowercase();
        self.advisories
            .values()
            .filter(|a| {
                a.id.to_lowercase().contains(&keyword_lower)
                    || a.title.to_lowercase().contains(&keyword_lower)
                    || a.description.to_lowercase().contains(&keyword_lower)
            })
            .collect()
    }

    /// Get advisories by severity
    pub fn by_severity(&self, severity: Severity) -> Vec<&Advisory> {
        self.advisories
            .values()
            .filter(|a| a.severity == severity)
            .collect()
    }

    /// Get all CVE IDs
    pub fn get_all_cve_ids(&self) -> Vec<&str> {
        self.advisories
            .values()
            .filter_map(|a| a.cve_id())
            .collect()
    }

    /// Get advisory count
    pub fn len(&self) -> usize {
        self.advisories.len()
    }

    /// Check if database is empty
    pub fn is_empty(&self) -> bool {
        self.advisories.is_empty()
    }

    /// Clear the database
    pub fn clear(&mut self) {
        self.advisories.clear();
        self.last_updated = None;
    }
}

/// Suppression list for ignoring specific vulnerabilities
#[allow(dead_code)]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SuppressionList {
    /// Suppressed vulnerability IDs with reasons
    suppressions: HashMap<String, Suppression>,
}

/// A suppression entry
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suppression {
    /// Vulnerability ID
    pub id: String,
    /// Reason for suppression
    pub reason: String,
    /// Expiration date (optional)
    pub expires: Option<String>,
    /// Who added this suppression
    pub added_by: Option<String>,
    /// When it was added
    pub added_at: Option<String>,
}

#[allow(dead_code)]
impl SuppressionList {
    /// Create a new empty suppression list
    pub fn new() -> Self {
        Self::default()
    }

    /// Load suppression list from file
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read suppression file: {}", e))?;

        // Support both JSON and TOML formats
        if path.extension().map(|e| e == "toml").unwrap_or(false) {
            toml::from_str(&content)
                .map_err(|e| format!("Failed to parse suppression TOML: {}", e))
        } else {
            serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse suppression JSON: {}", e))
        }
    }

    /// Check if a vulnerability is suppressed
    pub fn is_suppressed(&self, id: &str) -> bool {
        if let Some(suppression) = self.suppressions.get(id) {
            // Check expiration
            if let Some(ref expires) = suppression.expires {
                if let Ok(expiry) = chrono::NaiveDate::parse_from_str(expires, "%Y-%m-%d") {
                    let today = chrono::Local::now().date_naive();
                    if today > expiry {
                        return false; // Suppression expired
                    }
                }
            }
            return true;
        }
        false
    }

    /// Get suppression reason
    pub fn get_reason(&self, id: &str) -> Option<&str> {
        self.suppressions.get(id).map(|s| s.reason.as_str())
    }

    /// Add a suppression
    pub fn add(&mut self, suppression: Suppression) {
        self.suppressions.insert(suppression.id.clone(), suppression);
    }

    /// Remove a suppression
    pub fn remove(&mut self, id: &str) -> Option<Suppression> {
        self.suppressions.remove(id)
    }

    /// Get all suppressions
    pub fn all(&self) -> impl Iterator<Item = &Suppression> {
        self.suppressions.values()
    }

    /// Get count of active suppressions
    pub fn len(&self) -> usize {
        self.suppressions.len()
    }

    /// Check if list is empty
    pub fn is_empty(&self) -> bool {
        self.suppressions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advisory_database() {
        let mut db = AdvisoryDatabase::new();
        assert!(db.is_empty());

        let advisory = Advisory {
            id: "CVE-2024-1234".to_string(),
            aliases: vec![],
            title: "Test vulnerability".to_string(),
            description: "A test vulnerability description".to_string(),
            severity: Severity::High,
            cvss_score: Some(7.5),
            cvss_vector: None,
            url: None,
            published: None,
            updated: None,
            cwe_ids: vec![],
            references: vec![],
        };

        db.add_advisory(advisory);
        assert_eq!(db.len(), 1);

        let found = db.get_advisory("CVE-2024-1234");
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Test vulnerability");
    }

    #[test]
    fn test_suppression_list() {
        let mut list = SuppressionList::new();
        assert!(list.is_empty());

        list.add(Suppression {
            id: "CVE-2024-1234".to_string(),
            reason: "False positive".to_string(),
            expires: None,
            added_by: None,
            added_at: None,
        });

        assert!(list.is_suppressed("CVE-2024-1234"));
        assert!(!list.is_suppressed("CVE-2024-9999"));
    }
}
