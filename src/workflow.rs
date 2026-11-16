mod workflow_loader;

use serde::Deserialize;
use std::path::PathBuf;

#[cfg(test)]
pub(crate) use workflow_loader::WorkflowDataJson;
pub use workflow_loader::load_from_directory;

#[derive(Debug, Deserialize, Clone)]
pub struct WorkflowData {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug)]
pub enum WorkflowState {
    Data(WorkflowData),
    Error(serde_json::Error),
}

#[derive(Debug)]
pub struct Workflow {
    pub state: WorkflowState,
    /// No file path indicates draft workflow
    pub file_path: Option<PathBuf>,
}

impl Workflow {
    pub(crate) fn new(json: workflow_loader::WorkflowDataJson) -> Self {
        match serde_json::from_str(&json.content) {
            Ok(data) => Workflow {
                state: WorkflowState::Data(data),
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
            state: WorkflowState::Data(WorkflowData {
                name: "Draft workflow".to_string(),
                description: None,
            }),
            file_path: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            WorkflowState::Data(data) => {
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
            WorkflowState::Data(_) => panic!("Expected Error variant"),
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
            WorkflowState::Data(_) => panic!("Expected Error variant"),
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
            WorkflowState::Data(_) => panic!("Expected Error variant"),
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
            WorkflowState::Data(_) => panic!("Expected Error variant"),
        }
    }

    #[test]
    fn test_workflow_state_debug() {
        let data = WorkflowData {
            name: "My Workflow".to_string(),
            description: Some("description".to_string()),
        };
        let state = WorkflowState::Data(data);
        let debug_str = format!("{state:?}");
        assert!(debug_str.contains("My Workflow"));
    }

    #[test]
    fn test_workflow_state_variant_match() {
        let state = WorkflowState::Data(WorkflowData {
            name: "ok".to_string(),
            description: None,
        });
        match state {
            WorkflowState::Data(d) => assert_eq!(d.name, "ok"),
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
}
