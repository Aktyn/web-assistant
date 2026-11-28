mod agent_config;
mod browser_config;
mod colors_config;
mod config;

pub use agent_config::AgentConfig;
pub use browser_config::BrowserConfig;
pub use colors_config::HexColorsConfig;
pub use config::{Config, DEFAULT_CONFIG_FILE, create_default_config, load_config, parse_jsonc};
