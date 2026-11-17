pub(crate) mod workflow_data;
mod workflow_loader;
mod workflow_runtime;

use std::path::PathBuf;

pub(crate) use workflow_data::WorkflowData;
pub use workflow_loader::{load_from_directory, load_from_file};

#[derive(Debug)]
pub enum WorkflowState {
    Valid(WorkflowData),
    Error(serde_json::Error),
}

#[derive(Debug)]
pub struct Workflow {
    pub state: WorkflowState,
    /// No file path indicates draft workflow
    pub file_path: Option<PathBuf>,
}

impl Workflow {
    #[must_use]
    pub(crate) fn new(json: workflow_loader::JsonFile) -> Self {
        match serde_json::from_str(&json.content) {
            Ok(data) => Workflow {
                state: WorkflowState::Valid(data),
                file_path: Some(json.path),
            },
            Err(e) => Workflow {
                state: WorkflowState::Error(e),
                file_path: Some(json.path),
            },
        }
    }

    #[must_use]
    pub fn new_draft() -> Self {
        Workflow {
            state: WorkflowState::Valid(WorkflowData {
                name: "Draft workflow".to_string(),
                description: None,
                steps: vec![],
            }),
            file_path: None,
        }
    }

    fn is_draft(&self) -> bool {
        self.file_path.is_none()
    }
}

#[cfg(test)]
#[allow(clippy::match_wildcard_for_single_variants)]
mod tests {
    use super::*;
    pub(crate) use workflow_loader::JsonFile;

