// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! NOLINT comment injection for suppressing lint issues.
//!
//! Supports language-specific comment formats:
//! - C/C++/ObjC: `// NOLINT(category)`
//! - Python: `# noqa: CODE` or `# type: ignore`
//! - Rust: `#[allow(clippy::rule)]` (attribute above the line)
//! - TypeScript/JavaScript: `// eslint-disable-next-line rule`
//! - Go: `//nolint:rule`
//! - Java: `@SuppressWarnings("rule")` or `// NOPMD`

use crate::utils::types::LintIssue;
use crate::Language;
use std::fs;

/// Result of adding a NOLINT comment
#[derive(Debug)]
pub enum NolintResult {
    /// Successfully added the comment
    Success,
    /// File was not modified (e.g., comment already exists)
    AlreadyIgnored,
    /// Failed to add comment
    Error(String),
}

/// Add a NOLINT comment to suppress the given issue.
///
/// The comment format depends on the language and linter source:
/// - C/C++/ObjC (clang-tidy): `// NOLINTNEXTLINE(check-name)`
/// - C/C++/ObjC (cpplint): `// NOLINT(category/rule)`
/// - Python (ruff/flake8): `# noqa: CODE`
/// - Python (mypy): `# type: ignore[error-code]`
/// - Rust (clippy): `#[allow(clippy::rule)]`
/// - TypeScript/JavaScript (eslint): `// eslint-disable-next-line rule`
/// - Go (golangci-lint): `//nolint:rule`
/// - Java (checkstyle): `// CHECKSTYLE:OFF`
///
/// # Arguments
/// * `issue` - The lint issue to suppress
///
/// # Returns
/// * `NolintResult` indicating success, already-ignored, or error
pub fn add_nolint_comment(issue: &LintIssue) -> NolintResult {
    let file_path = &issue.file_path;
    let line_num = issue.line;

    // Read file content
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => return NolintResult::Error(format!("Failed to read file: {}", e)),
    };

    let lines: Vec<&str> = content.lines().collect();

    // Validate line number
    if line_num == 0 || line_num > lines.len() {
        return NolintResult::Error(format!(
            "Invalid line number {} (file has {} lines)",
            line_num,
            lines.len()
        ));
    }

    let line_idx = line_num - 1;
    let current_line = lines[line_idx];

    // Determine language and generate appropriate comment
    let lang = issue.language.unwrap_or_else(|| {
        Language::from_path(file_path).unwrap_or(Language::Cpp)
    });

    let source = issue.source.as_deref().unwrap_or("");
    let code = issue.code.as_deref().unwrap_or("");

    // Check if already has a nolint comment
    if has_nolint_comment(current_line, lang, source) {
        return NolintResult::AlreadyIgnored;
    }

    // Generate the new content based on language and insertion strategy
    let new_content = match generate_nolint_content(&lines, line_idx, lang, source, code) {
        Ok(c) => c,
        Err(e) => return NolintResult::Error(e),
    };

    // Write back to file
    match fs::write(file_path, new_content) {
        Ok(_) => NolintResult::Success,
        Err(e) => NolintResult::Error(format!("Failed to write file: {}", e)),
    }
}

/// Check if a line already has a NOLINT-style comment
fn has_nolint_comment(line: &str, lang: Language, _source: &str) -> bool {
    let line_upper = line.to_uppercase();

    match lang {
        Language::Cpp | Language::ObjectiveC => {
            line_upper.contains("NOLINT") || line_upper.contains("NOLINTNEXTLINE")
        }
        Language::Python => {
            line.contains("# noqa") || line.contains("# type: ignore")
        }
        Language::Rust => {
            // Rust uses attributes, check if line above has #[allow(...)]
            // This is a simple check; the insertion logic handles the full case
            false
        }
        Language::TypeScript | Language::JavaScript => {
            line.contains("eslint-disable") || line.contains("@ts-ignore")
        }
        Language::Go => {
            line.contains("//nolint") || line.contains("// nolint")
        }
        Language::Java => {
            line.contains("@SuppressWarnings")
                || line_upper.contains("NOPMD")
                || line_upper.contains("CHECKSTYLE")
        }
    }
}

