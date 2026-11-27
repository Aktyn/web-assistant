mod file_iterator;

use anyhow::Result;
use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use strum::{EnumDiscriminants, EnumIter};

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

    /// Order of the iteration
    /// Possible values are: default, reversed, random
    #[serde(default = "IterationOrder::default")]
    pub order: IterationOrder,

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

#[derive(Debug, Deserialize, Serialize, Clone, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter))]
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

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum IterationOrder {
    #[default]
    Default,
    Reversed,
    Random,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_for_each_step_deserialize_minimal() {
        let json = r#"{"contextName":"item","items":{"type":"file","sourceFile":"data.json"},"steps":[],
"preserveContext":false}"#;
        let step: ForEachStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.context_name, "item");
        assert!(matches!(step.items, IterationItems::File(_)));
        assert!(!step.preserve_context);
    }

    #[test]
    fn test_for_each_step_preserve_context_default_false() {
        let json = r#"{"contextName":"i","items":{"type":"file","sourceFile":"f"},"steps":[]}"#;
        let step: ForEachStep = serde_json::from_str(json).unwrap();
        assert!(!step.preserve_context);
    }

    #[test]
    fn test_for_each_step_preserve_context_true() {
        let json = r#"{"contextName":"i","items":{"type":"file","sourceFile":"f"},"steps":[],"preserveContext":true}"#;
        let step: ForEachStep = serde_json::from_str(json).unwrap();
        assert!(step.preserve_context);
    }

    #[test]
    fn test_for_each_step_on_error_steps() {
        let json = r#"{"contextName":"i","items":{"type":"file","sourceFile":"f"},"steps":[],"onError":[{"type":"abort","message":"fail"}]}"#;
        let step: ForEachStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.on_error.as_ref().map(|v| v.len()), Some(1));
    }

    #[test]
    fn test_for_each_step_clone() {
        let step = ForEachStep {
            context_name: "x".to_string(),
            items: IterationItems::File(FileIterator {
                source_file: PathBuf::from("/tmp/data.json"),
            }),
            preserve_context: false,
            order: IterationOrder::default(),
            steps: vec![],
            on_error: None,
        };
        let cloned = step.clone();
        assert_eq!(cloned.context_name, step.context_name);
    }

    #[test]
    fn test_iteration_items_unknown_type_fails() {
        let json = r#"{"type":"unknown-source"}"#;
        let result: Result<IterationItems, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_file_iterator_roundtrip() {
        let iter = FileIterator {
            source_file: PathBuf::from("/tmp/items.csv"),
        };
        let json = serde_json::to_string(&iter).unwrap();
        let parsed: FileIterator = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.source_file, iter.source_file);
    }

    #[test]
    fn test_for_each_step_full_roundtrip() {
        let step = ForEachStep {
            context_name: "item".to_string(),
            items: IterationItems::File(FileIterator {
                source_file: PathBuf::from("data.json"),
            }),
            preserve_context: true,
            order: IterationOrder::Reversed,
            steps: vec![],
            on_error: Some(vec![]),
        };
        let json = serde_json::to_string(&step).unwrap();
        let parsed: ForEachStep = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.context_name, "item");
        assert!(parsed.preserve_context);
        assert!(matches!(parsed.order, IterationOrder::Reversed));
    }
}
