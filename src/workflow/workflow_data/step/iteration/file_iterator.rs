use anyhow::{Context as _, Result, anyhow};
use jsonc_parser::{ParseOptions, parse_to_serde_value};
use std::path::Path;
use tokio::fs;

use super::context;

// type SerdeJsonObject = serde_json::Map<String, serde_json::Value>;

struct FileContextItem {
    data: serde_json::Value,
}

impl context::WorkflowContextItem for FileContextItem {
    fn get_data(&self) -> &serde_json::Value {
        &self.data
    }

    fn get_value(&self, path: &context::PropertyPath) -> Option<serde_json::Value> {
        todo!()
    }

    fn update(&self, path: &context::PropertyPath, value: serde_json::Value) -> bool {
        todo!()
    }
}

impl FileContextItem {
    fn new(data: serde_json::Value) -> Self {
        Self { data }
    }
}

pub(super) async fn get_file_iterator_items(
    iterator: &super::FileIterator,
    workflow_file_path: &Path,
) -> Result<Vec<Box<dyn context::WorkflowContextItem>>> {
    let source_path: &Path = if iterator.source_file.is_relative() {
        &workflow_file_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(iterator.source_file.as_path())
    } else {
        iterator.source_file.as_path()
    };

    if !source_path.exists() || !source_path.is_file() {
        return Err(anyhow!(
            "Source file does not exist or is not a file: {}",
            source_path.display()
        ));
    }

    let content = fs::read_to_string(&source_path)
        .await
        .with_context(|| "Cannot read source file")?;

    let source_json =
        parse_to_serde_value::<serde_json::Value>(&content, &ParseOptions::default())?;

    if let serde_json::Value::Array(array) = source_json {
        Ok(array
            .into_iter()
            .map(|item| {
                Box::new(FileContextItem::new(item))
            })
            .collect::<Vec<Box<dyn context::WorkflowContextItem>>>())
    } else {
        Err(anyhow!(
            "Source file must be a json/jsonc file containing an array: {}",
            source_path.display()
        ))
    }
}
