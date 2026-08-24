// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Language-specific complexity analyzers.

mod go;
mod java;
mod python;
mod rust;
mod typescript;

pub use go::GoComplexityAnalyzer;
pub use java::JavaComplexityAnalyzer;
pub use python::PythonComplexityAnalyzer;
pub use rust::RustComplexityAnalyzer;
pub use typescript::TypeScriptComplexityAnalyzer;

/// Whether a trimmed line is prose or a directive rather than the start of a
/// function.
///
/// A detector that looks for a keyword anywhere on the line will happily read
/// `// Helper function to format a document` as declaring a function called
/// `to`, and then measure everything up to the next balanced brace as its
/// body. `#` covers Python comments, C preprocessor lines and Rust attributes
/// — none of which declare a function either.
pub(crate) fn is_comment_line(trimmed: &str) -> bool {
    trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
        || trimmed.starts_with('#')
}

/// Count the `{` and `}` on a line that actually open and close code blocks.
///
/// Braces inside string literals, char literals and `//` comments do not.
/// Counting them is not a rounding error: one unbalanced brace in a string —
/// `"else {"` in a keyword table, say — means the enclosing function never
/// appears to end, so every function after it is folded into that one and
/// reported with their combined complexity.
///
/// ponytail: a `/* … */` comment spanning several lines is still counted,
/// because no caller carries state between lines. Single-line block comments
/// are handled.
pub(crate) fn count_code_braces(line: &str) -> (i32, i32) {
    let bytes: Vec<char> = line.chars().collect();
    let (mut opens, mut closes) = (0, 0);
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            '/' if bytes.get(i + 1) == Some(&'/') => break,
            '/' if bytes.get(i + 1) == Some(&'*') => {
                i = skip_block_comment(&bytes, i + 2);
                continue;
            }
            '"' => {
                i = skip_string(&bytes, i + 1, '"');
                continue;
            }
            // A quote is only a char literal when it closes like one; in Rust
            // it is far more often a lifetime (`&'a str`).
            '\'' if is_char_literal(&bytes, i) => {
                i = skip_string(&bytes, i + 1, '\'');
                continue;
            }
            '{' => opens += 1,
            '}' => closes += 1,
            _ => {}
        }
        i += 1;
    }

    (opens, closes)
}

/// Index just past the closing `delim`, honoring backslash escapes.
fn skip_string(chars: &[char], mut i: usize, delim: char) -> usize {
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 2,
            c if c == delim => return i + 1,
            _ => i += 1,
        }
    }
    i
}

/// Index just past a closing `*/`, or the end of the line.
fn skip_block_comment(chars: &[char], mut i: usize) -> usize {
    while i + 1 < chars.len() {
        if chars[i] == '*' && chars[i + 1] == '/' {
            return i + 2;
        }
        i += 1;
    }
    chars.len()
}

/// Whether the quote at `i` opens a char literal (`'a'`, `'\n'`) rather than a
/// lifetime (`'a`).
fn is_char_literal(chars: &[char], i: usize) -> bool {
    match chars.get(i + 1) {
        Some('\\') => chars.get(i + 3) == Some(&'\'') || chars.get(i + 2) == Some(&'\''),
        Some(_) => chars.get(i + 2) == Some(&'\''),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_lines_never_declare_functions() {
        // The line that made the TypeScript analyzer report a function `to`.
        assert!(is_comment_line("// Helper function to format a document"));
        assert!(is_comment_line("* @param filePath"));
        assert!(is_comment_line("# def not_a_function():"));
        assert!(is_comment_line("#[allow(dead_code)]"));
        assert!(!is_comment_line("function format(x) {"));
    }

    #[test]
    fn braces_in_strings_do_not_open_blocks() {
        // The line that broke boundary detection in the Rust analyzer.
        let line = r#"    let keywords = ["else {", "loop{"];"#;
        assert_eq!(count_code_braces(line), (0, 0));
    }

    #[test]
    fn real_braces_still_count() {
        assert_eq!(count_code_braces("fn main() {"), (1, 0));
        assert_eq!(count_code_braces("}"), (0, 1));
        assert_eq!(count_code_braces("if x { y() } else { z() }"), (2, 2));
    }

    #[test]
    fn comments_are_ignored() {
        assert_eq!(count_code_braces("let x = 1; // }"), (0, 0));
        assert_eq!(count_code_braces("foo(); /* { */ bar();"), (0, 0));
        assert_eq!(count_code_braces("/* } */ {"), (1, 0));
    }

    #[test]
    fn lifetimes_are_not_char_literals() {
        // A naive quote-toggle would swallow the rest of this line.
        assert_eq!(count_code_braces("fn f<'a>(x: &'a str) {"), (1, 0));
        assert_eq!(count_code_braces("let c = '}';"), (0, 0));
        assert_eq!(count_code_braces(r"let c = '\'';"), (0, 0));
    }

    #[test]
    fn escaped_quotes_do_not_end_a_string() {
        assert_eq!(count_code_braces(r#"let s = "a\"{"; "#), (0, 0));
    }
}
