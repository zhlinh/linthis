// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Built-in secrets pattern scanner.
//!
//! Detects hardcoded secrets, API keys, tokens, and credentials by matching
//! common value patterns in source code. No external tool required.
//!
//! Patterns cover: OpenAI/Anthropic API keys, AWS keys, GitHub tokens,
//! private keys, JWTs, generic high-entropy strings assigned to secret-like variables.

use std::path::{Path, PathBuf};

use regex::Regex;
use serde::Deserialize;

use crate::security::sast::finding::SastFinding;
use crate::security::sast::scanner::{SastScanOptions, SastScanner};
use crate::security::vulnerability::Severity;

/// A built-in secret detection pattern (compile-time).
struct SecretPattern {
    /// Pattern identifier
    id: &'static str,
    /// Human-readable description
    description: &'static str,
    /// Regex to match the secret value
    regex: &'static str,
    /// Severity level
    severity: Severity,
    /// CWE identifier
    cwe: &'static str,
}

/// User-defined secret pattern from config file.
///
/// Config file format (TOML):
/// ```toml
/// # .linthis/secrets.toml
///
/// [[patterns]]
/// id = "secrets/internal-token"
/// description = "Internal service token detected"
/// regex = '"tok_[A-Za-z0-9]{32,}"'
/// severity = "high"    # critical, high, medium, low
/// cwe = "CWE-798"
///
/// # Disable a built-in pattern
/// [disabled]
/// rules = ["secrets/jwt-token"]
/// ```
#[derive(Debug, Deserialize)]
struct UserSecretsConfig {
    #[serde(default)]
    patterns: Vec<UserPattern>,
    #[serde(default)]
    disabled: Option<DisabledRules>,
}

#[derive(Debug, Deserialize)]
struct UserPattern {
    id: String,
    description: String,
    regex: String,
    #[serde(default = "default_severity")]
    severity: String,
    #[serde(default = "default_cwe")]
    cwe: String,
}

#[derive(Debug, Deserialize)]
struct DisabledRules {
    #[serde(default)]
    rules: Vec<String>,
}

fn default_severity() -> String {
    "medium".to_string()
}

fn default_cwe() -> String {
    "CWE-798".to_string()
}

/// A compiled pattern ready for matching (from built-in or user config).
struct CompiledPattern {
    id: String,
    description: String,
    regex: Regex,
    severity: Severity,
    cwe: String,
}

