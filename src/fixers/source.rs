// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Source code fixers for cpplint issues (C/C++/Objective-C).
//! These fixers handle issues that clang-format doesn't fix.

use crate::utils::unicode::{break_text_at_width, get_column_width};
use crate::Result;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Source fixer for various cpplint issues (C/C++/Objective-C)
pub struct SourceFixer;

impl SourceFixer {
    /// Fix comment spacing: "//comment" -> "// comment", "///comment" -> "/// comment"
    /// clang-format doesn't fix non-ASCII (e.g., Chinese) comments
    /// NOTE: This only modifies actual comments, not `//` inside string literals
    /// Uses the same detection logic as cpplint's IsCppString function
    pub fn fix_comment_spacing(path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to read file: {}", e)))?;

        let mut modified = false;
        let mut result_lines = Vec::new();

        for line in content.lines() {
            let fixed_line = Self::fix_comment_spacing_line(line);
            if fixed_line != line {
                modified = true;
            }
            result_lines.push(fixed_line);
        }

        if modified {
            let mut result = result_lines.join("\n");
            // Preserve trailing newline if original had one
            if content.ends_with('\n') {
                result.push('\n');
            }
            fs::write(path, result)
                .map_err(|e| crate::LintisError::Formatter(format!("Failed to write file: {}", e)))?;
        }

        Ok(())
    }

    /// Fix comment spacing for a single line
    fn fix_comment_spacing_line(line: &str) -> String {
        // Find the real comment position (not inside a string)
        let Some(comment_pos) = Self::find_real_comment_pos(line) else {
            return line.to_string();
        };

        let before_comment = &line[..comment_pos];
        let comment_part = &line[comment_pos..];

        // Count consecutive slashes
        let slash_count = comment_part.chars().take_while(|&c| c == '/').count();
        let after_slashes = &comment_part[slash_count..];

        // Check if space is needed
        if !after_slashes.is_empty() {
            let first_char = after_slashes.chars().next().unwrap();
            if first_char != ' ' && first_char != '\n' && first_char != '\r' {
                // Need to add space
                return format!(
                    "{}{} {}",
                    before_comment,
                    "/".repeat(slash_count),
                    after_slashes
                );
            }
        }

        line.to_string()
    }

    /// Find the position of the first real // comment (not inside a string)
    fn find_real_comment_pos(line: &str) -> Option<usize> {
        let mut search_start = 0;

        loop {
            // Find next // starting from search_start
            let rest = &line[search_start..];
            let Some(rel_pos) = rest.find("//") else {
                return None;
            };

            let abs_pos = search_start + rel_pos;
            let before_comment = &line[..abs_pos];

            // Check if this // is inside a string
            if !Self::is_in_cpp_string(before_comment) {
                // This is a real comment
                return Some(abs_pos);
            }

            // This // is inside a string, continue searching after it
            search_start = abs_pos + 2;
        }
    }

    /// Check if the line ends inside a string constant (cpplint's IsCppString logic)
    /// This checks if appending a character would place it inside a string.
    fn is_in_cpp_string(line: &str) -> bool {
        // Replace \\ with XX to handle escaped backslashes
        let line = line.replace("\\\\", "XX");

        // Count quotes: total " minus escaped \" minus '"' (quote in char literal)
        let total_quotes = line.matches('"').count();
        let escaped_quotes = line.matches("\\\"").count();
        let char_literal_quotes = line.matches("'\"'").count();

        let effective_quotes = total_quotes - escaped_quotes - char_literal_quotes;

        // If odd number of quotes, we're inside a string
        (effective_quotes & 1) == 1
    }

