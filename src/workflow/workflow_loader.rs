use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

use crate::utils;

pub(crate) struct JsonFile {
    pub path: PathBuf,
    pub content: String,
}

impl JsonFile {
    async fn new(path: &Path) -> Result<Self> {
        if !path.exists() {
            anyhow::bail!("File does not exist: \"{}\"", path.display());
        }
        if !path.is_file() {
            anyhow::bail!("Given path doesn't point to a file: \"{}\"", path.display());
        }

        let extension = path.extension().and_then(|ext| ext.to_str());
        if extension == Some("json") || extension == Some("jsonc") {
            let content = tokio::fs::read_to_string(&path).await?;
            return Ok(Self {
                path: path.to_path_buf(),
                content,
            });
        }

        Err(anyhow!("Wrong file extension: \"{}\"", path.display()))
    }

    /// Test-only constructor that builds a `JsonFile` from raw content
    /// without touching the filesystem. Available only in test builds.
    #[cfg(test)]
    pub(crate) fn from_raw(path: PathBuf, content: String) -> Self {
        Self { path, content }
    }
}

/// # Errors
/// This function will return an error if the file does not exist or is not a valid json file.
pub async fn load_from_file(workflow_file: &Path) -> Result<super::Workflow> {
    let json = JsonFile::new(workflow_file).await?;
    Ok(super::Workflow::new(json))
}

/// # Errors
/// This function will return an error if the directory does not exist or is not a directory.
pub async fn load_from_directory(workflows_directory: &Path) -> Result<Vec<super::Workflow>> {
    if workflows_directory.exists() && !workflows_directory.is_dir() {
        anyhow::bail!(
            "Workflows path exists but is not a directory: \"{}\"",
            workflows_directory.display()
        );
    }

    let json_files = utils::read_json_files(workflows_directory).await?;
    let mut workflows = Vec::with_capacity(json_files.len());
    for entry in &json_files {
        match JsonFile::new(&entry.path()).await {
            Ok(json) => workflows.push(super::Workflow::new(json)),
            Err(e) => log::error!(
                "Error loading workflow from file: \"{}\": {}",
                entry.path().display(),
                e
            ),
        }
    }
    Ok(workflows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::WorkflowState;

    #[tokio::test]
    async fn test_load_from_directory_with_valid_json_file() {
        let tmp = tempfile::tempdir().unwrap();
        let workflow_path = tmp.path().join("test.json");
        tokio::fs::write(
            &workflow_path,
            r#"{"name": "Test Workflow", "description": "A test", "steps":[]}"#,
        )
        .await
        .unwrap();
        let workflows = load_from_directory(tmp.path()).await.unwrap();
        assert_eq!(workflows.len(), 1);
        match &workflows[0].state {
            WorkflowState::Valid(data) => {
                assert_eq!(data.name, "Test Workflow");
                assert_eq!(data.description.as_deref(), Some("A test"));
            }
            WorkflowState::Error(_) => panic!("Expected Valid variant"),
        }
    }

    #[tokio::test]
    async fn test_load_from_directory_ignores_non_json_files() {
        let tmp = tempfile::tempdir().unwrap();
        tokio::fs::write(tmp.path().join("readme.txt"), "hello")
            .await
            .unwrap();
        tokio::fs::write(tmp.path().join("config.yaml"), "key: value")
            .await
            .unwrap();
        let workflows = load_from_directory(tmp.path()).await.unwrap();
        assert!(workflows.is_empty());
    }

    #[tokio::test]
    async fn test_load_from_directory_with_invalid_json_reports_error() {
        let tmp = tempfile::tempdir().unwrap();
        tokio::fs::write(tmp.path().join("bad.json"), "{invalid json!!!")
            .await
            .unwrap();
        let workflows = load_from_directory(tmp.path()).await.unwrap();
        assert_eq!(workflows.len(), 1);
        match &workflows[0].state {
            WorkflowState::Error(_) => {} // expected
            WorkflowState::Valid(_) => panic!("Expected Error variant"),
        }
    }

    #[tokio::test]
    async fn test_load_from_directory_nonexistent_returns_empty() {
        let nonexistent = PathBuf::from("/tmp/nonexistent_dir_for_tests_xyz");
        let result = load_from_directory(&nonexistent).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_load_from_directory_path_is_file_not_dir() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let result = load_from_directory(tmp.path()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_load_from_directory_nested_dirs_only_scans_top_level() {
        let tmp = tempfile::tempdir().unwrap();
        let subdir = tmp.path().join("subdir");
        tokio::fs::create_dir(&subdir).await.unwrap();
        tokio::fs::write(
            subdir.join("nested.json"),
            r#"{"name": "Nested","steps":[]}"#,
        )
        .await
        .unwrap();
        // Only top-level files should be loaded, not nested ones.
        tokio::fs::write(tmp.path().join("top.json"), r#"{"name": "Top","steps":[]}"#)
            .await
            .unwrap();
        let workflows = load_from_directory(tmp.path()).await.unwrap();
        assert_eq!(workflows.len(), 1);
        match &workflows[0].state {
            WorkflowState::Valid(data) => assert_eq!(data.name, "Top"),
            WorkflowState::Error(_) => panic!("Expected Valid variant"),
        }
    }

    #[test]
    fn test_workflow_data_json_from_raw() {
        let json = JsonFile::from_raw(
            PathBuf::from("/tmp/raw-test.json"),
            "{\"name\": \"Raw\"}".to_string(),
        );
        assert_eq!(json.path, PathBuf::from("/tmp/raw-test.json"));
        assert_eq!(json.content, "{\"name\": \"Raw\"}");
    }

    #[tokio::test]
    async fn test_load_from_file_with_valid_workflow() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("workflow.json");
        tokio::fs::write(
            &path,
            r#"{"name":"FromFile","description":"desc","steps":[]}"#,
        )
        .await
        .unwrap();
        let workflow = load_from_file(&path).await.unwrap();
        assert!(workflow.file_path.is_some());
        match &workflow.state {
            WorkflowState::Valid(data) => {
                assert_eq!(data.name, "FromFile");
                assert_eq!(data.description.as_deref(), Some("desc"));
            }
            WorkflowState::Error(_) => panic!("Expected Valid variant"),
        }
    }

    #[tokio::test]
    async fn test_load_from_file_with_invalid_json() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("bad.json");
        tokio::fs::write(&path, "{invalid json!!!").await.unwrap();
        let result = load_from_file(&path).await;
        assert!(result.is_ok());
        let workflow = result.unwrap();
        assert!(matches!(workflow.state, WorkflowState::Error(_)));
    }

    #[tokio::test]
    async fn test_load_from_file_with_nonexistent_path() {
        let nonexistent = PathBuf::from("/tmp/does_not_exist_for_loader_xyz.json");
        let result = load_from_file(&nonexistent).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("File does not exist")
        );
    }

    #[tokio::test]
    async fn test_load_from_file_with_wrong_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("workflow.txt");
        tokio::fs::write(&path, r#"{"name":"WrongExt"}"#)
            .await
            .unwrap();
        let result = load_from_file(&path).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Wrong file extension")
        );
    }
}
