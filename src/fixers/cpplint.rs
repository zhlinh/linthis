// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Cpplint auto-fixer for C/C++ files.
//!
//! Fixes common cpplint issues by parsing cpplint output:
//! - `build/header_guard`: Fixes header guard naming based on cpplint suggestion
//! - `readability/todo`: Adds username to TODO comments
//! - `legal/copyright`: Inserts copyright header

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use regex::Regex;

/// Configuration for cpplint fixes
#[derive(Debug, Clone)]
pub struct CpplintFixerConfig {
    /// How to fix header guards: "fix_name" or "pragma_once"
    pub header_guard_mode: HeaderGuardMode,
    /// Username for TODO comments (default: git user or $USER)
    pub todo_username: Option<String>,
    /// Copyright template (with {year} placeholder)
    pub copyright_template: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HeaderGuardMode {
    /// Fix the header guard name based on cpplint suggestion
    FixName,
    /// Convert to #pragma once
    PragmaOnce,
    /// Don't fix header guards
    Disabled,
}

impl Default for CpplintFixerConfig {
    fn default() -> Self {
        Self {
            header_guard_mode: HeaderGuardMode::FixName,
            todo_username: None,
            copyright_template: None,
        }
    }
}

/// Parsed cpplint error
#[derive(Debug, Clone)]
struct CpplintError {
    line: usize,
    message: String,
    category: String,
}

/// Cpplint auto-fixer
pub struct CpplintFixer {
    config: CpplintFixerConfig,
    /// Cached username
    cached_username: Option<String>,
}

impl CpplintFixer {
    pub fn new() -> Self {
        Self {
            config: CpplintFixerConfig::default(),
            cached_username: None,
        }
    }

    pub fn with_config(config: CpplintFixerConfig) -> Self {
        Self {
            config,
            cached_username: None,
        }
    }

