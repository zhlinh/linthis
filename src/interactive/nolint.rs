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

/// Diff information for a single line change
#[derive(Debug, Clone)]
pub struct LineDiff {
    pub line_number: usize,
    pub old_content: String,
    pub new_content: String,
    /// Context line before the change (for display)
    pub context_before: Option<String>,
    /// Context line after the change (for display)
    pub context_after: Option<String>,
}

/// Result of adding a NOLINT comment
#[derive(Debug)]
pub enum NolintResult {
    /// Successfully added the comment with diff information
    Success(Vec<LineDiff>),
    /// File was not modified (e.g., comment already exists)
    AlreadyIgnored,
    /// Failed to add comment
    Error(super::InteractiveError),
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
    use super::InteractiveError;

    let file_path = &issue.file_path;
    let line_num = issue.line;

    // Read file content
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            return NolintResult::Error(InteractiveError::FileOperation(format!(
                "Failed to read file '{}': {}",
                file_path.display(),
                e
            )))
        }
    };

    let lines: Vec<&str> = content.lines().collect();

    // Validate line number
    if line_num == 0 || line_num > lines.len() {
        return NolintResult::Error(InteractiveError::InvalidLineNumber {
            line: line_num,
            total: lines.len(),
        });
    }

    let line_idx = line_num - 1;
    let current_line = lines[line_idx];

    // Debug: Print line information
    eprintln!("[DEBUG] issue.line = {}, line_idx = {}, current_line = {:?}",
              line_num, line_idx, current_line);
    eprintln!("[DEBUG] issue.code_line = {:?}", issue.code_line);

    // Verify the line content matches what was recorded during check
    // This handles cases where line numbers shift due to previous modifications
    let actual_line_idx = if let Some(ref expected_code) = issue.code_line {
        let expected_trimmed = expected_code.trim();
        let current_trimmed = current_line.trim();

        // Check if current line matches expected content
        if current_trimmed == expected_trimmed {
            eprintln!("[DEBUG] Line content matches expected, using line {}", line_num);
            line_idx
        } else {
            // Line content doesn't match - file was modified or line numbers shifted
            eprintln!("[DEBUG] Line content mismatch!");
            eprintln!("[DEBUG]   Expected: {:?}", expected_trimmed);
            eprintln!("[DEBUG]   Current:  {:?}", current_trimmed);
            eprintln!("[DEBUG] Searching for matching content...");

            // Search within a reasonable range (±10 lines)
            let search_start = line_idx.saturating_sub(10);
            let search_end = (line_idx + 11).min(lines.len());

            let mut found_idx = None;
            let mut best_match_score: i32 = 0;

            for (i, line) in lines.iter().enumerate().skip(search_start).take(search_end - search_start) {
                let line_trimmed = line.trim();

                // Calculate base similarity score
                let base_score: i32 = if line_trimmed == expected_trimmed {
                    // Exact match is best
                    1000
                } else if line_trimmed.contains(expected_trimmed) || expected_trimmed.contains(line_trimmed) {
                    // Substring match
                    500
                } else {
                    // Check common tokens
                    let line_tokens: Vec<&str> = line_trimmed.split_whitespace().collect();
                    let expected_tokens: Vec<&str> = expected_trimmed.split_whitespace().collect();
                    let common_tokens = line_tokens.iter()
                        .filter(|t| expected_tokens.contains(t))
                        .count() as i32;
                    common_tokens * 50
                };

                // Apply distance penalty (prefer lines closer to original position)
                let distance = i.abs_diff(line_idx) as i32;
                let score = base_score - (distance * 5);

                // Only consider matches with good score (>= 400 for high confidence)
                if score > best_match_score && score >= 400 {
                    best_match_score = score;
                    found_idx = Some(i);
                    eprintln!("[DEBUG] Found match at line {}: base_score={}, distance={}, final_score={}, content={:?}",
                             i + 1, base_score, distance, score, line);
                }
            }

            if let Some(idx) = found_idx {
                eprintln!("[DEBUG] Using matched line {} instead of {}", idx + 1, line_num);
                idx
            } else {
                eprintln!("[DEBUG] No good match found, using original line {}", line_num);
                line_idx
            }
        }
    } else {
        // No code_line recorded, use original line number
        eprintln!("[DEBUG] No code_line recorded, using original line {}", line_num);
        line_idx
    };

    let line_idx = actual_line_idx;
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
    let (new_content, diffs) = match generate_nolint_content(&lines, line_idx, lang, source, code) {
        Ok(c) => c,
        Err(e) => return NolintResult::Error(e),
    };

    // Write back to file
    match fs::write(file_path, new_content) {
        Ok(_) => NolintResult::Success(diffs),
        Err(e) => NolintResult::Error(super::InteractiveError::FileOperation(format!(
            "Failed to write file '{}': {}",
            file_path.display(),
            e
        ))),
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
        // New languages - use generic ignore comment pattern
        Language::Dart => {
            line.contains("// ignore:") || line.contains("//ignore:")
        }
        Language::Swift => {
            line.contains("swiftlint:disable") || line.contains("// swiftlint:")
        }
        Language::Kotlin => {
            line.contains("@Suppress") || line_upper.contains("KTLINT-DISABLE")
        }
        Language::Lua => {
            line.contains("-- luacheck:") || line.contains("--luacheck:")
        }
        Language::Shell => {
            line.contains("# shellcheck disable") || line.contains("#shellcheck disable")
        }
        Language::Ruby => {
            line.contains("# rubocop:disable") || line.contains("#rubocop:disable")
        }
        Language::Php => {
            line.contains("// phpcs:ignore") || line.contains("//phpcs:ignore")
        }
        Language::Scala => {
            line.contains("// scalafix:ok") || line.contains("//scalafix:ok")
        }
        Language::CSharp => {
            line.contains("#pragma warning disable") || line.contains("// ReSharper disable")
        }
    }
}

