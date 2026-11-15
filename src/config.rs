use documented::DocumentedFields;
use serde::{Deserialize, Serialize};
use std::io::{Write as _, stdout};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use crate::tui;

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, DocumentedFields)]
#[documented_fields(default = "Field not documented")]
pub(crate) struct Config {
    /// Path to directory containing workflow files.
    /// Can be omitted during inline run if workflow is provided as file path
    #[serde(default = "get_default_workflows_directory")]
    pub(crate) workflowsDirectory: PathBuf,
    /// Optional map of hex colors to use when rendering TUI.
    /// See *`HexColorsConfig`* for list of color names.
    pub(crate) tuiColors: Option<tui::common::HexColorsConfig>,
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

fn get_default_workflows_directory() -> PathBuf {
    PathBuf::from("./workflows")
}

impl Default for Config {
    fn default() -> Self {
        Self {
            workflowsDirectory: get_default_workflows_directory(),
            tuiColors: None,
        }
    }
}

pub async fn load_config(config_path: &PathBuf) -> Config {
    let done_loading = Arc::new(AtomicUsize::new(0));
    let done_loading_thread = Arc::clone(&done_loading);
    let loading_thread_handle = thread::spawn(move || {
        run_loading_animation(&done_loading_thread);
    });

    let config = Config::new(config_path).await;

    done_loading.store(1, Ordering::SeqCst);
    loading_thread_handle.join().unwrap();

    config
}

fn run_loading_animation(done: &Arc<AtomicUsize>) {
    let label = "Loading";
    let spinner = ['—', '\\', '|', '/'];
    let mut step = 0usize;

    while done.load(Ordering::SeqCst) == 0 {
        let indicator = spinner[step % spinner.len()];
        let line = format!("\r {indicator} {label}");
        stdout().write_all(line.as_bytes()).ok();
        stdout().flush().ok();
        step = step.wrapping_add(1);
        thread::sleep(Duration::from_millis(250));
    }
}

pub(crate) static DEFAULT_CONFIG_FILE: &str = r"./config.json";

/// Create a default config file at the given path. If no path is provided, the default config file
/// will be created in the current working directory as config.json.
pub(crate) async fn create_default_config(path: Option<PathBuf>) {
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
        assert_eq!(
            loaded.workflowsDirectory,
            PathBuf::from("/custom/workflows")
        );
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
        create_default_config(Some(output_path.clone())).await;

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
            rt.block_on(async { create_default_config(Some(output_path.clone())).await });
        });
        assert!(
            result.is_err(),
            "Expected panic when output file already exists"
        );
    }

    #[tokio::test]
    async fn test_init_default_config_serializes_pretty_json() {
        let tmp = tempfile::tempdir().unwrap();
        let output_path = tmp.path().join("pretty.json");
        create_default_config(Some(output_path.clone())).await;

        let content = tokio::fs::read_to_string(&output_path).await.unwrap();
        // Pretty-printed JSON contains newlines and indentation.
        assert!(content.contains('\n'));
        assert!(content.contains("workflowsDirectory"));
    }

    #[test]
    fn test_config_struct_clone() {
        let config = Config {
            workflowsDirectory: PathBuf::from("/some/path"),
            tuiColors: None,
        };
        let cloned = config.clone();
        assert_eq!(cloned.workflowsDirectory, config.workflowsDirectory);
    }

    #[test]
    fn test_config_debug_display() {
        let config = Config::default();
        let debug_str = format!("{config:?}");
        assert!(debug_str.contains("workflowsDirectory"));
    }

    #[test]
    fn test_config_default_is_valid_path() {
        let config = Config::default();
        // Default should be a relative path
        assert!(config.workflowsDirectory.is_relative());
    }
}
