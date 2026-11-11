pub mod workflow;

use std::{ffi::OsStr, path::PathBuf};
use tokio::fs::DirEntry;

use crate::cli::Cli;
use workflow::{Workflow, WorkflowDataJson};

#[derive(Default)]
pub struct Core {
    pub workflows_directory: PathBuf,
    pub workflows: Vec<Workflow>,
}

impl Core {
    pub async fn new(cli: Cli) -> Self {
        let workflows_directory = cli.workflows_path.expect("workflows path is required");

        if workflows_directory.exists() && !workflows_directory.is_dir() {
            panic!(
                "Workflows path already exists but is not a directory: \"{}\"",
                workflows_directory.display()
            );
        }

        let json_files = read_json_files(&workflows_directory).await;
        let mut workflows = Vec::with_capacity(json_files.len());
        for entry in &json_files {
            if let Some(json) = WorkflowDataJson::new(entry).await {
                workflows.push(Workflow::new(json));
            }
        }

        Self {
            workflows_directory,
            workflows,
        }
    }
}

async fn read_json_files(directory_path: &PathBuf) -> Vec<DirEntry> {
    if !directory_path.exists() {
        return vec![];
    }

    let mut dir = match tokio::fs::read_dir(directory_path).await {
        Ok(dir) => dir,
        Err(e) => {
            log::error!("error reading workflows directory: {}", e);
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
                log::error!("error reading directory entry: {}", e);
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
