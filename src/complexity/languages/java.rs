// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Java complexity analyzer.

use std::path::Path;

use crate::complexity::analyzer::LanguageComplexityAnalyzer;
use crate::complexity::metrics::{ComplexityMetrics, FileMetrics, FunctionMetrics};

/// Java complexity analyzer
pub struct JavaComplexityAnalyzer;

impl JavaComplexityAnalyzer {
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
            "while ", "while(", "do ", "catch ", "&&", "||", " ? ", // ternary
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

            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
                continue;
            }

            let opens = line.matches('{').count() as i32;
            let closes = line.matches('}').count() as i32;

            let control_keywords = [
                "if ", "if(", "else", "switch", "for ", "for(", "while ", "do ", "catch", "try ",
            ];
            for keyword in control_keywords {
                if trimmed.starts_with(keyword) || trimmed.contains(&format!(" {}", keyword)) {
                    complexity += 1 + nesting_level as u32;
                }
            }

            complexity += line.matches("&&").count() as u32;
            complexity += line.matches("||").count() as u32;

            // Lambda adds complexity
            if line.contains("->") && line.contains('{') {
                complexity += 1;
            }

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
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
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

impl Default for JavaComplexityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageComplexityAnalyzer for JavaComplexityAnalyzer {
    fn name(&self) -> &str {
        "java-complexity"
    }

    fn extensions(&self) -> &[&str] {
        &["java"]
    }

    fn language(&self) -> &str {
        "java"
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
            .filter(|line| line.trim().starts_with("import "))
            .count() as u32;

        // Find classes and methods
        let mut in_method = false;
        let mut method_start = 0;
        let mut method_name = String::new();
        let mut brace_count = 0;
        let mut current_class: Option<String> = None;
        let mut class_brace_level = 0;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Track classes
            if (trimmed.contains("class ") || trimmed.contains("interface "))
                && trimmed.contains('{')
            {
                if let Some(name) = extract_class_name(trimmed) {
                    current_class = Some(name);
                    file_metrics.classes += 1;
                    class_brace_level = brace_count + 1;
                }
            }

            // Detect method definitions
            if !in_method && current_class.is_some() {
                if let Some(name) = detect_java_method(trimmed) {
                    in_method = true;
                    method_start = i + 1;
                    method_name = name;
                    // Reset brace count relative to method start
                    brace_count = 0;
                }
            }

            if in_method {
                brace_count += line.matches('{').count() as i32;
                brace_count -= line.matches('}').count() as i32;

                if brace_count <= 0 && line.contains('}') {
                    let end_line = i + 1;
                    let metrics =
                        self.analyze_function(content, method_start as u32, end_line as u32);

                    let mut func = FunctionMetrics::new(
                        &method_name,
                        method_start as u32,
                        end_line as u32,
                    );
                    func.metrics = metrics;
                    func.parent = current_class.clone();
                    func.kind = "method".to_string();

                    file_metrics.functions.push(func);
                    in_method = false;
                }
            } else {
                // Track class brace level when not in a method
                brace_count += line.matches('{').count() as i32;
                brace_count -= line.matches('}').count() as i32;

                // Check if we exited the class
                if brace_count < class_brace_level && current_class.is_some() {
                    current_class = None;
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

fn detect_java_method(line: &str) -> Option<String> {
    let line = line.trim();

    // Skip field declarations, annotations, class declarations
    if line.starts_with('@')
        || line.starts_with("//")
        || line.starts_with("/*")
        || line.contains(" class ")
        || line.contains(" interface ")
        || line.contains(" enum ")
        || line.ends_with(';')
    {
        return None;
    }

    // Look for method pattern: modifiers? type name(
    if !line.contains('(') || !line.contains('{') && !line.ends_with(')') {
        return None;
    }

    // Remove modifiers
    let clean = line
        .replace("public ", "")
        .replace("private ", "")
        .replace("protected ", "")
        .replace("static ", "")
        .replace("final ", "")
        .replace("abstract ", "")
        .replace("synchronized ", "")
        .replace("native ", "")
        .replace("@Override ", "");

    let clean = clean.trim();

    // Find method name (word before opening paren)
    if let Some(paren_pos) = clean.find('(') {
        let before_paren = clean[..paren_pos].trim();
        let parts: Vec<&str> = before_paren.split_whitespace().collect();

        if parts.len() >= 2 {
            // Last part is method name, second to last is return type
            let name = parts.last()?;

            // Skip constructors that look like class names (start with uppercase)
            // but allow regular methods
            if !name.is_empty() && is_valid_method_name(name) {
                return Some(name.to_string());
            }
        } else if parts.len() == 1 {
            // Could be a constructor
            let name = parts[0];
            if !name.is_empty() && is_valid_method_name(name) {
                return Some(name.to_string());
            }
        }
    }

    None
}

fn is_valid_method_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('<')
        && !name.contains('>')
        && name != "if"
        && name != "for"
        && name != "while"
        && name != "switch"
        && name != "try"
        && name != "catch"
}

fn extract_class_name(line: &str) -> Option<String> {
    let line = line.trim();

    let class_pos = line.find("class ").or_else(|| line.find("interface "))?;
    let keyword_len = if line.contains("class ") { 6 } else { 10 };

    let rest = &line[class_pos + keyword_len..];
    let end = rest.find(|c: char| c == '<' || c == '{' || c == ' ' || c == '(')?;
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
    fn test_java_analyzer_creation() {
        let analyzer = JavaComplexityAnalyzer::new();
        assert_eq!(analyzer.language(), "java");
        assert!(analyzer.extensions().contains(&"java"));
    }

    #[test]
    fn test_detect_method() {
        assert_eq!(
            detect_java_method("public void main(String[] args) {"),
            Some("main".to_string())
        );
        assert_eq!(
            detect_java_method("private int calculate(int x) {"),
            Some("calculate".to_string())
        );
        assert_eq!(
            detect_java_method("public static void run() {"),
            Some("run".to_string())
        );
    }

    #[test]
    fn test_analyze_simple_class() {
        let analyzer = JavaComplexityAnalyzer::new();
        let content = r#"
public class Example {
    public void simple() {
        int x = 1;
    }
}
"#;
        let result = analyzer.analyze_file(Path::new("Example.java"), content);
        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert_eq!(metrics.classes, 1);
        assert_eq!(metrics.functions.len(), 1);
    }

    #[test]
    fn test_complexity_with_conditionals() {
        let analyzer = JavaComplexityAnalyzer::new();
        let content = r#"
public class Example {
    public String complex(int x) {
        if (x > 0) {
            if (x > 10) {
                return "big";
            } else {
                return "small";
            }
        } else if (x < 0) {
            return "negative";
        }
        return "zero";
    }
}
"#;
        let result = analyzer
            .analyze_file(Path::new("Example.java"), content)
            .unwrap();
        assert!(!result.functions.is_empty());
        assert!(result.functions[0].metrics.cyclomatic > 1);
    }
}
