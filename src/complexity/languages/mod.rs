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
mod kotlin;
mod python;
mod rust;
mod typescript;

pub use go::GoComplexityAnalyzer;
pub use java::JavaComplexityAnalyzer;
pub use kotlin::KotlinComplexityAnalyzer;
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

/// What one scanned line contributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineKind {
    Blank,
    Comment,
    Code,
}

/// One line's contribution: the braces that actually open and close code
/// blocks, and whether the line is code at all.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LineScan {
    pub opens: i32,
    pub closes: i32,
    pub kind: LineKind,
}

impl LineScan {
    pub(crate) fn is_code(&self) -> bool {
        self.kind == LineKind::Code
    }
}

/// Walks C-family source a line at a time, carrying `/* … */` state across
/// line boundaries.
///
/// Two things depend on getting this right, and both used to get it wrong
/// because each line was judged on its own:
///
/// * **Braces.** A brace inside a string, a char literal or a comment opens no
///   block. One unbalanced brace — `"else {"` in a keyword table, or a `{` in a
///   commented-out block — means the enclosing function never appears to end,
///   so every function after it is folded into that one and reported with their
///   combined complexity.
/// * **Line counts.** `sloc` is what the function-length limit measures, so
///   prose counted as code can report a well-documented function as too long.
///
/// A per-line check cannot see that the middle line of
///
/// ```text
/// /* one
///    two */
/// ```
///
/// is still comment: it starts with neither `/*` nor `*`. Only a scanner that
/// remembers the open comment gets it right.
pub(crate) struct SourceScanner {
    in_block: bool,
}

/// What the scanner found at one position, and where to continue from.
enum Token {
    /// Everything from here to the end of the line is comment.
    RestIsComment,
    /// Comment text ending at this index.
    Comment(usize),
    /// Code text ending at this index.
    Code(usize),
    /// A `{` — continue at this index.
    Open(usize),
    /// A `}` — continue at this index.
    Close(usize),
    /// Whitespace, which is neither.
    Space(usize),
}

impl SourceScanner {
    pub(crate) fn new() -> Self {
        Self { in_block: false }
    }

    /// Scan the next line and advance the comment state.
    ///
    /// A line carrying both code and comment counts as code, so `foo(); // why`
    /// is a source line.
    pub(crate) fn scan(&mut self, line: &str) -> LineScan {
        if line.trim().is_empty() {
            // Whitespace opens and closes nothing, so the state is unchanged
            // whether or not we are inside a comment.
            return LineScan {
                opens: 0,
                closes: 0,
                kind: LineKind::Blank,
            };
        }

        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        let (mut opens, mut closes) = (0, 0);
        let mut saw_code = false;
        let mut saw_comment = false;

        while i < chars.len() {
            match self.next_token(&chars, i) {
                Token::RestIsComment => {
                    saw_comment = true;
                    break;
                }
                Token::Comment(next) => {
                    saw_comment = true;
                    i = next;
                }
                Token::Code(next) => {
                    saw_code = true;
                    i = next;
                }
                Token::Open(next) => {
                    saw_code = true;
                    opens += 1;
                    i = next;
                }
                Token::Close(next) => {
                    saw_code = true;
                    closes += 1;
                    i = next;
                }
                Token::Space(next) => i = next,
            }
        }

        let kind = if saw_code {
            LineKind::Code
        } else if saw_comment {
            LineKind::Comment
        } else {
            LineKind::Blank
        };

        LineScan {
            opens,
            closes,
            kind,
        }
    }

    /// Identify the token at `i`, consuming any comment state change it causes.
    fn next_token(&mut self, chars: &[char], i: usize) -> Token {
        // Inside an open comment nothing else is recognized until it closes.
        if self.in_block {
            return match block_comment_end(chars, i) {
                Some(end) => {
                    self.in_block = false;
                    Token::Comment(end)
                }
                None => Token::RestIsComment,
            };
        }

        match chars[i] {
            // A `/*` inside a line comment opens nothing.
            '/' if chars.get(i + 1) == Some(&'/') => Token::RestIsComment,
            '/' if chars.get(i + 1) == Some(&'*') => match block_comment_end(chars, i + 2) {
                Some(end) => Token::Comment(end),
                None => {
                    self.in_block = true;
                    Token::RestIsComment
                }
            },
            // Braces and comment openers inside a literal mean nothing.
            '"' | '\'' if opens_literal(chars, i) => {
                Token::Code(skip_string(chars, i + 1, chars[i]))
            }
            '{' => Token::Open(i + 1),
            '}' => Token::Close(i + 1),
            c if c.is_whitespace() => Token::Space(i + 1),
            _ => Token::Code(i + 1),
        }
    }
}

/// Scan every line of a block of source in one pass.
///
/// Analyzers index into the result rather than re-scanning, which both shares
/// the comment state and lets a loop skip over a function body without losing
/// its place.
pub(crate) fn scan_lines(lines: &[&str]) -> Vec<LineScan> {
    let mut scanner = SourceScanner::new();
    lines.iter().map(|line| scanner.scan(line)).collect()
}

/// How many lines of a block of source carry code, and how many carry only
/// comment.
pub(crate) struct LineCounts {
    pub source: u32,
    pub comment: u32,
}

