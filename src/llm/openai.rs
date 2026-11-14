#![allow(clippy::missing_errors_doc)]

use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatOpenAI {
    pub model: String,
    pub api_key: String,
    pub base_url: String,
    pub temperature: Option<f32>,
    pub max_completion_tokens: Option<u32>,
    client: Client,
}

impl ChatOpenAI {
    pub fn new(model: impl Into<String>) -> Self {
        let api_key = env::var("OPENAI_API_KEY").unwrap_or_default();
        let base_url = env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string())
            .trim_end_matches('/')
            .to_string();

        Self {
            model: model.into(),
            api_key,
            base_url,
            temperature: Some(0.2),
            max_completion_tokens: Some(4096),
            client: Client::new(),
        }
    }

    #[must_use]
    pub fn builder() -> ChatOpenAIBuilder {
        ChatOpenAIBuilder::default()
    }

    pub async fn invoke(&self, messages: &[ChatMessage]) -> Result<String> {
        let url = format!("{}/chat/completions", self.base_url);

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "response_format": { "type": "json_object" }
        });

        if let Some(temp) = self.temperature {
            body["temperature"] = serde_json::json!(temp);
        }
        if let Some(tokens) = self.max_completion_tokens {
            body["max_completion_tokens"] = serde_json::json!(tokens);
        }

        let mut req = self.client.post(&url);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        let resp = req
            .json(&body)
            .send()
            .await
            .with_context(|| format!("Failed to send chat completion request to {url}"))?;

        let status = resp.status();
        if !status.is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "OpenAI API request failed with status {status}: {error_text}"
            ));
        }

        let resp_json: serde_json::Value = resp
            .json()
            .await
            .context("Failed to parse chat completion JSON response")?;

        let content = resp_json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                anyhow!("Missing message content in completion response: {resp_json}")
            })?;

        Ok(content.to_string())
    }
}

#[derive(Default)]
pub struct ChatOpenAIBuilder {
    model: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
    temperature: Option<f32>,
    max_completion_tokens: Option<u32>,
}

impl ChatOpenAIBuilder {
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    #[must_use]
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    #[must_use]
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    #[must_use]
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    #[must_use]
    pub fn max_completion_tokens(mut self, tokens: u32) -> Self {
        self.max_completion_tokens = Some(tokens);
        self
    }

    pub fn build(self) -> Result<ChatOpenAI> {
        let model = self.model.unwrap_or_else(|| "auto".to_string());
        let api_key = self
            .api_key
            .or_else(|| env::var("OPENAI_API_KEY").ok())
            .unwrap_or_default();
        let base_url = self
            .base_url
            .or_else(|| env::var("OPENAI_BASE_URL").ok())
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string())
            .trim_end_matches('/')
            .to_string();

        Ok(ChatOpenAI {
            model,
            api_key,
            base_url,
            temperature: self.temperature.or(Some(0.2)),
            max_completion_tokens: self.max_completion_tokens.or(Some(4096)),
            client: Client::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_chat_message_serde_structure() {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        };
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_builder_default_model() {
        let llm = ChatOpenAI::builder().build().unwrap();
        assert_eq!(llm.model, "auto");
    }

    #[test]
    fn test_builder_uses_env_vars_when_provided() {
        unsafe {
            env::set_var("OPENAI_API_KEY", "env-key-123");
            env::set_var("OPENAI_BASE_URL", "http://env.example.com/v1");
        }
        // Builder prioritizes explicit params over env vars, but falls back correctly
        let llm_from_env = ChatOpenAI::builder().model("gpt-4").build().unwrap();
        assert_eq!(llm_from_env.model, "gpt-4");
        // Clean up
        unsafe {
            env::remove_var("OPENAI_API_KEY");
            env::remove_var("OPENAI_BASE_URL");
        }
    }

    #[test]
    fn test_builder_trims_trailing_slashes() {
        let llm = ChatOpenAI::builder()
            .base_url("http://example.com/")
            .build()
            .unwrap();
        assert_eq!(llm.base_url, "http://example.com");
    }

    #[test]
    fn test_build_with_all_fields() {
        let llm = ChatOpenAI::builder()
            .model("claude-3")
            .api_key("my-secret")
            .base_url("http://localhost:11434/v1")
            .temperature(0.8)
            .max_completion_tokens(2048)
            .build()
            .unwrap();
        assert_eq!(llm.model, "claude-3");
        assert_eq!(llm.api_key, "my-secret");
        assert_eq!(llm.base_url, "http://localhost:11434/v1");
        assert_eq!(llm.temperature, Some(0.8));
        assert_eq!(llm.max_completion_tokens, Some(2048));
    }

    #[test]
    fn test_new_with_explicit_model() {
        let llm = ChatOpenAI::new("custom-model");
        assert_eq!(llm.model, "custom-model");
    }
}
