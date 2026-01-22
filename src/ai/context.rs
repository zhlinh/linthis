// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Code context extraction for AI-assisted fixes.

use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

/// Options for context extraction
#[derive(Debug, Clone)]
pub struct ContextOptions {
    /// Lines of context before the issue
    pub lines_before: usize,
    /// Lines of context after the issue
    pub lines_after: usize,
    /// Maximum total context size in characters
    pub max_context_chars: usize,
    /// Include function/class scope
    pub include_scope: bool,
    /// Include imports/dependencies
    pub include_imports: bool,
}

impl Default for ContextOptions {
    fn default() -> Self {
        Self {
            lines_before: 10,
            lines_after: 10,
            max_context_chars: 4000,
            include_scope: true,
            include_imports: true,
        }
    }
}

/// Extracted code context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeContext {
    /// File path
    pub file_path: String,
    /// Language
    pub language: String,
    /// The problematic line(s)
    pub issue_lines: String,
    /// Line number of the issue
    pub line_number: u32,
    /// Context before the issue
    pub before: String,
    /// Context after the issue
    pub after: String,
    /// Full function/method containing the issue (if available)
    pub scope: Option<String>,
    /// Imports at the top of the file
    pub imports: Option<String>,
    /// The full relevant snippet
    pub full_snippet: String,
}

impl CodeContext {
    /// Create a new code context
    pub fn new(file_path: &str, language: &str, line_number: u32) -> Self {
        Self {
            file_path: file_path.to_string(),
            language: language.to_string(),
            issue_lines: String::new(),
            line_number,
            before: String::new(),
            after: String::new(),
            scope: None,
            imports: None,
            full_snippet: String::new(),
        }
    }

    /// Get the total character count
    pub fn char_count(&self) -> usize {
        self.full_snippet.len()
    }
}

/// Extract code context around a specific line
pub fn extract_context(
    path: &Path,
    line_number: u32,
    options: &ContextOptions,
) -> Result<CodeContext, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    if line_number == 0 || line_number as usize > total_lines {
        return Err(format!(
            "Line number {} out of range (file has {} lines)",
            line_number, total_lines
        ));
    }

    let idx = line_number as usize - 1;
    let language = detect_language(path);

    let mut context = CodeContext::new(
        &path.to_string_lossy(),
        &language,
        line_number,
    );

    // Extract issue line
    context.issue_lines = lines[idx].to_string();

    // Extract before context
    let start = idx.saturating_sub(options.lines_before);
    if start < idx {
        context.before = lines[start..idx].join("\n");
    }

    // Extract after context
    let end = (idx + 1 + options.lines_after).min(total_lines);
    if idx + 1 < end {
        context.after = lines[idx + 1..end].join("\n");
    }

    // Build full snippet
    context.full_snippet = format!(
        "{}\n>>> {} <<<\n{}",
        context.before,
        context.issue_lines,
        context.after
    );

    // Extract imports if requested
    if options.include_imports {
        context.imports = Some(extract_imports(&lines, &language));
    }

    // Extract enclosing scope if requested
    if options.include_scope {
        context.scope = extract_scope(&lines, idx, &language);
    }

    // Truncate if needed
    if context.char_count() > options.max_context_chars {
        context.full_snippet = truncate_context(&context.full_snippet, options.max_context_chars);
    }

    Ok(context)
}

/// Detect language from file extension
fn detect_language(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| match ext {
            "rs" => "rust",
            "py" | "pyw" => "python",
            "js" | "jsx" | "mjs" => "javascript",
            "ts" | "tsx" | "mts" => "typescript",
            "go" => "go",
            "java" => "java",
            "cpp" | "cc" | "cxx" | "c" | "h" | "hpp" => "cpp",
            "swift" => "swift",
            "kt" | "kts" => "kotlin",
            "rb" => "ruby",
            "php" => "php",
            "sh" | "bash" => "shell",
            _ => ext,
        })
        .unwrap_or("unknown")
        .to_string()
}

/// Extract imports from the beginning of a file
fn extract_imports(lines: &[&str], language: &str) -> String {
    let import_patterns: Vec<&str> = match language {
        "rust" => vec!["use ", "extern crate ", "mod "],
        "python" => vec!["import ", "from "],
        "javascript" | "typescript" => vec!["import ", "require(", "export "],
        "go" => vec!["import "],
        "java" => vec!["import ", "package "],
        "cpp" => vec!["#include ", "#pragma ", "using "],
        "swift" => vec!["import "],
        "kotlin" => vec!["import ", "package "],
        "ruby" => vec!["require ", "require_relative ", "load "],
        "php" => vec!["use ", "require ", "include ", "namespace "],
        _ => vec![],
    };

    let mut imports = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if import_patterns.iter().any(|p| trimmed.starts_with(p)) {
            imports.push(*line);
        } else if !trimmed.is_empty()
            && !trimmed.starts_with("//")
            && !trimmed.starts_with("#")
            && !trimmed.starts_with("/*")
            && !trimmed.starts_with("*")
        {
            // Stop at first non-import, non-comment line
            break;
        }
    }

    imports.join("\n")
}

