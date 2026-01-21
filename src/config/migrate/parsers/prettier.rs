// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Prettier configuration parser.

use regex::Regex;
use std::path::Path;

/// Parsed Prettier configuration
#[derive(Debug, Default)]
pub struct PrettierConfig {
    pub tab_width: Option<u32>,
    pub use_tabs: bool,
    pub semi: Option<bool>,
    pub single_quote: Option<bool>,
    pub trailing_comma: Option<String>,
    pub bracket_spacing: Option<bool>,
    pub print_width: Option<u32>,
    pub arrow_parens: Option<String>,
    pub end_of_line: Option<String>,
    pub prose_wrap: Option<String>,
}

impl PrettierConfig {
    /// Convert to JavaScript object notation
    pub fn to_js_object(&self) -> String {
        let mut lines = Vec::new();

        if let Some(width) = self.tab_width {
            lines.push(format!("  tabWidth: {},", width));
        }
        if self.use_tabs {
            lines.push("  useTabs: true,".to_string());
        }
        if let Some(semi) = self.semi {
            lines.push(format!("  semi: {},", semi));
        }
        if let Some(sq) = self.single_quote {
            lines.push(format!("  singleQuote: {},", sq));
        }
        if let Some(ref tc) = self.trailing_comma {
            lines.push(format!("  trailingComma: '{}',", tc));
        }
        if let Some(bs) = self.bracket_spacing {
            lines.push(format!("  bracketSpacing: {},", bs));
        }
        if let Some(pw) = self.print_width {
            lines.push(format!("  printWidth: {},", pw));
        }
        if let Some(ref ap) = self.arrow_parens {
            lines.push(format!("  arrowParens: '{}',", ap));
        }
        if let Some(ref eol) = self.end_of_line {
            lines.push(format!("  endOfLine: '{}',", eol));
        }
        if let Some(ref pw) = self.prose_wrap {
            lines.push(format!("  proseWrap: '{}',", pw));
        }

        if lines.is_empty() {
            "{}".to_string()
        } else {
            format!("{{\n{}\n}}", lines.join("\n"))
        }
    }
}

/// Parse Prettier configuration from a file
pub fn parse(path: &Path) -> Result<PrettierConfig, String> {
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
        "js" | "cjs" => parse_js(&content),
        _ => Ok(PrettierConfig::default()),
    }
}

fn parse_json(content: &str) -> Result<PrettierConfig, String> {
    let json: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("Invalid JSON: {}", e))?;
    extract_prettier_config(&json)
}

fn parse_yaml(content: &str) -> Result<PrettierConfig, String> {
    let yaml: serde_json::Value =
        serde_yaml::from_str(content).map_err(|e| format!("Invalid YAML: {}", e))?;
    extract_prettier_config(&yaml)
}

fn parse_from_package_json(content: &str) -> Result<PrettierConfig, String> {
    let json: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("Invalid JSON: {}", e))?;
    if let Some(prettier) = json.get("prettier") {
        extract_prettier_config(prettier)
    } else {
        Ok(PrettierConfig::default())
    }
}

fn parse_js(content: &str) -> Result<PrettierConfig, String> {
    // Best-effort extraction from JS module exports
    let mut config = PrettierConfig::default();

    // Extract numeric values
    if let Some(val) = extract_js_number(content, "tabWidth") {
        config.tab_width = Some(val);
    }
    if let Some(val) = extract_js_number(content, "printWidth") {
        config.print_width = Some(val);
    }

    // Extract boolean values
    if let Some(val) = extract_js_bool(content, "semi") {
        config.semi = Some(val);
    }
    if let Some(val) = extract_js_bool(content, "singleQuote") {
        config.single_quote = Some(val);
    }
    if let Some(val) = extract_js_bool(content, "useTabs") {
        config.use_tabs = val;
    }
    if let Some(val) = extract_js_bool(content, "bracketSpacing") {
        config.bracket_spacing = Some(val);
    }

    // Extract string values
    if let Some(val) = extract_js_string(content, "trailingComma") {
        config.trailing_comma = Some(val);
    }
    if let Some(val) = extract_js_string(content, "arrowParens") {
        config.arrow_parens = Some(val);
    }
    if let Some(val) = extract_js_string(content, "endOfLine") {
        config.end_of_line = Some(val);
    }
    if let Some(val) = extract_js_string(content, "proseWrap") {
        config.prose_wrap = Some(val);
    }

    Ok(config)
}

fn extract_prettier_config(value: &serde_json::Value) -> Result<PrettierConfig, String> {
    let config = PrettierConfig {
        tab_width: value
            .get("tabWidth")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        use_tabs: value
            .get("useTabs")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        semi: value.get("semi").and_then(|v| v.as_bool()),
        single_quote: value.get("singleQuote").and_then(|v| v.as_bool()),
        trailing_comma: value
            .get("trailingComma")
            .and_then(|v| v.as_str())
            .map(String::from),
        bracket_spacing: value.get("bracketSpacing").and_then(|v| v.as_bool()),
        print_width: value
            .get("printWidth")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        arrow_parens: value
            .get("arrowParens")
            .and_then(|v| v.as_str())
            .map(String::from),
        end_of_line: value
            .get("endOfLine")
            .and_then(|v| v.as_str())
            .map(String::from),
        prose_wrap: value
            .get("proseWrap")
            .and_then(|v| v.as_str())
            .map(String::from),
    };

    Ok(config)
}

// JS extraction helpers
fn extract_js_number(content: &str, key: &str) -> Option<u32> {
    let re = Regex::new(&format!(r#"{}:\s*(\d+)"#, key)).ok()?;
    re.captures(content)?.get(1)?.as_str().parse().ok()
}

fn extract_js_bool(content: &str, key: &str) -> Option<bool> {
    let re = Regex::new(&format!(r#"{}:\s*(true|false)"#, key)).ok()?;
    re.captures(content)?.get(1)?.as_str().parse().ok()
}

fn extract_js_string(content: &str, key: &str) -> Option<String> {
    let re = Regex::new(&format!(r#"{}:\s*['"]([^'"]+)['"]"#, key)).ok()?;
    Some(re.captures(content)?.get(1)?.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_config() {
        let json = r#"{
            "tabWidth": 4,
            "semi": true,
            "singleQuote": true,
            "trailingComma": "es5"
        }"#;

        let config = parse_json(json).unwrap();
        assert_eq!(config.tab_width, Some(4));
        assert_eq!(config.semi, Some(true));
        assert_eq!(config.single_quote, Some(true));
        assert_eq!(config.trailing_comma, Some("es5".to_string()));
    }

    #[test]
    fn test_to_js_object() {
        let config = PrettierConfig {
            tab_width: Some(4),
            semi: Some(true),
            single_quote: Some(true),
            ..Default::default()
        };

        let js = config.to_js_object();
        assert!(js.contains("tabWidth: 4"));
        assert!(js.contains("semi: true"));
        assert!(js.contains("singleQuote: true"));
    }
}
