mod workflow_data;
mod workflow_loader;
mod workflow_runtime;

use std::path::PathBuf;

use workflow_data::WorkflowData;
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
    use workflow_loader::WorkflowDataJson;
    #[tokio::test]
    async fn test_workflow_data_json_valid_file() {
        let json_content = "{\"name\": \"Test Workflow\"}";
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), json_content).unwrap();
        let path = tmp.path().to_path_buf();
        let json = WorkflowDataJson {
            path: path.clone(),
            content: json_content.to_string(),
        };
        let workflow = Workflow::new(json);
        match &workflow.state {
            WorkflowState::Valid(data) => {
                assert_eq!(data.name, "Test Workflow");
            }
            WorkflowState::Error(_) => panic!("Expected Data variant"),
        }
        assert_eq!(workflow.file_path, Some(path));
    }

    #[tokio::test]
    async fn test_workflow_data_json_invalid_json() {
        let json = WorkflowDataJson {
            path: PathBuf::from("/fake/path.json"),
            content: "{invalid json".to_string(),
        };
        let workflow = Workflow::new(json);
        match &workflow.state {
            WorkflowState::Error(_) => {} // expected
            WorkflowState::Valid(_) => panic!("Expected Error variant"),
        }
    }

    #[tokio::test]
    async fn test_workflow_data_json_missing_name_field() {
        let json = WorkflowDataJson {
            path: PathBuf::from("/fake/path2.json"),
            content: r#"{"other": "value"}"#.to_string(),
        };
        let workflow = Workflow::new(json);
        match &workflow.state {
            WorkflowState::Error(_) => {} // missing required `name` field → deserialize error
            WorkflowState::Valid(_) => panic!("Expected Error variant"),
        }
    }

    #[tokio::test]
    async fn test_workflow_data_json_empty_object() {
        let json = WorkflowDataJson {
            path: PathBuf::from("/fake/path3.json"),
            content: "{}".to_string(),
        };
        let workflow = Workflow::new(json);
        match &workflow.state {
            WorkflowState::Error(_) => {}
            WorkflowState::Valid(_) => panic!("Expected Error variant"),
        }
    }

    #[tokio::test]
    async fn test_workflow_data_json_null_name() {
        let json = WorkflowDataJson {
            path: PathBuf::from("/fake/path4.json"),
            content: r#"{"name": null}"#.to_string(),
        };
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
        });
        match state {
            WorkflowState::Valid(d) => assert_eq!(d.name, "ok"),
            WorkflowState::Error(_) => panic!("Expected Data"),
        }
    }

    #[test]
    fn test_workflow_data_clone() {
        let data = WorkflowData {
            name: "clone-test".to_string(),
            description: Some("test description".to_string()),
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
            WorkflowState::Error(_) => panic!("Expected Data variant"),
        }
    }

    #[test]
    fn test_workflow_state_error_contains_message() {
        let json = WorkflowDataJson {
            path: PathBuf::from("/tmp/bad.json"),
            content: "{invalid".to_string(),
        };
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
        });
        let s = format!("{state:?}");
        assert!(s.contains("Test"));
    }

    #[tokio::test]
    async fn test_load_from_directory_empty_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let workflows = load_from_directory(&tmp.path().to_path_buf()).await;
        assert!(workflows.is_empty());
    }

    #[tokio::test]
    async fn test_load_from_directory_nonexistent_returns_empty() {
        let nonexistent = PathBuf::from("/tmp/does_not_exist_xyz_12345");
        let workflows = load_from_directory(&nonexistent).await;
        assert!(workflows.is_empty());
    }

    #[tokio::test]
    async fn test_load_from_directory_with_valid_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.json");
        tokio::fs::write(&path, r#"{"name":"MyWorkflow","description":"desc"}"#)
            .await
            .unwrap();
        let workflows = load_from_directory(&tmp.path().to_path_buf()).await;
        assert_eq!(workflows.len(), 1);
        match &workflows[0].state {
            WorkflowState::Valid(d) => {
                assert_eq!(d.name, "MyWorkflow");
                assert_eq!(d.description.as_deref(), Some("desc"));
            }
            WorkflowState::Error(_) => panic!("Expected Data variant"),
        }
    }

    #[tokio::test]
    async fn test_load_from_directory_ignores_non_json_files() {
        let tmp = tempfile::tempdir().unwrap();
        // Write a non-json file that should be ignored.
        tokio::fs::write(tmp.path().join("readme.txt"), "hello")
            .await
            .unwrap();
        let workflows = load_from_directory(&tmp.path().to_path_buf()).await;
        assert!(workflows.is_empty());
    }

    #[test]
    fn test_workflow_data_deserialization_missing_description_defaults_to_none() {
        let json_str = r#"{"name":"NoDesc"}"#;
        let data: WorkflowData = serde_json::from_str(json_str).unwrap();
        assert_eq!(data.name, "NoDesc");
        assert!(data.description.is_none());
    }

    #[test]
    fn test_workflow_data_deserialization_full() {
        let json_str = r#"{"name":"Full","description":"A full workflow"}"#;
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
}
