// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Python complexity analyzer.

use std::path::Path;

use crate::complexity::analyzer::LanguageComplexityAnalyzer;
use crate::complexity::metrics::{ComplexityMetrics, FileMetrics, FunctionMetrics};

/// Python complexity analyzer
pub struct PythonComplexityAnalyzer;

impl PythonComplexityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn analyze_function(&self, lines: &[&str]) -> ComplexityMetrics {
        let mut metrics = ComplexityMetrics::new();
        let func_content = lines.join("\n");

        metrics.cyclomatic = self.calculate_cyclomatic(&func_content);
        metrics.cognitive = self.calculate_cognitive(lines);
        metrics.max_nesting = self.calculate_nesting(lines);

        metrics.loc = lines.len() as u32;
        metrics.sloc = lines
            .iter()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with('#')
            })
            .count() as u32;

        // Count parameters from first line (simplified)
        if let Some(first_line) = lines.first() {
            if let Some(start) = first_line.find('(') {
                if let Some(end) = first_line.find(')') {
                    let params = &first_line[start + 1..end];
                    if !params.trim().is_empty() {
                        // Don't count 'self' or 'cls'
                        let count = params
                            .split(',')
                            .filter(|p| {
                                let p = p.trim().split(':').next().unwrap_or("").trim();
                                !p.is_empty() && p != "self" && p != "cls"
                            })
                            .count();
                        metrics.parameters = count as u32;
                    }
                }
            }
        }

        metrics.returns = func_content.matches("\n    return ").count() as u32
            + func_content.matches("\nreturn ").count() as u32
            + if func_content.starts_with("return ") { 1 } else { 0 };

        metrics
    }

    fn calculate_cyclomatic(&self, content: &str) -> u32 {
        let mut complexity = 1;

        // Python control flow keywords
        let keywords = [
            " if ", "\nif ", " elif ", "\nelif ", " else:", "\nelse:",
            " for ", "\nfor ", " while ", "\nwhile ", " except:", "\nexcept ",
            " and ", " or ",
        ];

        for keyword in keywords {
            complexity += content.matches(keyword).count() as u32;
        }

        // Comprehension conditions
        complexity += content.matches(" if ").count().saturating_sub(
            content.matches(" elif ").count() + content.matches("\nif ").count()
        ) as u32;

        complexity
    }

    fn calculate_cognitive(&self, lines: &[&str]) -> u32 {
        let mut complexity = 0;
        let base_indent = lines.first().map(|l| get_indent(l)).unwrap_or(0);

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let indent = get_indent(line);
            let nesting_level = (indent.saturating_sub(base_indent)) / 4;

            // Control keywords
            let control_keywords = ["if ", "elif ", "else:", "for ", "while ", "except", "with "];
            for keyword in control_keywords {
                if trimmed.starts_with(keyword) {
                    complexity += 1 + nesting_level as u32;
                }
            }

            // Boolean operators
            complexity += line.matches(" and ").count() as u32;
            complexity += line.matches(" or ").count() as u32;
        }

        complexity
    }

    fn calculate_nesting(&self, lines: &[&str]) -> u32 {
        let base_indent = lines.first().map(|l| get_indent(l)).unwrap_or(0);
        let mut max_nesting = 0;

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let indent = get_indent(line);
            let nesting = (indent.saturating_sub(base_indent)) / 4;
            max_nesting = max_nesting.max(nesting);
        }

        max_nesting as u32
    }
}

impl Default for PythonComplexityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageComplexityAnalyzer for PythonComplexityAnalyzer {
    fn name(&self) -> &str {
        "python-complexity"
    }

    fn extensions(&self) -> &[&str] {
        &["py", "pyw"]
    }

    fn language(&self) -> &str {
        "python"
    }

