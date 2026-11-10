pub mod workflow;

//TODO: use tokio async filesystem operations
use std::{
    ffi::OsStr,
    fs::{self, DirEntry},
    path::PathBuf,
};

use crate::{cli::Cli, core::workflow::WorkflowDataJson};
use workflow::Workflow;

#[derive(Default)]
pub struct Core {
    pub workflows_directory: PathBuf,
    pub workflows: Vec<Workflow>,
}

impl Core {
    pub fn new(cli: Cli) -> Self {
        let workflows_directory = cli.workflows_path.expect("workflows path is required");

        if workflows_directory.exists() && !workflows_directory.is_dir() {
            panic!(
                "Workflows path already exists but is not a directory: \"{}\"",
                workflows_directory.display()
            );
        }

        let json_files = read_json_files(&workflows_directory);
        let workflows = json_files
            .iter()
            .filter_map(|entry| WorkflowDataJson::new(entry).map(Workflow::new))
            .collect();

        Self {
            workflows_directory,
            workflows,
        }
    }
}

fn read_json_files(directory_path: &PathBuf) -> Vec<DirEntry> {
    if !directory_path.exists() {
        return vec![];
    }

    let dir = fs::read_dir(directory_path);

    match dir {
        Ok(dir) => dir.filter_map(filter_json_files).collect(),
        Err(e) => {
            log::error!("error reading workflows directory: {}", e);
            vec![]
        }
    }
}

fn filter_json_files(entry: Result<DirEntry, std::io::Error>) -> Option<DirEntry> {
    match entry {
        Ok(entry) => {
            if entry.path().is_file() && entry.path().extension() == Some(OsStr::new("json")) {
                Some(entry)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}