/// Extract the enclosing function/class scope
fn extract_scope(lines: &[&str], issue_idx: usize, language: &str) -> Option<String> {
    // Find the start of the enclosing function/method
    let start = find_scope_start(lines, issue_idx, language)?;

    // Find the end of the scope
    let end = find_scope_end(lines, start, language);

    if start < lines.len() && end <= lines.len() && start < end {
        Some(lines[start..end].join("\n"))
    } else {
        None
    }
}

/// Find the start of the enclosing scope
fn find_scope_start(lines: &[&str], idx: usize, language: &str) -> Option<usize> {
    let function_patterns: Vec<&str> = match language {
        "rust" => vec!["fn ", "pub fn ", "async fn ", "pub async fn "],
        "python" => vec!["def ", "async def ", "class "],
        "javascript" | "typescript" => vec!["function ", "const ", "let ", "class ", "async "],
        "go" => vec!["func "],
        "java" => vec!["public ", "private ", "protected ", "void ", "class "],
        "cpp" => vec!["void ", "int ", "bool ", "class ", "struct "],
        "swift" => vec!["func ", "class ", "struct "],
        "kotlin" => vec!["fun ", "class ", "object "],
        _ => vec![],
    };

    for i in (0..=idx).rev() {
        let trimmed = lines[i].trim();
        if function_patterns.iter().any(|p| trimmed.starts_with(p) || trimmed.contains(p)) {
            return Some(i);
        }
    }

    None
}

/// Find the end of a scope (matching braces or indentation)
fn find_scope_end(lines: &[&str], start: usize, language: &str) -> usize {
    // For Python, use indentation
    if language == "python" {
        let base_indent = lines[start].len() - lines[start].trim_start().len();
        for i in start + 1..lines.len() {
            let line = lines[i];
            if line.trim().is_empty() {
                continue;
            }
            let indent = line.len() - line.trim_start().len();
            if indent <= base_indent {
                return i;
            }
        }
        return lines.len();
    }

    // For brace-based languages
    let mut brace_count = 0;
    let mut found_open = false;

    for i in start..lines.len() {
        for ch in lines[i].chars() {
            if ch == '{' {
                brace_count += 1;
                found_open = true;
            } else if ch == '}' {
                brace_count -= 1;
                if found_open && brace_count == 0 {
                    return i + 1;
                }
            }
        }
    }

    // If we can't find the end, return a reasonable range
    (start + 50).min(lines.len())
}

/// Truncate context to a maximum size while keeping the issue line
fn truncate_context(content: &str, max_chars: usize) -> String {
    if content.len() <= max_chars {
        return content.to_string();
    }

    // Find the issue marker
    if let Some(marker_pos) = content.find(">>>") {
        // Keep content around the marker
        let start = marker_pos.saturating_sub(max_chars / 2);
        let end = (marker_pos + max_chars / 2).min(content.len());

        let mut truncated = String::new();
        if start > 0 {
            truncated.push_str("...\n");
        }
        truncated.push_str(&content[start..end]);
        if end < content.len() {
            truncated.push_str("\n...");
        }
        truncated
    } else {
        // Just truncate from the end
        content[..max_chars].to_string() + "..."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_context_options_default() {
        let options = ContextOptions::default();
        assert_eq!(options.lines_before, 10);
        assert_eq!(options.lines_after, 10);
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language(Path::new("test.rs")), "rust");
        assert_eq!(detect_language(Path::new("test.py")), "python");
        assert_eq!(detect_language(Path::new("test.ts")), "typescript");
        assert_eq!(detect_language(Path::new("test.go")), "go");
    }

    #[test]
    fn test_extract_context() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "fn main() {{").unwrap();
        writeln!(file, "    let x = 1;").unwrap();
        writeln!(file, "    let y = 2;").unwrap();
        writeln!(file, "    println!(\"{{}}\", x + y);").unwrap();
        writeln!(file, "}}").unwrap();

        let options = ContextOptions::default();
        let context = extract_context(file.path(), 3, &options).unwrap();

        assert_eq!(context.line_number, 3);
        assert!(context.issue_lines.contains("let y = 2"));
    }

    #[test]
    fn test_extract_imports() {
        let lines = vec![
            "use std::io;",
            "use std::path::Path;",
            "",
            "fn main() {",
        ];
        let imports = extract_imports(&lines, "rust");
        assert!(imports.contains("use std::io"));
        assert!(imports.contains("use std::path::Path"));
    }
}
