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

    if issue.line == 0 || issue.line > lines.len() {
        return NolintResult::Error(InteractiveError::InvalidLineNumber {
            line: issue.line,
            total: lines.len(),
        });
    }

    // The recorded line may have shifted since the check ran.
    let line_idx = resolve_line_index(&lines, issue.line - 1, issue.code_line.as_deref());

    let lang = issue
        .language
        .unwrap_or_else(|| Language::from_path(file_path).unwrap_or(Language::Cpp));
    let source = issue.source.as_deref().unwrap_or("");
    let code = issue.code.as_deref().unwrap_or("");

    if has_nolint_comment(lines[line_idx], lang, source) {
        return NolintResult::AlreadyIgnored;
    }

    let (new_content, diffs) = match generate_nolint_content(&lines, line_idx, lang, source, code) {
        Ok(c) => c,
        Err(e) => return NolintResult::Error(e),
    };

    match fs::write(file_path, new_content) {
        Ok(_) => NolintResult::Success(diffs),
        Err(e) => NolintResult::Error(super::InteractiveError::FileOperation(format!(
            "Failed to write file '{}': {}",
            file_path.display(),
            e
        ))),
    }
}

/// Find the line the issue actually refers to now.
///
/// Line numbers come from an earlier check run, so an edit since then can have
/// moved the code. When the recorded text no longer matches, look for it
/// within ±10 lines and take the best-scoring candidate; if nothing scores
/// well enough, keep the original number rather than guess.
fn resolve_line_index(lines: &[&str], line_idx: usize, expected: Option<&str>) -> usize {
    let Some(expected) = expected.map(str::trim) else {
        return line_idx;
    };
    if lines[line_idx].trim() == expected {
        return line_idx;
    }

    const SEARCH_RADIUS: usize = 10;
    /// Below this, a candidate is too weak to be worth moving to.
    const MIN_SCORE: i32 = 400;

    let start = line_idx.saturating_sub(SEARCH_RADIUS);
    let end = (line_idx + SEARCH_RADIUS + 1).min(lines.len());

    lines[start..end]
        .iter()
        .enumerate()
        .map(|(offset, line)| (start + offset, line))
        .filter_map(|(i, line)| {
            let score = match_score(line.trim(), expected) - (i.abs_diff(line_idx) as i32 * 5);
            (score >= MIN_SCORE).then_some((score, i))
        })
        .max_by_key(|(score, _)| *score)
        .map(|(_, i)| i)
        .unwrap_or(line_idx)
}

/// How closely a candidate line resembles the recorded one: exact, substring,
/// or shared-token overlap.
fn match_score(candidate: &str, expected: &str) -> i32 {
    if candidate == expected {
        return 1000;
    }
    if candidate.contains(expected) || expected.contains(candidate) {
        return 500;
    }
    let expected_tokens: Vec<&str> = expected.split_whitespace().collect();
    let common = candidate
        .split_whitespace()
        .filter(|t| expected_tokens.contains(t))
        .count() as i32;
    common * 50
}

/// Check if a line already has a NOLINT-style comment
fn has_nolint_comment(line: &str, lang: Language, _source: &str) -> bool {
    let haystack = line.to_uppercase();
    existing_markers(lang)
        .iter()
        .any(|m| haystack.contains(&m.to_uppercase()))
}

