// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Go complexity analyzer.

use std::path::Path;

use crate::complexity::analyzer::LanguageComplexityAnalyzer;
use crate::complexity::metrics::{ComplexityMetrics, FileMetrics, FunctionMetrics};

/// Go complexity analyzer
pub struct GoComplexityAnalyzer;

impl GoComplexityAnalyzer {
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
                !trimmed.is_empty() && !trimmed.starts_with("//")
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

        metrics.returns = func_content.matches("\n\treturn ").count() as u32
            + func_content.matches("\n    return ").count() as u32
            + func_content.matches("\nreturn ").count() as u32;

        metrics
    }

    fn calculate_cyclomatic(&self, content: &str) -> u32 {
        let mut complexity = 1;

        let keywords = [
            "if ", "else if", "else {", "switch ", "case ", "for ", "select {",
            "&&", "||",
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

            if trimmed.starts_with("//") {
                continue;
            }

            let opens = line.matches('{').count() as i32;
            let closes = line.matches('}').count() as i32;

            let control_keywords = ["if ", "else", "switch ", "for ", "select ", "case "];
            for keyword in control_keywords {
                if trimmed.starts_with(keyword) || trimmed.contains(&format!(" {}", keyword)) {
                    complexity += 1 + nesting_level as u32;
                }
            }

            complexity += line.matches("&&").count() as u32;
            complexity += line.matches("||").count() as u32;

            // Go routine adds complexity
            if trimmed.contains("go func") {
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

impl Default for GoComplexityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageComplexityAnalyzer for GoComplexityAnalyzer {
    fn name(&self) -> &str {
        "go-complexity"
    }

    fn extensions(&self) -> &[&str] {
        &["go"]
    }

    fn language(&self) -> &str {
        "go"
    }

    fn analyze_file(&self, path: &Path, content: &str) -> Result<FileMetrics, String> {
        let mut file_metrics = FileMetrics::new(path.to_path_buf(), self.language());
        let lines: Vec<&str> = content.lines().collect();

        file_metrics.metrics.loc = lines.len() as u32;
        file_metrics.metrics.sloc = lines
            .iter()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with("//")
            })
            .count() as u32;

        file_metrics.metrics.comment_lines = lines
            .iter()
            .filter(|line| line.trim().starts_with("//"))
            .count() as u32;

        file_metrics.imports = lines
            .iter()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("import ") || trimmed.starts_with("\"")
            })
            .count() as u32;

        // Find functions
        let mut in_function = false;
        let mut function_start = 0;
        let mut function_name = String::new();
        let mut receiver_type: Option<String> = None;
        let mut brace_count = 0;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Track structs
            if trimmed.starts_with("type ") && trimmed.contains("struct") {
                file_metrics.classes += 1;
            }

            // Detect function definitions
            if !in_function {
                if let Some((name, receiver)) = detect_go_function(trimmed) {
                    in_function = true;
                    function_start = i + 1;
                    function_name = name;
                    receiver_type = receiver;
                    brace_count = 0;
                }
            }

            if in_function {
                brace_count += line.matches('{').count() as i32;
                brace_count -= line.matches('}').count() as i32;

                if brace_count <= 0 && line.contains('}') {
                    let end_line = i + 1;
                    let metrics =
                        self.analyze_function(content, function_start as u32, end_line as u32);

                    let mut func = FunctionMetrics::new(
                        &function_name,
                        function_start as u32,
                        end_line as u32,
                    );
                    func.metrics = metrics;
                    func.parent = receiver_type.clone();

                    if receiver_type.is_some() {
                        func.kind = "method".to_string();
                    }

                    file_metrics.functions.push(func);
                    in_function = false;
                    receiver_type = None;
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

fn detect_go_function(line: &str) -> Option<(String, Option<String>)> {
    let line = line.trim();

    if !line.starts_with("func ") {
        return None;
    }

    let rest = &line[5..]; // Skip "func "

    // Check for method (receiver)
    let (receiver, name_start) = if rest.starts_with('(') {
        // Method with receiver: func (r *Type) Name()
        let receiver_end = rest.find(')')?;
        let receiver_content = &rest[1..receiver_end];
        // Extract type from receiver
        let receiver_type = receiver_content
            .split_whitespace()
            .last()?
            .trim_start_matches('*');
        (Some(receiver_type.to_string()), receiver_end + 2) // +2 for ") "
    } else {
        (None, 0)
    };

    let name_part = &rest[name_start..];
    let name_end = name_part.find('(')?;
    let name = name_part[..name_end].trim();

    if !name.is_empty() {
        Some((name.to_string(), receiver))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_analyzer_creation() {
        let analyzer = GoComplexityAnalyzer::new();
        assert_eq!(analyzer.language(), "go");
        assert!(analyzer.extensions().contains(&"go"));
    }

    #[test]
    fn test_detect_function() {
        assert_eq!(
            detect_go_function("func main() {"),
            Some(("main".to_string(), None))
        );
        assert_eq!(
            detect_go_function("func (s *Server) Start() {"),
            Some(("Start".to_string(), Some("Server".to_string())))
        );
        assert_eq!(
            detect_go_function("func NewServer() *Server {"),
            Some(("NewServer".to_string(), None))
        );
    }

    #[test]
    fn test_analyze_simple_function() {
        let analyzer = GoComplexityAnalyzer::new();
        let content = r#"
package main

func simple() {
    x := 1
    _ = x
}
"#;
        let result = analyzer.analyze_file(Path::new("test.go"), content);
        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert_eq!(metrics.functions.len(), 1);
    }

    #[test]
    fn test_complexity_with_conditionals() {
        let analyzer = GoComplexityAnalyzer::new();
        let content = r#"
package main

func complex(x int) string {
    if x > 0 {
        if x > 10 {
            return "big"
        } else {
            return "small"
        }
    } else if x < 0 {
        return "negative"
    }
    return "zero"
}
"#;
        let result = analyzer.analyze_file(Path::new("test.go"), content).unwrap();
        assert!(!result.functions.is_empty());
        assert!(result.functions[0].metrics.cyclomatic > 1);
    }
}