    /// Check if cpplint is available
    fn has_cpplint() -> bool {
        Command::new("cpplint")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Run cpplint and get errors
    fn run_cpplint(path: &Path) -> Vec<CpplintError> {
        if !Self::has_cpplint() {
            return Vec::new();
        }

        let output = Command::new("cpplint")
            .arg(path)
            .output();

        let output = match output {
            Ok(o) => o,
            Err(_) => return Vec::new(),
        };

        // cpplint outputs to stderr
        let stderr = String::from_utf8_lossy(&output.stderr);
        Self::parse_cpplint_output(&stderr)
    }

    /// Parse cpplint output into structured errors
    fn parse_cpplint_output(output: &str) -> Vec<CpplintError> {
        let mut errors = Vec::new();

        // Format: file:line: message [category] [confidence]
        // Example: test.h:8: #ifndef header guard has wrong style, please use: FOO_H_ [build/header_guard] [5]
        let re = Regex::new(r"^[^:]+:(\d+):\s*(.+?)\s*\[([^\]]+)\]").unwrap();

        for line in output.lines() {
            if let Some(caps) = re.captures(line) {
                if let Ok(line_num) = caps[1].parse::<usize>() {
                    errors.push(CpplintError {
                        line: line_num,
                        message: caps[2].to_string(),
                        category: caps[3].to_string(),
                    });
                }
            }
        }

        errors
    }

    /// Get the username for TODO comments
    fn get_username(&mut self) -> String {
        if let Some(ref username) = self.cached_username {
            return username.clone();
        }

        // 1. Use configured username if set
        if let Some(ref username) = self.config.todo_username {
            self.cached_username = Some(username.clone());
            return username.clone();
        }

        // 2. Try git config user.name
        if let Ok(output) = Command::new("git")
            .args(["config", "user.name"])
            .output()
        {
            if output.status.success() {
                let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !name.is_empty() {
                    // Convert to lowercase and replace spaces with underscores
                    let username = name.to_lowercase().replace(' ', "_");
                    self.cached_username = Some(username.clone());
                    return username;
                }
            }
        }

        // 3. Fall back to $USER environment variable
        if let Ok(user) = env::var("USER") {
            self.cached_username = Some(user.clone());
            return user;
        }

        // 4. Ultimate fallback
        "unknown".to_string()
    }

    /// Fix all cpplint issues in a file
    pub fn fix_file(&mut self, path: &Path) -> Result<bool, String> {
        if !path.exists() {
            return Err(format!("File not found: {}", path.display()));
        }

        // Run cpplint to get errors
        let errors = Self::run_cpplint(path);
        if errors.is_empty() {
            return Ok(false);
        }

        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut modified = false;

        // Process errors
        for error in &errors {
            match error.category.as_str() {
                "build/header_guard" => {
                    if self.config.header_guard_mode == HeaderGuardMode::FixName {
                        if self.fix_header_guard_from_error(&mut lines, error) {
                            modified = true;
                        }
                    } else if self.config.header_guard_mode == HeaderGuardMode::PragmaOnce {
                        if self.convert_to_pragma_once(&mut lines) {
                            modified = true;
                        }
                    }
                }
                "readability/todo" => {
                    if self.fix_todo_from_error(&mut lines, error) {
                        modified = true;
                    }
                }
                "legal/copyright" => {
                    if self.fix_copyright_from_error(&mut lines) {
                        modified = true;
                    }
                }
                "readability/casting" => {
                    if self.fix_c_style_cast(&mut lines, error) {
                        modified = true;
                    }
                }
                "readability/check" => {
                    if self.fix_assert_check(&mut lines, error) {
                        modified = true;
                    }
                }
                _ => {}
            }
        }

        if modified {
            let new_content = lines.join("\n") + if content.ends_with('\n') { "\n" } else { "" };
            fs::write(path, new_content)
                .map_err(|e| format!("Failed to write file: {}", e))?;
        }

        Ok(modified)
    }

    /// Fix header guard based on cpplint error message
    fn fix_header_guard_from_error(&self, lines: &mut [String], error: &CpplintError) -> bool {
        // Extract suggested guard name from message
        // Message format: "#ifndef header guard has wrong style, please use: GUARD_NAME_"
        // Or: "#endif line should be "#endif  // GUARD_NAME_""

        let suggested_guard = if error.message.contains("please use:") {
            // Extract from "#ifndef header guard has wrong style, please use: GUARD_NAME_"
            error.message
                .split("please use:")
                .nth(1)
                .map(|s| s.trim().to_string())
        } else if error.message.contains("#endif line should be") {
            // Extract from "#endif line should be "#endif  // GUARD_NAME_""
            Regex::new(r#"#endif\s+//\s+(\w+)"#)
                .ok()
                .and_then(|re| re.captures(&error.message))
                .and_then(|caps| caps.get(1))
                .map(|m| m.as_str().to_string())
        } else {
            None
        };

        let suggested_guard = match suggested_guard {
            Some(g) => g,
            None => return false,
        };

        let line_idx = error.line.saturating_sub(1);
        if line_idx >= lines.len() {
            return false;
        }

        let line = &lines[line_idx];

        // Fix #ifndef line
        if line.trim().starts_with("#ifndef") {
            lines[line_idx] = format!("#ifndef {}", suggested_guard);
            // Also fix the #define on the next line
            if line_idx + 1 < lines.len() && lines[line_idx + 1].trim().starts_with("#define") {
                lines[line_idx + 1] = format!("#define {}", suggested_guard);
            }
            return true;
        }

        // Fix #endif line
        if line.trim().starts_with("#endif") {
            lines[line_idx] = format!("#endif  // {}", suggested_guard);
            return true;
        }

        false
    }

    /// Convert header guards to #pragma once
    fn convert_to_pragma_once(&self, lines: &mut Vec<String>) -> bool {
        // Find #ifndef, #define, #endif pattern
        let mut ifndef_idx: Option<usize> = None;
        let mut define_idx: Option<usize> = None;
        let mut endif_idx: Option<usize> = None;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if ifndef_idx.is_none() && trimmed.starts_with("#ifndef") {
                ifndef_idx = Some(i);
            } else if ifndef_idx.is_some() && define_idx.is_none() && trimmed.starts_with("#define") {
                define_idx = Some(i);
            } else if trimmed.starts_with("#endif") {
                endif_idx = Some(i);
            }
        }

        let (ifndef_idx, define_idx, endif_idx) = match (ifndef_idx, define_idx, endif_idx) {
            (Some(a), Some(b), Some(c)) => (a, b, c),
            _ => return false,
        };

        // Verify structure
        if define_idx != ifndef_idx + 1 {
            return false;
        }

        // Check if already using #pragma once
        if lines.iter().any(|l| l.trim() == "#pragma once") {
            return false;
        }

        // Remove old guards and add #pragma once
        lines[ifndef_idx] = "#pragma once".to_string();
        lines[define_idx] = String::new();
        lines[endif_idx] = String::new();

        // Clean up empty lines
        lines.retain(|l| !l.is_empty() || l.trim() != "");

        true
    }

    /// Fix TODO comment based on cpplint error
    fn fix_todo_from_error(&mut self, lines: &mut [String], error: &CpplintError) -> bool {
        // Message: "Missing username in TODO; it should look like "// TODO(my_username): Stuff.""
        if !error.message.contains("Missing username in TODO") {
            return false;
        }

        let line_idx = error.line.saturating_sub(1);
        if line_idx >= lines.len() {
            return false;
        }

        let line = &lines[line_idx];
        let username = self.get_username();

        // Find TODO and add username
        if let Some(todo_pos) = line.find("TODO") {
            let prefix = &line[..todo_pos];
            let after_todo = &line[todo_pos + 4..];

            // Check if already has username
            if after_todo.trim_start().starts_with('(') {
                return false;
            }

            // Extract the rest of the TODO message
            let rest = after_todo.trim_start_matches([':', ' ']).trim();

            lines[line_idx] = if rest.is_empty() {
                format!("{}TODO({}): ", prefix, username)
            } else {
                format!("{}TODO({}): {}", prefix, username, rest)
            };

            return true;
        }

        false
    }

    /// Fix copyright based on cpplint error
    fn fix_copyright_from_error(&self, lines: &mut Vec<String>) -> bool {
        // Check if copyright already exists
        let first_lines: String = lines.iter().take(10).cloned().collect::<Vec<_>>().join("\n");
        if first_lines.to_lowercase().contains("copyright") {
            return false;
        }

        // Get copyright template
        let template = match &self.config.copyright_template {
            Some(t) => t.clone(),
            None => return false,
        };

        // Replace {year} with current year
        let year = chrono::Utc::now().format("%Y").to_string();
        let copyright = template.replace("{year}", &year);

        // Insert at the beginning
        let copyright_lines: Vec<String> = copyright.lines().map(|s| s.to_string()).collect();

        // Insert copyright lines at the beginning
        for (i, cline) in copyright_lines.into_iter().enumerate() {
            lines.insert(i, cline);
        }
        lines.insert(copyright.lines().count(), String::new()); // Add empty line after copyright

        true
    }

    /// Fix C-style cast to C++ style cast
    /// E.g., `(void*)0` -> `nullptr`, `(Type*)expr` -> `reinterpret_cast<Type*>(expr)`
    fn fix_c_style_cast(&self, lines: &mut [String], error: &CpplintError) -> bool {
        let line_idx = error.line.saturating_sub(1);
        if line_idx >= lines.len() {
            return false;
        }

        let line = &lines[line_idx];

        // Pattern 1: (void*)0 or ((void*)0) -> nullptr
        let nullptr_re = Regex::new(r"\(\(void\s*\*\)\s*0\)|\(void\s*\*\)\s*0").ok();
        if let Some(re) = nullptr_re {
            if re.is_match(line) {
                lines[line_idx] = re.replace_all(line, "nullptr").to_string();
                return true;
            }
        }

        // Pattern 2: (Type*)expr -> reinterpret_cast<Type*>(expr)
        // This is more complex and risky, so we only handle simple cases
        let cast_re = Regex::new(r"\((\w+\s*\*+)\)\s*(\w+)").ok();
        if let Some(re) = cast_re {
            if let Some(caps) = re.captures(line) {
                let cast_type = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let expr = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                if !cast_type.is_empty() && !expr.is_empty() {
                    let replacement = format!("reinterpret_cast<{}>({})", cast_type, expr);
                    lines[line_idx] = re.replace(line, replacement.as_str()).to_string();
                    return true;
                }
            }
        }

        false
    }

    /// Fix ASSERT_TRUE(a == b) -> ASSERT_EQ(a, b)
    /// And ASSERT_TRUE(a != b) -> ASSERT_NE(a, b)
    fn fix_assert_check(&self, lines: &mut [String], error: &CpplintError) -> bool {
        if !error.message.contains("Consider using ASSERT_") {
            return false;
        }

        let line_idx = error.line.saturating_sub(1);
        if line_idx >= lines.len() {
            return false;
        }

        let line = &lines[line_idx];

        // ASSERT_TRUE(a == b) -> ASSERT_EQ(a, b)
        let eq_re = Regex::new(r"ASSERT_TRUE\s*\(\s*(.+?)\s*==\s*(.+?)\s*\)").ok();
        if let Some(re) = eq_re {
            if let Some(caps) = re.captures(line) {
                let lhs = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
                let rhs = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                if !lhs.is_empty() && !rhs.is_empty() {
                    let replacement = format!("ASSERT_EQ({}, {})", lhs, rhs);
                    lines[line_idx] = re.replace(line, replacement.as_str()).to_string();
                    return true;
                }
            }
        }

        // ASSERT_TRUE(a != b) -> ASSERT_NE(a, b)
        let ne_re = Regex::new(r"ASSERT_TRUE\s*\(\s*(.+?)\s*!=\s*(.+?)\s*\)").ok();
        if let Some(re) = ne_re {
            if let Some(caps) = re.captures(line) {
                let lhs = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
                let rhs = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                if !lhs.is_empty() && !rhs.is_empty() {
                    let replacement = format!("ASSERT_NE({}, {})", lhs, rhs);
                    lines[line_idx] = re.replace(line, replacement.as_str()).to_string();
                    return true;
                }
            }
        }

        // ASSERT_FALSE(a == b) -> ASSERT_NE(a, b)
        let false_eq_re = Regex::new(r"ASSERT_FALSE\s*\(\s*(.+?)\s*==\s*(.+?)\s*\)").ok();
        if let Some(re) = false_eq_re {
            if let Some(caps) = re.captures(line) {
                let lhs = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
                let rhs = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                if !lhs.is_empty() && !rhs.is_empty() {
                    let replacement = format!("ASSERT_NE({}, {})", lhs, rhs);
                    lines[line_idx] = re.replace(line, replacement.as_str()).to_string();
                    return true;
                }
            }
        }

        false
    }
}

impl Default for CpplintFixer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpplint_output() {
        let output = r##"test.h:8:  #ifndef header guard has wrong style, please use: FOO_BAR_H_  [build/header_guard] [5]
test.h:76:  #endif line should be "#endif  // FOO_BAR_H_"  [build/header_guard] [5]
test.cc:17:  Missing username in TODO; it should look like "// TODO(my_username): Stuff."  [readability/todo] [2]
"##;

