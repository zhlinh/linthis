// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Format presets for different coding standards.
//!
//! Presets define formatting and linting rules based on popular style guides:
//! - Google: Google's coding standards
//! - Standard: Community standard (e.g., StandardJS for JS)
//! - Airbnb: Airbnb's style guide

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Available format presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PresetName {
    /// Google style guide
    Google,
    /// Community standard style
    Standard,
    /// Airbnb style guide
    Airbnb,
}

impl PresetName {
    /// Parse preset name from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "google" => Some(PresetName::Google),
            "standard" => Some(PresetName::Standard),
            "airbnb" => Some(PresetName::Airbnb),
            _ => None,
        }
    }

    /// Get the preset name as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            PresetName::Google => "google",
            PresetName::Standard => "standard",
            PresetName::Airbnb => "airbnb",
        }
    }

    /// List all available presets
    pub fn all() -> &'static [PresetName] {
        &[PresetName::Google, PresetName::Standard, PresetName::Airbnb]
    }
}

/// Format rules for a specific language
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LanguageRules {
    /// Indentation style ("tabs" or number of spaces)
    #[serde(default)]
    pub indent: Option<IndentStyle>,
    /// Maximum line length
    #[serde(default)]
    pub max_line_length: Option<usize>,
    /// Use semicolons (for JS/TS)
    #[serde(default)]
    pub semicolons: Option<bool>,
    /// Quote style ("single" or "double")
    #[serde(default)]
    pub quotes: Option<QuoteStyle>,
    /// Trailing commas ("none", "es5", "all")
    #[serde(default)]
    pub trailing_commas: Option<TrailingCommaStyle>,
    /// Additional linter rules to enable
    #[serde(default)]
    pub enable_rules: Vec<String>,
    /// Linter rules to disable
    #[serde(default)]
    pub disable_rules: Vec<String>,
}

/// Indentation style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IndentStyle {
    Tabs,
    Spaces(u8),
}

impl Default for IndentStyle {
    fn default() -> Self {
        IndentStyle::Spaces(4)
    }
}

/// Quote style for strings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuoteStyle {
    Single,
    #[default]
    Double,
}

/// Trailing comma style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrailingCommaStyle {
    #[default]
    None,
    Es5,
    All,
}

/// A complete format preset
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Preset {
    /// Preset name
    pub name: String,
    /// Description
    pub description: String,
    /// Per-language rules
    #[serde(default)]
    pub languages: HashMap<String, LanguageRules>,
}

impl Preset {
    /// Get the Google preset
    pub fn google() -> Self {
        let mut languages = HashMap::new();

        // JavaScript/TypeScript rules
        languages.insert(
            "javascript".to_string(),
            LanguageRules {
                indent: Some(IndentStyle::Spaces(2)),
                max_line_length: Some(80),
                semicolons: Some(true),
                quotes: Some(QuoteStyle::Single),
                trailing_commas: Some(TrailingCommaStyle::Es5),
                ..Default::default()
            },
        );

        languages.insert(
            "typescript".to_string(),
            LanguageRules {
                indent: Some(IndentStyle::Spaces(2)),
                max_line_length: Some(80),
                semicolons: Some(true),
                quotes: Some(QuoteStyle::Single),
                trailing_commas: Some(TrailingCommaStyle::Es5),
                ..Default::default()
            },
        );

        // Python rules
        languages.insert(
            "python".to_string(),
            LanguageRules {
                indent: Some(IndentStyle::Spaces(4)),
                max_line_length: Some(80),
                ..Default::default()
            },
        );

        // C++ rules
        languages.insert(
            "cpp".to_string(),
            LanguageRules {
                indent: Some(IndentStyle::Spaces(2)),
                max_line_length: Some(80),
                ..Default::default()
            },
        );

        // Java rules
        languages.insert(
            "java".to_string(),
            LanguageRules {
                indent: Some(IndentStyle::Spaces(2)),
                max_line_length: Some(100),
                ..Default::default()
            },
        );

        // Go rules
        languages.insert(
            "go".to_string(),
            LanguageRules {
                indent: Some(IndentStyle::Tabs),
                max_line_length: None, // Go doesn't enforce line length
                ..Default::default()
            },
        );

        // Rust rules
        languages.insert(
            "rust".to_string(),
            LanguageRules {
                indent: Some(IndentStyle::Spaces(4)),
                max_line_length: Some(100),
                ..Default::default()
            },
        );

        Preset {
            name: "google".to_string(),
            description: "Google's coding standards".to_string(),
            languages,
        }
    }