/// Markers that mean "this line is already suppressed" for a language.
///
/// Matching is case-insensitive, so `# NOQA` counts like `# noqa` — a linter
/// accepts both, and a false negative here means adding a second, redundant
/// comment.
fn existing_markers(lang: Language) -> &'static [&'static str] {
    match lang {
        Language::Cpp | Language::ObjectiveC => &["NOLINT"],
        Language::Python => &["# noqa", "# type: ignore"],
        // Rust suppression is an attribute on the line above, which this
        // single-line check cannot see; the insertion logic handles it.
        Language::Rust => &[],
        Language::TypeScript | Language::JavaScript => &["eslint-disable", "@ts-ignore"],
        Language::Go => &["//nolint", "// nolint"],
        Language::Java => &["@SuppressWarnings", "NOPMD", "CHECKSTYLE"],
        Language::Dart => &["// ignore:", "//ignore:"],
        Language::Swift => &["swiftlint:disable", "// swiftlint:"],
        Language::Kotlin => &["@Suppress", "KTLINT-DISABLE"],
        Language::Lua => &["-- luacheck:", "--luacheck:"],
        Language::Shell => &["# shellcheck disable", "#shellcheck disable"],
        Language::Ruby => &["# rubocop:disable", "#rubocop:disable"],
        Language::Php => &["// phpcs:ignore", "//phpcs:ignore"],
        Language::Scala => &["// scalafix:ok", "//scalafix:ok"],
        Language::CSharp => &["#pragma warning disable", "// ReSharper disable"],
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
/// Where a suppression comment goes relative to the offending line.
///
/// The fifteen languages only ever do one of these three things; before this
/// was an enum each language repeated the whole insert-and-diff loop.
enum Placement {
    /// Its own line above the target, at the target's indentation.
    Above(String),
    /// Appended to the target line. A blank line is left alone.
    Append(String),
    /// Appended, unless that would push the line past `MAX_LINE_LENGTH` — then
    /// `fallback` goes above instead, which is a different comment form
    /// (`NOLINTNEXTLINE`, `# fmt: skip`).
    AppendOrAbove { inline: String, fallback: String },
}

/// Longest line we are willing to create by appending a comment.
const MAX_LINE_LENGTH: usize = 100;

impl Placement {
    /// The comment as it would appear inline, which is also what we show the
    /// user as "the way to ignore this".
    fn comment(&self) -> &str {
        match self {
            Placement::Above(text) | Placement::Append(text) => text,
            Placement::AppendOrAbove { inline, .. } => inline,
        }
    }
}

/// The suppression this language and linter understand, and where it goes.
fn placement_for(lang: Language, source: &str, code: &str) -> Placement {
    match lang {
        Language::Cpp | Language::ObjectiveC => Placement::AppendOrAbove {
            inline: generate_cpp_nolint(source, code),
            fallback: generate_cpp_nolintnextline(source, code),
        },
        Language::Python => {
            let noqa = generate_python_noqa(source, code);
            Placement::AppendOrAbove {
                fallback: format!("# fmt: skip - {}", noqa),
                inline: noqa,
            }
        }
        Language::Rust => Placement::Above(generate_rust_allow(code)),
        Language::TypeScript | Language::JavaScript => {
            Placement::Above(generate_eslint_disable(code))
        }
        Language::Go => Placement::Append(generate_go_nolint(code)),
        Language::Java => Placement::Above(generate_java_suppress(source, code)),
        Language::Dart => Placement::Above(generate_dart_ignore(code)),
        Language::Swift => Placement::Above(generate_swift_disable(code)),
        Language::Kotlin => Placement::Above(generate_kotlin_suppress(code)),
        Language::Lua => Placement::Append(generate_lua_ignore(code)),
        Language::Shell => Placement::Above(format!("# shellcheck disable={}", code)),
        Language::Ruby => Placement::Append(generate_ruby_disable(code)),
        Language::Php => Placement::Above(format!("// phpcs:ignore {}", code)),
        Language::Scala => Placement::Append(generate_scala_ok(code)),
        Language::CSharp => Placement::Above(format!("#pragma warning disable {}", code)),
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

    let indent = get_indentation(lines[line_idx]);
    let placement = placement_for(lang, source, code);

    for (i, line) in lines.iter().enumerate() {
        if i != line_idx {
            result_lines.push(line.to_string());
            continue;
        }
        insert_at_target(
            &placement,
            lines,
            i,
            indent,
            &mut result_lines,
            &mut diffs,
        );
    }

    // Join with newlines, preserving original line ending style
    let newline = if lines.iter().any(|l| l.ends_with('\r')) {
        "\r\n"
    } else {
        "\n"
    };

    Ok((result_lines.join(newline) + newline, diffs))
}

/// Emit the target line (and the comment) into `result_lines`, recording the
/// diff for the interactive preview.
fn insert_at_target(
    placement: &Placement,
    lines: &[&str],
    i: usize,
    indent: &str,
    result_lines: &mut Vec<String>,
    diffs: &mut Vec<LineDiff>,
) {
    let line = lines[i];

    match placement {
        Placement::Above(text) => {
            let new_line = format!("{}{}", indent, text);
            diffs.push(create_diff_with_context(
                lines,
                i,
                i + 1,
                String::new(),
                new_line.clone(),
            ));
            result_lines.push(new_line);
            result_lines.push(line.to_string());
        }
        Placement::Append(text) => {
            if line.trim().is_empty() {
                result_lines.push(line.to_string());
                return;
            }
            let new_line = format!("{} {}", line, text);
            diffs.push(create_diff_with_context(
                lines,
                i,
                i + 1,
                line.to_string(),
                new_line.clone(),
            ));
            result_lines.push(new_line);
        }
        Placement::AppendOrAbove { inline, fallback } => {
            if line.trim().is_empty() {
                result_lines.push(line.to_string());
                return;
            }
            let appended = format!("{}  {}", line, inline);
            if appended.len() <= MAX_LINE_LENGTH {
                diffs.push(create_diff_with_context(
                    lines,
                    i,
                    i + 1,
                    line.to_string(),
                    appended.clone(),
                ));
                result_lines.push(appended);
                return;
            }

            let above = format!("{}{}", indent, fallback);
            // The inserted line has no "before" when it lands at the top of
            // the file, and its "after" is the target line itself.
            if !result_lines.is_empty() {
                diffs.push(LineDiff {
                    line_number: i + 1,
                    old_content: String::new(),
                    new_content: above.clone(),
                    context_before: i.checked_sub(1).and_then(|p| lines.get(p)).map(|s| s.to_string()),
                    context_after: Some(line.to_string()),
                });
            }
            result_lines.push(above);
            result_lines.push(line.to_string());
        }
    }
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
/// The comment that suppresses `issue` where it stands, e.g. `# noqa: E501`.
///
/// This is the raw comment only — no prose. Callers that show it as an action
/// wrap it themselves (see [`describe_nolint_action`]); the ignore hints in
/// lint output print it verbatim for the user to paste.
pub fn suppression_comment(issue: &LintIssue) -> String {
    let lang = issue
        .language
        .unwrap_or_else(|| Language::from_path(&issue.file_path).unwrap_or(Language::Cpp));

    placement_for(
        lang,
        issue.source.as_deref().unwrap_or(""),
        issue.code.as_deref().unwrap_or(""),
    )
    .comment()
    .to_string()
}

/// Describe the nolint action for the interactive prompt.
pub fn describe_nolint_action(issue: &LintIssue) -> String {
    format!("Add: {}", suppression_comment(issue))
}

#[cfg(test)]
mod tests {

    // The three placements below used to be fifteen copies of the same loop.
    // These pin the behaviour each copy had, so the shared loop cannot drift.

    fn generate(lines: &[&str], idx: usize, lang: Language, source: &str, code: &str) -> String {
        generate_nolint_content(lines, idx, lang, source, code)
            .unwrap()
            .0
    }

    #[test]
    fn above_placement_inserts_an_indented_line() {
        let out = generate(&["fn a() {", "    let x = 1;", "}"], 1, Language::Rust, "clippy", "foo");
        assert_eq!(
            out,
            "fn a() {\n    #[allow(clippy::foo)]\n    let x = 1;\n}\n"
        );
    }

    #[test]
    fn append_placement_appends_and_skips_blank_lines() {
        let out = generate(&["x := 1"], 0, Language::Go, "golangci-lint", "errcheck");
        assert_eq!(out, "x := 1 //nolint:errcheck\n");

        // A blank target line is left exactly as it was.
        let blank = generate(&["", "y := 2"], 0, Language::Go, "golangci-lint", "errcheck");
        assert_eq!(blank, "\ny := 2\n");
    }

    #[test]
    fn append_or_above_falls_back_when_the_line_gets_too_long() {
        let short = generate(&["x = 1"], 0, Language::Python, "ruff", "E501");
        assert_eq!(short, "x = 1  # noqa: E501\n");

        let long_line = format!("    x = \"{}\"", "a".repeat(90));
        let out = generate(&[long_line.as_str()], 0, Language::Python, "ruff", "E501");
        // Too long to append, so the comment goes above at the same indent.
        assert_eq!(out, format!("    # fmt: skip - # noqa: E501\n{long_line}\n"));
    }

    #[test]
    fn crlf_line_endings_survive() {
        let out = generate(&["x := 1\r"], 0, Language::Go, "golangci-lint", "errcheck");
        assert!(out.ends_with("\r\n"), "got {out:?}");
    }

    #[test]
    fn resolve_line_index_follows_shifted_code() {
        let lines = ["fn a() {", "    let x = 1;", "    let y = 2;", "}"];

        // Recorded text still where it was recorded.
        assert_eq!(resolve_line_index(&lines, 1, Some("let x = 1;")), 1);
        // Recorded text moved down one line after an edit above it.
        assert_eq!(resolve_line_index(&lines, 1, Some("let y = 2;")), 2);
        // Nothing resembles it — stay put rather than guess.
        assert_eq!(resolve_line_index(&lines, 1, Some("totally_unrelated()")), 1);
        // No recorded text at all.
        assert_eq!(resolve_line_index(&lines, 1, None), 1);
    }

    #[test]
    fn existing_suppressions_are_detected_case_insensitively() {
        assert!(has_nolint_comment("x = 1  # noqa: E501", Language::Python, ""));
        assert!(has_nolint_comment("x = 1  # NOQA", Language::Python, ""));
        assert!(has_nolint_comment("int x;  // NOLINT", Language::Cpp, ""));
        assert!(!has_nolint_comment("x = 1", Language::Python, ""));
    }
    use super::*;
    use crate::utils::types::Severity;
    use std::path::PathBuf;

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
