use anyhow::{Context as _, Result, anyhow};
use jsonc_parser::{ParseOptions, parse_to_serde_value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::context;

#[derive(Debug)]
struct FileContextItem {
    data: serde_json::Value,
    /// Shared in-memory array. All items from the same source file share this
    /// so that updates to one item remain visible to others without round-tripping
    /// through the filesystem on every write.
    array: Arc<Mutex<Vec<serde_json::Value>>>,
    /// Index of the current item within the shared array.
    index: usize,
    file_path: PathBuf,
}

impl context::WorkflowContextItem for FileContextItem {
    fn get_data(&self) -> &serde_json::Value {
        &self.data
    }

    fn update(
        &mut self,
        path: &context::PropertyPath,
        value: &serde_json::Value,
    ) -> Result<&serde_json::Value> {
        self.data = context::set_nested_property(&self.data, path, value.clone());

        if let Ok(mut arr) = self.array.lock()
            && self.index < arr.len()
        {
            arr[self.index] = self.data.clone();
            log::info!("Updating file: {}", self.file_path.display());

            let json_array_content = serde_json::to_string_pretty(&*arr)?;
            std::fs::write(&self.file_path, &json_array_content)
                .context("Failed to write updated file")?;
        }

        Ok(self.get_data())
    }
}

impl FileContextItem {
    fn new(
        data: serde_json::Value,
        array: Arc<Mutex<Vec<serde_json::Value>>>,
        index: usize,
        file_path: &Path,
    ) -> Self {
        Self {
            data,
            array,
            index,
            file_path: file_path.to_path_buf(),
        }
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

    let content = tokio::fs::read_to_string(&source_path)
        .await
        .with_context(|| "Cannot read source file")?;

    let source_json =
        parse_to_serde_value::<serde_json::Value>(&content, &ParseOptions::default())?;

    if let serde_json::Value::Array(array) = source_json {
        let shared_array: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(array));

        let items = shared_array
            .lock()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let context_item =
                    FileContextItem::new(item.clone(), shared_array.clone(), index, source_path);
                Box::new(context_item) as Box<dyn context::WorkflowContextItem>
            })
            .collect();

        Ok(items)
    } else {
        Err(anyhow!(
            "Source file must be a json/jsonc file containing an array: {}",
            source_path.display()
        ))
    }
}
