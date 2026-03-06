// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! AI provider implementations.

use serde::{Deserialize, Serialize};
use std::env;
use std::process::{Command, Stdio};

/// Supported AI provider types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AiProviderKind {
    /// Anthropic Claude API
    #[default]
    Claude,
    /// Claude CLI (claude -p command)
    ClaudeCli,
    /// CodeBuddy API
    CodeBuddy,
    /// CodeBuddy CLI (codebuddy -p command)
    CodeBuddyCli,
    /// OpenAI GPT API
    OpenAi,
    /// OpenAI Codex CLI (codex exec command)
    CodexCli,
    /// Google Gemini API
    Gemini,
    /// Gemini CLI (gemini command)
    GeminiCli,
    /// Local LLM (Ollama, llama.cpp, etc.)
    Local,
    /// Custom provider defined in config
    Custom(String),
    /// Mock provider for testing
    Mock,
}

impl std::str::FromStr for AiProviderKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" | "anthropic" => Ok(Self::Claude),
            "claude-cli" | "claudecli" => Ok(Self::ClaudeCli),
            "codebuddy" | "buddy" => Ok(Self::CodeBuddy),
            "codebuddy-cli" | "codebuddycli" | "cli" => Ok(Self::CodeBuddyCli),
            "openai" | "gpt" => Ok(Self::OpenAi),
            "codex-cli" | "codexcli" | "codex" => Ok(Self::CodexCli),
            "gemini" | "google" => Ok(Self::Gemini),
            "gemini-cli" | "geminicli" => Ok(Self::GeminiCli),
            "local" | "ollama" | "llama" => Ok(Self::Local),
            "mock" | "test" => Ok(Self::Mock),
            other => Ok(Self::Custom(other.to_string())),
        }
    }
}

/// All user-facing AI providers with their CLI name and description (excludes Mock)
pub const ALL_AI_PROVIDERS: &[(AiProviderKind, &str, &str)] = &[
    (AiProviderKind::Claude, "claude", "Anthropic Claude API"),
    (AiProviderKind::ClaudeCli, "claude-cli", "Claude CLI"),
    (AiProviderKind::CodeBuddy, "codebuddy", "CodeBuddy API"),
    (
        AiProviderKind::CodeBuddyCli,
        "codebuddy-cli",
        "CodeBuddy CLI",
    ),
    (AiProviderKind::OpenAi, "openai", "OpenAI GPT API"),
    (AiProviderKind::CodexCli, "codex-cli", "OpenAI Codex CLI"),
    (AiProviderKind::Gemini, "gemini", "Google Gemini API"),
    (AiProviderKind::GeminiCli, "gemini-cli", "Gemini CLI"),
    (AiProviderKind::Local, "local", "Local LLM (Ollama)"),
];

/// Resolved custom provider with all args computed (template + overrides).
#[derive(Debug, Clone)]
pub struct CustomProviderResolved {
    /// Provider name
    pub name: String,
    /// Whether this is a CLI or API provider
    pub is_cli: bool,
    /// CLI command (for CLI providers)
    pub command: Option<String>,
    /// Args for prompt mode (prompt appended at end)
    pub prompt_args: Vec<String>,
    /// Args for fix mode (prompt appended at end)
    pub fix_args: Vec<String>,
    /// System prompt argument name
    pub system_prompt_arg: Option<String>,
    /// API style for API providers: "openai", "anthropic", "gemini"
    pub api_style: Option<String>,
    /// API endpoint
    pub endpoint: Option<String>,
    /// Model name
    pub model: Option<String>,
    /// API key environment variable
    pub api_key_env: Option<String>,
    /// Fallback provider name
    pub fallback: Option<String>,
}

/// CLI arg templates for common coding agent styles.
struct CliTemplate {
    prompt_args: &'static [&'static str],
    fix_args: &'static [&'static str],
    system_prompt_arg: &'static str,
}

const CLAUDE_LIKE_TEMPLATE: CliTemplate = CliTemplate {
    prompt_args: &["-p", "--output-format", "text"],
    fix_args: &[
        "-p",
        "--output-format",
        "text",
        "--allowedTools",
        "Edit,Read,Grep,Glob",
        "--dangerously-skip-permissions",
        "--settings",
        r#"{"hooks":{}}"#,
        "--",
    ],
    system_prompt_arg: "--system-prompt",
};

const CODEX_LIKE_TEMPLATE: CliTemplate = CliTemplate {
    prompt_args: &["exec", "--dangerously-bypass-approvals-and-sandbox"],
    fix_args: &["exec", "--dangerously-bypass-approvals-and-sandbox"],
    system_prompt_arg: "",
};

const GEMINI_LIKE_TEMPLATE: CliTemplate = CliTemplate {
    prompt_args: &[],
    fix_args: &["-y"],
    system_prompt_arg: "",
};

fn resolve_cli_style(name: &str) -> Option<&'static CliTemplate> {
    match name {
        "claude" => Some(&CLAUDE_LIKE_TEMPLATE),
        "codex" => Some(&CODEX_LIKE_TEMPLATE),
        "gemini" => Some(&GEMINI_LIKE_TEMPLATE),
        _ => None,
    }
}

/// Resolve a custom provider config into a fully computed provider.
pub fn resolve_custom_provider(
    name: &str,
    config: &crate::config::CustomProvider,
) -> Result<CustomProviderResolved, String> {
    let is_cli = config.kind != "api";

    // Start from cli_style defaults if specified
    let template = config
        .cli_style
        .as_deref()
        .and_then(resolve_cli_style);

    let (prompt_args, fix_args, sys_arg) = if let Some(tmpl) = template {
        (
            tmpl.prompt_args.iter().map(|s| s.to_string()).collect(),
            tmpl.fix_args.iter().map(|s| s.to_string()).collect(),
            if tmpl.system_prompt_arg.is_empty() {
                None
            } else {
                Some(tmpl.system_prompt_arg.to_string())
            },
        )
    } else if is_cli {
        // Default: just pass prompt as trailing arg (minimal)
        (vec![], vec![], None)
    } else {
        (vec![], vec![], None)
    };

    // Override with explicit config values
    let prompt_args = config.prompt_args.clone().unwrap_or(prompt_args);
    let fix_args = config.fix_args.clone().unwrap_or(fix_args);
    let system_prompt_arg = config.system_prompt_arg.clone().or(sys_arg);

    if is_cli && config.command.is_none() {
        return Err(format!(
            "Custom CLI provider '{}' requires 'command' field",
            name
        ));
    }

    if !is_cli && config.api_style.is_none() {
        return Err(format!(
            "Custom API provider '{}' requires 'api_style' field (openai, anthropic, or gemini)",
            name
        ));
    }

    Ok(CustomProviderResolved {
        name: name.to_string(),
        is_cli,
        command: config.command.clone(),
        prompt_args,
        fix_args,
        system_prompt_arg,
        api_style: config.api_style.clone(),
        endpoint: config.endpoint.clone(),
        model: config.model.clone(),
        api_key_env: config.api_key_env.clone(),
        fallback: config.fallback.clone(),
    })
}

