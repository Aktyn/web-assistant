use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};

/// `AgentConfig`. Used to configure AI browser agent
#[derive(Debug, Default, Serialize, Deserialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct AgentConfig {
    /// Name of the model to use
    pub model: String,

    /// API key for the AI provider
    #[serde(alias = "apiKey", rename = "apiKey")]
    pub api_key: String,

    /// Base URL of AI API
    #[serde(alias = "baseUrl", rename = "baseUrl")]
    pub base_url: String,
}
