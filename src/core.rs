pub mod config;
pub(crate) mod workflow;

use std::{ffi::OsStr, path::PathBuf};
use tokio::fs::DirEntry;

use crate::cli::Cli;
use config::Config;
use workflow::{Workflow, WorkflowDataJson};

pub struct Core {
    pub(crate) config: Config,
    pub(crate) workflows: Vec<Workflow>,
}

impl Core {
    pub(crate) async fn new(cli: Cli) -> Self {
        let config_path = cli.config_path.expect("Config path is required");
        let config = Config::new(&config_path).await;
        let workflows_directory = &config.workflowsDirectory;

        assert!(
            !workflows_directory.exists() || workflows_directory.is_dir(),
            "Workflows path already exists but is not a directory: \"{}\"",
            workflows_directory.display()
        );

        let json_files = read_json_files(workflows_directory).await;
        let mut workflows = Vec::with_capacity(json_files.len());
        for entry in &json_files {
            if let Some(json) = WorkflowDataJson::new(entry).await {
                workflows.push(Workflow::new(json));
            }
        }

        Self { config, workflows }
    }
}

async fn read_json_files(directory_path: &PathBuf) -> Vec<DirEntry> {
    if !directory_path.exists() {
        return vec![];
    }

    let mut dir = match tokio::fs::read_dir(directory_path).await {
        Ok(dir) => dir,
        Err(e) => {
            log::error!("error reading workflows directory: {e}");
            return vec![];
        }
    };

    let mut entries = Vec::new();
    loop {
        match dir.next_entry().await {
            Ok(Some(entry)) => {
                if filter_json_files(&entry).await {
                    entries.push(entry);
                }
            }
            Ok(None) => break,
            Err(e) => {
                log::error!("error reading directory entry: {e}");
            }
        }
    }
    entries
}

async fn filter_json_files(entry: &DirEntry) -> bool {
    let is_file = match entry.file_type().await {
        Ok(file_type) => file_type.is_file(),
        Err(_) => return false,
    };

    is_file && entry.path().extension() == Some(OsStr::new("json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    /// Write a config.json into `parent_dir` pointing at `workflows_dir`, then build a
    /// `Cli` whose `config_path` resolves to it. `Core::new` reads the config to
    /// discover the workflows directory, so the test must supply a real config
    /// file rather than a bare `Cli` struct literal.
    fn make_cli(parent_dir: &std::path::Path, workflows_dir: &std::path::Path) -> Cli {
        let config_path = parent_dir.join("config.json");
        let config = json!({ "workflowsDirectory": workflows_dir });
        std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();
        Cli {
            config_path: Some(config_path),
            command: None,
        }
    }

    fn setup_workflow_dir(files: &[(&str, &str)]) -> TempDir {
        let tmp = TempDir::new().unwrap();
        for (name, content) in files {
            let path = tmp.path().join(name);
            std::fs::write(path, content).unwrap();
        }
        tmp
    }

    #[tokio::test]
    async fn test_core_new_with_empty_directory() {
        let tmp = setup_workflow_dir(&[]);
        let parent_tmp = TempDir::new().unwrap();
        let cli = make_cli(parent_tmp.path(), tmp.path());
        let core = Core::new(cli).await;
        assert_eq!(core.workflows.len(), 0);
        assert_eq!(core.config.workflowsDirectory, tmp.path());
    }

    #[tokio::test]
    async fn test_core_new_with_valid_workflow_file() {
        let tmp = setup_workflow_dir(&[("valid.json", r#"{"name": "My Workflow"}"#)]);
        let parent_tmp = TempDir::new().unwrap();
        let cli = make_cli(parent_tmp.path(), tmp.path());
        let core = Core::new(cli).await;
        assert_eq!(core.workflows.len(), 1);
        // Verify the workflow loaded successfully
        match &core.workflows[0].state {
            workflow::WorkflowState::Data(data) => {
                assert_eq!(data.name, "My Workflow");
            }
            workflow::WorkflowState::Error(_) => panic!("Expected valid workflow data"),
        }
    }

    #[tokio::test]
    async fn test_core_new_filters_non_json_files() {
        let tmp = setup_workflow_dir(&[
            ("workflow.json", r#"{"name": "Valid"}"#),
            ("notes.txt", "This should be ignored"),
            ("script.py", "import os"),
        ]);
        let parent_tmp = TempDir::new().unwrap();
        let cli = make_cli(parent_tmp.path(), tmp.path());
        let core = Core::new(cli).await;
        assert_eq!(core.workflows.len(), 1);
    }

    #[tokio::test]
    async fn test_core_new_skips_invalid_json() {
        let tmp = setup_workflow_dir(&[
            ("good.json", r#"{"name": "Good"}"#),
            ("bad.json", "{invalid json content"),
        ]);
        let parent_tmp = TempDir::new().unwrap();
        let cli = make_cli(parent_tmp.path(), tmp.path());
        let core = Core::new(cli).await;
        // Both files enter the pipeline (they're both .json). The invalid one
        // does not get skipped — it is stored as a Workflow in Error state.
        assert_eq!(core.workflows.len(), 2);
        // Check that we have exactly one valid and one error workflow
        let (valid, errors): (Vec<_>, Vec<_>) = core
            .workflows
            .iter()
            .partition(|w| matches!(w.state, workflow::WorkflowState::Data(_)));
        assert_eq!(valid.len(), 1);
        assert_eq!(errors.len(), 1);
        match &valid[0].state {
            workflow::WorkflowState::Data(data) => assert_eq!(data.name, "Good"),
            workflow::WorkflowState::Error(_) => unreachable!(),
        }
    }

    #[tokio::test]
    async fn test_read_json_files_on_nonexistent_directory() {
        let non_existent = PathBuf::from("/tmp/nonexistent-dir-for-testing-12345");
        let entries = read_json_files(&non_existent).await;
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_filter_json_files_includes_json_excludes_others() {
        let tmp = TempDir::new().unwrap();
        let json_path = tmp.path().join("test.json");
        let txt_path = tmp.path().join("test.txt");
        std::fs::write(&json_path, "{}").unwrap();
        std::fs::write(&txt_path, "text").unwrap();

        // Cannot directly call filter_json_files since it takes DirEntry;
        // instead verify via the higher-level read_json_files.
        let parent_tmp = TempDir::new().unwrap();
        let cli = make_cli(parent_tmp.path(), tmp.path());
        let core = Core::new(cli).await;
        // Only .json file enters the pipeline; .txt is filtered out.
        // The json file has {} which deserializes to an error (missing `name` field),
        // but it still counts as one workflow entry (in Error state).
        assert_eq!(core.workflows.len(), 1);
    }
}
