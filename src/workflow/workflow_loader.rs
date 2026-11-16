use std::path::PathBuf;
use tokio::fs::DirEntry;

use crate::utils;

pub struct WorkflowDataJson {
    pub path: PathBuf,
    pub content: String,
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

pub async fn load_from_directory(workflows_directory: &PathBuf) -> Vec<super::Workflow> {
    assert!(
        !workflows_directory.exists() || workflows_directory.is_dir(),
        "Workflows path already exists but is not a directory: \"{}\"",
        workflows_directory.display()
    );

    let json_files = utils::read_json_files(workflows_directory).await;
    let mut workflows = Vec::with_capacity(json_files.len());
    for entry in &json_files {
        if let Some(json) = WorkflowDataJson::new(entry).await {
            workflows.push(super::Workflow::new(json));
        }
    }

    workflows
}
