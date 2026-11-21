use std::path::PathBuf;

use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};

/// `ForEachStep` - iterates over each item in given data
#[derive(Debug, Serialize, Deserialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct ForEachStep {
    /// Name of the context used to store the iterating item
    #[serde(alias = "contextName", rename = "contextName")]
    context_name: String,

    /// Specifies external source of items
    /// Note that source file can be modified
    /// with "update-context" step
    /// Contains "type" property (enum)
    /// See specific fields for each type:
    /// "file" --> `FileIterator`
    items: IterationItems,

    /// Whether to preserve or reset context after each iteration completes
    #[serde(
        default = "bool::default",
        alias = "preserveContext",
        rename = "preserveContext"
    )]
    preserve_context: bool,

    /// List of steps to run in each iteration
    steps: Vec<super::WorkflowStepType>,
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