/// Thread-local storage for the resolved custom provider used by the current operation.
/// This is set before provider operations and read by Custom provider implementations.
use std::cell::RefCell;

thread_local! {
    static CUSTOM_PROVIDER: RefCell<Option<CustomProviderResolved>> = const { RefCell::new(None) };
}

/// Set the active custom provider for the current thread.
pub fn set_custom_provider(provider: Option<CustomProviderResolved>) {
    CUSTOM_PROVIDER.with(|p| {
        *p.borrow_mut() = provider;
    });
}

/// Get a clone of the active custom provider for the current thread.
pub fn get_custom_provider() -> Option<CustomProviderResolved> {
    CUSTOM_PROVIDER.with(|p| p.borrow().clone())
}

/// Check if a single provider is available in the current environment.
pub fn is_provider_available(kind: &AiProviderKind) -> bool {
    match kind {
        AiProviderKind::Claude => {
            env::var("ANTHROPIC_AUTH_TOKEN").is_ok() || env::var("ANTHROPIC_API_KEY").is_ok()
        }
        AiProviderKind::ClaudeCli => is_cli_available("claude"),
        AiProviderKind::CodeBuddy => env::var("CODEBUDDY_API_KEY").is_ok(),
        AiProviderKind::CodeBuddyCli => is_cli_available("codebuddy"),
        AiProviderKind::OpenAi => env::var("OPENAI_API_KEY").is_ok(),
        AiProviderKind::CodexCli => is_cli_available("codex"),
        AiProviderKind::Gemini => {
            env::var("GEMINI_API_KEY").is_ok() || env::var("GOOGLE_API_KEY").is_ok()
        }
        AiProviderKind::GeminiCli => is_cli_available("gemini"),
        AiProviderKind::Local => env::var("LINTHIS_AI_ENDPOINT").is_ok(),
        AiProviderKind::Custom(_) => {
            // Check custom provider availability via thread-local
            if let Some(ref cp) = get_custom_provider() {
                if cp.is_cli {
                    cp.command.as_deref().map(is_cli_available).unwrap_or(false)
                } else {
                    cp.api_key_env
                        .as_ref()
                        .map(|env_name| env::var(env_name).is_ok())
                        .unwrap_or(true)
                }
            } else {
                false
            }
        }
        AiProviderKind::Mock => true,
    }
}

