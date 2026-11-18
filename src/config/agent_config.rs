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
    // /// Whether to run browser in headless mode
    // pub headless: bool,
    // /// The path to the browser executable
    // /// If omitted, the browser will be searched for in common locations
    // #[serde(alias = "executablePath", rename = "executablePath")]
    // pub executable_path: Option<PathBuf>,
    // /// The path to the user data directory
    // /// Useful to preserve browser state between runs
    // #[serde(alias = "userDataDir", rename = "userDataDir")]
    // pub user_data_dir: Option<PathBuf>,
    // /// The path to the downloads directory
    // #[serde(alias = "downloadsDir", rename = "downloadsDir")]
    // pub downloads_dir: Option<PathBuf>,
    // /// The URL of the Chrome DevTools Protocol (CDP) endpoint
    // #[allow(clippy::doc_markdown)]
    // #[serde(alias = "cdpUrl", rename = "cdpUrl")]
    // pub cdp_url: Option<String>,
}