    /// Get the Standard preset (community standards)
    pub fn standard() -> Self {
        let mut languages = HashMap::new();

        // JavaScript/TypeScript rules (StandardJS)
        languages.insert(
            "javascript".to_string(),
            LanguageRules {
                indent: Some(IndentStyle::Spaces(2)),
                max_line_length: None,
                semicolons: Some(false), // StandardJS: no semicolons
                quotes: Some(QuoteStyle::Single),
                trailing_commas: Some(TrailingCommaStyle::None),
                ..Default::default()
            },
        );

        languages.insert(
            "typescript".to_string(),
            LanguageRules {
                indent: Some(IndentStyle::Spaces(2)),
                max_line_length: None,
                semicolons: Some(false),
                quotes: Some(QuoteStyle::Single),
                trailing_commas: Some(TrailingCommaStyle::None),
                ..Default::default()
            },
        );

        // Python rules (PEP 8)
        languages.insert(
            "python".to_string(),
            LanguageRules {
                indent: Some(IndentStyle::Spaces(4)),
                max_line_length: Some(79),
                ..Default::default()
            },
        );

        // C++ rules
        languages.insert(
            "cpp".to_string(),
            LanguageRules {
                indent: Some(IndentStyle::Spaces(4)),
                max_line_length: Some(120),
                ..Default::default()
            },
        );

        // Go rules (standard gofmt)
        languages.insert(
            "go".to_string(),
            LanguageRules {
                indent: Some(IndentStyle::Tabs),
                max_line_length: None,
                ..Default::default()
            },
        );

        // Rust rules (standard rustfmt)
        languages.insert(
            "rust".to_string(),
            LanguageRules {
                indent: Some(IndentStyle::Spaces(4)),
                max_line_length: Some(100),
                ..Default::default()
            },
        );

        Preset {
            name: "standard".to_string(),
            description: "Community standard styles (PEP 8, StandardJS, etc.)".to_string(),
            languages,
        }
    }

    /// Get the Airbnb preset
    pub fn airbnb() -> Self {
        let mut languages = HashMap::new();

        // JavaScript/TypeScript rules (Airbnb style)
        languages.insert(
            "javascript".to_string(),
            LanguageRules {
                indent: Some(IndentStyle::Spaces(2)),
                max_line_length: Some(100),
                semicolons: Some(true),
                quotes: Some(QuoteStyle::Single),
                trailing_commas: Some(TrailingCommaStyle::All),
                ..Default::default()
            },
        );

        languages.insert(
            "typescript".to_string(),
            LanguageRules {
                indent: Some(IndentStyle::Spaces(2)),
                max_line_length: Some(100),
                semicolons: Some(true),
                quotes: Some(QuoteStyle::Single),
                trailing_commas: Some(TrailingCommaStyle::All),
                ..Default::default()
            },
        );

        // Python rules
        languages.insert(
            "python".to_string(),
            LanguageRules {
                indent: Some(IndentStyle::Spaces(4)),
                max_line_length: Some(100),
                ..Default::default()
            },
        );

        // Go rules
        languages.insert(
            "go".to_string(),
            LanguageRules {
                indent: Some(IndentStyle::Tabs),
                max_line_length: Some(120),
                ..Default::default()
            },
        );

        // Rust rules
        languages.insert(
            "rust".to_string(),
            LanguageRules {
                indent: Some(IndentStyle::Spaces(4)),
                max_line_length: Some(100),
                ..Default::default()
            },
        );

        Preset {
            name: "airbnb".to_string(),
            description: "Airbnb's style guide".to_string(),
            languages,
        }
    }

    /// Load a preset by name
    pub fn load(name: PresetName) -> Self {
        match name {
            PresetName::Google => Self::google(),
            PresetName::Standard => Self::standard(),
            PresetName::Airbnb => Self::airbnb(),
        }
    }

    /// Get rules for a specific language
    pub fn get_language_rules(&self, language: &str) -> Option<&LanguageRules> {
        self.languages.get(language)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_name_parse() {
        assert_eq!(PresetName::parse("google"), Some(PresetName::Google));
        assert_eq!(PresetName::parse("GOOGLE"), Some(PresetName::Google));
        assert_eq!(PresetName::parse("standard"), Some(PresetName::Standard));
        assert_eq!(PresetName::parse("airbnb"), Some(PresetName::Airbnb));
        assert_eq!(PresetName::parse("unknown"), None);
    }

    #[test]
    fn test_google_preset() {
        let preset = Preset::google();
        assert_eq!(preset.name, "google");

        let js_rules = preset.get_language_rules("javascript").unwrap();
        assert_eq!(js_rules.indent, Some(IndentStyle::Spaces(2)));
        assert_eq!(js_rules.semicolons, Some(true));
    }

    #[test]
    fn test_standard_preset() {
        let preset = Preset::standard();
        assert_eq!(preset.name, "standard");

        let js_rules = preset.get_language_rules("javascript").unwrap();
        assert_eq!(js_rules.semicolons, Some(false)); // StandardJS has no semicolons
    }

    #[test]
    fn test_airbnb_preset() {
        let preset = Preset::airbnb();
        assert_eq!(preset.name, "airbnb");

        let js_rules = preset.get_language_rules("javascript").unwrap();
        assert_eq!(js_rules.trailing_commas, Some(TrailingCommaStyle::All));
    }
}
