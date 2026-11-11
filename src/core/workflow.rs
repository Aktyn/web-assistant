use serde::Deserialize;
use serde_json;
use std::path::PathBuf;
use tokio::fs::DirEntry;

#[derive(Debug, Deserialize)]
pub struct WorkflowData {
    pub name: String,
}

#[derive(Debug)]
pub enum WorkflowState {
    Data(WorkflowData),
    Error(serde_json::Error),
}

#[derive(Debug)]
pub struct Workflow {
    pub state: WorkflowState,
    pub file_path: PathBuf,
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
}
