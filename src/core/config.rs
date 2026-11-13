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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_default_config_has_workflows_directory() {
        let config = Config::default();
        assert_eq!(config.workflowsDirectory, PathBuf::from("./workflows"));
    }

    #[test]
    fn test_default_config_file_constant() {
        assert_eq!(DEFAULT_CONFIG_FILE, "./config.json");
    }

    #[tokio::test]
    async fn test_config_new_with_nonexistent_path_returns_default() {
        // Path that does not exist → Config::new returns default config.
        let non_existent = PathBuf::from("/tmp/nonexistent-config-for-testing-12345.json");
        let config = Config::new(&non_existent).await;
        assert_eq!(config.workflowsDirectory, PathBuf::from("./workflows"));
    }

    #[tokio::test]
    async fn test_config_new_with_valid_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let config = json!({ "workflowsDirectory": "/custom/workflows" });
        std::fs::write(tmp.path(), serde_json::to_string_pretty(&config).unwrap()).unwrap();

        let path = PathBuf::from(tmp.path());
        let loaded = Config::new(&path).await;
        assert_eq!(loaded.workflowsDirectory, PathBuf::from("/custom/workflows"));
    }

    #[tokio::test]
    async fn test_config_new_with_empty_object_panics() {
        // `workflowsDirectory` is a required field; an empty JSON object
        // fails to deserialize and Config::new panics on unwrap.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "{}").unwrap();
        let path = PathBuf::from(tmp.path());
        let result = std::panic::catch_unwind(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async { Config::new(&path).await });
        });
        assert!(result.is_err(), "Expected panic on empty JSON object");
    }

    #[tokio::test]
    async fn test_config_new_with_directory_path_panics() {
        // When the path exists but is a directory, Config::new asserts and panics.
        let tmp = tempfile::tempdir().unwrap();
        let path = PathBuf::from(tmp.path());
        let result = std::panic::catch_unwind(|| {
            // We cannot easily run async code inside catch_unwind without a runtime,
            // so we use a block_on approach via a fresh runtime.
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async { Config::new(&path).await });
        });
        assert!(result.is_err(), "Expected panic when path is a directory");
    }

    #[tokio::test]
    async fn test_init_default_config_creates_file() {
        let tmp = tempfile::tempdir().unwrap();
        let output_path = tmp.path().join("config.json");
        init_default_config(Some(output_path.clone())).await;

        assert!(output_path.exists());
        assert!(output_path.is_file());

        let content = tokio::fs::read_to_string(&output_path).await.unwrap();
        let parsed: Config = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.workflowsDirectory, PathBuf::from("./workflows"));
    }

    #[tokio::test]
    async fn test_init_default_config_with_existing_file_panics() {
        let tmp = tempfile::tempdir().unwrap();
        let output_path = tmp.path().join("existing.json");
        std::fs::write(&output_path, "{}").unwrap();

        let result = std::panic::catch_unwind(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async { init_default_config(Some(output_path.clone())).await });
        });
        assert!(result.is_err(), "Expected panic when output file already exists");
    }

    #[tokio::test]
    async fn test_init_default_config_serializes_pretty_json() {
        let tmp = tempfile::tempdir().unwrap();
        let output_path = tmp.path().join("pretty.json");
        init_default_config(Some(output_path.clone())).await;

        let content = tokio::fs::read_to_string(&output_path).await.unwrap();
        // Pretty-printed JSON contains newlines and indentation.
        assert!(content.contains('\n'));
        assert!(content.contains("workflowsDirectory"));
    }

    #[test]
    fn test_config_struct_clone() {
        let config = Config {
            workflowsDirectory: PathBuf::from("/some/path"),
        };
        let cloned = config.clone();
        assert_eq!(cloned.workflowsDirectory, config.workflowsDirectory);
    }
}
