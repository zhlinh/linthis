// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Language detection and extension mapping utilities.

use crate::Language;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

// Extension to language mapping.
lazy_static::lazy_static! {
    static ref EXTENSION_MAP: HashMap<&'static str, Language> = {
        let mut m = HashMap::new();
        // C++
        m.insert("c", Language::Cpp);
        m.insert("cc", Language::Cpp);
        m.insert("cpp", Language::Cpp);
        m.insert("cxx", Language::Cpp);
        m.insert("h", Language::Cpp);
        m.insert("hpp", Language::Cpp);
        m.insert("hxx", Language::Cpp);
        // Objective-C
        m.insert("m", Language::ObjectiveC);
        m.insert("mm", Language::ObjectiveC);
        // Java
        m.insert("java", Language::Java);
        // Python
        m.insert("py", Language::Python);
        m.insert("pyw", Language::Python);
        // Rust
        m.insert("rs", Language::Rust);
        // Go
        m.insert("go", Language::Go);
        // JavaScript
        m.insert("js", Language::JavaScript);
        m.insert("jsx", Language::JavaScript);
        m.insert("mjs", Language::JavaScript);
        m.insert("cjs", Language::JavaScript);
        // TypeScript
        m.insert("ts", Language::TypeScript);
        m.insert("tsx", Language::TypeScript);
        m.insert("mts", Language::TypeScript);
        m.insert("cts", Language::TypeScript);
        m
    };
}

/// Get language from file extension.
pub fn language_from_extension(ext: &str) -> Option<Language> {
    EXTENSION_MAP.get(ext.to_lowercase().as_str()).copied()
}

/// Get language from file path.
pub fn language_from_path(path: &Path) -> Option<Language> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(language_from_extension)
}

/// Detect languages used in a directory by counting file extensions.
pub fn detect_languages(root: &Path) -> Vec<Language> {
    let mut counts: HashMap<Language, usize> = HashMap::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        if let Some(lang) = language_from_path(entry.path()) {
            *counts.entry(lang).or_insert(0) += 1;
        }
    }

    // Sort by count (descending) and return languages
    let mut langs: Vec<_> = counts.into_iter().collect();
    langs.sort_by(|a, b| b.1.cmp(&a.1));
    langs.into_iter().map(|(lang, _)| lang).collect()
}

/// Get the primary language of a project (most common by file count).
pub fn detect_primary_language(root: &Path) -> Option<Language> {
    detect_languages(root).into_iter().next()
}

/// Parse language names from comma-separated string.
pub fn parse_languages(input: &str) -> Vec<Language> {
    input
        .split(',')
        .filter_map(|s| Language::from_name(s.trim()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_mapping() {
        assert_eq!(language_from_extension("rs"), Some(Language::Rust));
        assert_eq!(language_from_extension("py"), Some(Language::Python));
        assert_eq!(language_from_extension("ts"), Some(Language::TypeScript));
        assert_eq!(language_from_extension("cpp"), Some(Language::Cpp));
        assert_eq!(language_from_extension("unknown"), None);
    }

    #[test]
    fn test_parse_languages() {
        let langs = parse_languages("rust,python,typescript");
        assert_eq!(langs.len(), 3);
        assert!(langs.contains(&Language::Rust));
        assert!(langs.contains(&Language::Python));
        assert!(langs.contains(&Language::TypeScript));
    }
}