/// Check if a CLI command is available on the system.
fn is_cli_available(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Detect which AI providers are available in the current environment.
///
/// Returns a list of (provider kind, is_available) tuples for all user-facing providers.
/// This is a lightweight check that doesn't require a full AiProvider instance.
pub fn detect_available_providers() -> Vec<(AiProviderKind, bool)> {
    ALL_AI_PROVIDERS
        .iter()
        .map(|(kind, _, _)| (kind.clone(), is_provider_available(kind)))
        .collect()
}

/// Try to find a compatible fallback provider when the requested one is unavailable.
///
/// Returns `Some((fallback_kind, message))` if a fallback is found, `None` otherwise.
/// For built-in providers, API↔CLI pairs within the same family are considered.
/// For custom providers, uses the `fallback` field from config.
pub fn try_fallback_provider(kind: &AiProviderKind) -> Option<(AiProviderKind, String)> {
    // Already available, no fallback needed
    if is_provider_available(kind) {
        return None;
    }

    let (fallback, from_name, to_name, hint) = match kind {
        AiProviderKind::Claude => (
            AiProviderKind::ClaudeCli,
            "claude (API)",
            "claude-cli",
            "Set ANTHROPIC_AUTH_TOKEN or ANTHROPIC_API_KEY to use Claude API directly".to_string(),
        ),
        AiProviderKind::ClaudeCli => (
            AiProviderKind::Claude,
            "claude-cli",
            "claude (API)",
            "Install Claude CLI to use claude-cli provider".to_string(),
        ),
        AiProviderKind::CodeBuddy => (
            AiProviderKind::CodeBuddyCli,
            "codebuddy (API)",
            "codebuddy-cli",
            "Set CODEBUDDY_API_KEY to use CodeBuddy API directly".to_string(),
        ),
        AiProviderKind::CodeBuddyCli => (
            AiProviderKind::CodeBuddy,
            "codebuddy-cli",
            "codebuddy (API)",
            "Install CodeBuddy CLI to use codebuddy-cli provider".to_string(),
        ),
        AiProviderKind::OpenAi => (
            AiProviderKind::CodexCli,
            "openai (API)",
            "codex-cli",
            "Set OPENAI_API_KEY to use OpenAI API directly".to_string(),
        ),
        AiProviderKind::CodexCli => (
            AiProviderKind::OpenAi,
            "codex-cli",
            "openai (API)",
            "Install Codex CLI (npm install -g @openai/codex) to use codex-cli provider".to_string(),
        ),
        AiProviderKind::Gemini => (
            AiProviderKind::GeminiCli,
            "gemini (API)",
            "gemini-cli",
            "Set GEMINI_API_KEY or GOOGLE_API_KEY to use Gemini API directly".to_string(),
        ),
        AiProviderKind::GeminiCli => (
            AiProviderKind::Gemini,
            "gemini-cli",
            "gemini (API)",
            "Install Gemini CLI (npm install -g @google/gemini-cli) to use gemini-cli provider".to_string(),
        ),
        AiProviderKind::Custom(name) => {
            // Check custom provider's fallback field
            if let Some(ref cp) = get_custom_provider() {
                if let Some(ref fb) = cp.fallback {
                    let fallback_kind: AiProviderKind = fb.parse().unwrap_or_default();
                    if is_provider_available(&fallback_kind) {
                        return Some((
                            fallback_kind,
                            format!(
                                "Custom provider '{}' is not available, falling back to {}",
                                name, fb
                            ),
                        ));
                    }
                }
            }
            return None;
        }
        _ => return None,
    };

    if is_provider_available(&fallback) {
        Some((
            fallback,
            format!(
                "Provider {} is not available, falling back to {} ({})",
                from_name, to_name, hint
            ),
        ))
    } else {
        None
    }
}

impl AiProviderKind {
    /// Get the CLI name for this provider kind (e.g., "claude", "claude-cli")
    pub fn cli_name(&self) -> String {
        match self {
            AiProviderKind::Custom(name) => name.clone(),
            _ => ALL_AI_PROVIDERS
                .iter()
                .find(|(k, _, _)| k == self)
                .map(|(_, name, _)| name.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
        }
    }
}

/// Configuration for AI provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProviderConfig {
    /// Provider type
    pub kind: AiProviderKind,
    /// API key (or None for local/mock)
    pub api_key: Option<String>,
    /// API endpoint URL (optional override)
    pub endpoint: Option<String>,
    /// Model name to use
    pub model: String,
    /// Maximum tokens for response
    pub max_tokens: u32,
    /// Temperature for generation
    pub temperature: f32,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for AiProviderConfig {
    fn default() -> Self {
        Self {
            kind: AiProviderKind::Claude,
            api_key: None,
            endpoint: None,
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 2048,
            temperature: 0.3,
            timeout_secs: 60,
        }
    }
}

impl AiProviderConfig {
    /// Create Claude API configuration
    ///
    /// Uses environment variables:
    /// - ANTHROPIC_AUTH_TOKEN or ANTHROPIC_API_KEY for authentication
    /// - ANTHROPIC_BASE_URL for custom endpoint (optional)
    pub fn claude() -> Self {
        Self {
            kind: AiProviderKind::Claude,
            model: "claude-sonnet-4-20250514".to_string(),
            ..Default::default()
        }
    }

    /// Create Claude CLI configuration (uses `claude -p` command)
    pub fn claude_cli() -> Self {
        Self {
            kind: AiProviderKind::ClaudeCli,
            model: "claude-cli".to_string(),
            ..Default::default()
        }
    }

    /// Create CodeBuddy API configuration
    ///
    /// Uses environment variables:
    /// - CODEBUDDY_API_KEY for authentication
    /// - CODEBUDDY_BASE_URL for custom endpoint (optional)
    pub fn codebuddy() -> Self {
        Self {
            kind: AiProviderKind::CodeBuddy,
            model: "deepseek-v3.1".to_string(),
            ..Default::default()
        }
    }

    /// Create CodeBuddy CLI configuration (uses `codebuddy -p` command)
    pub fn codebuddy_cli() -> Self {
        Self {
            kind: AiProviderKind::CodeBuddyCli,
            model: "codebuddy-cli".to_string(),
            ..Default::default()
        }
    }

    /// Create OpenAI configuration
    pub fn openai() -> Self {
        Self {
            kind: AiProviderKind::OpenAi,
            model: "gpt-4o".to_string(),
            ..Default::default()
        }
    }

    /// Create Codex CLI configuration (uses `codex exec` command)
    pub fn codex_cli() -> Self {
        Self {
            kind: AiProviderKind::CodexCli,
            model: "codex-cli".to_string(),
            ..Default::default()
        }
    }

    /// Create Gemini API configuration
    ///
    /// Uses environment variables:
    /// - GEMINI_API_KEY or GOOGLE_API_KEY for authentication
    pub fn gemini() -> Self {
        Self {
            kind: AiProviderKind::Gemini,
            model: "gemini-2.5-flash".to_string(),
            ..Default::default()
        }
    }

    /// Create Gemini CLI configuration (uses `gemini` command)
    pub fn gemini_cli() -> Self {
        Self {
            kind: AiProviderKind::GeminiCli,
            model: "gemini-cli".to_string(),
            ..Default::default()
        }
    }

    /// Create local LLM configuration
    pub fn local() -> Self {
        Self {
            kind: AiProviderKind::Local,
            endpoint: Some("http://localhost:11434".to_string()),
            model: "codellama:7b".to_string(),
            ..Default::default()
        }
    }

    /// Create mock configuration for testing
    pub fn mock() -> Self {
        Self {
            kind: AiProviderKind::Mock,
            model: "mock".to_string(),
            ..Default::default()
        }
    }
}

/// AI Provider trait for different backends
pub trait AiProviderTrait: Send + Sync {
    /// Get provider name
    fn name(&self) -> &str;

    /// Check if provider is available (API key set, service reachable)
    fn is_available(&self) -> bool;

    /// Generate a completion for the given prompt
    fn complete(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String, String>;
}

/// Main AI provider implementation
pub struct AiProvider {
    config: AiProviderConfig,
}

impl AiProvider {
    /// Create a new AI provider with configuration
    pub fn new(config: AiProviderConfig) -> Self {
        Self { config }
    }

    /// Create from environment variables
    ///
    /// Supported environment variables:
    /// - LINTHIS_AI_PROVIDER: claude, claude-cli, openai, local, mock
    /// - ANTHROPIC_AUTH_TOKEN or ANTHROPIC_API_KEY: Anthropic API key
    /// - ANTHROPIC_BASE_URL: Custom Anthropic API endpoint
    /// - OPENAI_API_KEY: OpenAI API key
    /// - LINTHIS_AI_MODEL: Custom model name
    /// - LINTHIS_AI_ENDPOINT: Custom endpoint for local LLM
    pub fn from_env() -> Self {
        let kind = env::var("LINTHIS_AI_PROVIDER")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(AiProviderKind::Claude);

        // Get API key based on provider type
        let api_key = match kind {
            AiProviderKind::Claude => env::var("ANTHROPIC_AUTH_TOKEN")
                .or_else(|_| env::var("ANTHROPIC_API_KEY"))
                .ok(),
            AiProviderKind::CodeBuddy => env::var("CODEBUDDY_API_KEY").ok(),
            AiProviderKind::OpenAi | AiProviderKind::CodexCli => {
                env::var("OPENAI_API_KEY").ok()
            }
            AiProviderKind::Gemini => env::var("GEMINI_API_KEY")
                .or_else(|_| env::var("GOOGLE_API_KEY"))
                .ok(),
            _ => None,
        };

        let model = env::var("LINTHIS_AI_MODEL").unwrap_or_else(|_| {
            match &kind {
                AiProviderKind::Claude => "claude-sonnet-4-20250514".to_string(),
                AiProviderKind::ClaudeCli => "claude-cli".to_string(),
                AiProviderKind::CodeBuddy => "deepseek-v3.1".to_string(),
                AiProviderKind::CodeBuddyCli => "codebuddy-cli".to_string(),
                AiProviderKind::OpenAi => "gpt-4o".to_string(),
                AiProviderKind::CodexCli => "codex-cli".to_string(),
                AiProviderKind::Gemini => "gemini-2.5-flash".to_string(),
                AiProviderKind::GeminiCli => "gemini-cli".to_string(),
                AiProviderKind::Local => "codellama:7b".to_string(),
                AiProviderKind::Custom(name) => name.clone(),
                AiProviderKind::Mock => "mock".to_string(),
            }
        });

        // Get endpoint based on provider type
        let endpoint = match kind {
            AiProviderKind::Claude => env::var("ANTHROPIC_BASE_URL").ok(),
            AiProviderKind::CodeBuddy => env::var("CODEBUDDY_BASE_URL").ok(),
            _ => env::var("LINTHIS_AI_ENDPOINT").ok(),
        };

        Self {
            config: AiProviderConfig {
                kind,
                api_key,
                endpoint,
                model,
                ..Default::default()
            },
        }
    }

    /// Get the provider configuration
    pub fn config(&self) -> &AiProviderConfig {
        &self.config
    }

    /// Check if API key is configured
    pub fn has_api_key(&self) -> bool {
        self.config.api_key.is_some()
    }
}

impl Default for AiProvider {
    fn default() -> Self {
        Self::from_env()
    }
}

impl AiProviderTrait for AiProvider {
    fn name(&self) -> &str {
        match &self.config.kind {
            AiProviderKind::Claude => "Claude API",
            AiProviderKind::ClaudeCli => "Claude CLI",
            AiProviderKind::CodeBuddy => "CodeBuddy API",
            AiProviderKind::CodeBuddyCli => "CodeBuddy CLI",
            AiProviderKind::OpenAi => "OpenAI",
            AiProviderKind::CodexCli => "Codex CLI",
            AiProviderKind::Gemini => "Gemini API",
            AiProviderKind::GeminiCli => "Gemini CLI",
            AiProviderKind::Local => "Local LLM",
            AiProviderKind::Custom(_) => "Custom",
            AiProviderKind::Mock => "Mock",
        }
    }

    fn is_available(&self) -> bool {
        is_provider_available(&self.config.kind)
    }

    fn complete(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String, String> {
        match &self.config.kind {
            AiProviderKind::Claude => self.complete_claude(prompt, system_prompt),
            AiProviderKind::ClaudeCli => self.complete_claude_cli(prompt, system_prompt),
            AiProviderKind::CodeBuddy => self.complete_codebuddy(prompt, system_prompt),
            AiProviderKind::CodeBuddyCli => self.complete_codebuddy_cli(prompt, system_prompt),
            AiProviderKind::OpenAi => self.complete_openai(prompt, system_prompt),
            AiProviderKind::CodexCli => self.complete_codex_cli(prompt, system_prompt),
            AiProviderKind::Gemini => self.complete_gemini(prompt, system_prompt),
            AiProviderKind::GeminiCli => self.complete_gemini_cli(prompt, system_prompt),
            AiProviderKind::Local => self.complete_local(prompt, system_prompt),
            AiProviderKind::Custom(_) => self.complete_custom(prompt, system_prompt),
            AiProviderKind::Mock => self.complete_mock(prompt, system_prompt),
        }
    }
}

impl AiProvider {
    fn complete_claude(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String, String> {
        // Try ANTHROPIC_AUTH_TOKEN first, then ANTHROPIC_API_KEY, then config
        let api_key = env::var("ANTHROPIC_AUTH_TOKEN")
            .or_else(|_| env::var("ANTHROPIC_API_KEY"))
            .ok()
            .or_else(|| self.config.api_key.clone())
            .ok_or_else(|| "Anthropic API key not set. Set ANTHROPIC_AUTH_TOKEN or ANTHROPIC_API_KEY environment variable.".to_string())?;

        // Try ANTHROPIC_BASE_URL first, then config endpoint
        let base_url = env::var("ANTHROPIC_BASE_URL")
            .ok()
            .or_else(|| self.config.endpoint.clone());

        let endpoint = if let Some(base) = base_url {
            // Ensure the URL ends with /v1/messages
            let base = base.trim_end_matches('/');
            if base.ends_with("/v1/messages") {
                base.to_string()
            } else if base.ends_with("/v1") {
                format!("{}/messages", base)
            } else {
                format!("{}/v1/messages", base)
            }
        } else {
            "https://api.anthropic.com/v1/messages".to_string()
        };

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(self.config.timeout_secs))
            .build()
            .map_err(|e| e.to_string())?;

        let messages = vec![
            serde_json::json!({
                "role": "user",
                "content": prompt
            })
        ];

        let mut body = serde_json::json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "messages": messages
        });

        if let Some(sys) = system_prompt {
            body["system"] = serde_json::json!(sys);
        }

        let response = client
            .post(&endpoint)
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().unwrap_or_default();
            return Err(format!("API error ({}): {}", status, text));
        }

        let result: serde_json::Value = response.json()
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        result["content"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "No content in response".to_string())
    }

    fn complete_claude_cli(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String, String> {
        use std::process::{Command, Stdio};

        // Build the command with optional system prompt
        let mut cmd = Command::new("claude");
        cmd.arg("-p")
            .arg("--output-format")
            .arg("text");

        // Add system prompt if provided
        if let Some(sys) = system_prompt {
            cmd.arg("--system-prompt").arg(sys);
        }

        // Add the main prompt
        cmd.arg(prompt);

        // Configure I/O
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn claude command: {}", e))?;

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for claude command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Claude CLI error: {}", stderr));
        }

        let response = String::from_utf8_lossy(&output.stdout).to_string();

        if response.trim().is_empty() {
            return Err("Empty response from Claude CLI".to_string());
        }

        Ok(response)
    }

    /// Fix a file directly using Claude CLI with Edit tool
    /// Returns the diff of changes made
    pub fn fix_file_with_cli(
        &self,
        file_path: &std::path::Path,
        issues: &[(usize, String, String)], // (line, message, code)
    ) -> Result<String, String> {
        use std::process::Stdio;

        // Only works with CLI providers
        if !matches!(self.config.kind, AiProviderKind::ClaudeCli | AiProviderKind::CodeBuddyCli | AiProviderKind::CodexCli | AiProviderKind::GeminiCli | AiProviderKind::Custom(_)) {
            return Err("fix_file_with_cli only works with CLI providers".to_string());
        }

        // Read original content for backup
        let original_content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Build issue descriptions
        let issues_desc: Vec<String> = issues
            .iter()
            .map(|(line, msg, code)| format!("- Line {}: {} ({})", line, msg, code))
            .collect();

        // Build prompt for direct file editing
        let prompt = format!(
            r#"Fix the following lint issues in file "{}":

{}

IMPORTANT:
- Use the Edit tool to fix each issue directly in the file
- Make MINIMAL changes - only fix what each error describes
- Preserve all formatting and indentation

CRITICAL FOR C/C++ INTERFACE CHANGES:
- If you change a function/method signature (parameters, return type, qualifiers):
  1. First use Read or Grep tools to find the function declaration (in .h/.hpp files)
  2. Also find the function definition/implementation (in .cpp files)
  3. Search for all callers of this function across the project
  4. Update ALL of these locations to match the new signature
  5. Verify parameter passing matches (e.g., if changing & to *, callers should pass & or change to pointer)
- For reference-to-pointer changes (e.g., AClass &a → AClass *a):
  * In header: update declaration
  * In implementation: update definition AND all uses of the parameter
  * In callers: change from passing object to passing &object (or adjust pointer usage)
- Only after updating all related locations, respond with "Done"

If you're unsure about related locations, use Grep to search for the function name first."#,
            file_path.display(),
            issues_desc.join("\n")
        );

        // Run CLI with Edit tool allowed
        let provider_name = self.config.kind.cli_name();
        let mut cmd = build_cli_fix_command(&self.config.kind, &prompt);

        // Set working directory to file's parent
        if let Some(parent) = file_path.parent() {
            cmd.current_dir(parent);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn {} command: {}", provider_name, e))?;

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for {} command: {}", provider_name, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Restore original file on error
            let _ = std::fs::write(file_path, &original_content);
            return Err(format!("{} CLI error: {}", provider_name, stderr));
        }

        // Read modified content
        let modified_content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read modified file: {}", e))?;

        // Generate diff
        let diff = generate_unified_diff(&original_content, &modified_content, file_path);

        // Store original for potential restore
        // (caller can restore if needed)

        Ok(diff)
    }

    /// Fix multiple files in a single CLI invocation (batch mode).
    /// Returns a map of file_path -> diff for each file that was modified.
    pub fn fix_files_batch_with_cli(
        &self,
        files: &[(&std::path::Path, &[(usize, String, String)])],
        working_dir: &std::path::Path,
    ) -> Result<std::collections::HashMap<std::path::PathBuf, String>, String> {
        use std::process::Stdio;

        if !matches!(self.config.kind, AiProviderKind::ClaudeCli | AiProviderKind::CodeBuddyCli | AiProviderKind::CodexCli | AiProviderKind::GeminiCli | AiProviderKind::Custom(_)) {
            return Err("fix_files_batch_with_cli only works with CLI providers".to_string());
        }

        // Read and backup original contents
        let mut originals: Vec<(std::path::PathBuf, String)> = Vec::new();
        for (file_path, _) in files {
            let content = std::fs::read_to_string(file_path)
                .map_err(|e| format!("Failed to read {}: {}", file_path.display(), e))?;
            originals.push((file_path.to_path_buf(), content));
        }

        // Build prompt with all files and their issues
        let mut file_sections = Vec::new();
        for (file_path, issues) in files {
            let issues_desc: Vec<String> = issues
                .iter()
                .map(|(line, msg, code)| format!("  - Line {}: {} ({})", line, msg, code))
                .collect();
            file_sections.push(format!(
                "File \"{}\":\n{}",
                file_path.display(),
                issues_desc.join("\n")
            ));
        }

        let prompt = format!(
            r#"Fix the following lint issues in multiple files:

{}

IMPORTANT:
- Use the Edit tool to fix each issue directly in each file
- Make MINIMAL changes - only fix what each error describes
- Preserve all formatting and indentation
- Process ALL files listed above

CRITICAL FOR C/C++ INTERFACE CHANGES:
- If you change a function/method signature (parameters, return type, qualifiers):
  1. Use Read/Grep tools to find ALL related locations:
     - Function declaration (in .h/.hpp header files)
     - Function definition/implementation (in .cpp source files)
     - All call sites across the entire project
  2. Update ALL of these locations consistently
  3. For reference-to-pointer changes (e.g., AClass &a → AClass *a):
     * Update header declaration
     * Update source file definition AND parameter usage inside the function
     * Update ALL callers to pass compatible arguments (e.g., &obj instead of obj)
  4. Verify the change won't break compilation by checking ALL usages

Example workflow for signature change:
1. Grep for function name to find all occurrences
2. Read each file to understand context
3. Edit declaration, definition, AND all call sites
4. Only after all related edits are complete, respond with "Done"

If unsure about impact, use Grep extensively to find all references first."#,
            file_sections.join("\n\n")
        );

        let provider_name = self.config.kind.cli_name();
        let mut cmd = build_cli_fix_command(&self.config.kind, &prompt);
        cmd.current_dir(working_dir);

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn {} command: {}", provider_name, e))?;

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for {} command: {}", provider_name, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Restore all originals on error
            for (path, content) in &originals {
                let _ = std::fs::write(path, content);
            }
            return Err(format!("{} CLI error: {}", provider_name, stderr));
        }

        // Compare each file to generate diffs
        let mut diffs = std::collections::HashMap::new();
        for (path, original) in &originals {
            if let Ok(modified) = std::fs::read_to_string(path) {
                let diff = generate_unified_diff(original, &modified, path);
                if !diff.is_empty() {
                    diffs.insert(path.clone(), diff);
                }
            }
        }

        Ok(diffs)
    }

    fn complete_codebuddy(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String, String> {
        // Try CODEBUDDY_API_KEY from env, then config
        let api_key = env::var("CODEBUDDY_API_KEY")
            .ok()
            .or_else(|| self.config.api_key.clone())
            .ok_or_else(|| "CodeBuddy API key not set. Set CODEBUDDY_API_KEY environment variable.".to_string())?;

        // Try CODEBUDDY_BASE_URL first, then config endpoint
        let base_url = env::var("CODEBUDDY_BASE_URL")
            .ok()
            .or_else(|| self.config.endpoint.clone());

        let endpoint = if let Some(base) = base_url {
            // Ensure the URL ends with /v1/chat/completions
            let base = base.trim_end_matches('/');
            if base.ends_with("/v1/chat/completions") {
                base.to_string()
            } else if base.ends_with("/v1") {
                format!("{}/chat/completions", base)
            } else {
                format!("{}/v1/chat/completions", base)
            }
        } else {
            // Default CodeBuddy API endpoint (OpenAI-compatible)
            "https://api.codebuddy.cn/v1/chat/completions".to_string()
        };

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(self.config.timeout_secs))
            .build()
            .map_err(|e| e.to_string())?;

        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": sys
            }));
        }
        messages.push(serde_json::json!({
            "role": "user",
            "content": prompt
        }));

        let body = serde_json::json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "messages": messages
        });

        let response = client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().unwrap_or_default();
            return Err(format!("API error ({}): {}", status, text));
        }

        let result: serde_json::Value = response.json()
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        result["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "No content in response".to_string())
    }

    fn complete_codebuddy_cli(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String, String> {
        use std::process::{Command, Stdio};

        // Build the command with optional system prompt
        let mut cmd = Command::new("codebuddy");
        cmd.arg("-p")
            .arg("--output-format")
            .arg("text");

        // Add system prompt if provided
        if let Some(sys) = system_prompt {
            cmd.arg("--append-system-prompt").arg(sys);
        }

        // Add the main prompt
        cmd.arg(prompt);

        // Configure I/O
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn codebuddy command: {}", e))?;

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for codebuddy command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("CodeBuddy CLI error: {}", stderr));
        }

        let response = String::from_utf8_lossy(&output.stdout).to_string();

        if response.trim().is_empty() {
            return Err("Empty response from CodeBuddy CLI".to_string());
        }

        Ok(response)
    }

    fn complete_openai(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String, String> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| "OpenAI API key not set. Set OPENAI_API_KEY environment variable.".to_string())?;

        let endpoint = self.config.endpoint.as_deref()
            .unwrap_or("https://api.openai.com/v1/chat/completions");

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(self.config.timeout_secs))
            .build()
            .map_err(|e| e.to_string())?;

        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": sys
            }));
        }
        messages.push(serde_json::json!({
            "role": "user",
            "content": prompt
        }));

        let body = serde_json::json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "messages": messages
        });

        let response = client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().unwrap_or_default();
            return Err(format!("API error ({}): {}", status, text));
        }

        let result: serde_json::Value = response.json()
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        result["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "No content in response".to_string())
    }

    fn complete_codex_cli(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String, String> {
        use std::process::{Command, Stdio};

        let mut cmd = Command::new("codex");
        cmd.arg("exec")
            .arg("--dangerously-bypass-approvals-and-sandbox");

        // Add the main prompt (with system prompt prepended if provided)
        let full_prompt = if let Some(sys) = system_prompt {
            format!("{}\n\n{}", sys, prompt)
        } else {
            prompt.to_string()
        };
        cmd.arg(&full_prompt);

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn codex command: {}", e))?;

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for codex command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Codex CLI error: {}", stderr));
        }

        let response = String::from_utf8_lossy(&output.stdout).to_string();

        if response.trim().is_empty() {
            return Err("Empty response from Codex CLI".to_string());
        }

        Ok(response)
    }

    fn complete_gemini(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String, String> {
        let api_key = env::var("GEMINI_API_KEY")
            .or_else(|_| env::var("GOOGLE_API_KEY"))
            .ok()
            .or_else(|| self.config.api_key.clone())
            .ok_or_else(|| "Gemini API key not set. Set GEMINI_API_KEY or GOOGLE_API_KEY environment variable.".to_string())?;

        let endpoint = self.config.endpoint.as_deref()
            .unwrap_or("https://generativelanguage.googleapis.com");
        let endpoint = endpoint.trim_end_matches('/');

        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            endpoint, self.config.model, api_key
        );

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(self.config.timeout_secs))
            .build()
            .map_err(|e| e.to_string())?;

        let mut body = serde_json::json!({
            "contents": [{
                "parts": [{"text": prompt}]
            }],
            "generationConfig": {
                "maxOutputTokens": self.config.max_tokens,
                "temperature": self.config.temperature
            }
        });

        if let Some(sys) = system_prompt {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{"text": sys}]
            });
        }

        let response = client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().unwrap_or_default();
            return Err(format!("API error ({}): {}", status, text));
        }

        let result: serde_json::Value = response.json()
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        result["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "No content in response".to_string())
    }

    fn complete_gemini_cli(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String, String> {
        use std::process::{Command, Stdio};

        let mut cmd = Command::new("gemini");

        // Gemini CLI uses positional prompt for non-interactive mode
        // Prepend system prompt if provided
        let full_prompt = if let Some(sys) = system_prompt {
            format!("{}\n\n{}", sys, prompt)
        } else {
            prompt.to_string()
        };
        cmd.arg(&full_prompt);

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn gemini command: {}", e))?;

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for gemini command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Gemini CLI error: {}", stderr));
        }

        let response = String::from_utf8_lossy(&output.stdout).to_string();

        if response.trim().is_empty() {
            return Err("Empty response from Gemini CLI".to_string());
        }

        Ok(response)
    }

    fn complete_local(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String, String> {
        let endpoint = self.config.endpoint.as_deref()
            .ok_or_else(|| "Local LLM endpoint not set. Set LINTHIS_AI_ENDPOINT environment variable.".to_string())?;

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(self.config.timeout_secs))
            .build()
            .map_err(|e| e.to_string())?;

        // Ollama-compatible API
        let full_prompt = if let Some(sys) = system_prompt {
            format!("{}\n\n{}", sys, prompt)
        } else {
            prompt.to_string()
        };

        let body = serde_json::json!({
            "model": self.config.model,
            "prompt": full_prompt,
            "stream": false,
            "options": {
                "temperature": self.config.temperature,
                "num_predict": self.config.max_tokens
            }
        });

        let url = format!("{}/api/generate", endpoint.trim_end_matches('/'));

        let response = client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().unwrap_or_default();
            return Err(format!("API error ({}): {}", status, text));
        }

        let result: serde_json::Value = response.json()
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        result["response"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "No response in output".to_string())
    }

    fn complete_custom(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String, String> {
        let cp = get_custom_provider()
            .ok_or_else(|| "Custom provider not configured. Define it in [ai.custom_providers] config.".to_string())?;

        if cp.is_cli {
            // CLI custom provider: spawn command with prompt_args + prompt
            use std::process::{Command, Stdio};

            let cmd_name = cp.command.as_deref().unwrap_or("echo");
            let mut cmd = Command::new(cmd_name);

            for arg in &cp.prompt_args {
                cmd.arg(arg);
            }

            // Add system prompt if supported and provided
            if let (Some(ref sys_arg), Some(sys)) = (&cp.system_prompt_arg, system_prompt) {
                if !sys_arg.is_empty() {
                    cmd.arg(sys_arg).arg(sys);
                }
            }

            cmd.arg(prompt);
            cmd.stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let child = cmd.spawn()
                .map_err(|e| format!("Failed to spawn '{}': {}", cmd_name, e))?;
            let output = child.wait_with_output()
                .map_err(|e| format!("Failed to wait for '{}': {}", cmd_name, e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("{} error: {}", cmd_name, stderr));
            }

            let response = String::from_utf8_lossy(&output.stdout).to_string();
            if response.trim().is_empty() {
                return Err(format!("Empty response from {}", cmd_name));
            }
            Ok(response)
        } else {
            // API custom provider: dispatch based on api_style
            let api_key = cp.api_key_env.as_ref()
                .and_then(|env_name| env::var(env_name).ok())
                .or_else(|| self.config.api_key.clone())
                .ok_or_else(|| format!("API key not set for custom provider '{}'", cp.name))?;

            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(self.config.timeout_secs))
                .build()
                .map_err(|e| e.to_string())?;

            match cp.api_style.as_deref() {
                Some("openai") => {
                    let endpoint = cp.endpoint.as_deref()
                        .unwrap_or("https://api.openai.com/v1/chat/completions");
                    let model = cp.model.as_deref().unwrap_or("gpt-4");

                    let mut messages = Vec::new();
                    if let Some(sys) = system_prompt {
                        messages.push(serde_json::json!({"role": "system", "content": sys}));
                    }
                    messages.push(serde_json::json!({"role": "user", "content": prompt}));

                    let body = serde_json::json!({
                        "model": model,
                        "max_tokens": self.config.max_tokens,
                        "temperature": self.config.temperature,
                        "messages": messages
                    });

                    let response = client.post(endpoint)
                        .header("Authorization", format!("Bearer {}", api_key))
                        .header("content-type", "application/json")
                        .json(&body).send()
                        .map_err(|e| format!("Request failed: {}", e))?;

                    if !response.status().is_success() {
                        let status = response.status();
                        let text = response.text().unwrap_or_default();
                        return Err(format!("API error ({}): {}", status, text));
                    }

                    let result: serde_json::Value = response.json()
                        .map_err(|e| format!("Failed to parse response: {}", e))?;
                    result["choices"][0]["message"]["content"].as_str()
                        .map(|s| s.to_string())
                        .ok_or_else(|| "No content in response".to_string())
                }
                Some("anthropic") => {
                    let endpoint = cp.endpoint.as_deref()
                        .unwrap_or("https://api.anthropic.com/v1/messages");
                    let model = cp.model.as_deref().unwrap_or("claude-sonnet-4-20250514");

                    let mut body = serde_json::json!({
                        "model": model,
                        "max_tokens": self.config.max_tokens,
                        "messages": [{"role": "user", "content": prompt}]
                    });
                    if let Some(sys) = system_prompt {
                        body["system"] = serde_json::json!(sys);
                    }

                    let response = client.post(endpoint)
                        .header("x-api-key", &api_key)
                        .header("anthropic-version", "2023-06-01")
                        .header("content-type", "application/json")
                        .json(&body).send()
                        .map_err(|e| format!("Request failed: {}", e))?;

                    if !response.status().is_success() {
                        let status = response.status();
                        let text = response.text().unwrap_or_default();
                        return Err(format!("API error ({}): {}", status, text));
                    }

                    let result: serde_json::Value = response.json()
                        .map_err(|e| format!("Failed to parse response: {}", e))?;
                    result["content"][0]["text"].as_str()
                        .map(|s| s.to_string())
                        .ok_or_else(|| "No content in response".to_string())
                }
                Some("gemini") => {
                    let model = cp.model.as_deref().unwrap_or("gemini-2.0-flash");
                    let endpoint = cp.endpoint.clone()
                        .unwrap_or_else(|| format!(
                            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
                            model
                        ));
                    let url = format!("{}?key={}", endpoint, api_key);

                    let mut body = serde_json::json!({
                        "contents": [{"parts": [{"text": prompt}]}],
                        "generationConfig": {
                            "maxOutputTokens": self.config.max_tokens,
                            "temperature": self.config.temperature
                        }
                    });
                    if let Some(sys) = system_prompt {
                        body["systemInstruction"] = serde_json::json!({"parts": [{"text": sys}]});
                    }

                    let response = client.post(&url)
                        .header("content-type", "application/json")
                        .json(&body).send()
                        .map_err(|e| format!("Request failed: {}", e))?;

                    if !response.status().is_success() {
                        let status = response.status();
                        let text = response.text().unwrap_or_default();
                        return Err(format!("API error ({}): {}", status, text));
                    }

                    let result: serde_json::Value = response.json()
                        .map_err(|e| format!("Failed to parse response: {}", e))?;
                    result["candidates"][0]["content"]["parts"][0]["text"].as_str()
                        .map(|s| s.to_string())
                        .ok_or_else(|| "No content in response".to_string())
                }
                Some(other) => Err(format!("Unknown api_style '{}' for custom provider '{}'", other, cp.name)),
                None => Err(format!("Custom API provider '{}' requires 'api_style'", cp.name)),
            }
        }
    }

    fn complete_mock(&self, prompt: &str, _system_prompt: Option<&str>) -> Result<String, String> {
        // Extract line number from prompt if available
        let line_num = if let Some(pos) = prompt.find("Line: ") {
            let rest = &prompt[pos + 6..];
            rest.lines()
                .next()
                .and_then(|s| s.trim().parse::<usize>().ok())
                .unwrap_or(1)
        } else {
            1
        };

        // Return a mock diff response for testing
        Ok(format!(
            r#"Here's the fix:

```diff
@@ -{line},1 +{line},1 @@
-    original_line
+    fixed_line  // mock fix
```

Note: This is a mock AI response for testing."#,
            line = line_num
        ))
    }
}