    fn analyze_file(&self, path: &Path, content: &str) -> Result<FileMetrics, String> {
        let mut file_metrics = FileMetrics::new(path.to_path_buf(), self.language());
        let lines: Vec<&str> = content.lines().collect();

        file_metrics.metrics.loc = lines.len() as u32;
        file_metrics.metrics.sloc = lines
            .iter()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with('#')
            })
            .count() as u32;

        file_metrics.metrics.comment_lines = lines
            .iter()
            .filter(|line| line.trim().starts_with('#'))
            .count() as u32;

        file_metrics.imports = lines
            .iter()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("import ") || trimmed.starts_with("from ")
            })
            .count() as u32;

        // Find functions and methods
        let mut i = 0;
        let mut current_class: Option<String> = None;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            // Track classes
            if trimmed.starts_with("class ") {
                if let Some(name) = extract_class_name(trimmed) {
                    current_class = Some(name);
                    file_metrics.classes += 1;
                }
            }

            // Detect function/method definitions
            if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
                let function_start = i;
                let function_name = extract_function_name(trimmed).unwrap_or_default();
                let base_indent = get_indent(line);

                // Find function end
                let mut function_end = i;
                for j in (i + 1)..lines.len() {
                    let next_line = lines[j];
                    let next_trimmed = next_line.trim();

                    // Empty lines or comments don't end functions
                    if next_trimmed.is_empty() || next_trimmed.starts_with('#') {
                        function_end = j;
                        continue;
                    }

                    let next_indent = get_indent(next_line);
                    if next_indent <= base_indent {
                        // Line is at same or lower indentation - function ended
                        break;
                    }
                    function_end = j;
                }

                let func_lines = &lines[function_start..=function_end];
                let metrics = self.analyze_function(func_lines);

                let mut func = FunctionMetrics::new(
                    &function_name,
                    (function_start + 1) as u32,
                    (function_end + 1) as u32,
                );
                func.metrics = metrics;
                func.parent = current_class.clone();

                // Determine kind
                if trimmed.starts_with("async def ") {
                    func.kind = "async function".to_string();
                } else if current_class.is_some() {
                    func.kind = "method".to_string();
                }

                file_metrics.functions.push(func);
                i = function_end;
            }

            i += 1;
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

fn get_indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn extract_function_name(line: &str) -> Option<String> {
    let line = line.trim();
    let start = if line.starts_with("async def ") {
        10
    } else if line.starts_with("def ") {
        4
    } else {
        return None;
    };

    let rest = &line[start..];
    let end = rest.find('(')?;
    let name = rest[..end].trim();
    if !name.is_empty() {
        Some(name.to_string())
    } else {
        None
    }
}

fn extract_class_name(line: &str) -> Option<String> {
    let line = line.trim();
    let start = line.find("class ")? + 6;
    let rest = &line[start..];
    let end = rest.find(|c: char| c == '(' || c == ':')?;
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
    fn test_python_analyzer_creation() {
        let analyzer = PythonComplexityAnalyzer::new();
        assert_eq!(analyzer.language(), "python");
        assert!(analyzer.extensions().contains(&"py"));
    }

    #[test]
    fn test_extract_function_name() {
        assert_eq!(
            extract_function_name("def hello():"),
            Some("hello".to_string())
        );
        assert_eq!(
            extract_function_name("async def foo():"),
            Some("foo".to_string())
        );
        assert_eq!(
            extract_function_name("def bar(x, y):"),
            Some("bar".to_string())
        );
    }

    #[test]
    fn test_analyze_simple_function() {
        let analyzer = PythonComplexityAnalyzer::new();
        let content = r#"
def simple():
    x = 1
    return x
"#;
        let result = analyzer.analyze_file(Path::new("test.py"), content);
        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert_eq!(metrics.functions.len(), 1);
    }

    #[test]
    fn test_complexity_with_conditionals() {
        let analyzer = PythonComplexityAnalyzer::new();
        let content = r#"
def complex_func(x):
    if x > 0:
        if x > 10:
            return "big"
        else:
            return "small"
    elif x < 0:
        return "negative"
    else:
        return "zero"
"#;
        let result = analyzer.analyze_file(Path::new("test.py"), content).unwrap();
        assert!(!result.functions.is_empty());
        assert!(result.functions[0].metrics.cyclomatic > 1);
    }
}
