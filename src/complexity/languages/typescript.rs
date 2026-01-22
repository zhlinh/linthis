// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! TypeScript/JavaScript complexity analyzer.

use std::path::Path;

use crate::complexity::analyzer::LanguageComplexityAnalyzer;
use crate::complexity::metrics::{ComplexityMetrics, FileMetrics, FunctionMetrics};

/// TypeScript/JavaScript complexity analyzer
pub struct TypeScriptComplexityAnalyzer;

impl TypeScriptComplexityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn analyze_function(&self, content: &str, start_line: u32, end_line: u32) -> ComplexityMetrics {
        let mut metrics = ComplexityMetrics::new();
        let lines: Vec<&str> = content.lines().collect();

        let start = start_line.saturating_sub(1) as usize;
        let end = (end_line as usize).min(lines.len());

        if start >= end {
            return metrics;
        }

        let func_lines = &lines[start..end];
        let func_content = func_lines.join("\n");

        metrics.cyclomatic = self.calculate_cyclomatic(&func_content);
        metrics.cognitive = self.calculate_cognitive(&func_content);
        metrics.max_nesting = self.calculate_nesting(&func_content);

        metrics.loc = (end - start) as u32;
        metrics.sloc = func_lines
            .iter()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty()
                    && !trimmed.starts_with("//")
                    && !trimmed.starts_with("/*")
                    && !trimmed.starts_with("*")
            })
            .count() as u32;

        // Count parameters
        if let Some(params_start) = func_content.find('(') {
            if let Some(params_end) = func_content.find(')') {
                let params = &func_content[params_start + 1..params_end];
                if !params.trim().is_empty() {
                    metrics.parameters = params.split(',').count() as u32;
                }
            }
        }

        metrics.returns = func_content.matches("return ").count() as u32
            + func_content.matches("return;").count() as u32;

        metrics
    }

    fn calculate_cyclomatic(&self, content: &str) -> u32 {
        let mut complexity = 1;

        let keywords = [
            "if ", "if(", "else if", "else {", "switch ", "case ", "for ", "for(",
            "while ", "while(", "do ", "catch ", "&&", "||", "??", "?.",
            " ? ", // ternary
        ];

        for keyword in keywords {
            complexity += content.matches(keyword).count() as u32;
        }

        complexity
    }

    fn calculate_cognitive(&self, content: &str) -> u32 {
        let mut complexity = 0;
        let mut nesting_level = 0;

        for line in content.lines() {
            let trimmed = line.trim();

            // Skip comments
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
                continue;
            }

            let opens = line.matches('{').count() as i32;
            let closes = line.matches('}').count() as i32;

            let control_keywords = [
                "if ", "if(", "else", "switch", "for ", "for(", "while ", "do ", "catch",
            ];
            for keyword in control_keywords {
                if trimmed.contains(keyword) {
                    complexity += 1 + nesting_level as u32;
                }
            }

            complexity += line.matches("&&").count() as u32;
            complexity += line.matches("||").count() as u32;

            nesting_level += opens - closes;
            if nesting_level < 0 {
                nesting_level = 0;
            }
        }

        complexity
    }

    fn calculate_nesting(&self, content: &str) -> u32 {
        let mut max_nesting: u32 = 0;
        let mut current_nesting: u32 = 0;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }

            for ch in line.chars() {
                match ch {
                    '{' => {
                        current_nesting += 1;
                        max_nesting = max_nesting.max(current_nesting);
                    }
                    '}' => {
                        current_nesting = current_nesting.saturating_sub(1);
                    }
                    _ => {}
                }
            }
        }

        max_nesting
    }
}

impl Default for TypeScriptComplexityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageComplexityAnalyzer for TypeScriptComplexityAnalyzer {
    fn name(&self) -> &str {
        "typescript-complexity"
    }

    fn extensions(&self) -> &[&str] {
        &["ts", "tsx", "js", "jsx", "mjs", "cjs"]
    }

    fn language(&self) -> &str {
        "typescript"
    }

