// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! C/C++ language formatter using clang-format, clang-tidy --fix, and cpplint fixer.

use crate::fixers::cpplint::{CpplintFixer, CpplintFixerConfig, HeaderGuardMode};
use crate::fixers::source::SourceFixer;
use crate::formatters::Formatter;
use crate::utils::types::FormatResult;
use crate::{Language, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

/// C/C++ formatter using clang-format, clang-tidy --fix, and cpplint fixer.
pub struct CppFormatter {
    /// Enable clang-tidy --fix for auto-fixing lint issues
    use_clang_tidy_fix: bool,
    /// Enable cpplint fixer for header guards, TODOs, etc.
    use_cpplint_fix: bool,
    /// Custom compile_commands.json directory path
    compile_commands_dir: Option<PathBuf>,
    /// Cpplint fixer instance (wrapped in Mutex for interior mutability)
    cpplint_fixer: Mutex<CpplintFixer>,
}

impl CppFormatter {
    pub fn new() -> Self {
        Self {
            use_clang_tidy_fix: true,  // Enable by default
            use_cpplint_fix: true,     // Enable by default
            compile_commands_dir: None,
            cpplint_fixer: Mutex::new(CpplintFixer::new()),
        }
    }

    /// Enable or disable clang-tidy --fix
    pub fn with_clang_tidy_fix(mut self, enable: bool) -> Self {
        self.use_clang_tidy_fix = enable;
        self
    }

    /// Enable or disable cpplint fixer
    pub fn with_cpplint_fix(mut self, enable: bool) -> Self {
        self.use_cpplint_fix = enable;
        self
    }

    /// Set custom compile_commands.json directory
    pub fn with_compile_commands_dir(mut self, path: PathBuf) -> Self {
        self.compile_commands_dir = Some(path);
        self
    }

    /// Configure cpplint fixer
    pub fn with_cpplint_config(self, config: CpplintFixerConfig) -> Self {
        *self.cpplint_fixer.lock().unwrap() = CpplintFixer::with_config(config);
        self
    }

    /// Set header guard mode
    pub fn with_header_guard_mode(self, mode: HeaderGuardMode) -> Self {
        {
            let mut fixer = self.cpplint_fixer.lock().unwrap();
            let config = CpplintFixerConfig {
                header_guard_mode: mode,
                ..Default::default()
            };
            *fixer = CpplintFixer::with_config(config);
        }
        self
    }

    /// Check if clang-tidy is available
    fn has_clang_tidy() -> bool {
        Command::new("clang-tidy")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Find .clang-tidy config file by walking up from file path
    fn find_clang_tidy_config(start_path: &Path) -> Option<PathBuf> {
        let mut current = if start_path.is_file() {
            start_path.parent()?.to_path_buf()
        } else {
            start_path.to_path_buf()
        };

        loop {
            let config_path = current.join(".clang-tidy");
            if config_path.exists() {
                return Some(config_path);
            }
            if !current.pop() {
                break;
            }
        }
        None
    }

    /// Find .clang-format config file for a specific language.
    /// First checks .linthis/configs/{language}/.clang-format, then walks up directories.
    fn find_clang_format_config(start_path: &Path, language: &str) -> Option<PathBuf> {
        // First, check .linthis/configs/{language}/.clang-format
        let mut current = if start_path.is_file() {
            start_path.parent()?.to_path_buf()
        } else {
            start_path.to_path_buf()
        };

        // Walk up to find .linthis directory
        let mut search_dir = current.clone();
        loop {
            let linthis_config = search_dir.join(".linthis").join("configs").join(language).join(".clang-format");
            if linthis_config.exists() {
                return Some(linthis_config);
            }
            if !search_dir.pop() {
                break;
            }
        }

        // Fall back to traditional .clang-format search in parent directories
        loop {
            let config_path = current.join(".clang-format");
            if config_path.exists() {
                return Some(config_path);
            }
            if !current.pop() {
                break;
            }
        }
        None
    }

    /// Find compile_commands.json recursively
    fn find_compile_commands(start_path: &Path) -> Option<PathBuf> {
        let mut current = if start_path.is_file() {
            start_path.parent()?.to_path_buf()
        } else {
            start_path.to_path_buf()
        };

        loop {
            // Check current directory
            if current.join("compile_commands.json").exists() {
                return Some(current.clone());
            }

            // Check common build directories
            for build_dir in &["build", "Build", "out", "cmake-build-debug", "cmake-build-release"] {
                let compile_db = current.join(build_dir).join("compile_commands.json");
                if compile_db.exists() {
                    return Some(current.join(build_dir));
                }
            }

            // Recursively search build-like directories (up to 6 levels)
            if let Some(found) = Self::find_compile_commands_recursive(&current, 0, 6) {
                return Some(found);
            }

            if !current.pop() {
                break;
            }
        }
        None
    }

    fn find_compile_commands_recursive(dir: &Path, depth: usize, max_depth: usize) -> Option<PathBuf> {
        if depth >= max_depth {
            return None;
        }

        let entries = std::fs::read_dir(dir).ok()?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let name = path.file_name().and_then(|n| n.to_str())?;
            let name_lower = name.to_lowercase();

            let is_build_dir = name_lower.starts_with("cmake")
                || name_lower.starts_with("build")
                || name_lower.starts_with("out")
                || name_lower.ends_with("-build")
                || name_lower.ends_with("_build")
                || (depth > 0
                    && (name_lower.contains("android")
                        || name_lower.contains("ios")
                        || name_lower.contains("linux")
                        || name_lower.contains("windows")
                        || name_lower.contains("arm")
                        || name_lower.contains("x86")
                        || name_lower.contains("static")
                        || name_lower.contains("shared")
                        || name_lower.contains("debug")
                        || name_lower.contains("release")));

            if is_build_dir {
                if path.join("compile_commands.json").exists() {
                    return Some(path);
                }
                if let Some(found) = Self::find_compile_commands_recursive(&path, depth + 1, max_depth) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// Run clang-tidy --fix on a file
    fn run_clang_tidy_fix(&self, path: &Path) -> Result<bool> {
        if !Self::has_clang_tidy() {
            return Ok(false);
        }

        let mut cmd = Command::new("clang-tidy");
        cmd.arg(path);
        cmd.arg("--fix");
        cmd.arg("--fix-errors"); // Also fix errors, not just warnings

        // Add config file if found
        if let Some(config) = Self::find_clang_tidy_config(path) {
            cmd.arg(format!("--config-file={}", config.display()));
        }

        // Add compile_commands.json path
        if let Some(ref build_path) = self.compile_commands_dir {
            cmd.arg(format!("-p={}", build_path.display()));
        } else if let Some(build_path) = Self::find_compile_commands(path) {
            cmd.arg(format!("-p={}", build_path.display()));
        } else {
            cmd.arg("--");
        }

        let output = cmd.output().map_err(|e| {
            crate::LintisError::Formatter(format!("Failed to run clang-tidy --fix: {}", e))
        })?;

        // clang-tidy returns non-zero if there are unfixable issues, but fix still works
        Ok(output.status.success() || !output.stdout.is_empty())
    }
}

impl Default for CppFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for CppFormatter {
    fn name(&self) -> &str {
        match (self.use_clang_tidy_fix && Self::has_clang_tidy(), self.use_cpplint_fix) {
            (true, true) => "clang-format + clang-tidy + cpplint-fix",
            (true, false) => "clang-format + clang-tidy",
            (false, true) => "clang-format + cpplint-fix",
            (false, false) => "clang-format",
        }
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::Cpp, Language::ObjectiveC]
    }

    fn format(&self, path: &Path) -> Result<FormatResult> {
        // Detect language from file extension
        let language = Self::detect_language(path);

        // Read original content for comparison
        let original = fs::read_to_string(path)
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to read file: {}", e)))?;

        // Step 1: Run cpplint fixer (fixes header guards, TODOs, copyright)
        // For OC files, we still run the fixer but it will skip unsafe categories
        // (like readability/casting which misinterprets OC method signatures)
        if self.use_cpplint_fix {
            if let Ok(mut fixer) = self.cpplint_fixer.lock() {
                fixer.set_is_objc(language == "oc");
                let _ = fixer.fix_file(path); // Ignore errors, continue with other fixes
            }
        }

        // Step 2: Run clang-tidy --fix (fixes code issues like C-style casts)
        // Skip for Objective-C files as clang-tidy doesn't handle OC properly
        if self.use_clang_tidy_fix && language != "oc" {
            let _ = self.run_clang_tidy_fix(path); // Ignore errors, clang-format will still run
        }

        // Step 3: Run clang-format (-i modifies in place)
        let mut cmd = Command::new("clang-format");
        cmd.arg("-i");

        // Use language-specific config if found, otherwise fall back to Google style
        if let Some(config_path) = Self::find_clang_format_config(path, language) {
            cmd.arg(format!("-style=file:{}", config_path.display()));
        } else {
            cmd.arg("-style=Google");
        }

        let output = cmd.arg(path).output().map_err(|e| {
            crate::LintisError::Formatter(format!("Failed to run clang-format: {}", e))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(FormatResult::error(
                path.to_path_buf(),
                format!("clang-format failed: {}", stderr),
            ));
        }

        // Step 4: Fix comment spacing (clang-format doesn't fix non-ASCII comments like Chinese)
        // This fixes "//comment" -> "// comment" for all characters
        SourceFixer::fix_comment_spacing(path)?;

        // Step 5: Fix TODO comments (add username from git blame)
        SourceFixer::fix_todo_comments(path)?;

        // Step 6: Fix lone semicolons (remove lines with only semicolon)
        SourceFixer::fix_lone_semicolon(path)?;

        // Step 7: Fix long comment lines (break at appropriate points)
        // OC uses 150 char limit, C++ uses 120
        let max_line_length = if language == "oc" { 150 } else { 120 };
        SourceFixer::fix_long_comments(path, max_line_length)?;

        // Step 8: Fix pragma separators (OC only) - convert "-- -- --" to "#pragma mark -"
        if language == "oc" {
            SourceFixer::fix_pragma_separators(path)?;
        }

        // Read new content and compare
        let new_content = fs::read_to_string(path).map_err(|e| {
            crate::LintisError::Formatter(format!("Failed to read formatted file: {}", e))
        })?;

        if original == new_content {
            Ok(FormatResult::unchanged(path.to_path_buf()))
        } else {
            Ok(FormatResult::changed(path.to_path_buf()))
        }
    }

    fn check(&self, path: &Path) -> Result<bool> {
        // Detect language from file extension
        let language = Self::detect_language(path);

        // Read current content
        let current = fs::read_to_string(path)
            .map_err(|e| crate::LintisError::Formatter(format!("Failed to read file: {}", e)))?;

        // Run clang-format to get formatted output (without -i)
        let mut cmd = Command::new("clang-format");

        // Use language-specific config if found, otherwise fall back to Google style
        if let Some(config_path) = Self::find_clang_format_config(path, language) {
            cmd.arg(format!("-style=file:{}", config_path.display()));
        } else {
            cmd.arg("-style=Google");
        }

        let output = cmd.arg(path).output().map_err(|e| {
            crate::LintisError::Formatter(format!("Failed to run clang-format: {}", e))
        })?;

        let formatted = String::from_utf8_lossy(&output.stdout);

        // If they differ, file needs formatting
        Ok(current != formatted.as_ref())
    }

    fn is_available(&self) -> bool {
        Command::new("clang-format")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

impl CppFormatter {
    /// Detect language from file extension and content.
    /// For .h files, checks content for OC syntax to determine if it's OC or C++.
    fn detect_language(path: &Path) -> &'static str {
        let debug = std::env::var("LINTHIS_DEBUG").is_ok();

        match path.extension().and_then(|e| e.to_str()) {
            Some("m") | Some("mm") | Some("M") | Some("MM") => {
                if debug {
                    eprintln!("[cpp-formatter] {} detected as OC (by extension)", path.display());
                }
                "oc"
            }
            Some("h") | Some("H") => {
                // For header files, check content for OC-specific syntax
                if Self::contains_objc_syntax(path) {
                    if debug {
                        eprintln!("[cpp-formatter] {} detected as OC (by content)", path.display());
                    }
                    "oc"
                } else {
                    if debug {
                        eprintln!("[cpp-formatter] {} detected as C++ (no OC syntax found)", path.display());
                    }
                    "cpp"
                }
            }
            _ => {
                if debug {
                    eprintln!("[cpp-formatter] {} detected as C++ (by extension)", path.display());
                }
                "cpp"
            }
        }
    }

    /// Check if a file contains Objective-C specific syntax.
    fn contains_objc_syntax(path: &Path) -> bool {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return false,
        };

        // OC-specific patterns (exact string matches)
        let oc_patterns = [
            "@import",  // OC module import: @import UIKit;
            "@interface",
            "@implementation",
            "@protocol",
            "@property",
            "@synthesize",
            "@dynamic",
            "@selector",
            "@class",
            "@end",
            "NS_ASSUME_NONNULL_BEGIN",
            "NS_ENUM",
            "NS_OPTIONS",
            "nullable",
            "nonnull",
            "+ (", // OC class method
            "- (", // OC instance method
            " @\"", // OC string literal: @"string"
            " @[",  // OC array literal: @[@"a", @"b"]
        ];

        for pattern in oc_patterns {
            if content.contains(pattern) {
                return true;
            }
        }

        // Check for Foundation types: NS followed by uppercase letter (e.g., NSString, NSArray)
        // This follows Apple's naming convention and won't match C++ namespaces (which are lowercase)
        if Self::contains_ns_type(&content) {
            return true;
        }

        false
    }

    /// Check if content contains Foundation types (NS followed by uppercase letter).
    /// Examples: NSString, NSArray, NSDictionary, NSObject, NSURL, etc.
    fn contains_ns_type(content: &str) -> bool {
        let bytes = content.as_bytes();
        let len = bytes.len();

        // Look for "NS" followed by an uppercase letter A-Z
        for i in 0..len.saturating_sub(2) {
            if bytes[i] == b'N' && bytes[i + 1] == b'S' {
                let next_char = bytes[i + 2];
                // Check if next char is uppercase A-Z (ASCII 65-90)
                if (b'A'..=b'Z').contains(&next_char) {
                    // Make sure it's not part of a longer identifier before "NS"
                    // (i.e., NS should be at word boundary)
                    if i == 0 || !is_identifier_char(bytes[i - 1]) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Check if a byte is a valid identifier character (alphanumeric or underscore)
fn is_identifier_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_temp_header(content: &str) -> NamedTempFile {
        let mut file = tempfile::Builder::new()
            .suffix(".h")
            .tempfile()
            .unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    // ==================== detect_language tests ====================

    #[test]
    fn test_detect_language_m_file() {
        let path = std::path::Path::new("test.m");
        assert_eq!(CppFormatter::detect_language(path), "oc");
    }

    #[test]
    fn test_detect_language_mm_file() {
        let path = std::path::Path::new("test.mm");
        assert_eq!(CppFormatter::detect_language(path), "oc");
    }

    #[test]
    fn test_detect_language_cpp_file() {
        let path = std::path::Path::new("test.cpp");
        assert_eq!(CppFormatter::detect_language(path), "cpp");
    }

    #[test]
    fn test_detect_language_h_file_cpp() {
        let file = create_temp_header("#include <iostream>\nvoid foo();\n");
        assert_eq!(CppFormatter::detect_language(file.path()), "cpp");
    }

    #[test]
    fn test_detect_language_h_file_oc_interface() {
        let file = create_temp_header("@interface MyClass : NSObject\n@end\n");
        assert_eq!(CppFormatter::detect_language(file.path()), "oc");
    }

    #[test]
    fn test_detect_language_h_file_oc_property() {
        let file = create_temp_header("@property (nonatomic) NSString *name;\n");
        assert_eq!(CppFormatter::detect_language(file.path()), "oc");
    }

    // ==================== contains_objc_syntax tests ====================

    #[test]
    fn test_contains_objc_syntax_interface() {
        let file = create_temp_header("@interface Test\n@end\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    #[test]
    fn test_contains_objc_syntax_implementation() {
        let file = create_temp_header("@implementation Test\n@end\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    #[test]
    fn test_contains_objc_syntax_protocol() {
        let file = create_temp_header("@protocol MyProtocol\n@end\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    #[test]
    fn test_contains_objc_syntax_ns_enum() {
        let file = create_temp_header("typedef NS_ENUM(NSUInteger, MyEnum) {\n};\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    #[test]
    fn test_contains_objc_syntax_ns_options() {
        let file = create_temp_header("typedef NS_OPTIONS(NSUInteger, MyOptions) {\n};\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    #[test]
    fn test_contains_objc_syntax_nsinteger() {
        let file = create_temp_header("- (NSInteger)count;\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    #[test]
    fn test_contains_objc_syntax_nsuinteger() {
        let file = create_temp_header("NSUInteger value = 0;\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    #[test]
    fn test_contains_objc_syntax_nsstring() {
        let file = create_temp_header("NSString *name;\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    #[test]
    fn test_contains_objc_syntax_nsarray() {
        let file = create_temp_header("NSArray *items;\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    #[test]
    fn test_contains_objc_syntax_nsdictionary() {
        let file = create_temp_header("NSDictionary *dict;\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    #[test]
    fn test_contains_objc_syntax_nsobject() {
        let file = create_temp_header("@interface MyClass : NSObject\n@end\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    #[test]
    fn test_contains_objc_syntax_nsurl() {
        let file = create_temp_header("NSURL *url;\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    #[test]
    fn test_contains_objc_syntax_nserror() {
        let file = create_temp_header("NSError *error;\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    #[test]
    fn test_contains_objc_syntax_string_literal() {
        let file = create_temp_header("NSString *s = @\"hello\";\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    // ==================== contains_ns_type tests ====================

    #[test]
    fn test_contains_ns_type_nsstring() {
        assert!(CppFormatter::contains_ns_type("NSString *name;"));
    }

    #[test]
    fn test_contains_ns_type_nsarray() {
        assert!(CppFormatter::contains_ns_type("NSArray<NSString *> *items;"));
    }

    #[test]
    fn test_contains_ns_type_at_line_start() {
        assert!(CppFormatter::contains_ns_type("NSObject *obj;"));
    }

    #[test]
    fn test_contains_ns_type_after_space() {
        assert!(CppFormatter::contains_ns_type("id<NSCopying> obj;"));
    }

    #[test]
    fn test_contains_ns_type_after_paren() {
        assert!(CppFormatter::contains_ns_type("(NSString *)value"));
    }

    #[test]
    fn test_contains_ns_type_no_false_positive_dns() {
        // "DNS" should not match because D is before NS
        assert!(!CppFormatter::contains_ns_type("DNSResolver resolver;"));
    }

    #[test]
    fn test_contains_ns_type_no_false_positive_lowercase() {
        // "NSfoo" where next char is lowercase should not match
        // But actually NS followed by lowercase is rare, let's test NS alone
        assert!(!CppFormatter::contains_ns_type("namespace ns { }"));
    }

    #[test]
    fn test_contains_ns_type_no_false_positive_part_of_word() {
        // "AwesomeNSString" - NS is part of larger identifier
        assert!(!CppFormatter::contains_ns_type("AwesomeNSString x;"));
    }

    #[test]
    fn test_contains_ns_type_pure_cpp() {
        assert!(!CppFormatter::contains_ns_type("#include <vector>\nstd::vector<int> v;"));
    }

    #[test]
    fn test_contains_objc_syntax_array_literal() {
        let file = create_temp_header("NSArray *arr = @[@\"a\", @\"b\"];\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    #[test]
    fn test_contains_objc_syntax_class_method() {
        let file = create_temp_header("+ (instancetype)sharedInstance;\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    #[test]
    fn test_contains_objc_syntax_instance_method() {
        let file = create_temp_header("- (void)doSomething;\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    #[test]
    fn test_contains_objc_syntax_nullable() {
        let file = create_temp_header("nullable NSString *name;\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    #[test]
    fn test_contains_objc_syntax_nonnull() {
        let file = create_temp_header("nonnull NSString *name;\n");
        assert!(CppFormatter::contains_objc_syntax(file.path()));
    }

    #[test]
    fn test_contains_objc_syntax_pure_cpp() {
        let file = create_temp_header("#include <vector>\nstd::vector<int> v;\n");
        assert!(!CppFormatter::contains_objc_syntax(file.path()));
    }
}
