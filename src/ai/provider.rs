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
    /// Anthropic Claude
    #[default]
    Claude,
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
    /// Create Claude configuration
    pub fn claude() -> Self {
        Self {
            kind: AiProviderKind::Claude,
            model: "claude-3-5-sonnet-20241022".to_string(),
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
    pub fn from_env() -> Self {
        let kind = env::var("LINTHIS_AI_PROVIDER")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(AiProviderKind::Claude);

        let api_key = match kind {
            AiProviderKind::Claude => env::var("ANTHROPIC_API_KEY").ok(),
            AiProviderKind::OpenAi => env::var("OPENAI_API_KEY").ok(),
            _ => None,
        };

        let model = env::var("LINTHIS_AI_MODEL").unwrap_or_else(|_| {
            match kind {
                AiProviderKind::Claude => "claude-3-5-sonnet-20241022".to_string(),
                AiProviderKind::OpenAi => "gpt-4o".to_string(),
                AiProviderKind::Local => "codellama:7b".to_string(),
                AiProviderKind::Mock => "mock".to_string(),
            }
        });

        let endpoint = env::var("LINTHIS_AI_ENDPOINT").ok();

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
            AiProviderKind::Claude => "Claude",
            AiProviderKind::OpenAi => "OpenAI",
            AiProviderKind::Local => "Local LLM",
            AiProviderKind::Mock => "Mock",
        }
    }

    fn is_available(&self) -> bool {
        match self.config.kind {
            AiProviderKind::Claude | AiProviderKind::OpenAi => self.config.api_key.is_some(),
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
            AiProviderKind::OpenAi => self.complete_openai(prompt, system_prompt),
            AiProviderKind::Local => self.complete_local(prompt, system_prompt),
            AiProviderKind::Mock => self.complete_mock(prompt, system_prompt),
        }
    }
}

impl AiProvider {
    fn complete_claude(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String, String> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| "Anthropic API key not set. Set ANTHROPIC_API_KEY environment variable.".to_string())?;

        let endpoint = self.config.endpoint.as_deref()
            .unwrap_or("https://api.anthropic.com/v1/messages");

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(self.config.timeout_secs))
            .build()
            .map_err(|e| e.to_string())?;

        let mut messages = vec![
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
            .post(endpoint)
            .header("x-api-key", api_key)
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
        // Return a mock response for testing
        Ok(format!(
            "// Mock AI suggestion for issue\n// Original prompt length: {} characters\n// Suggested fix:\nlet fixed_code = original_code.clone();",
            prompt.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_kind_parsing() {
        assert_eq!("claude".parse::<AiProviderKind>().unwrap(), AiProviderKind::Claude);
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
        assert_eq!(claude.name(), "Claude");

        let openai = AiProvider::new(AiProviderConfig::openai());
        assert_eq!(openai.name(), "OpenAI");
    }
}
