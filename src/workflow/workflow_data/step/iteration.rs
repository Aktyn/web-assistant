mod file_iterator;

use anyhow::Result;
use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub(crate) use super::super::super::context;

/// `ForEachStep` - iterates over each item in given data
#[derive(Debug, Serialize, Deserialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct ForEachStep {
    /// Name of the context used to store the iterating item
    #[serde(alias = "contextName", rename = "contextName")]
    pub context_name: String,

    /// Specifies external source of items
    /// Note that source file can be modified
    /// with "update-context" step
    /// Contains "type" property (enum)
    /// See specific fields for each type:
    /// "file" --> `FileIterator`
    pub items: IterationItems,

    /// Whether to preserve or reset context after each iteration completes
    #[serde(
        default = "bool::default",
        alias = "preserveContext",
        rename = "preserveContext"
    )]
    pub preserve_context: bool,

    /// List of steps to run in each iteration
    pub steps: Vec<super::WorkflowStepType>,

    /// Steps to perform when iteration fails
    /// For example, can be used to abort the workflow
    /// Note that error inside `on_error` steps will cause workflow failure
    /// Omit this property to continue iterating after error
    #[serde(alias = "onError", rename = "onError")]
    pub on_error: Option<Vec<super::WorkflowStepType>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum IterationItems {
    File(FileIterator),
}

/// `FileIterator` - iterates over each item in a file
#[derive(Debug, Deserialize, Serialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct FileIterator {
    /// Path to the file to iterate over
    #[serde(alias = "sourceFile", rename = "sourceFile")]
    pub source_file: PathBuf,
}

pub(crate) async fn get_iteration_items(
    items: &IterationItems,
    // Used for file iterator if source_file is relative
    workflow_file_path: &Path,
) -> Result<Vec<Box<dyn context::WorkflowContextItem>>> {
    match items {
        IterationItems::File(file_iterator) => {
            file_iterator::get_file_iterator_items(file_iterator, workflow_file_path).await
        }
    }
}
