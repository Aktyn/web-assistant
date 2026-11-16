use std::path::{Path, PathBuf};
use tokio::fs::DirEntry;

use crate::utils;

pub struct WorkflowDataJson {
    pub path: PathBuf,
    pub content: String,
}

impl WorkflowDataJson {
    pub(crate) async fn new(entry: &DirEntry) -> Option<Self> {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            let content = tokio::fs::read_to_string(&path).await.ok()?;
            Some(Self { path, content })
        } else {
            None
        }
    }

    /// Test-only constructor that builds a `WorkflowDataJson` from raw content
    /// without touching the filesystem. Available only in test builds.
    #[cfg(test)]
    pub(crate) fn from_raw(path: PathBuf, content: String) -> Self {
        Self { path, content }
    }
}

pub async fn load_from_directory(workflows_directory: &Path) -> Vec<super::Workflow> {
    if workflows_directory.exists() && !workflows_directory.is_dir() {
        log::error!(
            "Workflows path exists but is not a directory: \"{}\"",
            workflows_directory.display()
        );
        return vec![];
    }

    let json_files = utils::read_json_files(workflows_directory).await;
    let mut workflows = Vec::with_capacity(json_files.len());
    for entry in &json_files {
        if let Some(json) = WorkflowDataJson::new(entry).await {
            workflows.push(super::Workflow::new(json));
        }
    }

    workflows
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
            r#"{"name": "Test Workflow", "description": "A test"}"#,
        )
        .await
        .unwrap();
        let workflows = load_from_directory(tmp.path()).await;
        assert_eq!(workflows.len(), 1);
        match &workflows[0].state {
            WorkflowState::Data(data) => {
                assert_eq!(data.name, "Test Workflow");
                assert_eq!(data.description.as_deref(), Some("A test"));
            }
            WorkflowState::Error(_) => panic!("Expected Data variant"),
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
        let workflows = load_from_directory(tmp.path()).await;
        assert!(workflows.is_empty());
    }

    #[tokio::test]
    async fn test_load_from_directory_with_invalid_json_reports_error() {
        let tmp = tempfile::tempdir().unwrap();
        tokio::fs::write(tmp.path().join("bad.json"), "{invalid json!!!")
            .await
            .unwrap();
        let workflows = load_from_directory(tmp.path()).await;
        assert_eq!(workflows.len(), 1);
        match &workflows[0].state {
            WorkflowState::Error(_) => {} // expected
            _ => panic!("Expected Error variant"),
        }
    }

    #[tokio::test]
    async fn test_load_from_directory_nonexistent_returns_empty() {
        let nonexistent = PathBuf::from("/tmp/nonexistent_dir_for_tests_xyz");
        let workflows = load_from_directory(&nonexistent).await;
        assert!(workflows.is_empty());
    }

    #[tokio::test]
    async fn test_load_from_directory_path_is_file_not_dir() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let workflows = load_from_directory(tmp.path()).await;
        assert!(workflows.is_empty());
    }

    #[tokio::test]
    async fn test_load_from_directory_nested_dirs_only_scans_top_level() {
        let tmp = tempfile::tempdir().unwrap();
        let subdir = tmp.path().join("subdir");
        tokio::fs::create_dir(&subdir).await.unwrap();
        tokio::fs::write(subdir.join("nested.json"), r#"{"name": "Nested"}"#)
            .await
            .unwrap();
        // Only top-level files should be loaded, not nested ones.
        tokio::fs::write(tmp.path().join("top.json"), r#"{"name": "Top"}"#)
            .await
            .unwrap();
        let workflows = load_from_directory(tmp.path()).await;
        assert_eq!(workflows.len(), 1);
        match &workflows[0].state {
            WorkflowState::Data(data) => assert_eq!(data.name, "Top"),
            _ => panic!("Expected Data variant"),
        }
    }

    #[test]
    fn test_workflow_data_json_from_raw() {
        let json = WorkflowDataJson::from_raw(
            PathBuf::from("/tmp/raw-test.json"),
            "{\"name\": \"Raw\"}".to_string(),
        );
        assert_eq!(json.path, PathBuf::from("/tmp/raw-test.json"));
        assert_eq!(json.content, "{\"name\": \"Raw\"}");
    }
}