/// All built-in secret patterns.
const PATTERNS: &[SecretPattern] = &[
    // --- API Keys by prefix ---
    SecretPattern {
        id: "secrets/sk-prefix-key",
        description: "API key with sk- prefix detected (OpenAI, Anthropic, etc.)",
        regex: r#"["']sk-[A-Za-z0-9\-_]{16,}["']"#,
        severity: Severity::High,
        cwe: "CWE-798",
    },
    SecretPattern {
        id: "secrets/anthropic-api-key",
        description: "Anthropic API key detected",
        regex: r#"["']sk-ant-[A-Za-z0-9\-]{20,}["']"#,
        severity: Severity::High,
        cwe: "CWE-798",
    },
    SecretPattern {
        id: "secrets/aws-access-key",
        description: "AWS Access Key ID detected",
        regex: r#"["']AKIA[0-9A-Z]{16}["']"#,
        severity: Severity::Critical,
        cwe: "CWE-798",
    },
    SecretPattern {
        id: "secrets/github-token",
        description: "GitHub token detected",
        regex: r#"["'](ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{36,}["']"#,
        severity: Severity::High,
        cwe: "CWE-798",
    },
    SecretPattern {
        id: "secrets/gitlab-token",
        description: "GitLab token detected",
        regex: r#"["'](glpat|glptt)-[A-Za-z0-9\-]{20,}["']"#,
        severity: Severity::High,
        cwe: "CWE-798",
    },
    SecretPattern {
        id: "secrets/slack-token",
        description: "Slack token detected",
        regex: r#"["']xox[bpors]-[A-Za-z0-9\-]{10,}["']"#,
        severity: Severity::High,
        cwe: "CWE-798",
    },
    SecretPattern {
        id: "secrets/stripe-key",
        description: "Stripe API key detected",
        regex: r#"["'](sk|pk)_(test|live)_[A-Za-z0-9]{20,}["']"#,
        severity: Severity::High,
        cwe: "CWE-798",
    },
    SecretPattern {
        id: "secrets/google-api-key",
        description: "Google API key detected",
        regex: r#"["']AIza[0-9A-Za-z\-_]{35}["']"#,
        severity: Severity::High,
        cwe: "CWE-798",
    },
    // --- Credential patterns ---
    SecretPattern {
        id: "secrets/private-key",
        description: "Private key detected",
        regex: r#"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----"#,
        severity: Severity::Critical,
        cwe: "CWE-321",
    },
    SecretPattern {
        id: "secrets/jwt-token",
        description: "JWT token detected",
        regex: r#"["']eyJ[A-Za-z0-9\-_]+\.eyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_.+/=]+["']"#,
        severity: Severity::Medium,
        cwe: "CWE-798",
    },
    // --- Generic high-entropy secrets assigned to sensitive variable names ---
    SecretPattern {
        id: "secrets/generic-api-key",
        description: "Possible hardcoded API key or secret",
        regex: r#"(?i)(api[_-]?key|api[_-]?secret|access[_-]?key|secret[_-]?key)\s*=\s*["'][A-Za-z0-9\-_./+]{16,}["']"#,
        severity: Severity::Medium,
        cwe: "CWE-798",
    },
    SecretPattern {
        id: "secrets/generic-password",
        description: "Possible hardcoded password or token",
        regex: r#"(?i)(password|passwd|pwd|token|bearer)\s*=\s*["'][^"'\s]{8,}["']"#,
        severity: Severity::Medium,
        cwe: "CWE-798",
    },
    SecretPattern {
        id: "secrets/connection-string-password",
        description: "Hardcoded password in connection string detected",
        regex: r#"(?i)(password|pwd)\s*=\s*["'][^"'\s]{6,}["']"#,
        severity: Severity::Medium,
        cwe: "CWE-798",
    },
];

/// Built-in secrets pattern scanner. No external tools required.
///
/// Supports user-defined patterns via `.linthis/secrets.toml` or `--sast-config`.
pub struct SecretsScanner {
    compiled: Vec<CompiledPattern>,
}

impl SecretsScanner {
    pub fn new() -> Self {
        Self::with_config(None)
    }

    /// Create scanner with optional user config file.
    ///
    /// Config resolution follows linthis standard priority:
    /// 1. Local `secrets.toml` / `.secrets.toml` (searched from project root upward)
    /// 2. CLI plugin config (`--use-plugin`)
    /// 3. Project plugin config (`.linthis/config.toml` plugins)
    /// 4. Global plugin config (`~/.linthis/config.toml` plugins)
    /// 5. Built-in patterns only (no user config)
    ///
    /// The `config_path` should be resolved by `ConfigResolver` before calling this.
    /// Falls back to searching `.linthis/secrets.toml` if no resolver is available.
    pub fn with_config(config_path: Option<&Path>) -> Self {
        let mut disabled: Vec<String> = Vec::new();
        let mut user_patterns: Vec<CompiledPattern> = Vec::new();

        // Config resolution:
        // 1. Use explicitly provided path (from ConfigResolver or --sast-config)
        // 2. Fall back to standard .linthis/ locations
        let search_paths: Vec<PathBuf> = if let Some(p) = config_path {
            vec![p.to_path_buf()]
        } else {
            // Standard linthis config locations (project-level)
            let mut paths = vec![
                PathBuf::from("secrets.toml"),
                PathBuf::from(".secrets.toml"),
                PathBuf::from(".linthis/secrets.toml"),
                PathBuf::from(".linthis/configs/secrets.toml"),
            ];
            // Global config location
            if let Ok(home) = std::env::var("HOME") {
                paths.push(PathBuf::from(format!("{}/.linthis/secrets.toml", home)));
                paths.push(PathBuf::from(format!(
                    "{}/.linthis/configs/secrets.toml",
                    home
                )));
            }
            paths
        };

        for cfg_path in &search_paths {
            if let Ok(content) = std::fs::read_to_string(cfg_path) {
                if let Ok(config) = toml::from_str::<UserSecretsConfig>(&content) {
                    // Collect disabled rules
                    if let Some(ref d) = config.disabled {
                        disabled.extend(d.rules.clone());
                    }
                    // Compile user patterns
                    for p in config.patterns {
                        if let Ok(regex) = Regex::new(&p.regex) {
                            user_patterns.push(CompiledPattern {
                                id: p.id,
                                description: p.description,
                                regex,
                                severity: Severity::from_str(&p.severity),
                                cwe: p.cwe,
                            });
                        }
                    }
                    break; // Use the first config found (highest priority)
                }
            }
        }

        // Compile built-in patterns (skip disabled ones)
        let mut compiled: Vec<CompiledPattern> = PATTERNS
            .iter()
            .filter(|p| !disabled.contains(&p.id.to_string()))
            .filter_map(|p| {
                Regex::new(p.regex).ok().map(|r| CompiledPattern {
                    id: p.id.to_string(),
                    description: p.description.to_string(),
                    regex: r,
                    severity: p.severity,
                    cwe: p.cwe.to_string(),
                })
            })
            .collect();

        // Append user patterns (they run after built-in ones)
        compiled.append(&mut user_patterns);

        Self { compiled }
    }

