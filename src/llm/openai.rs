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
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

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
