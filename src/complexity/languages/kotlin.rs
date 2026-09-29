// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Kotlin complexity analyzer.
//!
//! Kotlin declares functions with an explicit `fun` keyword, so detection is a
//! keyword match rather than the return-type guessing the Java analyzer has to
//! do. Two shapes matter: a block body (`fun f() { … }`) and an expression body
//! (`fun f() = expr`), which has no braces and is one line unless it wraps.

use std::path::Path;

use crate::complexity::analyzer::LanguageComplexityAnalyzer;
use crate::complexity::metrics::{ComplexityMetrics, FileMetrics, FunctionMetrics};

/// Kotlin complexity analyzer
pub struct KotlinComplexityAnalyzer;

impl KotlinComplexityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn analyze_function(&self, lines: &[&str]) -> ComplexityMetrics {
        let mut metrics = ComplexityMetrics::new();
        let content = lines.join("\n");

        metrics.cyclomatic = self.calculate_cyclomatic(&content);
        metrics.cognitive = self.calculate_cognitive(&content);
        metrics.max_nesting = self.calculate_nesting(&content);

        metrics.loc = lines.len() as u32;
        metrics.sloc = super::count_lines(lines).source;

        metrics.parameters = count_params(&content);

        metrics.returns = content.matches("return ").count() as u32;

        metrics
    }

    fn calculate_cyclomatic(&self, content: &str) -> u32 {
        let mut complexity = 1;

        // `->` is a `when` branch or a lambda arrow; both are decision points.
        // `?:` (elvis) and `?.` (safe call) each introduce a null branch.
        let keywords = [
            "if ", "if(", "else if", "for ", "for(", "while ", "while(", "do ", "catch ", "catch(",
            "&&", "||", "?:", "?.", "->",
        ];

        for line in content.lines() {
            let trimmed = line.trim();
            if super::is_comment_line(trimmed) {
                continue;
            }
            for keyword in keywords {
                complexity += trimmed.matches(keyword).count() as u32;
            }
        }

        complexity
    }

    fn calculate_cognitive(&self, content: &str) -> u32 {
        let mut complexity = 0;
        let mut nesting_level = 0i32;

        let lines: Vec<&str> = content.lines().collect();
        for (line, scan) in lines.iter().zip(super::scan_lines(&lines)) {
            if !scan.is_code() {
                continue;
            }
            let trimmed = line.trim();
            let (opens, closes) = (scan.opens, scan.closes);

            let control_keywords = [
                "if ", "if(", "else", "when ", "when(", "for ", "for(", "while ", "do ", "catch",
                "try ",
            ];
            for keyword in control_keywords {
                if trimmed.starts_with(keyword) || trimmed.contains(&format!(" {}", keyword)) {
                    complexity += 1 + nesting_level.max(0) as u32;
                }
            }

            complexity += trimmed.matches("&&").count() as u32;
            complexity += trimmed.matches("||").count() as u32;
            complexity += trimmed.matches("?:").count() as u32;

            nesting_level += opens - closes;
            if nesting_level < 0 {
                nesting_level = 0;
            }
        }

        complexity
    }

    fn calculate_nesting(&self, content: &str) -> u32 {
        let mut max_nesting: u32 = 0;
        let mut current: u32 = 0;

        let lines: Vec<&str> = content.lines().collect();
        for scan in super::scan_lines(&lines) {
            if !scan.is_code() {
                continue;
            }
            current = current.saturating_add(scan.opens.max(0) as u32);
            max_nesting = max_nesting.max(current);
            current = current.saturating_sub(scan.closes.max(0) as u32);
        }

        max_nesting
    }
}

impl Default for KotlinComplexityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageComplexityAnalyzer for KotlinComplexityAnalyzer {
    fn name(&self) -> &str {
        "kotlin-complexity"
    }

    fn extensions(&self) -> &[&str] {
        &["kt", "kts"]
    }

    fn language(&self) -> &str {
        "kotlin"
    }

