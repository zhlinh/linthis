// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Rust complexity analyzer.

use std::path::Path;

use crate::complexity::analyzer::LanguageComplexityAnalyzer;
use crate::complexity::metrics::{ComplexityMetrics, FileMetrics, FunctionMetrics};

/// Rust complexity analyzer
pub struct RustComplexityAnalyzer;

impl RustComplexityAnalyzer {
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

        // Calculate cyclomatic complexity
        metrics.cyclomatic = self.calculate_cyclomatic(&func_content);

        // Calculate cognitive complexity
        metrics.cognitive = self.calculate_cognitive(&func_content);

        // Calculate nesting depth
        metrics.max_nesting = self.calculate_nesting(&func_content);

        // Line counts
        metrics.loc = (end - start) as u32;
        metrics.sloc = func_lines
            .iter()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with("//")
            })
            .count() as u32;

        // Count parameters (simplified)
        if let Some(params_start) = func_content.find('(') {
            if let Some(params_end) = func_content.find(')') {
                let params = &func_content[params_start + 1..params_end];
                if !params.trim().is_empty() {
                    metrics.parameters = params.split(',').count() as u32;
                }
            }
        }

        // Count returns
        metrics.returns = func_content.matches("return ").count() as u32
            + func_content.matches("return;").count() as u32;

        metrics
    }

    fn calculate_cyclomatic(&self, content: &str) -> u32 {
        let mut complexity = 1; // Base complexity

        // Control flow keywords
        let keywords = [
            "if ", "if(", "else if", "else {", "match ", "for ", "for(",
            "while ", "while(", "loop ", "loop{", "&&", "||", "?",
        ];

        for keyword in keywords {
            complexity += content.matches(keyword).count() as u32;
        }

        // Match arms (each arm adds complexity)
        complexity += content.matches("=>").count().saturating_sub(1) as u32;

        complexity
    }

    fn calculate_cognitive(&self, content: &str) -> u32 {
        let mut complexity = 0;
        let mut nesting_level = 0;

        for line in content.lines() {
            let trimmed = line.trim();

            // Skip comments
            if trimmed.starts_with("//") {
                continue;
            }

            // Track nesting
            let opens = line.matches('{').count() as i32;
            let closes = line.matches('}').count() as i32;

            // Control structures add complexity based on nesting
            let control_keywords = ["if ", "else", "match ", "for ", "while ", "loop "];
            for keyword in control_keywords {
                if trimmed.contains(keyword) {
                    complexity += 1 + nesting_level as u32;
                }
            }

            // Boolean operators
            complexity += line.matches("&&").count() as u32;
            complexity += line.matches("||").count() as u32;

            // Recursion (simplified check)
            if trimmed.contains("Self::") && trimmed.contains('(') {
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

impl Default for RustComplexityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageComplexityAnalyzer for RustComplexityAnalyzer {
    fn name(&self) -> &str {
        "rust-complexity"
    }

    fn extensions(&self) -> &[&str] {
        &["rs"]
    }

    fn language(&self) -> &str {
        "rust"
    }

    fn analyze_file(&self, path: &Path, content: &str) -> Result<FileMetrics, String> {
        let mut file_metrics = FileMetrics::new(path.to_path_buf(), self.language());
        let lines: Vec<&str> = content.lines().collect();

        // Count total lines
        file_metrics.metrics.loc = lines.len() as u32;
        file_metrics.metrics.sloc = lines
            .iter()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with("//")
            })
            .count() as u32;

        // Count comment lines
        file_metrics.metrics.comment_lines = lines
            .iter()
            .filter(|line| line.trim().starts_with("//") || line.trim().starts_with("///"))
            .count() as u32;

        // Count imports
        file_metrics.imports = lines
            .iter()
            .filter(|line| line.trim().starts_with("use "))
            .count() as u32;

        // Find and analyze functions
        let mut in_function = false;
        let mut function_start = 0;
        let mut function_name = String::new();
        let mut brace_count = 0;
        let mut current_struct: Option<String> = None;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Track structs/impls
            if trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ") {
                if let Some(name) = extract_name(trimmed, "struct ") {
                    current_struct = Some(name);
                    file_metrics.classes += 1;
                }
            } else if trimmed.starts_with("impl ") {
                if let Some(name) = extract_impl_name(trimmed) {
                    current_struct = Some(name);
                }
            }

            // Detect function definitions
            if !in_function {
                if let Some(name) = detect_rust_function(trimmed) {
                    in_function = true;
                    function_start = i + 1;
                    function_name = name;
                    brace_count = 0;
                }
            }

            if in_function {
                brace_count += line.matches('{').count() as i32;
                brace_count -= line.matches('}').count() as i32;

                if brace_count <= 0 && line.contains('}') {
                    // Function ended
                    let end_line = i + 1;
                    let metrics =
                        self.analyze_function(content, function_start as u32, end_line as u32);

                    let mut func = FunctionMetrics::new(
                        &function_name,
                        function_start as u32,
                        end_line as u32,
                    );
                    func.metrics = metrics;
                    func.parent = current_struct.clone();

                    file_metrics.functions.push(func);
                    in_function = false;
                }
            }

            // Reset struct context at end of impl block
            if trimmed == "}" && current_struct.is_some() && !in_function && brace_count <= 0 {
                // This is a simplification; a real parser would track impl blocks properly
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

fn detect_rust_function(line: &str) -> Option<String> {
    let line = line.trim();

    // Check for function definition
    if !line.contains("fn ") {
        return None;
    }

    // Skip function type declarations or trait bounds
    if line.starts_with("type ") || line.starts_with("trait ") {
        return None;
    }

    // Extract function name
    if let Some(fn_pos) = line.find("fn ") {
        let rest = &line[fn_pos + 3..];
        let name_end = rest.find(|c: char| c == '(' || c == '<' || c.is_whitespace())?;
        let name = rest[..name_end].trim();
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }

    None
}

fn extract_name(line: &str, keyword: &str) -> Option<String> {
    let start = line.find(keyword)? + keyword.len();
    let rest = &line[start..];
    let end = rest.find(|c: char| c == '<' || c == '{' || c == '(' || c.is_whitespace())?;
    let name = rest[..end].trim();
    if !name.is_empty() {
        Some(name.to_string())
    } else {
        None
    }
}

fn extract_impl_name(line: &str) -> Option<String> {
    let line = line.trim();
    // impl Foo or impl<T> Foo<T> or impl Trait for Foo
    let without_impl = line.strip_prefix("impl")?;
    let without_impl = without_impl.trim();

    // Skip generic parameters
    let rest = if without_impl.starts_with('<') {
        let end = find_matching_bracket(without_impl, '<', '>')?;
        without_impl[end + 1..].trim()
    } else {
        without_impl
    };

    // Check for "for" keyword (trait impl)
    if let Some(for_pos) = rest.find(" for ") {
        let after_for = &rest[for_pos + 5..];
        let end = after_for.find(|c: char| c == '<' || c == '{' || c.is_whitespace())?;
        return Some(after_for[..end].trim().to_string());
    }

    // Regular impl
    let end = rest.find(|c: char| c == '<' || c == '{' || c.is_whitespace())?;
    let name = rest[..end].trim();
    if !name.is_empty() {
        Some(name.to_string())
    } else {
        None
    }
}

fn find_matching_bracket(s: &str, open: char, close: char) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s.chars().enumerate() {
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_analyzer_creation() {
        let analyzer = RustComplexityAnalyzer::new();
        assert_eq!(analyzer.language(), "rust");
        assert!(analyzer.extensions().contains(&"rs"));
    }

    #[test]
    fn test_detect_function() {
        assert_eq!(detect_rust_function("fn main() {"), Some("main".to_string()));
        assert_eq!(detect_rust_function("pub fn foo() {"), Some("foo".to_string()));
        assert_eq!(
            detect_rust_function("pub async fn bar() {"),
            Some("bar".to_string())
        );
        assert_eq!(detect_rust_function("let x = 1;"), None);
    }

    #[test]
    fn test_analyze_simple_function() {
        let analyzer = RustComplexityAnalyzer::new();
        let content = r#"
fn simple() {
    let x = 1;
}
"#;
        let result = analyzer.analyze_file(Path::new("test.rs"), content);
        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert_eq!(metrics.functions.len(), 1);
    }

    #[test]
    fn test_cyclomatic_complexity() {
        let analyzer = RustComplexityAnalyzer::new();
        let content = r#"
fn complex() {
    if true {
        if false {
            println!("nested");
        }
    } else {
        for i in 0..10 {
            if i > 5 {
                break;
            }
        }
    }
}
"#;
        let result = analyzer.analyze_file(Path::new("test.rs"), content).unwrap();
        assert!(!result.functions.is_empty());
        // Should have complexity > 1 due to if/else/for
        assert!(result.functions[0].metrics.cyclomatic > 1);
    }
}