    fn scan_content(&self, file_path: &Path, content: &str) -> Vec<SastFinding> {
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        if is_binary_extension(ext) {
            return Vec::new();
        }

        let mut findings = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut next_line_directive: Option<String> = None;

        for (line_num, line) in lines.iter().enumerate() {
            let effective_directive = resolve_effective_directive(line, &mut next_line_directive);

            self.scan_line(
                file_path,
                ext,
                line,
                line_num,
                &effective_directive,
                &mut findings,
            );
        }

        findings
    }

    fn scan_line(
        &self,
        file_path: &Path,
        ext: &str,
        line: &str,
        line_num: usize,
        effective_directive: &Option<String>,
        findings: &mut Vec<SastFinding>,
    ) {
        for pattern in &self.compiled {
            if is_pattern_ignored(effective_directive, &pattern.id) {
                continue;
            }

            if let Some(m) = pattern.regex.find(line) {
                let matched = m.as_str();
                if is_placeholder_value(matched) {
                    continue;
                }

                findings.push(build_finding(pattern, file_path, ext, line, line_num, &m));
                break;
            }
        }
    }
}

/// Check whether a file extension indicates a binary/non-source file.
fn is_binary_extension(ext: &str) -> bool {
    matches!(
        ext,
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "ico"
            | "woff"
            | "woff2"
            | "ttf"
            | "eot"
            | "zip"
            | "tar"
            | "gz"
            | "bin"
            | "exe"
            | "dll"
            | "so"
            | "dylib"
            | "pdf"
            | "lock"
    )
}

/// Resolve the effective ignore directive for a line, consuming any pending
/// next-line directive and merging it with the inline directive.
fn resolve_effective_directive(
    line: &str,
    next_line_directive: &mut Option<String>,
) -> Option<String> {
    let ignore_directive = parse_ignore_directive(line);
    let ignore_next = parse_ignore_next_line_directive(line);

    let effective = if next_line_directive.is_some() {
        let d = next_line_directive.take();
        match (&d, &ignore_directive) {
            (_, Some(inline)) if inline == "secrets" => Some("secrets".to_string()),
            (Some(next), Some(_inline)) if next == "secrets" => Some("secrets".to_string()),
            (_, Some(inline)) => Some(inline.clone()),
            (d, None) => d.clone(),
        }
    } else {
        ignore_directive
    };

    if ignore_next.is_some() {
        *next_line_directive = ignore_next;
    }

    effective
}

/// Check whether a specific pattern is suppressed by the directive.
fn is_pattern_ignored(directive: &Option<String>, pattern_id: &str) -> bool {
    if let Some(ref d) = directive {
        d == "secrets" || *d == pattern_id
    } else {
        false
    }
}

/// Map a file extension to a language identifier.
fn ext_to_language(ext: &str) -> &'static str {
    match ext {
        "py" => "python",
        "js" | "jsx" | "mjs" => "javascript",
        "ts" | "tsx" => "typescript",
        "go" => "go",
        "rs" => "rust",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "c" | "h" => "c",
        "cpp" | "cc" | "hpp" => "cpp",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "yaml" | "yml" | "toml" | "json" | "env" | "cfg" | "ini" | "conf" | "properties" => {
            "config"
        }
        _ => "unknown",
    }
}