    /// Fix TODO comments using git blame for author
    /// Converts "TODO:" or "TODO(user):" to "TODO(blame_author):"
    pub fn fix_todo_comments(path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to read file: {}", e)))?;

        let mut result = Vec::new();
        let mut modified = false;

        for (i, line) in content.lines().enumerate() {
            // Look for TODO without proper format
            // Patterns: "TODO:", "TODO :", "TODO(something):" where something might be wrong
            if let Some(todo_pos) = line.find("TODO") {
                let after_todo = &line[todo_pos + 4..];

                // Check if it's already properly formatted with parentheses
                if after_todo.starts_with('(') {
                    // Already has parentheses, keep as is
                    result.push(line.to_string());
                    continue;
                }

                // Check if it's "TODO:" or "TODO :" pattern
                let trimmed = after_todo.trim_start();
                if trimmed.starts_with(':') {
                    // Get username from git blame
                    let username = Self::get_git_blame_author(path, i + 1)
                        .unwrap_or_else(Self::get_fallback_username);

                    // Find the position of ':' after TODO
                    let colon_offset = after_todo.find(':').unwrap();
                    let rest_start = todo_pos + 4 + colon_offset + 1;
                    let rest = if rest_start < line.len() {
                        line[rest_start..].trim_start()
                    } else {
                        ""
                    };

                    // Build new line (no trailing space if rest is empty)
                    let new_line = if rest.is_empty() {
                        format!("{}TODO({}):", &line[..todo_pos], username)
                    } else {
                        format!("{}TODO({}): {}", &line[..todo_pos], username, rest)
                    };

                    result.push(new_line);
                    modified = true;
                    continue;
                }
            }

            result.push(line.to_string());
        }

        if modified {
            let new_content = result.join("\n");
            // Preserve trailing newline if original had one
            let final_content = if content.ends_with('\n') {
                format!("{}\n", new_content)
            } else {
                new_content
            };
            fs::write(path, final_content)
                .map_err(|e| crate::LintisError::Formatter(format!("Failed to write file: {}", e)))?;
        }

        Ok(())
    }

    /// Get author from git blame for a specific line
    fn get_git_blame_author(path: &Path, line_number: usize) -> Option<String> {
        let output = Command::new("git")
            .args([
                "blame",
                "--porcelain",
                "-L",
                &format!("{},{}", line_number, line_number),
                "--",
            ])
            .arg(path)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.starts_with("author ") {
                return Some(line[7..].to_string());
            }
        }