/// Count the source and comment lines of C-family source.
///
/// Python is deliberately not routed through here: it has no block comments,
/// so its per-line `#` test is already right. (A `"""…"""` string used as a
/// comment is an expression statement, not a comment, and stays code.)
pub(crate) fn count_lines(lines: &[&str]) -> LineCounts {
    let mut counts = LineCounts {
        source: 0,
        comment: 0,
    };

    for scan in scan_lines(lines) {
        match scan.kind {
            LineKind::Code => counts.source += 1,
            LineKind::Comment => counts.comment += 1,
            LineKind::Blank => {}
        }
    }

    counts
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

/// Index just past a closing `*/`, or `None` when the comment runs on to the
/// next line.
///
/// The distinction matters to [`count_lines`]: a returned index of
/// `chars.len()` is ambiguous on its own, because `*/` ending the line and no
/// `*/` at all both land there.
fn block_comment_end(chars: &[char], mut i: usize) -> Option<usize> {
    while i + 1 < chars.len() {
        if chars[i] == '*' && chars[i + 1] == '/' {
            return Some(i + 2);
        }
        i += 1;
    }
    None
}

/// Whether the quote at `i` opens a literal whose contents should be skipped.
///
/// A double quote always does; a single quote only when it closes like a char
/// literal rather than naming a lifetime.
fn opens_literal(chars: &[char], i: usize) -> bool {
    chars[i] == '"' || is_char_literal(chars, i)
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

    /// Braces on a single line, scanned from a clean state.
    fn braces(line: &str) -> (i32, i32) {
        let s = SourceScanner::new().scan(line);
        (s.opens, s.closes)
    }

    /// Braces per line across a block, sharing comment state.
    fn braces_across(src: &str) -> Vec<(i32, i32)> {
        let lines: Vec<&str> = src.lines().collect();
        scan_lines(&lines)
            .iter()
            .map(|s| (s.opens, s.closes))
            .collect()
    }

    #[test]
    fn braces_in_strings_do_not_open_blocks() {
        // The line that broke boundary detection in the Rust analyzer.
        assert_eq!(braces(r#"    let keywords = ["else {", "loop{"];"#), (0, 0));
    }

    #[test]
    fn real_braces_still_count() {
        assert_eq!(braces("fn main() {"), (1, 0));
        assert_eq!(braces("}"), (0, 1));
        assert_eq!(braces("if x { y() } else { z() }"), (2, 2));
    }

    #[test]
    fn comments_are_ignored() {
        assert_eq!(braces("let x = 1; // }"), (0, 0));
        assert_eq!(braces("foo(); /* { */ bar();"), (0, 0));
        assert_eq!(braces("/* } */ {"), (1, 0));
    }

    #[test]
    fn braces_inside_a_multi_line_comment_open_nothing() {
        // A commented-out block used to leave the depth permanently unbalanced,
        // folding every later function into the one before it.
        let src = concat!("fn f() {\n", "    /*\n", "    if x {\n", "    */\n", "}\n",);
        assert_eq!(braces_across(src), [(1, 0), (0, 0), (0, 0), (0, 0), (0, 1)]);
    }

    #[test]
    fn a_comment_closing_mid_line_resumes_counting() {
        let src = concat!("/* opened\n", "   closed */ if x {\n");
        assert_eq!(braces_across(src), [(0, 0), (1, 0)]);
    }

    #[test]
    fn lifetimes_are_not_char_literals() {
        // A naive quote-toggle would swallow the rest of this line.
        assert_eq!(braces("fn f<'a>(x: &'a str) {"), (1, 0));
        assert_eq!(braces("let c = '}';"), (0, 0));
        assert_eq!(braces(r"let c = '\'';"), (0, 0));
    }

    #[test]
    fn escaped_quotes_do_not_end_a_string() {
        assert_eq!(braces(r#"let s = "a\"{"; "#), (0, 0));
    }

    fn counts(src: &str) -> (u32, u32) {
        let lines: Vec<&str> = src.lines().collect();
        let c = count_lines(&lines);
        (c.source, c.comment)
    }

    #[test]
    fn the_middle_of_a_block_comment_is_not_source() {
        // The whole point: line 2 starts with neither `/*` nor `*`.
        let src = concat!("/* one\n", "   two\n", "   three */\n", "let x = 1;\n");
        assert_eq!(counts(src), (1, 3));
    }

    #[test]
    fn rust_and_go_block_comments_are_recognized_at_all() {
        // Neither analyzer looked for `/*` before; all three lines were code.
        let src = concat!("/*\n", "let fake = 1;\n", "*/\n", "let real = 2;\n");
        assert_eq!(counts(src), (1, 3));
    }

    #[test]
    fn code_sharing_a_line_with_a_comment_is_source() {
        assert_eq!(counts("foo(); // why\n"), (1, 0));
        assert_eq!(counts("/* setup */ foo();\n"), (1, 0));
        assert_eq!(
            counts(concat!("foo(); /* trailing\n", "   still comment */\n")),
            (1, 1)
        );
        assert_eq!(counts("*/ foo();\n"), (1, 0));
    }

    #[test]
    fn a_comment_opener_inside_a_literal_opens_nothing() {
        let src = concat!("let s = \"/* not a comment\";\n", "let t = 2;\n");
        assert_eq!(counts(src), (2, 0));
    }

    #[test]
    fn a_line_comment_swallows_a_block_opener() {
        // `// ... /*` must not leave us inside a block comment.
        assert_eq!(counts(concat!("// see /* this\n", "let x = 1;\n")), (1, 1));
    }

    #[test]
    fn attributes_and_doc_comments_land_on_the_right_side() {
        // `#[derive]` is code; `///` is comment. `is_comment_line`, still used
        // for declaration detection, calls `#` a comment — a different question
        // from what counts as source.
        assert_eq!(counts(concat!("#[derive(Debug)]\n", "struct S;\n")), (2, 0));
        assert_eq!(counts(concat!("/// docs\n", "fn f() {}\n")), (1, 1));
    }

    #[test]
    fn blank_lines_count_as_neither() {
        assert_eq!(counts("\n   \n\n"), (0, 0));
        assert_eq!(counts(concat!("/* a\n", "\n", "   b */\n")), (0, 2));
    }
}
