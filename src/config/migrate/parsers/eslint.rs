// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! ESLint configuration parser.

use regex::Regex;
use std::path::Path;

/// Parsed ESLint rule value
#[derive(Debug, Clone)]
pub enum RuleValue {
    Off,
    Warn,
    Error,
    WithOptions(String, Vec<serde_json::Value>),
}

impl RuleValue {
    pub fn to_js_string(&self) -> String {
        match self {
            RuleValue::Off => "'off'".to_string(),
            RuleValue::Warn => "'warn'".to_string(),
            RuleValue::Error => "'error'".to_string(),
            RuleValue::WithOptions(level, opts) => {
                let opts_str: Vec<String> = opts
                    .iter()
                    .map(|o| serde_json::to_string(o).unwrap_or_default())
                    .collect();
                format!("['{}', {}]", level.to_lowercase(), opts_str.join(", "))
            }
        }
    }
}

/// Parsed ESLint configuration
#[derive(Debug, Default)]
pub struct ESLintConfig {
    pub extends: Vec<String>,
    pub rules: Vec<(String, RuleValue)>,
    pub env: Vec<String>,
    pub parser: Option<String>,
    pub plugins: Vec<String>,
    pub ignores: Vec<String>,
}

/// Parse ESLint configuration from a file
pub fn parse(path: &Path) -> Result<ESLintConfig, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Handle package.json separately
    if filename == "package.json" {
        return parse_from_package_json(&content);
    }

    match ext {
        "json" | "" => parse_json(&content),
        "yml" | "yaml" => parse_yaml(&content),
        "js" | "cjs" | "mjs" => parse_js(&content),
        _ => Ok(ESLintConfig::default()),
    }
}

fn parse_json(content: &str) -> Result<ESLintConfig, String> {
    let json: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("Invalid JSON: {}", e))?;
    extract_config_from_value(&json)
}

fn parse_yaml(content: &str) -> Result<ESLintConfig, String> {
    let yaml: serde_json::Value =
        serde_yaml::from_str(content).map_err(|e| format!("Invalid YAML: {}", e))?;
    extract_config_from_value(&yaml)
}

fn parse_from_package_json(content: &str) -> Result<ESLintConfig, String> {
    let json: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("Invalid JSON: {}", e))?;
    if let Some(eslint_config) = json.get("eslintConfig") {
        extract_config_from_value(eslint_config)
    } else {
        Ok(ESLintConfig::default())
    }
}

fn parse_js(content: &str) -> Result<ESLintConfig, String> {
    // For JS configs, we attempt a best-effort extraction
    // using simple regex patterns for common patterns
    let mut config = ESLintConfig::default();

    // Extract extends array
    if let Some(extends) = extract_js_array(content, "extends") {
        config.extends = extends;
    }

    // Extract plugins array
    if let Some(plugins) = extract_js_array(content, "plugins") {
        config.plugins = plugins;
    }

    // Extract env object (look for enabled envs)
    if let Some(env_section) = extract_js_object_section(content, "env") {
        config.env = parse_env_from_js(&env_section);
    }

    // Extract rules object
    if let Some(rules_section) = extract_js_object_section(content, "rules") {
        config.rules = parse_rules_from_js(&rules_section);
    }

    Ok(config)
}

fn extract_config_from_value(value: &serde_json::Value) -> Result<ESLintConfig, String> {
    let mut config = ESLintConfig::default();

    // Extract extends
    if let Some(extends) = value.get("extends") {
        if let Some(arr) = extends.as_array() {
            config.extends = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        } else if let Some(s) = extends.as_str() {
            config.extends.push(s.to_string());
        }
    }

    // Extract rules
    if let Some(rules) = value.get("rules").and_then(|r| r.as_object()) {
        for (name, rule_value) in rules {
            let parsed = parse_rule_value(rule_value);
            config.rules.push((name.clone(), parsed));
        }
    }

    // Extract env
    if let Some(env) = value.get("env").and_then(|e| e.as_object()) {
        config.env = env
            .iter()
            .filter(|(_, v)| v.as_bool() == Some(true))
            .map(|(k, _)| k.clone())
            .collect();
    }

    // Extract plugins
    if let Some(plugins) = value.get("plugins").and_then(|p| p.as_array()) {
        config.plugins = plugins
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
    }

    // Extract parser
    config.parser = value.get("parser").and_then(|p| p.as_str()).map(String::from);

    // Extract ignorePatterns
    if let Some(ignores) = value.get("ignorePatterns").and_then(|i| i.as_array()) {
        config.ignores = ignores
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
    }

    Ok(config)
}