/// Generate new file content with the NOLINT comment inserted
fn generate_nolint_content(
    lines: &[&str],
    line_idx: usize,
    lang: Language,
    source: &str,
    code: &str,
) -> Result<String, String> {
    let mut result_lines: Vec<String> = Vec::with_capacity(lines.len() + 1);

    // Get indentation of the target line
    let target_line = lines[line_idx];
    let indent = get_indentation(target_line);

    match lang {
        Language::Cpp | Language::ObjectiveC => {
            // C/C++/ObjC: Add NOLINT comment at end of line or NOLINTNEXTLINE above
            for (i, line) in lines.iter().enumerate() {
                if i == line_idx {
                    let nolint = generate_cpp_nolint(source, code);
                    // Append to end of line
                    if line.trim().is_empty() {
                        result_lines.push(line.to_string());
                    } else {
                        result_lines.push(format!("{}  {}", line, nolint));
                    }
                } else {
                    result_lines.push(line.to_string());
                }
            }
        }
        Language::Python => {
            // Python: Add # noqa at end of line
            for (i, line) in lines.iter().enumerate() {
                if i == line_idx {
                    let noqa = generate_python_noqa(source, code);
                    if line.trim().is_empty() {
                        result_lines.push(line.to_string());
                    } else {
                        result_lines.push(format!("{}  {}", line, noqa));
                    }
                } else {
                    result_lines.push(line.to_string());
                }
            }
        }
        Language::Rust => {
            // Rust: Add #[allow(...)] attribute on line above
            for (i, line) in lines.iter().enumerate() {
                if i == line_idx {
                    let allow = generate_rust_allow(code);
                    result_lines.push(format!("{}{}", indent, allow));
                    result_lines.push(line.to_string());
                } else {
                    result_lines.push(line.to_string());
                }
            }
        }
        Language::TypeScript | Language::JavaScript => {
            // TS/JS: Add eslint-disable-next-line comment above
            for (i, line) in lines.iter().enumerate() {
                if i == line_idx {
                    let disable = generate_eslint_disable(code);
                    result_lines.push(format!("{}{}", indent, disable));
                    result_lines.push(line.to_string());
                } else {
                    result_lines.push(line.to_string());
                }
            }
        }
        Language::Go => {
            // Go: Add //nolint comment at end of line
            for (i, line) in lines.iter().enumerate() {
                if i == line_idx {
                    let nolint = generate_go_nolint(code);
                    if line.trim().is_empty() {
                        result_lines.push(line.to_string());
                    } else {
                        result_lines.push(format!("{} {}", line, nolint));
                    }
                } else {
                    result_lines.push(line.to_string());
                }
            }
        }
        Language::Java => {
            // Java: Add @SuppressWarnings or // NOPMD above
            for (i, line) in lines.iter().enumerate() {
                if i == line_idx {
                    let suppress = generate_java_suppress(source, code);
                    result_lines.push(format!("{}{}", indent, suppress));
                    result_lines.push(line.to_string());
                } else {
                    result_lines.push(line.to_string());
                }
            }
        }
    }

    // Join with newlines, preserving original line ending style
    let newline = if lines.iter().any(|l| l.ends_with('\r')) {
        "\r\n"
    } else {
        "\n"
    };

    Ok(result_lines.join(newline) + newline)
}

/// Get the leading whitespace (indentation) of a line
fn get_indentation(line: &str) -> &str {
    let trimmed_len = line.trim_start().len();
    &line[..line.len() - trimmed_len]
}

/// Generate C/C++/ObjC NOLINT comment
fn generate_cpp_nolint(source: &str, code: &str) -> String {
    let source_lower = source.to_lowercase();

    if source_lower.contains("clang-tidy") || source_lower.contains("clang_tidy") {
        // clang-tidy format
        if code.is_empty() {
            "// NOLINT".to_string()
        } else {
            format!("// NOLINT({})", code)
        }
    } else if source_lower.contains("cpplint") {
        // cpplint format
        if code.is_empty() {
            "// NOLINT".to_string()
        } else {
            format!("// NOLINT({})", code)
        }
    } else {
        // Generic NOLINT
        if code.is_empty() {
            "// NOLINT".to_string()
        } else {
            format!("// NOLINT({})", code)
        }
    }
}

/// Generate Python noqa comment
fn generate_python_noqa(source: &str, code: &str) -> String {
    let source_lower = source.to_lowercase();

    if source_lower.contains("mypy") || source_lower.contains("type") {
        // mypy type checking
        if code.is_empty() {
            "# type: ignore".to_string()
        } else {
            format!("# type: ignore[{}]", code)
        }
    } else {
        // ruff/flake8/pyflakes
        if code.is_empty() {
            "# noqa".to_string()
        } else {
            format!("# noqa: {}", code)
        }
    }
}

/// Generate Rust #[allow(...)] attribute
fn generate_rust_allow(code: &str) -> String {
    if code.is_empty() {
        "#[allow(warnings)]".to_string()
    } else if code.starts_with("clippy::") {
        format!("#[allow({})]", code)
    } else {
        // Assume it's a clippy rule if not specified
        format!("#[allow(clippy::{})]", code)
    }
}

