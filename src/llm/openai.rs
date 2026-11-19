use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use std::env;

#[derive(Debug, Clone)]
pub struct ChatOpenAIConfig {
    pub model: String,
    pub api_key: String,
    pub base_url: String,
    pub temperature: Option<f32>,
    pub max_completion_tokens: Option<u32>,
}

impl Default for ChatOpenAIConfig {
    fn default() -> Self {
        let api_key = env::var("OPENAI_API_KEY").unwrap_or_default();
        let base_url = env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string())
            .trim_end_matches('/')
            .to_string();

        Self {
            model: String::from("auto"),
            api_key,
            base_url,
            temperature: Some(0.2),
            max_completion_tokens: Some(4096),
        }
    }
}

#[derive(Debug)]
pub struct ChatOpenAI {
    pub config: ChatOpenAIConfig,
    pub client: Client,
}

impl super::LLM for ChatOpenAI {
    async fn invoke(&self, messages: &[super::llm_base::ChatMessage]) -> Result<String> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "response_format": { "type": "json_object" }
        });

        if let Some(temp) = self.config.temperature {
            body["temperature"] = serde_json::json!(temp);
        }
        if let Some(tokens) = self.config.max_completion_tokens {
            body["max_completion_tokens"] = serde_json::json!(tokens);
        }

        let mut req = self.client.post(&url);
        if !self.config.api_key.is_empty() {
            req = req.bearer_auth(&self.config.api_key);
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

impl ChatOpenAI {
    pub fn new(config: ChatOpenAIConfig) -> Self {
        // let api_key = env::var("OPENAI_API_KEY").unwrap_or_default();
        // let base_url = env::var("OPENAI_BASE_URL")
        //     .unwrap_or_else(|_| "https://api.openai.com/v1".to_string())
        //     .trim_end_matches('/')
        //     .to_string();

        Self {
            config,
            client: Client::new(),
        }
    }

    //TODO: cleanup
    // #[must_use]
    // pub fn builder() -> ChatOpenAIBuilder {
    //     ChatOpenAIBuilder::default()
    // }
}

//TODO: cleanup
// #[derive(Default)]
// pub struct ChatOpenAIBuilder {
//     model: Option<String>,
//     api_key: Option<String>,
//     base_url: Option<String>,
//     temperature: Option<f32>,
//     max_completion_tokens: Option<u32>,
// }
//
// impl ChatOpenAIBuilder {
//     #[must_use]
//     pub fn model(mut self, model: impl Into<String>) -> Self {
//         self.model = Some(model.into());
//         self
//     }
//
//     #[must_use]
//     pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
//         self.api_key = Some(api_key.into());
//         self
//     }
//
//     #[must_use]
//     pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
//         self.base_url = Some(base_url.into());
//         self
//     }
//
//     #[must_use]
//     pub fn temperature(mut self, temperature: f32) -> Self {
//         self.temperature = Some(temperature);
//         self
//     }
//
//     #[must_use]
//     pub fn max_completion_tokens(mut self, tokens: u32) -> Self {
//         self.max_completion_tokens = Some(tokens);
//         self
//     }
//
//     #[must_use]
//     pub fn build(self) -> ChatOpenAI {
//         let model = self.model.unwrap_or_else(|| "auto".to_string());
//         let api_key = self
//             .api_key
//             .or_else(|| env::var("OPENAI_API_KEY").ok())
//             .unwrap_or_default();
//         let base_url = self
//             .base_url
//             .or_else(|| env::var("OPENAI_BASE_URL").ok())
//             .unwrap_or_else(|| "https://api.openai.com/v1".to_string())
//             .trim_end_matches('/')
//             .to_string();
//
//         ChatOpenAI {
//             model,
//             api_key,
//             base_url,
//             temperature: self.temperature.or(Some(0.2)),
//             max_completion_tokens: self.max_completion_tokens.or(Some(4096)),
//             client: Client::new(),
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::ChatMessage;

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
        let config = ChatOpenAIConfig::default();
        let llm = ChatOpenAI::new(config);
        assert_eq!(llm.config.model, "auto");
    }

    #[test]
    fn test_new_with_explicit_config() {
        let config = ChatOpenAIConfig {
            model: "gpt-4".into(),
            api_key: "test-key".into(),
            base_url: "http://localhost:8080".into(),
            temperature: Some(0.5),
            max_completion_tokens: Some(1024),
        };
        let llm = ChatOpenAI::new(config);
        assert_eq!(llm.config.model, "gpt-4");
        assert_eq!(llm.config.api_key, "test-key");
        assert_eq!(llm.config.base_url, "http://localhost:8080");
        assert_eq!(llm.config.temperature, Some(0.5));
        assert_eq!(llm.config.max_completion_tokens, Some(1024));
    }

    #[test]
    fn test_new_with_explicit_model() {
        let config = ChatOpenAIConfig {
            model: "custom-model".into(),
            ..Default::default()
        };
        let llm = ChatOpenAI::new(config);
        assert_eq!(llm.config.model, "custom-model");
    }

    #[test]
    fn test_llm_trait_implemented() {
        // Verify ChatOpenAI implements LLM trait
        let config = ChatOpenAIConfig::default();
        let _llm = ChatOpenAI::new(config);
        // This just verifies compilation
    }

    #[test]
    fn test_chat_openai_config_clone() {
        let config = ChatOpenAIConfig {
            model: "test".into(),
            api_key: "key".into(),
            base_url: "http://test.com".into(),
            temperature: Some(0.5),
            max_completion_tokens: Some(512),
        };
        let cloned = config.clone();
        assert_eq!(cloned.model, config.model);
        assert_eq!(cloned.api_key, config.api_key);
        assert_eq!(cloned.base_url, config.base_url);
    }
}
