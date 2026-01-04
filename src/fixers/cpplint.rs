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
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;

use regex::Regex;

// Installation state: 0 = not checked, 1 = installing, 2 = installed, 3 = failed
static CPPLINT_INSTALL_STATE: AtomicU8 = AtomicU8::new(0);
static INSTALL_LOCK: Mutex<()> = Mutex::new(());

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
    /// Whether the current file is Objective-C (skip unsafe fixes)
    is_objc: bool,
}

impl CpplintFixer {
    pub fn new() -> Self {
        Self {
            config: CpplintFixerConfig::default(),
            cached_username: None,
            is_objc: false,
        }
    }

    pub fn with_config(config: CpplintFixerConfig) -> Self {
        Self {
            config,
            cached_username: None,
            is_objc: false,
        }
    }

    /// Set whether the current file is Objective-C
    /// This will skip unsafe fix categories (like readability/casting)
    pub fn set_is_objc(&mut self, is_objc: bool) {
        self.is_objc = is_objc;
    }

    /// Check if cpplint is available
    fn has_cpplint() -> bool {
        Command::new("cpplint")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Try to auto-install cpplint using pip
    fn try_install_cpplint() -> bool {
        // Acquire lock to ensure only one thread installs
        let _lock = INSTALL_LOCK.lock().unwrap();

        // Double-check state after acquiring lock
        let state = CPPLINT_INSTALL_STATE.load(Ordering::SeqCst);
        if state != 0 {
            return state == 2; // Return true if already installed
        }

        // Set installing state
        CPPLINT_INSTALL_STATE.store(1, Ordering::SeqCst);

        eprintln!("\n📦 cpplint not found, attempting to install...");

        // Try pip first (more compatible on macOS), then pip3
        for pip_cmd in &["pip", "pip3"] {
            if !Command::new(pip_cmd)
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                continue;
            }

            eprintln!("   Using {} to install cpplint...", pip_cmd);

            // Run pip install with progress output
            let mut child = match Command::new(pip_cmd)
                .args(&["install", "cpplint", "--upgrade"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(child) => child,
                Err(e) => {
                    eprintln!("   ❌ Failed to start pip: {}", e);
                    continue;
                }
            };

            // Read and display output
            if let Some(stderr) = child.stderr.take() {
                let reader = BufReader::new(stderr);
                for line in reader.lines().flatten() {
                    // Filter and display relevant progress information
                    if line.contains("Collecting")
                        || line.contains("Downloading")
                        || line.contains("Installing")
                        || line.contains("Successfully")
                    {
                        eprintln!("   {}", line);
                    }
                }
            }

            // Wait for installation to complete
            match child.wait() {
                Ok(status) if status.success() => {
                    // Verify installation
                    if Self::has_cpplint() {
                        eprintln!("   ✓ cpplint installed successfully!\n");
                        CPPLINT_INSTALL_STATE.store(2, Ordering::SeqCst);
                        return true;
                    } else {
                        eprintln!("   ⚠️  Installation completed but cpplint not found in PATH");
                        eprintln!("   You may need to restart your terminal or add Python's bin directory to PATH\n");
                    }
                }
                Ok(status) => {
                    eprintln!("   ❌ Installation failed with exit code: {}", status);
                }
                Err(e) => {
                    eprintln!("   ❌ Failed to wait for pip: {}", e);
                }
            }
        }

        // Installation failed
        eprintln!("   ❌ Auto-installation failed. Please install manually:");
        eprintln!("      pip install cpplint");
        eprintln!("   Or if pip doesn't work:");
        eprintln!("      pip3 install cpplint\n");

        CPPLINT_INSTALL_STATE.store(3, Ordering::SeqCst);
        false
    }

    /// Run cpplint and get errors
    fn run_cpplint(path: &Path, is_objc: bool) -> Vec<CpplintError> {
        // Check if cpplint is available
        if !Self::has_cpplint() {
            let state = CPPLINT_INSTALL_STATE.load(Ordering::SeqCst);

            match state {
                0 => {
                    // First time detection - try to auto-install
                    if Self::try_install_cpplint() {
                        // Installation successful, continue
                    } else {
                        // Installation failed, skip cpplint
                        return Vec::new();
                    }
                }
                1 => {
                    // Installation in progress (another thread), skip for now
                    return Vec::new();
                }
                2 => {
                    // Should have been installed, but still not found
                    // This shouldn't happen, but skip silently
                    return Vec::new();
                }
                3 => {
                    // Installation failed previously, skip silently
                    return Vec::new();
                }
                _ => {
                    return Vec::new();
                }
            }
        }

        let mut cmd = Command::new("cpplint");

        // Add extensions for Objective-C files
        if is_objc {
            cmd.arg("--extensions=m,mm,h");
            cmd.arg("--linelength=150");
        } else {
            cmd.arg("--linelength=120");
        }

        cmd.arg(path);
        let output = cmd.output();

        let output = match output {
            Ok(o) => o,
            Err(e) => {
                eprintln!("[cpplint-fixer] Failed to run cpplint: {}", e);
                return Vec::new();
            }
        };

        // cpplint outputs to stderr
        let stderr = String::from_utf8_lossy(&output.stderr);
        let errors = Self::parse_cpplint_output(&stderr);

        if std::env::var("LINTHIS_DEBUG").is_ok() {
            eprintln!("[cpplint-fixer] {} cpplint stderr:\n{}", path.display(), stderr);
            eprintln!("[cpplint-fixer] Parsed {} errors", errors.len());
            for e in &errors {
                eprintln!("[cpplint-fixer]   line {}: {} [{}]", e.line, e.message, e.category);
            }
        }

        errors
    }

    /// Parse cpplint output into structured errors
    fn parse_cpplint_output(output: &str) -> Vec<CpplintError> {
        let mut errors = Vec::new();

        // Format: file:line: message [category] [confidence]
        // Example: test.h:8: #ifndef header guard has wrong style, please use: FOO_H_ [build/header_guard] [5]
        let re = Regex::new(r"^([^:]+):(\d+):\s*(.+?)\s*\[([^\]]+)\]").unwrap();

        for line in output.lines() {
            if let Some(caps) = re.captures(line) {
                let file_path = &caps[1];

                // Skip errors from system paths (SDK, frameworks, system includes)
                if file_path.starts_with("/Library/Developer/")
                    || file_path.starts_with("/System/Library/")
                    || file_path.starts_with("/usr/include/")
                    || file_path.starts_with("/usr/local/include/")
                    || file_path.contains("/SDKs/")
                    || file_path.contains(".framework/") {
                    continue;
                }

                if let Ok(line_num) = caps[2].parse::<usize>() {
                    errors.push(CpplintError {
                        line: line_num,
                        message: caps[3].to_string(),
                        category: caps[4].to_string(),
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
        if let Ok(output) = Command::new("git").args(["config", "user.name"]).output() {
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

        let debug = std::env::var("LINTHIS_DEBUG").is_ok();

        // Run cpplint to get errors (pass is_objc flag for correct options)
        let errors = Self::run_cpplint(path, self.is_objc);
        if errors.is_empty() {
            if debug {
                eprintln!("[cpplint-fixer] No errors found for {}", path.display());
            }
            return Ok(false);
        }

        if debug {
            eprintln!("[cpplint-fixer] Processing {} errors for {}", errors.len(), path.display());
        }

        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut modified = false;

        // Process errors
        for error in &errors {
            match error.category.as_str() {
                "build/header_guard" => {
                    // Skip header guard fixes for OC files - OC uses #import which handles include guards
                    if self.is_objc {
                        if debug {
                            eprintln!("[cpplint-fixer] Skipping build/header_guard for OC file");
                        }
                    } else if self.config.header_guard_mode == HeaderGuardMode::FixName {
                        if self.fix_header_guard_from_error(&mut lines, error) {
                            if debug {
                                eprintln!("[cpplint-fixer] Fixed header_guard at line {}", error.line);
                            }
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
                        if debug {
                            eprintln!("[cpplint-fixer] Fixed todo at line {}", error.line);
                        }
                        modified = true;
                    }
                }
                "legal/copyright" => {
                    if self.fix_copyright_from_error(&mut lines) {
                        modified = true;
                    }
                }
                "readability/casting" => {
                    // Skip C-style cast fixes for OC files - OC method signatures
                    // like `+ (UIImage *)method` are misinterpreted as C-style casts
                    if self.is_objc {
                        if debug {
                            eprintln!("[cpplint-fixer] Skipping readability/casting for OC file");
                        }
                    } else if self.fix_c_style_cast(&mut lines, error) {
                        modified = true;
                    }
                }
                "readability/check" => {
                    if self.fix_assert_check(&mut lines, error) {
                        modified = true;
                    }
                }
                "whitespace/comments" => {
                    if self.fix_comment_spacing(&mut lines, error) {
                        if debug {
                            eprintln!("[cpplint-fixer] Fixed comment spacing at line {}", error.line);
                        }
                        modified = true;
                    }
                }
                "whitespace/semicolon" => {
                    if self.fix_empty_semicolon(&mut lines, error) {
                        if debug {
                            eprintln!("[cpplint-fixer] Fixed empty semicolon at line {}", error.line);
                        }
                        modified = true;
                    }
                }
                "whitespace/comma" => {
                    if self.fix_comma_spacing(&mut lines, error) {
                        if debug {
                            eprintln!("[cpplint-fixer] Fixed comma spacing at line {}", error.line);
                        }
                        modified = true;
                    }
                }
                "whitespace/operators" => {
                    // Skip for OC files - @property (getter=xxx) syntax is valid OC
                    if self.is_objc {
                        if debug {
                            eprintln!("[cpplint-fixer] Skipping whitespace/operators for OC file");
                        }
                    } else if self.fix_operator_spacing(&mut lines, error) {
                        if debug {
                            eprintln!("[cpplint-fixer] Fixed operator spacing at line {}", error.line);
                        }
                        modified = true;
                    }
                }
                _ => {
                    if debug {
                        eprintln!("[cpplint-fixer] Skipping unsupported category: {}", error.category);
                    }
                }
            }
        }

        if modified {
            let new_content = lines.join("\n") + if content.ends_with('\n') { "\n" } else { "" };
            fs::write(path, new_content).map_err(|e| format!("Failed to write file: {}", e))?;
        }

        Ok(modified)
    }

    /// Fix header guard based on cpplint error message
    fn fix_header_guard_from_error(&self, lines: &mut Vec<String>, error: &CpplintError) -> bool {
        let debug = std::env::var("LINTHIS_DEBUG").is_ok();

        // Extract suggested guard name from message
        // Message formats:
        // 1. "#ifndef header guard has wrong style, please use: GUARD_NAME_"
        // 2. "#endif line should be "#endif  // GUARD_NAME_""
        // 3. "No #ifndef header guard found, suggested CPP variable is: GUARD_NAME_"

        if debug {
            eprintln!("[cpplint-fixer] fix_header_guard_from_error: line={}, msg={}", error.line, error.message);
        }

        let suggested_guard = if error.message.contains("please use:") {
            // Extract from "#ifndef header guard has wrong style, please use: GUARD_NAME_"
            error
                .message
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
        } else if error.message.contains("suggested CPP variable is:") {
            // Extract from "No #ifndef header guard found, suggested CPP variable is: GUARD_NAME_"
            error
                .message
                .split("suggested CPP variable is:")
                .nth(1)
                .map(|s| s.trim().to_string())
        } else {
            None
        };

        let suggested_guard = match suggested_guard {
            Some(g) => g,
            None => return false,
        };

        // Handle missing header guard (line 0 means no guard at all)
        if error.line == 0 || error.message.contains("No #ifndef header guard found") {
            return self.insert_header_guard(lines, &suggested_guard);
        }

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

    /// Insert header guard when none exists
    fn insert_header_guard(&self, lines: &mut Vec<String>, guard_name: &str) -> bool {
        // Find the insertion point (after copyright/license comments)
        let mut insert_idx = 0;
        let mut in_block_comment = false;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Track block comments
            if trimmed.starts_with("/*") {
                in_block_comment = true;
            }
            if in_block_comment {
                if trimmed.contains("*/") {
                    in_block_comment = false;
                }
                insert_idx = i + 1;
                continue;
            }

            // Skip line comments at the start (copyright headers)
            if trimmed.starts_with("//") {
                insert_idx = i + 1;
                continue;
            }

            // Skip empty lines after comments
            if trimmed.is_empty() && insert_idx > 0 {
                insert_idx = i + 1;
                continue;
            }

            // Found first real content
            break;
        }

        // Check if already has #ifndef (shouldn't happen, but be safe)
        if lines.iter().any(|l| l.trim().starts_with("#ifndef")) {
            return false;
        }

        // Insert header guard at the found position
        // Add empty line before if not at start and previous line isn't empty
        if insert_idx > 0 && !lines[insert_idx - 1].trim().is_empty() {
            lines.insert(insert_idx, String::new());
            insert_idx += 1;
        }

        lines.insert(insert_idx, format!("#ifndef {}", guard_name));
        lines.insert(insert_idx + 1, format!("#define {}", guard_name));
        lines.insert(insert_idx + 2, String::new());

        // Add #endif at the end
        // Ensure there's an empty line before #endif
        if !lines.last().map_or(true, |l| l.trim().is_empty()) {
            lines.push(String::new());
        }
        lines.push(format!("#endif  // {}", guard_name));

        true
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
            } else if ifndef_idx.is_some() && define_idx.is_none() && trimmed.starts_with("#define")
            {
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
        let first_lines: String = lines
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
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

    /// Fix comment spacing: "//comment" -> "// comment"
    /// NOTE: This only modifies actual comments, not `//` inside string literals
    /// Uses the same detection logic as cpplint's IsCppString function
    fn fix_comment_spacing(&self, lines: &mut [String], error: &CpplintError) -> bool {
        let line_idx = error.line.saturating_sub(1);
        if line_idx >= lines.len() {
            return false;
        }

        let line = &lines[line_idx];
        let fixed = Self::fix_comment_spacing_line(line);

        if fixed != *line {
            lines[line_idx] = fixed;
            true
        } else {
            false
        }
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

    /// Fix empty semicolon: replace lone `;` with `{}`
    /// Example: "    ;  // comment" -> "    {}  // comment"
    fn fix_empty_semicolon(&self, lines: &mut [String], error: &CpplintError) -> bool {
        if !error.message.contains("Line contains only semicolon") {
            return false;
        }

        let line_idx = error.line.saturating_sub(1);
        if line_idx >= lines.len() {
            return false;
        }

        let line = &lines[line_idx];

        // Find the position of the lone semicolon (only whitespace before it)
        // Pattern: start with whitespace, then a semicolon, optionally followed by comment or whitespace
        if let Some(re) = Regex::new(r"^(\s*);(\s*(?://.*)?)?$").ok() {
            if let Some(caps) = re.captures(line) {
                let indent = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let suffix = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                lines[line_idx] = format!("{}{}{}",indent, "{}", suffix);
                return true;
            }
        }

        false
    }

    /// Fix comma spacing: add space after comma
    /// Example: "foo(a,b)" -> "foo(a, b)"
    /// Skips #pragma mark lines
    fn fix_comma_spacing(&self, lines: &mut [String], error: &CpplintError) -> bool {
        if !error.message.contains("Missing space after ,") {
            return false;
        }

        let line_idx = error.line.saturating_sub(1);
        if line_idx >= lines.len() {
            return false;
        }

        let line = &lines[line_idx];

        // Skip #pragma mark lines - they contain descriptive text where comma is part of content
        if line.trim_start().starts_with("#pragma mark") {
            return false;
        }

        // Add space after comma (but not if already followed by space or end of string)
        // Be careful not to modify strings - use simple approach for now
        let mut result = String::with_capacity(line.len() + 10);
        let mut chars = line.chars().peekable();
        let mut modified = false;

        while let Some(c) = chars.next() {
            result.push(c);
            if c == ',' {
                if let Some(&next) = chars.peek() {
                    if next != ' ' && next != '\n' && next != '\r' {
                        result.push(' ');
                        modified = true;
                    }
                }
            }
        }

        if modified {
            lines[line_idx] = result;
            true
        } else {
            false
        }
    }

    /// Fix operator spacing: add spaces around =
    /// Example: "int x=1" -> "int x = 1"
    fn fix_operator_spacing(&self, lines: &mut [String], error: &CpplintError) -> bool {
        if !error.message.contains("Missing spaces around =") {
            return false;
        }

        let line_idx = error.line.saturating_sub(1);
        if line_idx >= lines.len() {
            return false;
        }

        let line = &lines[line_idx];

        // Use regex to find = without proper spacing
        // Match: not preceded by space + = + not followed by space or =
        // But avoid ==, !=, <=, >=, +=, -=, etc.
        if let Some(re) = Regex::new(r"([^\s=!<>+\-*/%&|^])=([^=\s])").ok() {
            let result = re.replace_all(line, "$1 = $2").to_string();
            if result != *line {
                lines[line_idx] = result;
                return true;
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
    fn test_parse_missing_header_guard() {
        // Test parsing "No #ifndef header guard found" error (line 0)
        let output = r##"test.h:0:  No #ifndef header guard found, suggested CPP variable is: TEST_H_  [build/header_guard] [5]
"##;

        let errors = CpplintFixer::parse_cpplint_output(output);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 0);
        assert_eq!(errors[0].category, "build/header_guard");
        assert!(errors[0].message.contains("suggested CPP variable is: TEST_H_"));
    }

    #[test]
    fn test_insert_missing_header_guard() {
        let fixer = CpplintFixer::new();

        // File without header guard
        let mut lines = vec![
            "// Copyright notice".to_string(),
            "".to_string(),
            "#include <stdio.h>".to_string(),
            "".to_string(),
            "void foo();".to_string(),
        ];

        let error = CpplintError {
            line: 0,
            message: "No #ifndef header guard found, suggested CPP variable is: TEST_H_".to_string(),
            category: "build/header_guard".to_string(),
        };

        assert!(fixer.fix_header_guard_from_error(&mut lines, &error));

        // Check that header guard was inserted after copyright
        assert!(lines.iter().any(|l| l.contains("#ifndef TEST_H_")));
        assert!(lines.iter().any(|l| l.contains("#define TEST_H_")));
        assert!(lines.iter().any(|l| l.contains("#endif  // TEST_H_")));
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
            message:
                r#"Missing username in TODO; it should look like "// TODO(my_username): Stuff.""#
                    .to_string(),
            category: "readability/todo".to_string(),
        };

        assert!(fixer.fix_todo_from_error(&mut lines, &error));
        assert_eq!(lines[0], "// TODO(testuser): fix this");
        assert_eq!(lines[1], "// TODO(existing): keep this");
    }

    #[test]
    fn test_fix_endif_line() {
        let fixer = CpplintFixer::new();

        let mut lines = vec![
            "#ifndef GUARD_H_".to_string(),
            "#define GUARD_H_".to_string(),
            "// content".to_string(),
            "#endif".to_string(),
        ];

        let error = CpplintError {
            line: 4,
            message: r##"#endif line should be "#endif  // GUARD_H_""##.to_string(),
            category: "build/header_guard".to_string(),
        };

        assert!(fixer.fix_header_guard_from_error(&mut lines, &error));
        assert_eq!(lines[3], "#endif  // GUARD_H_");
    }

    #[test]
    fn test_fix_comment_spacing_cpplint() {
        let fixer = CpplintFixer::new();

        let mut lines = vec![
            "int x = 1; //comment".to_string(),
            "int y = 2; // already spaced".to_string(),
        ];

        let error = CpplintError {
            line: 1,
            message: "Should have a space between // and comment".to_string(),
            category: "whitespace/comments".to_string(),
        };

        assert!(fixer.fix_comment_spacing(&mut lines, &error));
        assert_eq!(lines[0], "int x = 1; // comment");
        assert_eq!(lines[1], "int y = 2; // already spaced");
    }

    #[test]
    fn test_fix_comment_spacing_triple_slash() {
        let fixer = CpplintFixer::new();

        let mut lines = vec!["///doc comment".to_string()];

        let error = CpplintError {
            line: 1,
            message: "Should have a space between // and comment".to_string(),
            category: "whitespace/comments".to_string(),
        };

        assert!(fixer.fix_comment_spacing(&mut lines, &error));
        assert_eq!(lines[0], "/// doc comment");
    }

    #[test]
    fn test_fix_comment_spacing_preserves_url() {
        let fixer = CpplintFixer::new();

        // URLs like https:// should NOT be modified
        let mut lines = vec!["return @\"https://example.com\";".to_string()];

        let error = CpplintError {
            line: 1,
            message: "Should have a space between // and comment".to_string(),
            category: "whitespace/comments".to_string(),
        };

        // Should not modify URL
        assert!(!fixer.fix_comment_spacing(&mut lines, &error));
        assert_eq!(lines[0], "return @\"https://example.com\";");
    }

    #[test]
    fn test_fix_comment_spacing_url_and_comment() {
        let fixer = CpplintFixer::new();

        // Should preserve URL but fix comment
        let mut lines = vec!["NSString *url = @\"https://example.com\"; //comment".to_string()];

        let error = CpplintError {
            line: 1,
            message: "Should have a space between // and comment".to_string(),
            category: "whitespace/comments".to_string(),
        };

        assert!(fixer.fix_comment_spacing(&mut lines, &error));
        assert_eq!(lines[0], "NSString *url = @\"https://example.com\"; // comment");
    }

    #[test]
    fn test_insert_header_guard_after_block_comment() {
        let fixer = CpplintFixer::new();

        let mut lines = vec![
            "/*".to_string(),
            " * Copyright 2024".to_string(),
            " */".to_string(),
            "".to_string(),
            "#include <stdio.h>".to_string(),
        ];

        let error = CpplintError {
            line: 0,
            message: "No #ifndef header guard found, suggested CPP variable is: TEST_H_".to_string(),
            category: "build/header_guard".to_string(),
        };

        assert!(fixer.fix_header_guard_from_error(&mut lines, &error));

        // Find positions
        let ifndef_pos = lines.iter().position(|l| l.contains("#ifndef TEST_H_")).unwrap();
        let block_end_pos = lines.iter().position(|l| l.contains("*/")).unwrap();

        // Header guard should be after block comment
        assert!(ifndef_pos > block_end_pos);
    }

    #[test]
    fn test_parse_whitespace_comments_error() {
        let output = r##"test.cc:5:  Should have a space between // and comment  [whitespace/comments] [4]
"##;

        let errors = CpplintFixer::parse_cpplint_output(output);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 5);
        assert_eq!(errors[0].category, "whitespace/comments");
    }
}
