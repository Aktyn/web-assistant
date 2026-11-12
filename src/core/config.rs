use serde::{Deserialize, Serialize};
use serde_json;
use std::path::PathBuf;

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Config {
    pub(crate) workflowsDirectory: PathBuf,
}

impl Config {
    pub(crate) async fn new(config_path: &PathBuf) -> Self {
        if !config_path.exists() {
            log::info!(
                "Config file not found (\"{}\"), using defaults",
                config_path.display()
            );
            return Self::default();
        }

        assert!(
            config_path.is_file(),
            "Config path must point to a file. Path: \"{}\"",
            config_path.display()
        );

        let config_content = tokio::fs::read_to_string(config_path)
            .await
            .expect("Cannot read config file");

        serde_json::from_str(config_content.as_str()).unwrap()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            workflowsDirectory: PathBuf::from("./workflows"),
        }
    }
}

pub(crate) static DEFAULT_CONFIG_FILE: &str = r"./config.json";

pub(crate) async fn init_default_config(path: Option<PathBuf>) {
    let config_path = path.unwrap_or(DEFAULT_CONFIG_FILE.into());
    log::info!("Initializing default config from {}", config_path.display());

    assert!(
        !config_path.exists(),
        "\"{}\" file already exists. Delete it or choose a different output path.",
        config_path.display()
    );

    let default_config = Config::default();
    let json =
        serde_json::to_vec_pretty(&default_config).expect("Failed to serialize default config");
    tokio::fs::write(&config_path, &json)
        .await
        .expect("Failed to create config file");
}