    fn analyze_file(&self, path: &Path, content: &str) -> Result<FileMetrics, String> {
        let mut file_metrics = FileMetrics::new(path.to_path_buf(), self.language());
        let lines: Vec<&str> = content.lines().collect();

        file_metrics.metrics.loc = lines.len() as u32;
        let line_counts = super::count_lines(&lines);
        file_metrics.metrics.sloc = line_counts.source;
        file_metrics.metrics.comment_lines = line_counts.comment;
        file_metrics.imports = lines
            .iter()
            .filter(|line| line.trim().starts_with("import "))
            .count() as u32;
        let scans = super::scan_lines(&lines);
        file_metrics.classes = lines
            .iter()
            .zip(&scans)
            .filter(|(line, scan)| scan.is_code() && declares_type(line.trim()))
            .count() as u32;

        let mut i = 0;
        while i < lines.len() {
            let trimmed = lines[i].trim();
            if !scans[i].is_code() {
                i += 1;
                continue;
            }
            let Some(name) = detect_kotlin_function(trimmed) else {
                i += 1;
                continue;
            };

            let start = i;
            let end = function_end(&lines, &scans, i);
            let body = &lines[start..=end];

            let mut func = FunctionMetrics::new(&name, (start + 1) as u32, (end + 1) as u32);
            func.metrics = self.analyze_function(body);
            func.kind = "function".to_string();
            func.parent = enclosing_type(&lines[..start], &scans[..start]);
            file_metrics.functions.push(func);

            // Nested functions (Kotlin allows local `fun`) are folded into the
            // outer one, the same way every other analyzer here treats them.
            i = end + 1;
        }

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

/// Number of parameters in the declaration's parameter list.
///
/// Splitting on every comma would miscount `Map<String, Int>` as two parameters
/// and `(Int, Int) -> Unit` as three, so only commas at the top level of the
/// list count.
fn count_params(content: &str) -> u32 {
    // Blank out `->` first: its `>` would otherwise read as a generic close,
    // pushing the depth below zero so the comma after it looks top-level and
    // `(Int, Int) -> Unit` counts as two parameters instead of one.
    let content = content.replace("->", "  ");
    let start = match content.find('(') {
        Some(i) => i + 1,
        None => return 0,
    };

    let mut depth = 0i32;
    let mut params = 1u32;
    let mut seen_content = false;

    for ch in content[start..].chars() {
        if ch == ')' && depth == 0 {
            break;
        }
        match ch {
            '(' | '<' | '[' => depth += 1,
            ')' | '>' | ']' => depth = (depth - 1).max(0),
            ',' if depth == 0 => params += 1,
            _ => {}
        }
        seen_content |= !ch.is_whitespace();
    }

    if seen_content {
        params
    } else {
        0
    }
}

/// Index of the last line of the function starting at `start`.
///
/// A block body runs to the line where the brace depth returns to zero. An
/// expression body (`fun f() = …`) has no braces to balance, so it ends at the
/// first line that does not continue the expression.
fn function_end(lines: &[&str], scans: &[super::LineScan], start: usize) -> usize {
    let mut depth = 0i32;
    let mut seen_brace = false;

    for (offset, line) in lines[start..].iter().enumerate() {
        let i = start + offset;
        let scan = scans[i];
        if !scan.is_code() {
            continue;
        }

        if scan.opens > 0 {
            seen_brace = true;
        }
        depth += scan.opens - scan.closes;

        if seen_brace {
            if depth <= 0 {
                return i;
            }
            continue;
        }

        // Expression body: no brace opened yet. It ends when the line stops
        // trailing off into the next one.
        if !expression_continues(line.trim()) {
            return i;
        }
    }

    lines.len() - 1
}

/// Whether a braceless line hands the expression to the next line.
fn expression_continues(trimmed: &str) -> bool {
    trimmed.ends_with('=')
        || trimmed.ends_with('.')
        || trimmed.ends_with(',')
        || trimmed.ends_with('(')
        || trimmed.ends_with("?:")
        || trimmed.ends_with("&&")
        || trimmed.ends_with("||")
}

/// Name of the function declared on this line, if any.
fn detect_kotlin_function(trimmed: &str) -> Option<String> {
    if super::is_comment_line(trimmed) {
        return None;
    }

    // `fun` must be a whole word: either the line starts with it, or it follows
    // a modifier. `funnel(` and `refund()` must not match.
    let rest = if let Some(r) = trimmed.strip_prefix("fun ") {
        r
    } else {
        let idx = trimmed.find(" fun ")?;
        &trimmed[idx + 5..]
    };

    // Skip a generic parameter list: `fun <T> map(...)`.
    let rest = rest.trim_start();
    let rest = if rest.starts_with('<') {
        &rest[rest.find('>')? + 1..]
    } else {
        rest
    };

    let paren = rest.find('(')?;
    let before = rest[..paren].trim();

    // `fun Foo.bar(` is an extension function — the name is after the dot.
    let name = before.rsplit('.').next()?.trim();

    (!name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '`'))
    .then(|| name.to_string())
}

/// Whether the line declares a class-like type.
fn declares_type(trimmed: &str) -> bool {
    if super::is_comment_line(trimmed) {
        return false;
    }
    const KEYWORDS: &[&str] = &["class ", "interface ", "object "];
    KEYWORDS
        .iter()
        .any(|k| trimmed.starts_with(k) || trimmed.contains(&format!(" {}", k)))
}

/// Name of the innermost type declared before this point, if any.
fn enclosing_type(before: &[&str], scans: &[super::LineScan]) -> Option<String> {
    before.iter().zip(scans).rev().find_map(|(line, scan)| {
        let trimmed = line.trim();
        if !scan.is_code() || !declares_type(trimmed) {
            return None;
        }
        const KEYWORDS: &[&str] = &["class ", "interface ", "object "];
        let keyword = KEYWORDS.iter().find(|k| trimmed.contains(*k))?;
        let rest = &trimmed[trimmed.find(*keyword)? + keyword.len()..];
        let end = rest.find(['<', '{', '(', ':', ' ']).unwrap_or(rest.len());
        let name = rest[..end].trim();
        (!name.is_empty()).then(|| name.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_cover_scripts_too() {
        let a = KotlinComplexityAnalyzer::new();
        assert_eq!(a.language(), "kotlin");
        assert!(a.extensions().contains(&"kt"));
        assert!(a.extensions().contains(&"kts"));
    }

    #[test]
    fn detects_every_declaration_shape() {
        assert_eq!(detect_kotlin_function("fun main() {"), Some("main".into()));
        assert_eq!(
            detect_kotlin_function("private suspend fun load(id: Int): User {"),
            Some("load".into())
        );
        assert_eq!(
            detect_kotlin_function("fun <T> List<T>.second(): T ="),
            Some("second".into())
        );
        assert_eq!(
            detect_kotlin_function("internal fun Project.configure() {"),
            Some("configure".into())
        );
    }

    #[test]
    fn a_word_containing_fun_is_not_a_declaration() {
        assert_eq!(detect_kotlin_function("val refund = compute(1)"), None);
        assert_eq!(detect_kotlin_function("// fun notReally() {"), None);
        assert_eq!(detect_kotlin_function("funnel(input)"), None);
    }

    #[test]
    fn a_top_level_function_is_found_without_a_class() {
        let a = KotlinComplexityAnalyzer::new();
        let src = "fun greet(name: String) {\n    println(name)\n}\n";
        let m = a.analyze_file(Path::new("A.kt"), src).unwrap();
        assert_eq!(m.functions.len(), 1);
        assert_eq!(m.functions[0].name, "greet");
        assert_eq!(m.functions[0].parent, None);
    }

    #[test]
    fn a_method_records_its_enclosing_class() {
        let a = KotlinComplexityAnalyzer::new();
        let src = "class Greeter {\n    fun greet() {\n        println(1)\n    }\n}\n";
        let m = a.analyze_file(Path::new("A.kt"), src).unwrap();
        assert_eq!(m.functions.len(), 1);
        assert_eq!(m.functions[0].parent, Some("Greeter".into()));
        assert_eq!(m.classes, 1);
    }

    #[test]
    fn two_functions_do_not_collapse_into_one() {
        let a = KotlinComplexityAnalyzer::new();
        let src = concat!(
            "class C {\n",
            "    fun first() {\n",
            "        val s = \"else {\"\n",
            "    }\n",
            "\n",
            "    fun second() {\n",
            "        println(2)\n",
            "    }\n",
            "}\n",
        );
        let m = a.analyze_file(Path::new("C.kt"), src).unwrap();
        let names: Vec<_> = m.functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["first", "second"]);
    }

    #[test]
    fn an_expression_body_is_one_function() {
        let a = KotlinComplexityAnalyzer::new();
        let src = "fun double(x: Int) = x * 2\n\nfun triple(x: Int) = x * 3\n";
        let m = a.analyze_file(Path::new("A.kt"), src).unwrap();
        let names: Vec<_> = m.functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["double", "triple"]);
    }

    #[test]
    fn sloc_excludes_blanks_and_comments() {
        let a = KotlinComplexityAnalyzer::new();
        let src = concat!(
            "fun f() {\n",
            "    // a comment\n",
            "\n",
            "    /* another */\n",
            "    val x = 1\n",
            "}\n",
        );
        let m = a.analyze_file(Path::new("A.kt"), src).unwrap();
        let f = &m.functions[0];
        assert_eq!(f.lines(), 6, "raw span includes comments and blanks");
        assert_eq!(f.metrics.sloc, 3, "fun + val + closing brace");
    }

    #[test]
    fn sloc_excludes_the_middle_of_a_multi_line_block_comment() {
        let a = KotlinComplexityAnalyzer::new();
        let src = concat!(
            "fun f() {\n",
            "    /*\n",
            "     * Prose that starts with neither slash nor star:\n",
            "       val decoy = 1\n",
            "     */\n",
            "    val x = 1\n",
            "}\n",
        );
        let m = a.analyze_file(Path::new("A.kt"), src).unwrap();
        let f = &m.functions[0];
        assert_eq!(f.lines(), 7);
        assert_eq!(
            f.metrics.sloc, 3,
            "fun + val + closing brace; the commented-out `val decoy` is not code"
        );
    }

    #[test]
    fn generic_and_lambda_parameters_count_once_each() {
        assert_eq!(count_params("fun f() {"), 0);
        assert_eq!(count_params("fun f(a: Int) {"), 1);
        assert_eq!(count_params("fun f(a: Int, b: String) {"), 2);
        assert_eq!(count_params("fun f(m: Map<String, Int>) {"), 1);
        assert_eq!(count_params("fun f(cb: (Int, Int) -> Unit, x: Int) {"), 2);
    }

    #[test]
    fn branching_raises_complexity() {
        let a = KotlinComplexityAnalyzer::new();
        let src = concat!(
            "fun classify(x: Int): String {\n",
            "    if (x > 0) {\n",
            "        return \"pos\"\n",
            "    } else if (x < 0) {\n",
            "        return \"neg\"\n",
            "    }\n",
            "    return \"zero\"\n",
            "}\n",
        );
        let m = a.analyze_file(Path::new("A.kt"), src).unwrap();
        assert!(m.functions[0].metrics.cyclomatic > 1);
        assert!(m.functions[0].metrics.max_nesting >= 2);
    }
}
