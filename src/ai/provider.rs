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

/// Supported AI provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
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
    /// OpenAI GPT
    OpenAi,
    /// Local LLM (Ollama, llama.cpp, etc.)
    Local,
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
            "local" | "ollama" | "llama" => Ok(Self::Local),
            "mock" | "test" => Ok(Self::Mock),
            _ => Err(format!("Unknown AI provider: {}", s)),
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
            AiProviderKind::OpenAi => env::var("OPENAI_API_KEY").ok(),
            _ => None,
        };

        let model = env::var("LINTHIS_AI_MODEL").unwrap_or_else(|_| {
            match kind {
                AiProviderKind::Claude => "claude-sonnet-4-20250514".to_string(),
                AiProviderKind::ClaudeCli => "claude-cli".to_string(),
                AiProviderKind::CodeBuddy => "deepseek-v3.1".to_string(),
                AiProviderKind::CodeBuddyCli => "codebuddy-cli".to_string(),
                AiProviderKind::OpenAi => "gpt-4o".to_string(),
                AiProviderKind::Local => "codellama:7b".to_string(),
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
        match self.config.kind {
            AiProviderKind::Claude => "Claude API",
            AiProviderKind::ClaudeCli => "Claude CLI",
            AiProviderKind::CodeBuddy => "CodeBuddy API",
            AiProviderKind::CodeBuddyCli => "CodeBuddy CLI",
            AiProviderKind::OpenAi => "OpenAI",
            AiProviderKind::Local => "Local LLM",
            AiProviderKind::Mock => "Mock",
        }
    }

    fn is_available(&self) -> bool {
        match self.config.kind {
            AiProviderKind::Claude | AiProviderKind::CodeBuddy | AiProviderKind::OpenAi => {
                self.config.api_key.is_some()
            }
            AiProviderKind::ClaudeCli => {
                // Check if claude CLI is available
                std::process::Command::new("claude")
                    .arg("--version")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            }
            AiProviderKind::CodeBuddyCli => {
                // Check if codebuddy CLI is available
                std::process::Command::new("codebuddy")
                    .arg("--version")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            }
            AiProviderKind::Local => {
                // Check if local endpoint is reachable
                if let Some(ref endpoint) = self.config.endpoint {
                    // Simple check - in production, would ping the endpoint
                    !endpoint.is_empty()
                } else {
                    false
                }
            }
            AiProviderKind::Mock => true,
        }
    }

    fn complete(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String, String> {
        match self.config.kind {
            AiProviderKind::Claude => self.complete_claude(prompt, system_prompt),
            AiProviderKind::ClaudeCli => self.complete_claude_cli(prompt, system_prompt),
            AiProviderKind::CodeBuddy => self.complete_codebuddy(prompt, system_prompt),
            AiProviderKind::CodeBuddyCli => self.complete_codebuddy_cli(prompt, system_prompt),
            AiProviderKind::OpenAi => self.complete_openai(prompt, system_prompt),
            AiProviderKind::Local => self.complete_local(prompt, system_prompt),
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
        use std::process::{Command, Stdio};

        // Only works with CLI providers
        if !matches!(self.config.kind, AiProviderKind::ClaudeCli | AiProviderKind::CodeBuddyCli) {
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
- Do NOT add, remove, or modify any other code
- Preserve all formatting and indentation
- After fixing, respond with "Done" or describe what you fixed"#,
            file_path.display(),
            issues_desc.join("\n")
        );

        // Run CLI with Edit tool allowed
        let cli_cmd = match self.config.kind {
            AiProviderKind::ClaudeCli => "claude",
            AiProviderKind::CodeBuddyCli => "codebuddy",
            _ => unreachable!(),
        };

        let mut cmd = Command::new(cli_cmd);
        cmd.arg("-p")
            .arg("--output-format")
            .arg("text")
            .arg("--allowedTools")
            .arg("Edit,Read")
            .arg("--dangerously-skip-permissions")  // Auto-accept tool permissions
            .arg("--")  // Separate options from prompt
            .arg(&prompt);

        // Set working directory to file's parent
        if let Some(parent) = file_path.parent() {
            cmd.current_dir(parent);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn {} command: {}", cli_cmd, e))?;

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for {} command: {}", cli_cmd, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Restore original file on error
            let _ = std::fs::write(file_path, &original_content);
            return Err(format!("{} CLI error: {}", cli_cmd, stderr));
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

/// Generate unified diff between two strings
fn generate_unified_diff(original: &str, modified: &str, file_path: &std::path::Path) -> String {
    use std::fmt::Write;

    let original_lines: Vec<&str> = original.lines().collect();
    let modified_lines: Vec<&str> = modified.lines().collect();

    if original_lines == modified_lines {
        return String::new();
    }

    let mut diff = String::new();
    let file_name = file_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");

    writeln!(diff, "--- a/{}", file_name).ok();
    writeln!(diff, "+++ b/{}", file_name).ok();

    // Simple line-by-line diff
    let mut i = 0;
    let mut j = 0;

    while i < original_lines.len() || j < modified_lines.len() {
        if i < original_lines.len() && j < modified_lines.len() {
            if original_lines[i] == modified_lines[j] {
                i += 1;
                j += 1;
                continue;
            }
        }

        // Find the extent of the change
        let start_i = i;
        let start_j = j;

        // Skip different lines
        while i < original_lines.len() && j < modified_lines.len() && original_lines[i] != modified_lines[j] {
            i += 1;
            j += 1;
        }

        // Handle remaining lines
        if i >= original_lines.len() && j < modified_lines.len() {
            while j < modified_lines.len() {
                j += 1;
            }
        } else if j >= modified_lines.len() && i < original_lines.len() {
            while i < original_lines.len() {
                i += 1;
            }
        }

        // Output hunk
        let old_count = i - start_i;
        let new_count = j - start_j;

        writeln!(diff, "@@ -{},{} +{},{} @@", start_i + 1, old_count.max(1), start_j + 1, new_count.max(1)).ok();

        for k in start_i..i {
            if k < original_lines.len() {
                writeln!(diff, "-{}", original_lines[k]).ok();
            }
        }
        for k in start_j..j {
            if k < modified_lines.len() {
                writeln!(diff, "+{}", modified_lines[k]).ok();
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
    }
}