/// Mask a matched secret value for display.
fn mask_matched_value(matched: &str) -> String {
    if matched.len() > 12 {
        format!("{}...{}", &matched[..8], &matched[matched.len() - 4..])
    } else {
        matched.to_string()
    }
}

/// Build a `SastFinding` from a regex match on a line.
fn build_finding(
    pattern: &CompiledPattern,
    file_path: &Path,
    ext: &str,
    line: &str,
    line_num: usize,
    m: &regex::Match<'_>,
) -> SastFinding {
    let matched = m.as_str();
    let masked = mask_matched_value(matched);
    let lang = ext_to_language(ext);

    SastFinding {
        rule_id: pattern.id.clone(),
        severity: pattern.severity,
        message: format!("{} (matched: {})", pattern.description, masked),
        file_path: file_path.to_path_buf(),
        line: line_num + 1,
        column: Some(m.start() + 1),
        end_line: None,
        end_column: Some(m.end() + 1),
        code_snippet: Some(line.to_string()),
        fix_suggestion: Some("Move secret to environment variable or secrets manager".to_string()),
        category: "secrets".to_string(),
        cwe_ids: vec![pattern.cwe.clone()],
        source: "linthis-secrets".to_string(),
        language: lang.to_string(),
    }
}

impl Default for SecretsScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl SastScanner for SecretsScanner {
    fn name(&self) -> &str {
        "linthis-secrets"
    }

    fn supported_languages(&self) -> &[&str] {
        &["*"] // Scans all text files
    }

    fn is_available(&self) -> bool {
        true // Built-in, always available
    }

    fn scan(
        &self,
        path: &Path,
        files: &[PathBuf],
        _options: &SastScanOptions,
    ) -> Result<Vec<SastFinding>, String> {
        let mut all_findings = Vec::new();

        if files.is_empty() {
            // Walk the directory
            self.walk_and_scan(path, &mut all_findings);
        } else {
            for file in files {
                if let Ok(content) = std::fs::read_to_string(file) {
                    all_findings.extend(self.scan_content(file, &content));
                }
            }
        }

        Ok(all_findings)
    }

    fn install_hint(&self) -> String {
        "Built-in scanner, always available".to_string()
    }
}

impl SecretsScanner {
    fn walk_and_scan(&self, dir: &Path, findings: &mut Vec<SastFinding>) {
        let walker = match std::fs::read_dir(dir) {
            Ok(w) => w,
            Err(_) => return,
        };

        for entry in walker.flatten() {
            let path = entry.path();

            // Skip hidden dirs and common non-source dirs
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.')
                    || matches!(
                        name,
                        "node_modules"
                            | "vendor"
                            | "target"
                            | "__pycache__"
                            | "dist"
                            | "build"
                            | ".git"
                    )
                {
                    continue;
                }
            }

            if path.is_dir() {
                self.walk_and_scan(&path, findings);
            } else if path.is_file() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    findings.extend(self.scan_content(&path, &content));
                }
            }
        }
    }
}