fn parse_rule_value(value: &serde_json::Value) -> RuleValue {
    match value {
        serde_json::Value::String(s) => match s.as_str() {
            "off" => RuleValue::Off,
            "warn" => RuleValue::Warn,
            "error" => RuleValue::Error,
            _ => RuleValue::Off,
        },
        serde_json::Value::Number(n) => match n.as_u64() {
            Some(0) => RuleValue::Off,
            Some(1) => RuleValue::Warn,
            Some(2) => RuleValue::Error,
            _ => RuleValue::Off,
        },
        serde_json::Value::Array(arr) if !arr.is_empty() => {
            let level = parse_rule_value(&arr[0]);
            let level_str = match &level {
                RuleValue::Off => "off",
                RuleValue::Warn => "warn",
                RuleValue::Error => "error",
                _ => "off",
            };
            if arr.len() > 1 {
                RuleValue::WithOptions(level_str.to_string(), arr[1..].to_vec())
            } else {
                level
            }
        }
        _ => RuleValue::Off,
    }
}

// Helper functions for JS parsing
fn extract_js_array(content: &str, key: &str) -> Option<Vec<String>> {
    // Match patterns like: extends: ['a', 'b'] or extends: ["a", "b"]
    let pattern = format!(r#"{}:\s*\[([^\]]+)\]"#, key);
    let re = Regex::new(&pattern).ok()?;
    let captures = re.captures(content)?;
    let arr_content = captures.get(1)?.as_str();

    Some(
        arr_content
            .split(',')
            .filter_map(|s| {
                let trimmed = s.trim().trim_matches(|c| c == '\'' || c == '"');
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
            .collect(),
    )
}

fn extract_js_object_section(content: &str, key: &str) -> Option<String> {
    // Find the object after key:
    let key_pattern = format!(r"{}:\s*\{{", key);
    let start_match = content.find(&key_pattern.replace(r"\{", "{"))?;
    let rest = &content[start_match..];

    // Find opening brace
    let brace_start = rest.find('{')?;
    let rest = &rest[brace_start..];

    // Simple brace matching (doesn't handle strings with braces perfectly)
    let mut depth = 0;
    let mut end = 0;
    for (i, c) in rest.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = i + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    if end > 0 {
        Some(rest[..end].to_string())
    } else {
        None
    }
}

fn parse_env_from_js(env_section: &str) -> Vec<String> {
    let mut envs = Vec::new();

    // Match patterns like 'browser': true or browser: true
    let re = Regex::new(r#"['"]?(\w+)['"]?\s*:\s*true"#).unwrap();
    for cap in re.captures_iter(env_section) {
        if let Some(env_name) = cap.get(1) {
            envs.push(env_name.as_str().to_string());
        }
    }

    envs
}

fn parse_rules_from_js(rules_section: &str) -> Vec<(String, RuleValue)> {
    let mut rules = Vec::new();

    // Match patterns like 'rule-name': 'error' or "rule-name": 2
    // First try quoted values, then numeric values
    let re_quoted = Regex::new(r#"['"]([^'"]+)['"]\s*:\s*['"]?(off|warn|error)['"]?"#).unwrap();
    let re_numeric = Regex::new(r#"['"]([^'"]+)['"]\s*:\s*(0|1|2)"#).unwrap();

    for cap in re_quoted.captures_iter(rules_section) {
        let name = cap.get(1).map(|m| m.as_str().to_string());
        let value_str = cap.get(2).map(|m| m.as_str());

        if let (Some(name), Some(value)) = (name, value_str) {
            let rule_value = match value {
                "off" => RuleValue::Off,
                "warn" => RuleValue::Warn,
                "error" => RuleValue::Error,
                _ => RuleValue::Off,
            };
            rules.push((name, rule_value));
        }
    }

    for cap in re_numeric.captures_iter(rules_section) {
        let name = cap.get(1).map(|m| m.as_str().to_string());
        let value_str = cap.get(2).map(|m| m.as_str());

        if let (Some(name), Some(value)) = (name, value_str) {
            // Skip if we already have this rule from the quoted regex
            if rules.iter().any(|(n, _)| n == &name) {
                continue;
            }
            let rule_value = match value {
                "0" => RuleValue::Off,
                "1" => RuleValue::Warn,
                "2" => RuleValue::Error,
                _ => RuleValue::Off,
            };
            rules.push((name, rule_value));
        }
    }

    rules
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_config() {
        let json = r#"{
            "extends": ["eslint:recommended"],
            "rules": {
                "semi": "error",
                "no-unused-vars": "warn"
            },
            "env": {
                "browser": true,
                "node": false
            }
        }"#;

        let config = parse_json(json).unwrap();
        assert_eq!(config.extends, vec!["eslint:recommended"]);
        assert_eq!(config.rules.len(), 2);
        assert_eq!(config.env, vec!["browser"]);
    }

    #[test]
    fn test_parse_rule_value() {
        assert!(matches!(
            parse_rule_value(&serde_json::json!("error")),
            RuleValue::Error
        ));
        assert!(matches!(
            parse_rule_value(&serde_json::json!(2)),
            RuleValue::Error
        ));
        assert!(matches!(
            parse_rule_value(&serde_json::json!(["error", {"allow": ["warn"]}])),
            RuleValue::WithOptions(_, _)
        ));
    }
}