        None
    }

    /// Get fallback username from git config or environment
    fn get_fallback_username() -> String {
        // Try git config user.name
        if let Ok(output) = Command::new("git")
            .args(["config", "user.name"])
            .output()
        {
            if output.status.success() {
                let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !name.is_empty() {
                    return name;
                }
            }
        }

        // Fallback to environment variable
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "user".to_string())
    }

    /// Fix lone semicolons: remove lines that contain only whitespace and a semicolon
    /// cpplint warns: "Line contains only semicolon."
    pub fn fix_lone_semicolon(path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to read file: {}", e)))?;

        let mut result = String::with_capacity(content.len());
        let mut modified = false;

        for line in content.lines() {
            let trimmed = line.trim();
            // Skip lines that contain only a semicolon
            if trimmed == ";" {
                modified = true;
                continue;
            }
            result.push_str(line);
            result.push('\n');
        }

        // Preserve original trailing newline behavior
        if !content.ends_with('\n') && result.ends_with('\n') {
            result.pop();
        }

        // Only write if changed
        if modified {
            fs::write(path, result)
                .map_err(|e| crate::LintisError::Formatter(format!("Failed to write file: {}", e)))?;
        }

        Ok(())
    }

    /// Fix long comment lines by breaking them at appropriate points
    /// Handles Chinese comments which clang-format can't reflow properly
    pub fn fix_long_comments(path: &Path, max_length: usize) -> Result<()> {
        let content = fs::read_to_string(path)
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to read file: {}", e)))?;

        let mut result = String::with_capacity(content.len());
        let mut modified = false;

        for line in content.lines() {
            // Check if line exceeds max length
            // cpplint uses column width: CJK/wide chars = 2 columns
            let col_width = get_column_width(line);
            if col_width <= max_length {
                result.push_str(line);
                result.push('\n');
                continue;
            }

            // Try to break long comment lines
            if let Some(broken) = Self::break_long_comment_line(line, max_length) {
                result.push_str(&broken);
                modified = true;
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }

        // Preserve original trailing newline behavior
        if !content.ends_with('\n') && result.ends_with('\n') {
            result.pop();
        }

        if modified {
            fs::write(path, result)
                .map_err(|e| crate::LintisError::Formatter(format!("Failed to write file: {}", e)))?;
        }

        Ok(())
    }

    /// Break a long comment line into multiple lines
    /// Returns None if the line can't be broken (not a comment or no good break point)
    fn break_long_comment_line(line: &str, max_length: usize) -> Option<String> {
        let trimmed = line.trim_start();

        // Find the leading whitespace (indentation)
        let indent = &line[..line.len() - trimmed.len()];

        // Check for // comment
        if let Some(comment_start) = trimmed.find("//") {
            // Get content before and after //
            let before_comment = &trimmed[..comment_start];
            let comment_part = &trimmed[comment_start..];

            // If there's code before the comment, this is a trailing comment
            if !before_comment.trim().is_empty() {
                return Self::break_trailing_comment(indent, before_comment, comment_part, max_length);
            }

            // Pure comment line starting with //
            return Self::break_pure_comment(indent, comment_part, max_length);
        }

        None
    }

    /// Break a trailing comment (code // comment) into separate lines
    fn break_trailing_comment(
        indent: &str,
        code_part: &str,
        comment_part: &str,
        max_length: usize,
    ) -> Option<String> {
        let mut result = String::new();

        // First line: just the code part
        result.push_str(indent);
        result.push_str(code_part.trim_end());
        result.push('\n');

        // Determine comment prefix (// or /// etc.)
        let comment_content = comment_part.trim_start_matches('/');
        let slash_count = comment_part.len() - comment_content.len();
        let prefix: String = "/".repeat(slash_count);
        let content = comment_content.trim_start();

        // Break the comment content into lines
        // Use column width (CJK chars = 2 columns)
        let comment_indent = format!("{}{} ", indent, prefix);
        let indent_width = get_column_width(&comment_indent);
        let available_width = max_length.saturating_sub(indent_width);

        if available_width < 30 {
            return None; // Not enough space
        }

        let lines = break_text_at_width(content, available_width);
        for l in lines {
            result.push_str(&comment_indent);
            result.push_str(&l);
            result.push('\n');
        }

        Some(result)
    }

    /// Break a pure comment line (// comment) into multiple lines
    fn break_pure_comment(indent: &str, comment_part: &str, max_length: usize) -> Option<String> {
        // Determine comment prefix (// or /// etc.)
        let comment_content = comment_part.trim_start_matches('/');
        let slash_count = comment_part.len() - comment_content.len();
        let prefix: String = "/".repeat(slash_count);
        let content = comment_content.trim_start();

        // Use column width (CJK chars = 2 columns)
        let comment_indent = format!("{}{} ", indent, prefix);
        let indent_width = get_column_width(&comment_indent);
        let available_width = max_length.saturating_sub(indent_width);

        if available_width < 30 {
            return None; // Not enough space
        }

        let lines = break_text_at_width(content, available_width);
        if lines.len() <= 1 {
            return None; // No breaking needed or possible
        }

        let mut result = String::new();
        for l in lines {
            result.push_str(&comment_indent);
            result.push_str(&l);
            result.push('\n');
        }

        Some(result)
    }

    /// Fix pragma separator lines: convert "-- -- --" style to standard "#pragma mark -" format
    /// cpplint warns about "Extra space for operator --" but these are visual separators
    pub fn fix_pragma_separators(path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to read file: {}", e)))?;

        let mut result = String::with_capacity(content.len());
        let mut modified = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // Check for #pragma lines with separator patterns like "-- --" or "- - -"
            if trimmed.starts_with("#pragma") {
                // Check if it has repeated dash patterns (visual separators)
                if trimmed.contains("-- --") || trimmed.contains("- - -") {
                    let pragma_content = trimmed.trim_start_matches("#pragma").trim();

                    // Handle "#pragma mark - - - -" specially
                    if pragma_content.starts_with("mark") {
                        let after_mark = pragma_content.trim_start_matches("mark").trim();
                        // If it's all dashes and spaces, just use "#pragma mark -"
                        if after_mark.chars().all(|c| c == '-' || c == ' ') {
                            result.push_str("#pragma mark -");
                            result.push('\n');
                            modified = true;
                            continue;
                        }
                    }

                    // For other pragmas like "#pragma webview delegate-- -- --"
                    // Find where the separator starts
                    let separator_start = pragma_content
                        .find("-- ")
                        .or_else(|| pragma_content.find("- -"))
                        .or_else(|| pragma_content.find("--"))
                        .unwrap_or(pragma_content.len());

                    let section_name = pragma_content[..separator_start].trim().trim_end_matches('-').trim();

                    // Convert to standard #pragma mark format
                    let new_line = if section_name.is_empty() || section_name == "mark" {
                        "#pragma mark -".to_string()
                    } else {
                        format!("#pragma mark - {}", section_name)
                    };

                    result.push_str(&new_line);
                    result.push('\n');
                    modified = true;
                    continue;
                }
            }

            result.push_str(line);
            result.push('\n');
        }

        // Preserve original trailing newline behavior
        if !content.ends_with('\n') && result.ends_with('\n') {
            result.pop();
        }

        if modified {
            fs::write(path, result)
                .map_err(|e| crate::LintisError::Formatter(format!("Failed to write file: {}", e)))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_temp_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    fn read_temp_file(file: &NamedTempFile) -> String {
        fs::read_to_string(file.path()).unwrap()
    }

    // ==================== fix_comment_spacing tests ====================

    #[test]
    fn test_fix_comment_spacing_basic() {
        let file = create_temp_file("//comment\n");
        SourceFixer::fix_comment_spacing(file.path()).unwrap();
        assert_eq!(read_temp_file(&file), "// comment\n");
    }

    #[test]
    fn test_fix_comment_spacing_already_has_space() {
        let file = create_temp_file("// already spaced\n");
        SourceFixer::fix_comment_spacing(file.path()).unwrap();
        assert_eq!(read_temp_file(&file), "// already spaced\n");
    }

    #[test]
    fn test_fix_comment_spacing_triple_slash() {
        let file = create_temp_file("///doc comment\n");
        SourceFixer::fix_comment_spacing(file.path()).unwrap();
        assert_eq!(read_temp_file(&file), "/// doc comment\n");
    }

    #[test]
    fn test_fix_comment_spacing_chinese() {
        let file = create_temp_file("//中文注释\n");
        SourceFixer::fix_comment_spacing(file.path()).unwrap();
        assert_eq!(read_temp_file(&file), "// 中文注释\n");
    }

    #[test]
    fn test_fix_comment_spacing_empty_comment() {
        let file = create_temp_file("//\ncode();\n");
        SourceFixer::fix_comment_spacing(file.path()).unwrap();
        assert_eq!(read_temp_file(&file), "//\ncode();\n");
    }

    #[test]
    fn test_fix_comment_spacing_preserves_url() {
        // URLs like https:// should NOT be modified
        let file = create_temp_file("return @\"https://example.com\";\n");
        SourceFixer::fix_comment_spacing(file.path()).unwrap();
        assert_eq!(read_temp_file(&file), "return @\"https://example.com\";\n");
    }

    #[test]
    fn test_fix_comment_spacing_preserves_multiple_urls() {
        let content = r#"NSString *url1 = @"https://tmga.qq.com";
NSString *url2 = @"http://example.com/path";
NSString *url3 = @"file:///local/path";
"#;
        let file = create_temp_file(content);
        SourceFixer::fix_comment_spacing(file.path()).unwrap();
        assert_eq!(read_temp_file(&file), content);
    }

    #[test]
    fn test_fix_comment_spacing_url_and_comment() {
        // Should preserve URL but fix comment
        let file = create_temp_file("NSString *url = @\"https://example.com\"; //comment\n");
        SourceFixer::fix_comment_spacing(file.path()).unwrap();
        assert_eq!(read_temp_file(&file), "NSString *url = @\"https://example.com\"; // comment\n");
    }

    #[test]
    fn test_fix_comment_spacing_string_with_slashes() {
        // // inside a string should NOT be modified
        let file = create_temp_file("char *path = \"path//to//file\";\n");
        SourceFixer::fix_comment_spacing(file.path()).unwrap();
        assert_eq!(read_temp_file(&file), "char *path = \"path//to//file\";\n");
    }

    #[test]
    fn test_fix_comment_spacing_escaped_quote() {
        // Handle escaped quotes correctly
        let file = create_temp_file("char *s = \"he said \\\"hello//world\\\"\";\n");
        SourceFixer::fix_comment_spacing(file.path()).unwrap();
        assert_eq!(read_temp_file(&file), "char *s = \"he said \\\"hello//world\\\"\";\n");
    }

    #[test]
    fn test_fix_comment_spacing_char_literal() {
        // Don't get confused by single quotes
        let file = create_temp_file("char c = '/'; //comment\n");
        SourceFixer::fix_comment_spacing(file.path()).unwrap();
        assert_eq!(read_temp_file(&file), "char c = '/'; // comment\n");
    }

    #[test]
    fn test_fix_comment_spacing_quote_in_char_literal() {
        // '"' should not affect string detection
        let file = create_temp_file("char c = '\"'; //comment\n");
        SourceFixer::fix_comment_spacing(file.path()).unwrap();
        assert_eq!(read_temp_file(&file), "char c = '\"'; // comment\n");
    }

    #[test]
    fn test_is_in_cpp_string() {
        // Test the IsCppString logic
        assert!(!SourceFixer::is_in_cpp_string("int x = 1;"));
        assert!(SourceFixer::is_in_cpp_string("char *s = \"hello"));
        assert!(!SourceFixer::is_in_cpp_string("char *s = \"hello\""));
        assert!(SourceFixer::is_in_cpp_string("char *s = \"hello\\\""));
        assert!(!SourceFixer::is_in_cpp_string("char *s = \"hello\\\"\""));
        assert!(!SourceFixer::is_in_cpp_string("char c = '\"';"));
        assert!(SourceFixer::is_in_cpp_string("char *s = \"a\\\\"));
        assert!(!SourceFixer::is_in_cpp_string("char *s = \"a\\\\\""));
    }

    // ==================== fix_lone_semicolon tests ====================

    #[test]
    fn test_fix_lone_semicolon_removes() {
        let file = create_temp_file("code();\n;\nmore();\n");
        SourceFixer::fix_lone_semicolon(file.path()).unwrap();
        assert_eq!(read_temp_file(&file), "code();\nmore();\n");
    }

    #[test]
    fn test_fix_lone_semicolon_with_whitespace() {
        let file = create_temp_file("code();\n   ;   \nmore();\n");
        SourceFixer::fix_lone_semicolon(file.path()).unwrap();
        assert_eq!(read_temp_file(&file), "code();\nmore();\n");
    }

    #[test]
    fn test_fix_lone_semicolon_keeps_valid() {
        let file = create_temp_file("for(;;) {}\nwhile(1);\n");
        SourceFixer::fix_lone_semicolon(file.path()).unwrap();
        assert_eq!(read_temp_file(&file), "for(;;) {}\nwhile(1);\n");
    }

    // ==================== fix_long_comments tests ====================

    #[test]
    fn test_fix_long_comments_short_line() {
        let file = create_temp_file("// short comment\n");
        SourceFixer::fix_long_comments(file.path(), 120).unwrap();
        assert_eq!(read_temp_file(&file), "// short comment\n");
    }

    #[test]
    fn test_fix_long_comments_breaks_long_line() {
        // Create a comment that exceeds 80 chars
        let long_comment = format!("// {}\n", "x".repeat(100));
        let file = create_temp_file(&long_comment);
        SourceFixer::fix_long_comments(file.path(), 80).unwrap();
        let result = read_temp_file(&file);
        // Should be broken into multiple lines
        assert!(result.lines().count() > 1);
    }

    #[test]
    fn test_fix_long_comments_chinese() {
        // Chinese chars count as 2 columns each
        // 50 Chinese chars = 100 columns
        let chinese_comment = format!("// {}\n", "中".repeat(50));
        let file = create_temp_file(&chinese_comment);
        SourceFixer::fix_long_comments(file.path(), 80).unwrap();
        let result = read_temp_file(&file);
        // Should be broken due to column width exceeding 80
        assert!(result.lines().count() > 1 || result.lines().any(|l| l.len() < 100));
    }

    // ==================== fix_pragma_separators tests ====================

    #[test]
    fn test_fix_pragma_mark_separator() {
        let file = create_temp_file("#pragma mark - - - -\n");
        SourceFixer::fix_pragma_separators(file.path()).unwrap();
        assert_eq!(read_temp_file(&file), "#pragma mark -\n");
    }

    #[test]
    fn test_fix_pragma_with_section_name() {
        // Test non-mark pragma with separator pattern
        let file = create_temp_file("#pragma webview delegate-- -- --\n");
        SourceFixer::fix_pragma_separators(file.path()).unwrap();
        assert_eq!(read_temp_file(&file), "#pragma mark - webview delegate\n");
    }

    #[test]
    fn test_fix_pragma_keeps_normal() {
        let file = create_temp_file("#pragma mark - Normal Section\n");
        SourceFixer::fix_pragma_separators(file.path()).unwrap();
        assert_eq!(read_temp_file(&file), "#pragma mark - Normal Section\n");
    }
}
