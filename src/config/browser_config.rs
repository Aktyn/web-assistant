use documented::DocumentedFields;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize, Clone, DocumentedFields)]
#[documented_fields(default = "Browser configuration")]
pub struct BrowserConfig {
    pub headless: bool,
    #[serde(alias = "executablePath", rename = "executablePath")]
    pub executable_path: Option<PathBuf>,
    #[serde(alias = "userDataDir", rename = "userDataDir")]
    pub user_data_dir: Option<PathBuf>,
    #[serde(alias = "downloadsDir", rename = "downloadsDir")]
    pub downloads_dir: Option<PathBuf>,
    #[serde(alias = "cdpUrl", rename = "cdpUrl")]
    pub cdp_url: Option<String>,
}
