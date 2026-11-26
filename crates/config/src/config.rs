// mod agent_config;
// mod browser_config;
// mod colors_config;

use anyhow::{Context, Result};
use documented::{Documented, DocumentedFields};
use jsonc_parser::{ParseOptions, parse_to_serde_value};
use serde::{Deserialize, Serialize};
use std::io::{Write as _, stdout};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::agent_config::AgentConfig;
use crate::browser_config::BrowserConfig;
use crate::colors_config::HexColorsConfig;

/// Config must be a json file
#[derive(Debug, Serialize, Deserialize, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct Config {
    /// Path to directory containing workflow files.
    /// Can be omitted during inline run if workflow is provided as file path
    #[serde(
        default = "get_default_workflows_directory",
        alias = "workflowsDirectory",
        rename = "workflowsDirectory"
    )]
    pub workflows_directory: PathBuf,
    /// Optional map of hex colors to use when rendering TUI.
    /// See `HexColorsConfig` for list of color names.
    #[serde(alias = "tuiColors", rename = "tuiColors")]
    pub tui_colors: Option<HexColorsConfig>,

    /// See `BrowserConfig` for more information.
    #[serde(default = "BrowserConfig::default")]
    pub browser: BrowserConfig,

    /// See `AgentConfig` for more information.
    #[serde(default = "AgentConfig::default")]
    pub agent: AgentConfig,
}

impl Config {
    async fn new(config_path: &Path) -> Result<Self> {
        if !config_path.exists() {
            log::info!(
                "Config file not found (\"{}\"), using defaults",
                config_path.display()
            );
            return Ok(Self::default());
        }

        if !config_path.is_file() {
            anyhow::bail!(
                "Config path must point to a file. Path: \"{}\"",
                config_path.display()
            );
        }

        let config_content = tokio::fs::read_to_string(config_path)
            .await
            .context("Cannot read config file")?;

        let parsed = parse_to_serde_value(config_content.as_str(), &ParseOptions::default())
            .with_context(|| format!("Cannot parse config file: \"{}\"", config_path.display()))?;
        Ok(parsed)
    }
}

fn get_default_workflows_directory() -> PathBuf {
    PathBuf::from("./workflows")
}

impl Default for Config {
    fn default() -> Self {
        Self {
            workflows_directory: get_default_workflows_directory(),
            tui_colors: None,
            agent: AgentConfig::default(),
            browser: BrowserConfig::default(),
        }
    }
}

pub async fn load_config(config_path: &Path) -> Result<Config> {
    let done_loading = Arc::new(AtomicBool::new(false));
    let done_loading_thread = Arc::clone(&done_loading);
    let loading_thread_handle = thread::spawn(move || {
        run_loading_animation(&done_loading_thread);
    });

    let config = Config::new(config_path).await?;

    done_loading.store(true, Ordering::SeqCst);
    loading_thread_handle.join().ok();

    stdout().write_all(b"\n").ok();
    stdout().flush().ok();

    Ok(config)
}

fn run_loading_animation(done: &Arc<AtomicBool>) {
    let label = "Loading config";
    let spinner = ['—', '\\', '|', '/'];
    let mut step = 0usize;

    while !done.load(Ordering::SeqCst) {
        let indicator = spinner[step % spinner.len()];
        let line = format!("\r {indicator} {label}");
        stdout().write_all(line.as_bytes()).ok();
        stdout().flush().ok();
        step = step.wrapping_add(1);
        thread::sleep(Duration::from_millis(250));
    }
}

pub static DEFAULT_CONFIG_FILE: &str = r"./config.jsonc";

/// Create a default config file at the given path. If no path is provided, the default config file
/// will be created in the current working directory as config.json.
/// # Errors
/// This function will return an error if the config file already exists.
/// # Panics
/// This function will panic if the default config file cannot be serialized.
pub async fn create_default_config(path: Option<&Path>) -> Result<&Path> {
    let config_path = path.unwrap_or(DEFAULT_CONFIG_FILE.as_ref());
    log::info!("Initializing default config from {}", config_path.display());

    if config_path.exists() {
        anyhow::bail!(
            "\"{}\" file already exists. Delete it or choose a different output path.",
            config_path.display()
        );
    }

    let default_config = Config::default();
    let json =
        serde_json::to_vec_pretty(&default_config).context("Failed to serialize default config")?;
    tokio::fs::write(&config_path, &json)
        .await
        .context("Failed to create config file")?;

    Ok(config_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_default_config_has_workflows_directory() {
        let config = Config::default();
        assert_eq!(config.workflows_directory, PathBuf::from("./workflows"));
    }

    #[test]
    fn test_default_config_file_constant() {
        assert_eq!(DEFAULT_CONFIG_FILE, "./config.jsonc");
    }

    #[tokio::test]
    async fn test_config_new_with_nonexistent_path_returns_default() {
        // Path that does not exist → Config::new returns default config.
        let non_existent = PathBuf::from("/tmp/nonexistent-config-for-testing-12345.json");
        let config = Config::new(&non_existent).await;
        assert_eq!(config.workflows_directory, PathBuf::from("./workflows"));
    }

    #[tokio::test]
    async fn test_config_new_with_valid_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let config = json!({ "workflowsDirectory": "/custom/workflows" });
        std::fs::write(tmp.path(), serde_json::to_string_pretty(&config).unwrap()).unwrap();

        let path = PathBuf::from(tmp.path());
        let loaded = Config::new(&path).await;
        assert_eq!(
            loaded.workflows_directory,
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
        create_default_config(Some(&output_path)).await.unwrap();

        assert!(output_path.exists());
        assert!(output_path.is_file());

        let content = tokio::fs::read_to_string(&output_path).await.unwrap();
        let parsed: Config = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.workflows_directory, PathBuf::from("./workflows"));
    }

    #[tokio::test]
    async fn test_init_default_config_with_existing_file_panics() {
        let tmp = tempfile::tempdir().unwrap();
        let output_path = tmp.path().join("existing.json");
        std::fs::write(&output_path, "{}").unwrap();

        let result = create_default_config(Some(&output_path)).await;
        assert!(
            result.is_err(),
            "Expected error when output file already exists"
        );
    }

    #[tokio::test]
    async fn test_init_default_config_serializes_pretty_json() {
        let tmp = tempfile::tempdir().unwrap();
        let output_path = tmp.path().join("pretty.json");
        create_default_config(Some(&output_path)).await.unwrap();

        let content = tokio::fs::read_to_string(&output_path).await.unwrap();
        // Pretty-printed JSON contains newlines and indentation.
        assert!(content.contains('\n'));
        assert!(content.contains("workflowsDirectory"));
    }

    #[test]
    fn test_config_debug_display() {
        let config = Config::default();
        let debug_str = format!("{config:?}");
        assert!(debug_str.contains("workflows_directory"));
    }

    #[test]
    fn test_config_default_is_valid_path() {
        let config = Config::default();
        // Default should be a relative path
        assert!(config.workflows_directory.is_relative());
    }
}
