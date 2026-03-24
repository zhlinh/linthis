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

    fn scan_content(
        &self,
        file_path: &Path,
        content: &str,
    ) -> Vec<SastFinding> {
        let mut findings = Vec::new();
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        // Skip binary/non-source files
        if matches!(
            ext,
            "png" | "jpg" | "jpeg" | "gif" | "ico" | "woff" | "woff2" | "ttf"
                | "eot" | "zip" | "tar" | "gz" | "bin" | "exe" | "dll" | "so"
                | "dylib" | "pdf" | "lock"
        ) {
            return findings;
        }

        for (line_num, line) in content.lines().enumerate() {
            // Skip comment-only lines that look like documentation/examples
            let trimmed = line.trim();
            if trimmed.starts_with('#') && trimmed.contains("example") {
                continue;
            }

            for pattern in &self.compiled {
                if let Some(m) = pattern.regex.find(line) {
                    // Mask the secret value for display
                    let matched = m.as_str();
                    let masked = if matched.len() > 12 {
                        format!("{}...{}", &matched[..8], &matched[matched.len() - 4..])
                    } else {
                        matched.to_string()
                    };

                    let lang = match ext {
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
                        "yaml" | "yml" | "toml" | "json" | "env" | "cfg" | "ini"
                        | "conf" | "properties" => "config",
                        _ => "unknown",
                    };

                    findings.push(SastFinding {
                        rule_id: pattern.id.clone(),
                        severity: pattern.severity,
                        message: format!("{} (matched: {})", pattern.description, masked),
                        file_path: file_path.to_path_buf(),
                        line: line_num + 1,
                        column: Some(m.start() + 1),
                        end_line: None,
                        end_column: Some(m.end() + 1),
                        code_snippet: Some(line.to_string()),
                        fix_suggestion: Some(
                            "Move secret to environment variable or secrets manager"
                                .to_string(),
                        ),
                        category: "secrets".to_string(),
                        cwe_ids: vec![pattern.cwe.clone()],
                        source: "linthis-secrets".to_string(),
                        language: lang.to_string(),
                    });

                    // Only report the first match per line per pattern group
                    break;
                }
            }
        }

        findings
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
