// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Black and isort configuration parser (from pyproject.toml).

use std::path::Path;

/// Parsed Black configuration
#[derive(Debug, Default)]
pub struct BlackConfig {
    pub line_length: Option<u32>,
    pub target_version: Vec<String>,
    pub include: Option<String>,
    pub exclude: Option<String>,
    pub extend_exclude: Option<String>,
    pub skip_string_normalization: bool,
    pub skip_magic_trailing_comma: bool,
}

/// Parsed isort configuration
#[derive(Debug, Default)]
pub struct IsortConfig {
    pub profile: Option<String>,
    pub line_length: Option<u32>,
    pub known_first_party: Vec<String>,
    pub known_third_party: Vec<String>,
    pub skip: Vec<String>,
    pub skip_glob: Vec<String>,
    pub force_single_line: bool,
    pub combine_as_imports: bool,
    pub sections: Vec<String>,
}

/// Parse Black configuration from pyproject.toml
pub fn parse_black(path: &Path) -> Result<BlackConfig, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    let toml: toml::Value = content
        .parse()
        .map_err(|e| format!("Invalid TOML: {}", e))?;

    let mut config = BlackConfig::default();

    if let Some(black) = toml.get("tool").and_then(|t| t.get("black")) {
        config.line_length = black
            .get("line-length")
            .and_then(|v| v.as_integer())
            .map(|n| n as u32);

        config.target_version = black
            .get("target-version")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        config.include = black
            .get("include")
            .and_then(|v| v.as_str())
            .map(String::from);
        config.exclude = black
            .get("exclude")
            .and_then(|v| v.as_str())
            .map(String::from);
        config.extend_exclude = black
            .get("extend-exclude")
            .and_then(|v| v.as_str())
            .map(String::from);

        config.skip_string_normalization = black
            .get("skip-string-normalization")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        config.skip_magic_trailing_comma = black
            .get("skip-magic-trailing-comma")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
    }

    Ok(config)
}

/// Parse isort configuration from pyproject.toml
pub fn parse_isort(path: &Path) -> Result<IsortConfig, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    let toml: toml::Value = content
        .parse()
        .map_err(|e| format!("Invalid TOML: {}", e))?;

    let mut config = IsortConfig::default();

    if let Some(isort) = toml.get("tool").and_then(|t| t.get("isort")) {
        config.profile = isort
            .get("profile")
            .and_then(|v| v.as_str())
            .map(String::from);

        config.line_length = isort
            .get("line_length")
            .and_then(|v| v.as_integer())
            .map(|n| n as u32);

        config.known_first_party = extract_string_array(isort, "known_first_party");
        config.known_third_party = extract_string_array(isort, "known_third_party");
        config.skip = extract_string_array(isort, "skip");
        config.skip_glob = extract_string_array(isort, "skip_glob");
        config.sections = extract_string_array(isort, "sections");

        config.force_single_line = isort
            .get("force_single_line")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        config.combine_as_imports = isort
            .get("combine_as_imports")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
    }

    Ok(config)
}

fn extract_string_array(table: &toml::Value, key: &str) -> Vec<String> {
    table
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_black_config() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("pyproject.toml");
        std::fs::write(
            &path,
            r#"
[tool.black]
line-length = 100
target-version = ["py38", "py39"]
skip-string-normalization = true
"#,
        )
        .unwrap();

        let config = parse_black(&path).unwrap();
        assert_eq!(config.line_length, Some(100));
        assert_eq!(config.target_version, vec!["py38", "py39"]);
        assert!(config.skip_string_normalization);
    }

    #[test]
    fn test_parse_isort_config() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("pyproject.toml");
        std::fs::write(
            &path,
            r#"
[tool.isort]
profile = "black"
line_length = 100
known_first_party = ["myapp", "mylib"]
force_single_line = true
"#,
        )
        .unwrap();

        let config = parse_isort(&path).unwrap();
        assert_eq!(config.profile, Some("black".to_string()));
        assert_eq!(config.line_length, Some(100));
        assert_eq!(config.known_first_party, vec!["myapp", "mylib"]);
        assert!(config.force_single_line);
    }
}