/// Generate ESLint disable comment
fn generate_eslint_disable(code: &str) -> String {
    if code.is_empty() {
        "// eslint-disable-next-line".to_string()
    } else {
        format!("// eslint-disable-next-line {}", code)
    }
}

/// Generate Go nolint comment
fn generate_go_nolint(code: &str) -> String {
    if code.is_empty() {
        "//nolint".to_string()
    } else {
        format!("//nolint:{}", code)
    }
}

/// Generate Java suppress comment
fn generate_java_suppress(source: &str, code: &str) -> String {
    let source_lower = source.to_lowercase();

    if source_lower.contains("pmd") {
        "// NOPMD".to_string()
    } else if source_lower.contains("checkstyle") {
        "// CHECKSTYLE:OFF".to_string()
    } else {
        // Default to @SuppressWarnings
        if code.is_empty() {
            "@SuppressWarnings(\"all\")".to_string()
        } else {
            format!("@SuppressWarnings(\"{}\")", code)
        }
    }
}

/// Get a human-readable description of what NOLINT comment will be added
pub fn describe_nolint_action(issue: &LintIssue) -> String {
    let lang = issue.language.unwrap_or_else(|| {
        Language::from_path(&issue.file_path).unwrap_or(Language::Cpp)
    });
    let source = issue.source.as_deref().unwrap_or("");
    let code = issue.code.as_deref().unwrap_or("");

    match lang {
        Language::Cpp | Language::ObjectiveC => {
            format!("Add: {}", generate_cpp_nolint(source, code))
        }
        Language::Python => {
            format!("Add: {}", generate_python_noqa(source, code))
        }
        Language::Rust => {
            format!("Add: {}", generate_rust_allow(code))
        }
        Language::TypeScript | Language::JavaScript => {
            format!("Add: {}", generate_eslint_disable(code))
        }
        Language::Go => {
            format!("Add: {}", generate_go_nolint(code))
        }
        Language::Java => {
            format!("Add: {}", generate_java_suppress(source, code))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::utils::types::Severity;

    #[test]
    fn test_generate_cpp_nolint_clang_tidy() {
        let comment = generate_cpp_nolint("clang-tidy", "modernize-use-nullptr");
        assert_eq!(comment, "// NOLINT(modernize-use-nullptr)");
    }

    #[test]
    fn test_generate_cpp_nolint_cpplint() {
        let comment = generate_cpp_nolint("cpplint", "whitespace/newline");
        assert_eq!(comment, "// NOLINT(whitespace/newline)");
    }

    #[test]
    fn test_generate_cpp_nolint_empty() {
        let comment = generate_cpp_nolint("", "");
        assert_eq!(comment, "// NOLINT");
    }

    #[test]
    fn test_generate_python_noqa() {
        let comment = generate_python_noqa("ruff", "E501");
        assert_eq!(comment, "# noqa: E501");
    }

    #[test]
    fn test_generate_python_type_ignore() {
        let comment = generate_python_noqa("mypy", "arg-type");
        assert_eq!(comment, "# type: ignore[arg-type]");
    }

    #[test]
    fn test_generate_rust_allow() {
        let comment = generate_rust_allow("clippy::unwrap_used");
        assert_eq!(comment, "#[allow(clippy::unwrap_used)]");
    }

    #[test]
    fn test_generate_rust_allow_short() {
        let comment = generate_rust_allow("dead_code");
        assert_eq!(comment, "#[allow(clippy::dead_code)]");
    }

    #[test]
    fn test_generate_eslint_disable() {
        let comment = generate_eslint_disable("no-unused-vars");
        assert_eq!(comment, "// eslint-disable-next-line no-unused-vars");
    }

    #[test]
    fn test_generate_go_nolint() {
        let comment = generate_go_nolint("errcheck");
        assert_eq!(comment, "//nolint:errcheck");
    }

    #[test]
    fn test_generate_java_suppress() {
        let comment = generate_java_suppress("checkstyle", "");
        assert_eq!(comment, "// CHECKSTYLE:OFF");
    }

    #[test]
    fn test_get_indentation() {
        assert_eq!(get_indentation("    hello"), "    ");
        assert_eq!(get_indentation("\t\thello"), "\t\t");
        assert_eq!(get_indentation("hello"), "");
    }

    #[test]
    fn test_describe_nolint_action() {
        let issue = LintIssue::new(
            PathBuf::from("test.cpp"),
            10,
            "Test message".to_string(),
            Severity::Warning,
        )
        .with_language(Language::Cpp)
        .with_source("cpplint".to_string())
        .with_code("whitespace/newline".to_string());

        let desc = describe_nolint_action(&issue);
        assert!(desc.contains("NOLINT(whitespace/newline)"));
    }
}