/// Helper function to create LineDiff with context
fn create_diff_with_context(
    lines: &[&str],
    line_idx: usize,
    line_number: usize,
    old_content: String,
    new_content: String,
) -> LineDiff {
    // Get context before (one line before)
    let context_before = if line_idx > 0 {
        lines.get(line_idx - 1).map(|s| s.to_string())
    } else {
        None
    };

    // Get context after (one line after)
    let context_after = if line_idx + 1 < lines.len() {
        lines.get(line_idx + 1).map(|s| s.to_string())
    } else {
        None
    };

    LineDiff {
        line_number,
        old_content,
        new_content,
        context_before,
        context_after,
    }
}

/// Generate new file content with the NOLINT comment inserted
fn generate_nolint_content(
    lines: &[&str],
    line_idx: usize,
    lang: Language,
    source: &str,
    code: &str,
) -> Result<(String, Vec<LineDiff>), super::InteractiveError> {
    let mut result_lines: Vec<String> = Vec::with_capacity(lines.len() + 1);
    let mut diffs: Vec<LineDiff> = Vec::new();

    // Get indentation of the target line
    let target_line = lines[line_idx];
    let indent = get_indentation(target_line);

    match lang {
        Language::Cpp | Language::ObjectiveC => {
            // C/C++/ObjC: Smart NOLINT insertion
            // Use NOLINTNEXTLINE on previous line if adding NOLINT would exceed line length limit
            const MAX_LINE_LENGTH: usize = 100;

            for (i, line) in lines.iter().enumerate() {
                if i == line_idx {
                    let nolint = generate_cpp_nolint(source, code);

                    if line.trim().is_empty() {
                        result_lines.push(line.to_string());
                    } else {
                        // Check if adding NOLINT at end would exceed line length
                        let new_line_with_nolint = format!("{}  {}", line, nolint);

                        if new_line_with_nolint.len() > MAX_LINE_LENGTH {
                            // Use NOLINTNEXTLINE on previous line instead
                            let nolintnextline = generate_cpp_nolintnextline(source, code);
                            let prev_line = format!("{}{}", indent, nolintnextline);

                            // Insert NOLINTNEXTLINE before the target line
                            if !result_lines.is_empty() {
                                // For inserted line, context_after should be the target line itself
                                let context_before = if i > 0 {
                                    lines.get(i - 1).map(|s| s.to_string())
                                } else {
                                    None
                                };
                                let context_after = Some(line.to_string()); // Target line

                                diffs.push(LineDiff {
                                    line_number: i + 1,
                                    old_content: String::new(),
                                    new_content: prev_line.clone(),
                                    context_before,
                                    context_after,
                                });
                            }
                            result_lines.push(prev_line);
                            result_lines.push(line.to_string());
                        } else {
                            // Append NOLINT to end of line (line is short enough)
                            diffs.push(create_diff_with_context(
                                lines,
                                i,
                                i + 1,
                                line.to_string(),
                                new_line_with_nolint.clone(),
                            ));
                            result_lines.push(new_line_with_nolint);
                        }
                    }
                } else {
                    result_lines.push(line.to_string());
                }
            }
        }
        Language::Python => {
            // Python: Smart noqa insertion
            // Use comment on previous line if adding noqa would exceed line length limit
            const MAX_LINE_LENGTH: usize = 100;

            for (i, line) in lines.iter().enumerate() {
                if i == line_idx {
                    let noqa = generate_python_noqa(source, code);

                    if line.trim().is_empty() {
                        result_lines.push(line.to_string());
                    } else {
                        // Check if adding noqa at end would exceed line length
                        let new_line_with_noqa = format!("{}  {}", line, noqa);

                        if new_line_with_noqa.len() > MAX_LINE_LENGTH {
                            // Add comment on previous line instead
                            let prev_line = format!("{}# fmt: skip - {}", indent, noqa);

                            // Insert comment before the target line
                            if !result_lines.is_empty() {
                                // For inserted line, context_after should be the target line itself
                                let context_before = if i > 0 {
                                    lines.get(i - 1).map(|s| s.to_string())
                                } else {
                                    None
                                };
                                let context_after = Some(line.to_string()); // Target line

                                diffs.push(LineDiff {
                                    line_number: i + 1,
                                    old_content: String::new(),
                                    new_content: prev_line.clone(),
                                    context_before,
                                    context_after,
                                });
                            }
                            result_lines.push(prev_line);
                            result_lines.push(line.to_string());
                        } else {
                            // Append noqa to end of line (line is short enough)
                            diffs.push(create_diff_with_context(
                                lines,
                                i,
                                i + 1,
                                line.to_string(),
                                new_line_with_noqa.clone(),
                            ));
                            result_lines.push(new_line_with_noqa);
                        }
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
                    let new_line = format!("{}{}", indent, allow);
                    diffs.push(create_diff_with_context(
                        lines,
                        i,
                        i + 1,
                        String::new(),
                        new_line.clone(),
                    ));
                    result_lines.push(new_line);
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
                    let new_line = format!("{}{}", indent, disable);
                    diffs.push(create_diff_with_context(
                        lines,
                        i,
                        i + 1,
                        String::new(),
                        new_line.clone(),
                    ));
                    result_lines.push(new_line);
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
                        let new_line = format!("{} {}", line, nolint);
                        diffs.push(create_diff_with_context(
                            lines,
                            i,
                            i + 1,
                            line.to_string(),
                            new_line.clone(),
                        ));
                        result_lines.push(new_line);
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
                    let new_line = format!("{}{}", indent, suppress);
                    diffs.push(create_diff_with_context(
                        lines,
                        i,
                        i + 1,
                        String::new(),
                        new_line.clone(),
                    ));
                    result_lines.push(new_line);
                    result_lines.push(line.to_string());
                } else {
                    result_lines.push(line.to_string());
                }
            }
        }
        Language::Dart => {
            // Dart: Add // ignore: comment above
            for (i, line) in lines.iter().enumerate() {
                if i == line_idx {
                    let ignore = generate_dart_ignore(code);
                    let new_line = format!("{}{}", indent, ignore);
                    diffs.push(create_diff_with_context(
                        lines,
                        i,
                        i + 1,
                        String::new(),
                        new_line.clone(),
                    ));
                    result_lines.push(new_line);
                    result_lines.push(line.to_string());
                } else {
                    result_lines.push(line.to_string());
                }
            }
        }
        Language::Swift => {
            // Swift: Add // swiftlint:disable:next comment above
            for (i, line) in lines.iter().enumerate() {
                if i == line_idx {
                    let disable = generate_swift_disable(code);
                    let new_line = format!("{}{}", indent, disable);
                    diffs.push(create_diff_with_context(
                        lines,
                        i,
                        i + 1,
                        String::new(),
                        new_line.clone(),
                    ));
                    result_lines.push(new_line);
                    result_lines.push(line.to_string());
                } else {
                    result_lines.push(line.to_string());
                }
            }
        }
        Language::Kotlin => {
            // Kotlin: Add @Suppress annotation above
            for (i, line) in lines.iter().enumerate() {
                if i == line_idx {
                    let suppress = generate_kotlin_suppress(code);
                    let new_line = format!("{}{}", indent, suppress);
                    diffs.push(create_diff_with_context(
                        lines,
                        i,
                        i + 1,
                        String::new(),
                        new_line.clone(),
                    ));
                    result_lines.push(new_line);
                    result_lines.push(line.to_string());
                } else {
                    result_lines.push(line.to_string());
                }
            }
        }
        Language::Lua => {
            // Lua: Add -- luacheck: ignore comment at end of line
            for (i, line) in lines.iter().enumerate() {
                if i == line_idx {
                    let ignore = generate_lua_ignore(code);
                    if line.trim().is_empty() {
                        result_lines.push(line.to_string());
                    } else {
                        let new_line = format!("{} {}", line, ignore);
                        diffs.push(create_diff_with_context(
                            lines,
                            i,
                            i + 1,
                            line.to_string(),
                            new_line.clone(),
                        ));
                        result_lines.push(new_line);
                    }
                } else {
                    result_lines.push(line.to_string());
                }
            }
        }
        Language::Shell => {
            // Shell: Add # shellcheck disable=SCXXXX comment on previous line
            for (i, line) in lines.iter().enumerate() {
                if i == line_idx {
                    let new_line = format!("{}# shellcheck disable={}", indent, code);
                    diffs.push(create_diff_with_context(
                        lines,
                        i,
                        i + 1,
                        String::new(),
                        new_line.clone(),
                    ));
                    result_lines.push(new_line);
                    result_lines.push(line.to_string());
                } else {
                    result_lines.push(line.to_string());
                }
            }
        }
        Language::Ruby => {
            // Ruby: Add # rubocop:disable CopName comment at end of line
            for (i, line) in lines.iter().enumerate() {
                if i == line_idx {
                    let ignore = generate_ruby_disable(code);
                    if line.trim().is_empty() {
                        result_lines.push(line.to_string());
                    } else {
                        let new_line = format!("{} {}", line, ignore);
                        diffs.push(create_diff_with_context(
                            lines,
                            i,
                            i + 1,
                            line.to_string(),
                            new_line.clone(),
                        ));
                        result_lines.push(new_line);
                    }
                } else {
                    result_lines.push(line.to_string());
                }
            }
        }
        Language::Php => {
            // PHP: Add // phpcs:ignore comment on previous line
            for (i, line) in lines.iter().enumerate() {
                if i == line_idx {
                    let new_line = format!("{}// phpcs:ignore {}", indent, code);
                    diffs.push(create_diff_with_context(
                        lines,
                        i,
                        i + 1,
                        String::new(),
                        new_line.clone(),
                    ));
                    result_lines.push(new_line);
                    result_lines.push(line.to_string());
                } else {
                    result_lines.push(line.to_string());
                }
            }
        }
        Language::Scala => {
            // Scala: Add // scalafix:ok RuleName comment at end of line
            for (i, line) in lines.iter().enumerate() {
                if i == line_idx {
                    let ignore = generate_scala_ok(code);
                    if line.trim().is_empty() {
                        result_lines.push(line.to_string());
                    } else {
                        let new_line = format!("{} {}", line, ignore);
                        diffs.push(create_diff_with_context(
                            lines,
                            i,
                            i + 1,
                            line.to_string(),
                            new_line.clone(),
                        ));
                        result_lines.push(new_line);
                    }
                } else {
                    result_lines.push(line.to_string());
                }
            }
        }
        Language::CSharp => {
            // C#: Add #pragma warning disable XXXX on previous line
            for (i, line) in lines.iter().enumerate() {
                if i == line_idx {
                    let new_line = format!("{}#pragma warning disable {}", indent, code);
                    diffs.push(create_diff_with_context(
                        lines,
                        i,
                        i + 1,
                        String::new(),
                        new_line.clone(),
                    ));
                    result_lines.push(new_line);
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

    Ok((result_lines.join(newline) + newline, diffs))
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

/// Generate C/C++/ObjC NOLINTNEXTLINE comment (for previous line insertion)
fn generate_cpp_nolintnextline(source: &str, code: &str) -> String {
    let source_lower = source.to_lowercase();

    if source_lower.contains("clang-tidy") || source_lower.contains("clang_tidy") {
        // clang-tidy format
        if code.is_empty() {
            "// NOLINTNEXTLINE".to_string()
        } else {
            format!("// NOLINTNEXTLINE({})", code)
        }
    } else if source_lower.contains("cpplint") {
        // cpplint format
        if code.is_empty() {
            "// NOLINTNEXTLINE".to_string()
        } else {
            format!("// NOLINTNEXTLINE({})", code)
        }
    } else {
        // Generic NOLINTNEXTLINE
        if code.is_empty() {
            "// NOLINTNEXTLINE".to_string()
        } else {
            format!("// NOLINTNEXTLINE({})", code)
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

/// Generate Dart ignore comment
fn generate_dart_ignore(code: &str) -> String {
    if code.is_empty() {
        "// ignore: all".to_string()
    } else {
        format!("// ignore: {}", code)
    }
}

/// Generate Swift disable comment
fn generate_swift_disable(code: &str) -> String {
    if code.is_empty() {
        "// swiftlint:disable:next all".to_string()
    } else {
        format!("// swiftlint:disable:next {}", code)
    }
}

/// Generate Kotlin suppress annotation
fn generate_kotlin_suppress(code: &str) -> String {
    if code.is_empty() {
        "@Suppress(\"all\")".to_string()
    } else {
        format!("@Suppress(\"{}\")", code)
    }
}

/// Generate Lua ignore comment
fn generate_lua_ignore(code: &str) -> String {
    if code.is_empty() {
        "-- luacheck: ignore".to_string()
    } else {
        format!("-- luacheck: ignore {}", code)
    }
}

/// Generate Ruby rubocop:disable comment
fn generate_ruby_disable(code: &str) -> String {
    if code.is_empty() {
        "# rubocop:disable all".to_string()
    } else {
        format!("# rubocop:disable {}", code)
    }
}

/// Generate Scala scalafix:ok comment
fn generate_scala_ok(code: &str) -> String {
    if code.is_empty() {
        "// scalafix:ok".to_string()
    } else {
        format!("// scalafix:ok {}", code)
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
        Language::Dart => {
            format!("Add: {}", generate_dart_ignore(code))
        }
        Language::Swift => {
            format!("Add: {}", generate_swift_disable(code))
        }
        Language::Kotlin => {
            format!("Add: {}", generate_kotlin_suppress(code))
        }
        Language::Lua => {
            format!("Add: {}", generate_lua_ignore(code))
        }
        Language::Shell => {
            format!("Add: # shellcheck disable={}", code)
        }
        Language::Ruby => {
            format!("Add: {}", generate_ruby_disable(code))
        }
        Language::Php => {
            format!("Add: // phpcs:ignore {}", code)
        }
        Language::Scala => {
            format!("Add: {}", generate_scala_ok(code))
        }
        Language::CSharp => {
            format!("Add: #pragma warning disable {}", code)
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