/// Check if a matched value is a placeholder/dummy that should be ignored.
///
/// Detects patterns like: "xxx", "XXX", "****", "0000", "YOUR_TOKEN_HERE",
/// "placeholder", "example", "test", "dummy", "sample", "changeme", etc.
fn is_placeholder_value(matched: &str) -> bool {
    // Strip surrounding quotes
    let val = matched.trim_matches(|c| c == '"' || c == '\'');

    // Too short to be a real secret (after stripping known prefixes)
    // e.g., "sk-xxx" → strip "sk-" → "xxx" = 3 chars
    let core = val
        .trim_start_matches("sk-")
        .trim_start_matches("sk_test_")
        .trim_start_matches("sk_live_")
        .trim_start_matches("pk_test_")
        .trim_start_matches("pk_live_")
        .trim_start_matches("ghp_")
        .trim_start_matches("gho_")
        .trim_start_matches("glpat-")
        .trim_start_matches("xoxb-")
        .trim_start_matches("AKIA")
        .trim_start_matches("AIza");

    if core.len() < 4 {
        return true;
    }

    let lower = val.to_lowercase();

    // All same character (xxx, XXX, ***, 000, aaa, etc.)
    let chars: Vec<char> = core.chars().collect();
    if !chars.is_empty() && chars.iter().all(|c| *c == chars[0]) {
        return true;
    }

    // Common placeholder words
    let placeholder_words = [
        "your_",
        "my_",
        "insert",
        "replace",
        "change",
        "update",
        "put_",
        "placeholder",
        "example",
        "sample",
        "dummy",
        "fake",
        "mock",
        "test",
        "demo",
        "todo",
        "fixme",
        "changeme",
        "redacted",
        "xxxxxxxx",
        "abcdef",
        "123456",
        "000000",
    ];
    for word in &placeholder_words {
        if lower.contains(word) {
            return true;
        }
    }

    // Repeating pattern like "abcabc" or "xyzxyz".
    // Compare via the char vec rather than byte slicing — `core` may contain
    // multi-byte chars (e.g. "<｜tool▁calls▁begin｜>"), and slicing at
    // `core.len() / 2` would land mid-codepoint and panic.
    let n = chars.len();
    if n >= 6 {
        let half = n / 2;
        if chars[..half] == chars[half..half * 2] {
            return true;
        }
    }

    false
}

/// Parse inline ignore directive from a line.
///
/// Syntax: `# linthis:ignore <target>`
///   - `# linthis:ignore secrets`                — ignore all secrets checks on this line
///   - `# linthis:ignore secrets/sk-prefix-key`  — ignore specific rule on this line
///   - `// linthis:ignore secrets`               — C-style comment
///
/// Returns the ignore target, or None if not found or no target specified.
fn parse_ignore_directive(line: &str) -> Option<String> {
    // Must be "linthis:ignore" NOT followed by "-next-line"
    let marker = "linthis:ignore";
    let pos = line.find(marker)?;

    let after_marker = &line[pos + marker.len()..];
    // Reject "linthis:ignore-next-line" — that's a different directive
    if after_marker.starts_with("-next-line") {
        return None;
    }

    extract_ignore_target(after_marker)
}

/// Parse ignore-next-line directive from a line.
///
/// Syntax: `# linthis:ignore-next-line <target>`
///   - `# linthis:ignore-next-line secrets`                — ignore all secrets on next line
///   - `# linthis:ignore-next-line secrets/sk-prefix-key`  — ignore specific rule on next line
///
/// Returns the ignore target, or None if not found.
fn parse_ignore_next_line_directive(line: &str) -> Option<String> {
    let marker = "linthis:ignore-next-line";
    let pos = line.find(marker)?;
    let after_marker = &line[pos + marker.len()..];
    extract_ignore_target(after_marker)
}

/// Extract the target from text after a linthis:ignore marker.
fn extract_ignore_target(after_marker: &str) -> Option<String> {
    let trimmed = after_marker.trim();
    if trimmed.is_empty() {
        return None;
    }

    let target = trimmed
        .split_whitespace()
        .next()?
        .trim_end_matches("*/")
        .trim();

    if target.is_empty() {
        return None;
    }

    Some(target.to_string())
}

#[cfg(test)]
mod tests {
    use super::is_placeholder_value;

    #[test]
    fn handles_multibyte_chars_in_repeating_pattern_check() {
        // Regression: byte-index slicing at core.len() / 2 used to panic when
        // the half-point fell inside a multi-byte codepoint (e.g. '▁').
        let input = "TOKEN = \"<｜tool▁calls▁begin｜>\"";
        // Must not panic; result is a bool either way.
        let _ = is_placeholder_value(input);
    }

    #[test]
    fn detects_ascii_repeating_pattern() {
        assert!(is_placeholder_value("abcabc"));
        assert!(is_placeholder_value("xyzxyz"));
    }

    #[test]
    fn detects_multibyte_repeating_pattern() {
        // Compare via chars, not bytes — two "▁▁" halves should be equal.
        assert!(is_placeholder_value("▁▁▁▁▁▁"));
    }

    #[test]
    fn rejects_nonrepeating_mixed_content() {
        assert!(!is_placeholder_value("sk-9aK3p7Qz"));
    }
}