/// Build a CLI command for fixing files, tailored to the provider kind.
fn build_cli_fix_command(kind: &AiProviderKind, prompt: &str) -> Command {
    match kind {
        AiProviderKind::ClaudeCli => {
            let mut cmd = Command::new("claude");
            cmd.arg("-p")
                .arg("--output-format")
                .arg("text")
                .arg("--allowedTools")
                .arg("Edit,Read,Grep,Glob")
                .arg("--dangerously-skip-permissions")
                .arg("--settings")
                .arg(r#"{"hooks":{}}"#)
                .arg("--")
                .arg(prompt);
            cmd
        }
        AiProviderKind::CodeBuddyCli => {
            let mut cmd = Command::new("codebuddy");
            cmd.arg("-p")
                .arg("--output-format")
                .arg("text")
                .arg("--allowedTools")
                .arg("Edit,Read,Grep,Glob")
                .arg("--dangerously-skip-permissions")
                .arg("--settings")
                .arg(r#"{"hooks":{}}"#)
                .arg("--")
                .arg(prompt);
            cmd
        }
        AiProviderKind::CodexCli => {
            let mut cmd = Command::new("codex");
            cmd.arg("exec")
                .arg("--dangerously-bypass-approvals-and-sandbox")
                .arg(prompt);
            cmd
        }
        AiProviderKind::GeminiCli => {
            let mut cmd = Command::new("gemini");
            cmd.arg("-y") // YOLO mode: auto-accept all actions
                .arg(prompt);
            cmd
        }
        AiProviderKind::Custom(_) => {
            if let Some(cp) = get_custom_provider() {
                let cmd_name = cp.command.as_deref().unwrap_or("echo");
                let mut cmd = Command::new(cmd_name);
                for arg in &cp.fix_args {
                    cmd.arg(arg);
                }
                cmd.arg(prompt);
                cmd
            } else {
                panic!("Custom provider not set before build_cli_fix_command");
            }
        }
        _ => unreachable!("build_cli_fix_command called with non-CLI provider"),
    }
}

/// Generate unified diff between two strings
fn generate_unified_diff(original: &str, modified: &str, file_path: &std::path::Path) -> String {
    use similar::{ChangeTag, TextDiff};
    use std::fmt::Write;

    if original == modified {
        return String::new();
    }

    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");

    let text_diff = TextDiff::from_lines(original, modified);
    let mut diff = String::new();

    writeln!(diff, "--- a/{}", file_name).ok();
    writeln!(diff, "+++ b/{}", file_name).ok();

    // Use unified diff hunks with 3 lines of context
    for hunk in text_diff.unified_diff().context_radius(3).iter_hunks() {
        writeln!(diff, "{}", hunk.header()).ok();
        for change in hunk.iter_changes() {
            let sign = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            // Write the change line, ensuring it ends with newline
            let value = change.value();
            if value.ends_with('\n') {
                write!(diff, "{}{}", sign, value).ok();
            } else {
                writeln!(diff, "{}{}", sign, value).ok();
            }
        }
    }

    diff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_kind_parsing() {
        assert_eq!("claude".parse::<AiProviderKind>().unwrap(), AiProviderKind::Claude);
        assert_eq!("claude-cli".parse::<AiProviderKind>().unwrap(), AiProviderKind::ClaudeCli);
        assert_eq!("codebuddy".parse::<AiProviderKind>().unwrap(), AiProviderKind::CodeBuddy);
        assert_eq!("codebuddy-cli".parse::<AiProviderKind>().unwrap(), AiProviderKind::CodeBuddyCli);
        assert_eq!("cli".parse::<AiProviderKind>().unwrap(), AiProviderKind::CodeBuddyCli);
        assert_eq!("openai".parse::<AiProviderKind>().unwrap(), AiProviderKind::OpenAi);
        assert_eq!("codex-cli".parse::<AiProviderKind>().unwrap(), AiProviderKind::CodexCli);
        assert_eq!("codex".parse::<AiProviderKind>().unwrap(), AiProviderKind::CodexCli);
        assert_eq!("gemini".parse::<AiProviderKind>().unwrap(), AiProviderKind::Gemini);
        assert_eq!("google".parse::<AiProviderKind>().unwrap(), AiProviderKind::Gemini);
        assert_eq!("gemini-cli".parse::<AiProviderKind>().unwrap(), AiProviderKind::GeminiCli);
        assert_eq!("local".parse::<AiProviderKind>().unwrap(), AiProviderKind::Local);
        assert_eq!("mock".parse::<AiProviderKind>().unwrap(), AiProviderKind::Mock);
    }

    #[test]
    fn test_provider_config_defaults() {
        let config = AiProviderConfig::default();
        assert_eq!(config.kind, AiProviderKind::Claude);
        assert_eq!(config.max_tokens, 2048);
    }

    #[test]
    fn test_mock_provider() {
        let provider = AiProvider::new(AiProviderConfig::mock());
        assert!(provider.is_available());

        let result = provider.complete("test prompt", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mock_provider_available() {
        // Mock is always available
        assert!(is_provider_available(&AiProviderKind::Mock));
    }

    #[test]
    fn test_mock_no_fallback_needed() {
        // Mock is always available, so no fallback needed
        assert!(try_fallback_provider(&AiProviderKind::Mock).is_none());
    }

    #[test]
    fn test_cli_name() {
        assert_eq!(AiProviderKind::Claude.cli_name(), "claude");
        assert_eq!(AiProviderKind::ClaudeCli.cli_name(), "claude-cli");
        assert_eq!(AiProviderKind::CodeBuddy.cli_name(), "codebuddy");
        assert_eq!(AiProviderKind::CodeBuddyCli.cli_name(), "codebuddy-cli");
        assert_eq!(AiProviderKind::OpenAi.cli_name(), "openai");
        assert_eq!(AiProviderKind::CodexCli.cli_name(), "codex-cli");
        assert_eq!(AiProviderKind::Gemini.cli_name(), "gemini");
        assert_eq!(AiProviderKind::GeminiCli.cli_name(), "gemini-cli");
        assert_eq!(AiProviderKind::Local.cli_name(), "local");
    }

    #[test]
    fn test_provider_name() {
        let claude = AiProvider::new(AiProviderConfig::claude());
        assert_eq!(claude.name(), "Claude API");

        let claude_cli = AiProvider::new(AiProviderConfig::claude_cli());
        assert_eq!(claude_cli.name(), "Claude CLI");

        let codebuddy = AiProvider::new(AiProviderConfig::codebuddy());
        assert_eq!(codebuddy.name(), "CodeBuddy API");

        let codebuddy_cli = AiProvider::new(AiProviderConfig::codebuddy_cli());
        assert_eq!(codebuddy_cli.name(), "CodeBuddy CLI");

        let openai = AiProvider::new(AiProviderConfig::openai());
        assert_eq!(openai.name(), "OpenAI");

        let codex_cli = AiProvider::new(AiProviderConfig::codex_cli());
        assert_eq!(codex_cli.name(), "Codex CLI");

        let gemini = AiProvider::new(AiProviderConfig::gemini());
        assert_eq!(gemini.name(), "Gemini API");

        let gemini_cli = AiProvider::new(AiProviderConfig::gemini_cli());
        assert_eq!(gemini_cli.name(), "Gemini CLI");
    }
}
