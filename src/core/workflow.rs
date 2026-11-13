use serde::Deserialize;
use std::path::PathBuf;
use tokio::fs::DirEntry;

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct WorkflowData {
    pub(crate) name: String,
}

#[derive(Debug)]
pub(crate) enum WorkflowState {
    Data(WorkflowData),
    Error(serde_json::Error),
}

#[derive(Debug)]
pub(crate) struct Workflow {
    pub(crate) state: WorkflowState,
    pub(crate) file_path: PathBuf,
}

impl Workflow {
    pub(crate) fn new(json: WorkflowDataJson) -> Self {
        match serde_json::from_str(json.content.as_str()) {
            Ok(data) => Workflow {
                state: WorkflowState::Data(data),
                file_path: json.path,
            },
            Err(e) => Workflow {
                state: WorkflowState::Error(e),
                file_path: json.path,
            },
        }
    }
}

pub(crate) struct WorkflowDataJson {
    path: PathBuf,
    content: String,
}

impl WorkflowDataJson {
    pub(crate) async fn new(entry: &DirEntry) -> Option<Self> {
        if entry.path().extension().and_then(|ext| ext.to_str()) == Some("json") {
            let content = tokio::fs::read_to_string(entry.path()).await.ok()?;
            Some(Self {
                path: entry.path(),
                content,
            })
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
        assert_eq!(workflow.file_path, path);
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
        };
        let state = WorkflowState::Data(data);
        let debug_str = format!("{state:?}");
        assert!(debug_str.contains("My Workflow"));
    }

    #[test]
    fn test_workflow_state_variant_match() {
        let state = WorkflowState::Data(WorkflowData {
            name: "ok".to_string(),
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
        };
        let cloned = data.clone();
        assert_eq!(data.name, cloned.name);
    }
}
