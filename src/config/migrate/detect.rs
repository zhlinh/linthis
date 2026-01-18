// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Configuration detection module.

use std::path::Path;

use super::{DetectedConfig, Tool};

/// Detects existing linter/formatter configurations in a project.
pub fn detect_configs(
    project_root: &Path,
    tool_filter: Option<Tool>,
) -> Result<Vec<DetectedConfig>, String> {
    let mut detected = Vec::new();

    // ESLint config files
    if tool_filter.is_none() || tool_filter == Some(Tool::ESLint) {
        detected.extend(detect_eslint_configs(project_root)?);
    }

    // Prettier config files
    if tool_filter.is_none() || tool_filter == Some(Tool::Prettier) {
        detected.extend(detect_prettier_configs(project_root)?);
    }

    // Black/isort configs from pyproject.toml
    if tool_filter.is_none() || matches!(tool_filter, Some(Tool::Black) | Some(Tool::Isort)) {
        detected.extend(detect_python_configs(project_root, tool_filter)?);
    }

    Ok(detected)
}

/// Detect ESLint configuration files
fn detect_eslint_configs(root: &Path) -> Result<Vec<DetectedConfig>, String> {
    let mut configs = Vec::new();

    let config_names = [
        ".eslintrc.js",
        ".eslintrc.cjs",
        ".eslintrc.json",
        ".eslintrc.yml",
        ".eslintrc.yaml",
        ".eslintrc",
        "eslint.config.js",
        "eslint.config.mjs",
    ];

    for name in &config_names {
        let path = root.join(name);
        if path.exists() {
            configs.push(DetectedConfig {
                tool: Tool::ESLint,
                path,
                language: "javascript".to_string(),
            });
            break; // Only use first found config
        }
    }

    // Also check package.json for eslintConfig field
    if configs.is_empty() {
        let pkg_json = root.join("package.json");
        if pkg_json.exists() {
            if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if json.get("eslintConfig").is_some() {
                        configs.push(DetectedConfig {
                            tool: Tool::ESLint,
                            path: pkg_json,
                            language: "javascript".to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(configs)
}

/// Detect Prettier configuration files
fn detect_prettier_configs(root: &Path) -> Result<Vec<DetectedConfig>, String> {
    let mut configs = Vec::new();

    let config_names = [
        ".prettierrc",
        ".prettierrc.json",
        ".prettierrc.yml",
        ".prettierrc.yaml",
        ".prettierrc.js",
        ".prettierrc.cjs",
        "prettier.config.js",
        "prettier.config.cjs",
    ];

    for name in &config_names {
        let path = root.join(name);
        if path.exists() {
            configs.push(DetectedConfig {
                tool: Tool::Prettier,
                path,
                language: "javascript".to_string(),
            });
            break;
        }
    }

    // Check package.json for "prettier" field
    if configs.is_empty() {
        let pkg_json = root.join("package.json");
        if pkg_json.exists() {
            if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if json.get("prettier").is_some() {
                        configs.push(DetectedConfig {
                            tool: Tool::Prettier,
                            path: pkg_json,
                            language: "javascript".to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(configs)
}

/// Detect Python tool configs (Black, isort) from pyproject.toml
fn detect_python_configs(
    root: &Path,
    tool_filter: Option<Tool>,
) -> Result<Vec<DetectedConfig>, String> {
    let mut configs = Vec::new();

    let pyproject = root.join("pyproject.toml");
    if pyproject.exists() {
        if let Ok(content) = std::fs::read_to_string(&pyproject) {
            if let Ok(toml) = content.parse::<toml::Value>() {
                if let Some(tool) = toml.get("tool") {
                    // Check for [tool.black]
                    if (tool_filter.is_none() || tool_filter == Some(Tool::Black))
                        && tool.get("black").is_some()
                    {
                        configs.push(DetectedConfig {
                            tool: Tool::Black,
                            path: pyproject.clone(),
                            language: "python".to_string(),
                        });
                    }

                    // Check for [tool.isort]
                    if (tool_filter.is_none() || tool_filter == Some(Tool::Isort))
                        && tool.get("isort").is_some()
                    {
                        configs.push(DetectedConfig {
                            tool: Tool::Isort,
                            path: pyproject.clone(),
                            language: "python".to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(configs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_eslint_json_config() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(".eslintrc.json"),
            r#"{"extends": ["eslint:recommended"]}"#,
        )
        .unwrap();

        let configs = detect_configs(dir.path(), None).unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].tool, Tool::ESLint);
    }

    #[test]
    fn test_detect_prettier_config() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".prettierrc"), r#"{"tabWidth": 4}"#).unwrap();

        let configs = detect_configs(dir.path(), None).unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].tool, Tool::Prettier);
    }

    #[test]
    fn test_detect_black_config() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            r#"
[tool.black]
line-length = 100
"#,
        )
        .unwrap();

        let configs = detect_configs(dir.path(), Some(Tool::Black)).unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].tool, Tool::Black);
    }

    #[test]
    fn test_detect_no_configs() {
        let dir = TempDir::new().unwrap();
        let configs = detect_configs(dir.path(), None).unwrap();
        assert!(configs.is_empty());
    }
}