    fn analyze_file(&self, path: &Path, content: &str) -> Result<FileMetrics, String> {
        let mut file_metrics = FileMetrics::new(path.to_path_buf(), self.language());
        let lines: Vec<&str> = content.lines().collect();

        file_metrics.metrics.loc = lines.len() as u32;
        file_metrics.metrics.sloc = lines
            .iter()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty()
                    && !trimmed.starts_with("//")
                    && !trimmed.starts_with("/*")
                    && !trimmed.starts_with("*")
            })
            .count() as u32;

        file_metrics.metrics.comment_lines = lines
            .iter()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*")
            })
            .count() as u32;

        file_metrics.imports = lines
            .iter()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("import ")
                    || trimmed.starts_with("const ")
                        && trimmed.contains("require(")
            })
            .count() as u32;

        // Find and analyze functions
        let mut in_function = false;
        let mut function_start = 0;
        let mut function_name = String::new();
        let mut brace_count = 0;
        let mut current_class: Option<String> = None;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Track classes
            if trimmed.starts_with("class ") || trimmed.contains(" class ") {
                if let Some(name) = extract_class_name(trimmed) {
                    current_class = Some(name);
                    file_metrics.classes += 1;
                }
            }

            // Detect function definitions
            if !in_function {
                if let Some(name) = detect_ts_function(trimmed) {
                    in_function = true;
                    function_start = i + 1;
                    function_name = name;
                    brace_count = 0;
                }
            }

            if in_function {
                brace_count += line.matches('{').count() as i32;
                brace_count -= line.matches('}').count() as i32;

                if brace_count <= 0 && (line.contains('}') || trimmed.ends_with("};")) {
                    let end_line = i + 1;
                    let metrics =
                        self.analyze_function(content, function_start as u32, end_line as u32);

                    let mut func = FunctionMetrics::new(
                        &function_name,
                        function_start as u32,
                        end_line as u32,
                    );
                    func.metrics = metrics;
                    func.parent = current_class.clone();

                    file_metrics.functions.push(func);
                    in_function = false;
                }
            }
        }

        // Aggregate file-level metrics
        if !file_metrics.functions.is_empty() {
            file_metrics.metrics.cyclomatic = file_metrics
                .functions
                .iter()
                .map(|f| f.metrics.cyclomatic)
                .sum();
            file_metrics.metrics.cognitive = file_metrics
                .functions
                .iter()
                .map(|f| f.metrics.cognitive)
                .sum();
            file_metrics.metrics.max_nesting = file_metrics
                .functions
                .iter()
                .map(|f| f.metrics.max_nesting)
                .max()
                .unwrap_or(0);
        }

        Ok(file_metrics)
    }
}

fn detect_ts_function(line: &str) -> Option<String> {
    let line = line.trim();

    // Arrow function: const foo = () => or const foo = async () =>
    if (line.starts_with("const ") || line.starts_with("let ") || line.starts_with("var "))
        && (line.contains("=>") || line.contains("= function"))
    {
        let parts: Vec<&str> = line.split(|c| c == '=' || c == ':').collect();
        if !parts.is_empty() {
            let name = parts[0]
                .trim()
                .trim_start_matches("const ")
                .trim_start_matches("let ")
                .trim_start_matches("var ")
                .trim_start_matches("export ");
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }

    // Function declaration: function foo() or async function foo()
    if line.contains("function ") {
        let start = line.find("function ")? + 9;
        let rest = &line[start..];
        let end = rest.find(|c: char| c == '(' || c == '<' || c.is_whitespace())?;
        let name = rest[..end].trim();
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }

    // Class method: methodName() { or async methodName() {
    if line.contains('(') && (line.ends_with('{') || line.ends_with(") {")) {
        let clean = line
            .trim_start_matches("public ")
            .trim_start_matches("private ")
            .trim_start_matches("protected ")
            .trim_start_matches("static ")
            .trim_start_matches("async ")
            .trim_start_matches("override ");

        if let Some(paren_pos) = clean.find('(') {
            let name = clean[..paren_pos].trim();
            if !name.is_empty()
                && !name.contains(' ')
                && name != "if"
                && name != "for"
                && name != "while"
                && name != "switch"
            {
                return Some(name.to_string());
            }
        }
    }

    None
}

fn extract_class_name(line: &str) -> Option<String> {
    let start = line.find("class ")? + 6;
    let rest = &line[start..];
    let end = rest.find(|c: char| c == '<' || c == '{' || c == ' ')?;
    let name = rest[..end].trim();
    if !name.is_empty() {
        Some(name.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typescript_analyzer_creation() {
        let analyzer = TypeScriptComplexityAnalyzer::new();
        assert_eq!(analyzer.language(), "typescript");
        assert!(analyzer.extensions().contains(&"ts"));
        assert!(analyzer.extensions().contains(&"js"));
    }

    #[test]
    fn test_detect_function() {
        assert_eq!(
            detect_ts_function("function hello() {"),
            Some("hello".to_string())
        );
        assert_eq!(
            detect_ts_function("const foo = () => {"),
            Some("foo".to_string())
        );
        assert_eq!(
            detect_ts_function("async function bar() {"),
            Some("bar".to_string())
        );
    }

    #[test]
    fn test_analyze_simple_function() {
        let analyzer = TypeScriptComplexityAnalyzer::new();
        let content = r#"
function simple() {
    const x = 1;
}
"#;
        let result = analyzer.analyze_file(Path::new("test.ts"), content);
        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert_eq!(metrics.functions.len(), 1);
    }
}
