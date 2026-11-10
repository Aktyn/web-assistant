use serde::Deserialize;
use serde_json;
use std::{fs::DirEntry, path::PathBuf};

#[derive(Debug, Deserialize)]
struct WorkflowData {
    name: String,
    foo: u32,
}

#[derive(Debug)]
enum WorkflowState {
    Data(WorkflowData),
    Error(serde_json::Error),
}

#[derive(Debug)]
pub struct Workflow {
    state: WorkflowState,
    filePath: PathBuf,
}

impl Workflow {
    pub(crate) fn new(json: WorkflowDataJson) -> Self {
        match serde_json::from_str(json.content.as_str()) {
            Ok(data) => Workflow {
                state: WorkflowState::Data(data),
                filePath: json.path,
            },
            Err(e) => Workflow {
                state: WorkflowState::Error(e),
                filePath: json.path,
            },
        }
    }
}

pub(crate) struct WorkflowDataJson {
    path: PathBuf,
    content: String,
}

impl WorkflowDataJson {
    pub(crate) fn new(entry: &DirEntry) -> Option<Self> {
        if entry.path().extension().and_then(|ext| ext.to_str()) == Some("json") {
            let content = std::fs::read_to_string(entry.path()).ok()?;
            Some(Self {
                path: entry.path(),
                content,
            })
        } else {
            None
        }
    }
}