        let errors = CpplintFixer::parse_cpplint_output(output);
        assert_eq!(errors.len(), 3);

        assert_eq!(errors[0].line, 8);
        assert_eq!(errors[0].category, "build/header_guard");
        assert!(errors[0].message.contains("please use: FOO_BAR_H_"));

        assert_eq!(errors[1].line, 76);
        assert_eq!(errors[1].category, "build/header_guard");

        assert_eq!(errors[2].line, 17);
        assert_eq!(errors[2].category, "readability/todo");
    }

    #[test]
    fn test_fix_header_guard_from_error() {
        let fixer = CpplintFixer::new();

        let mut lines = vec![
            "#ifndef OLD_GUARD".to_string(),
            "#define OLD_GUARD".to_string(),
            "// content".to_string(),
            "#endif".to_string(),
        ];

        let error = CpplintError {
            line: 1,
            message: "#ifndef header guard has wrong style, please use: NEW_GUARD_H_".to_string(),
            category: "build/header_guard".to_string(),
        };

        assert!(fixer.fix_header_guard_from_error(&mut lines, &error));
        assert_eq!(lines[0], "#ifndef NEW_GUARD_H_");
        assert_eq!(lines[1], "#define NEW_GUARD_H_");
    }

    #[test]
    fn test_fix_todo_from_error() {
        let mut fixer = CpplintFixer::new();
        fixer.cached_username = Some("testuser".to_string());

        let mut lines = vec![
            "// TODO: fix this".to_string(),
            "// TODO(existing): keep this".to_string(),
        ];

        let error = CpplintError {
            line: 1,
            message: r#"Missing username in TODO; it should look like "// TODO(my_username): Stuff.""#.to_string(),
            category: "readability/todo".to_string(),
        };

        assert!(fixer.fix_todo_from_error(&mut lines, &error));
        assert_eq!(lines[0], "// TODO(testuser): fix this");
        assert_eq!(lines[1], "// TODO(existing): keep this");
    }
}