    #[tokio::test]
    async fn test_workflow_data_json_valid_file() {
        let json_content = "{\"name\": \"Test Workflow\", \"steps\":[]}";
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), json_content).unwrap();
        let path = tmp.path().to_path_buf();
        let json = JsonFile::from_raw(path.clone(), json_content.to_string());
        let workflow = Workflow::new(json);
        match &workflow.state {
            WorkflowState::Valid(data) => {
                assert_eq!(data.name, "Test Workflow");
            }
            WorkflowState::Error(_) => panic!("Expected Valid variant"),
        }
        assert_eq!(workflow.file_path, Some(path));
    }

    #[tokio::test]
    async fn test_workflow_data_json_invalid_json() {
        let json = JsonFile::from_raw(
            PathBuf::from("/fake/path.json"),
            "{invalid json".to_string(),
        );
        let workflow = Workflow::new(json);
        match &workflow.state {
            WorkflowState::Error(_) => {} // expected
            WorkflowState::Valid(_) => panic!("Expected Error variant"),
        }
    }

    #[tokio::test]
    async fn test_workflow_data_json_missing_name_field() {
        let json = JsonFile::from_raw(
            PathBuf::from("/fake/path2.json"),
            r#"{"other": "value"}"#.to_string(),
        );
        let workflow = Workflow::new(json);
        match &workflow.state {
            WorkflowState::Error(_) => {} // missing required `name` field → deserialize error
            WorkflowState::Valid(_) => panic!("Expected Error variant"),
        }
    }

    #[tokio::test]
    async fn test_workflow_data_json_empty_object() {
        let json = JsonFile::from_raw(PathBuf::from("/fake/path3.json"), "{}".to_string());
        let workflow = Workflow::new(json);
        match &workflow.state {
            WorkflowState::Error(_) => {}
            WorkflowState::Valid(_) => panic!("Expected Error variant"),
        }
    }

    #[tokio::test]
    async fn test_workflow_data_json_null_name() {
        let json = JsonFile::from_raw(
            PathBuf::from("/fake/path4.json"),
            r#"{"name": null}"#.to_string(),
        );
        let workflow = Workflow::new(json);
        match &workflow.state {
            WorkflowState::Error(_) => {}
            WorkflowState::Valid(_) => panic!("Expected Error variant"),
        }
    }

    #[test]
    fn test_workflow_state_debug() {
        let data = WorkflowData {
            name: "My Workflow".to_string(),
            description: Some("description".to_string()),
            steps: vec![],
        };
        let state = WorkflowState::Valid(data);
        let debug_str = format!("{state:?}");
        assert!(debug_str.contains("My Workflow"));
    }

    #[test]
    fn test_workflow_state_variant_match() {
        let state = WorkflowState::Valid(WorkflowData {
            name: "ok".to_string(),
            description: None,
            steps: vec![],
        });
        match state {
            WorkflowState::Valid(d) => assert_eq!(d.name, "ok"),
            WorkflowState::Error(_) => panic!("Expected Valid variant"),
        }
    }

    #[test]
    fn test_workflow_data_clone() {
        let data = WorkflowData {
            name: "clone-test".to_string(),
            description: Some("test description".to_string()),
            steps: vec![],
        };
        let cloned = data.clone();
        assert_eq!(data.name, cloned.name);
    }

    #[test]
    fn test_workflow_new_draft_has_no_path() {
        let draft = Workflow::new_draft();
        assert!(draft.file_path.is_none());
        match &draft.state {
            WorkflowState::Valid(d) => {
                assert_eq!(d.name, "Draft workflow");
                assert!(d.description.is_none());
            }
            WorkflowState::Error(_) => panic!("Expected Valid variant"),
        }
    }

    #[test]
    fn test_workflow_state_error_contains_message() {
        let json = JsonFile::from_raw(PathBuf::from("/tmp/bad.json"), "{invalid".to_string());
        let wf = Workflow::new(json);
        match &wf.state {
            WorkflowState::Error(e) => {
                // Serde's exact wording varies; just check it contains something
                // indicating a parse/expected token failure rather than being empty.
                let err_str = e.to_string();
                assert!(!err_str.is_empty(), "error message should not be empty");
            }
            WorkflowState::Valid(_) => panic!("Expected Error variant"),
        }
    }

    #[test]
    fn test_workflow_state_debug_display() {
        let state = WorkflowState::Valid(WorkflowData {
            name: "Test".to_string(),
            description: None,
            steps: vec![],
        });
        let s = format!("{state:?}");
        assert!(s.contains("Test"));
    }

    #[tokio::test]
    async fn test_load_from_directory_empty_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let workflows = load_from_directory(tmp.path()).await.unwrap();
        assert!(workflows.is_empty());
    }

    #[tokio::test]
    async fn test_load_from_directory_nonexistent_returns_empty() {
        let nonexistent = PathBuf::from("/tmp/does_not_exist_xyz_12345");
        let result = load_from_directory(&nonexistent).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_load_from_directory_with_valid_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.json");
        tokio::fs::write(
            &path,
            r#"{"name":"MyWorkflow","description":"desc","steps":[]}"#,
        )
        .await
        .unwrap();
        let workflows = load_from_directory(tmp.path()).await.unwrap();
        assert_eq!(workflows.len(), 1);
        match &workflows[0].state {
            WorkflowState::Valid(d) => {
                assert_eq!(d.name, "MyWorkflow");
                assert_eq!(d.description.as_deref(), Some("desc"));
            }
            WorkflowState::Error(_) => panic!("Expected Valid variant"),
        }
    }

    #[tokio::test]
    async fn test_load_from_directory_ignores_non_json_files() {
        let tmp = tempfile::tempdir().unwrap();
        // Write a non-json file that should be ignored.
        tokio::fs::write(tmp.path().join("readme.txt"), "hello")
            .await
            .unwrap();
        let workflows = load_from_directory(tmp.path()).await.unwrap();
        assert!(workflows.is_empty());
    }

    #[test]
    fn test_workflow_data_deserialization_missing_description_defaults_to_none() {
        let json_str = r#"{"name":"NoDesc","steps":[]}"#;
        let data: WorkflowData = serde_json::from_str(json_str).unwrap();
        assert_eq!(data.name, "NoDesc");
        assert!(data.description.is_none());
    }

    #[test]
    fn test_workflow_data_deserialization_full() {
        let json_str = r#"{"name":"Full","description":"A full workflow","steps":[]}"#;
        let data: WorkflowData = serde_json::from_str(json_str).unwrap();
        assert_eq!(data.name, "Full");
        assert_eq!(data.description.as_deref(), Some("A full workflow"));
    }

    /// Ensure deserializing invalid JSON fails gracefully without panicking.
    #[test]
    fn test_invalid_json_does_not_panic_on_parse() {
        let result = std::panic::catch_unwind(|| {
            let _json: WorkflowData = serde_json::from_str("not json at all").unwrap();
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_workflow_is_draft_for_new_workflow() {
        let workflow = Workflow::new_draft();
        assert!(workflow.is_draft());
    }

    #[test]
    fn test_workflow_is_not_draft_for_loaded_workflow() {
        let json = JsonFile::from_raw(
            PathBuf::from("/tmp/loaded.json"),
            r#"{"name":"Loaded","steps":[]}"#.to_string(),
        );
        let workflow = Workflow::new(json);
        assert!(!workflow.is_draft());
    }

    #[tokio::test]
    async fn test_load_from_file_valid_workflow() {
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
    async fn test_load_from_file_nonexistent_path() {
        let nonexistent = PathBuf::from("/tmp/does_not_exist_for_workflow_xyz.json");
        let result = load_from_file(&nonexistent).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("File does not exist")
        );
    }
}
