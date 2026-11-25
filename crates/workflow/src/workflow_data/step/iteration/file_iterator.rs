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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_data::step::iteration::FileIterator;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_get_file_iterator_items_valid_file() {
        let tmp = TempDir::new().unwrap();
        let wf_path = tmp.path().join("workflow.json");
        std::fs::write(&wf_path, r#"{"name":"test","steps":[]}"#).unwrap();

        let data_path = tmp.path().join("data.json");
        std::fs::write(&data_path, r#"[{"name":"Alice"},{"name":"Bob"}]"#).unwrap();

        let iterator = FileIterator {
            source_file: data_path.clone().into(),
        };
        let items = get_file_iterator_items(&iterator, &wf_path).await.unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_get_file_iterator_items_nonexistent_file() {
        let tmp = TempDir::new().unwrap();
        let wf_path = tmp.path().join("workflow.json");
        std::fs::write(&wf_path, r#"{}"#).unwrap();

        let iterator = FileIterator {
            source_file: PathBuf::from("nonexistent.json"),
        };
        let result = get_file_iterator_items(&iterator, &wf_path).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("does not exist or is not a file")
        );
    }

    #[tokio::test]
    async fn test_get_file_iterator_items_non_array_json() {
        let tmp = TempDir::new().unwrap();
        let wf_path = tmp.path().join("workflow.json");
        std::fs::write(&wf_path, r#"{}"#).unwrap();

        let data_path = tmp.path().join("data.json");
        std::fs::write(&data_path, r#"{"key":"value"}"#).unwrap();

        let iterator = FileIterator {
            source_file: data_path.clone().into(),
        };
        let result = get_file_iterator_items(&iterator, &wf_path).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must be a json/jsonc file containing an array")
        );
    }

    #[tokio::test]
    async fn test_get_file_iterator_items_relative_path_resolves_correctly() {
        let tmp = TempDir::new().unwrap();
        let wf_subdir = tmp.path().join("workflows");
        std::fs::create_dir_all(&wf_subdir).unwrap();
        let wf_path = wf_subdir.join("workflow.json");
        std::fs::write(&wf_path, r#"{}"#).unwrap();

        let data_path = wf_subdir.join("data.json");
        std::fs::write(&data_path, r#"[1, 2, 3]"#).unwrap();

        let iterator = FileIterator {
            source_file: PathBuf::from("data.json"),
        };
        let items = get_file_iterator_items(&iterator, &wf_path).await.unwrap();
        assert_eq!(items.len(), 3);
    }

    #[tokio::test]
    async fn test_get_file_iterator_items_absolute_path() {
        let tmp = TempDir::new().unwrap();
        let data_path = tmp.path().join("data.json");
        std::fs::write(&data_path, r#"[{"x":1}]"#).unwrap();

        let wf_path = tmp.path().join("workflow.json");
        std::fs::write(&wf_path, r#"{}"#).unwrap();

        let iterator = FileIterator {
            source_file: data_path.clone().into(),
        };
        let items = get_file_iterator_items(&iterator, &wf_path).await.unwrap();
        assert_eq!(items.len(), 1);
    }

    #[tokio::test]
    async fn test_file_context_item_can_update_and_write_back() {
        let tmp = TempDir::new().unwrap();
        let wf_path = tmp.path().join("workflow.json");
        std::fs::write(&wf_path, r#"{}"#).unwrap();

        let data_path = tmp.path().join("data.json");
        std::fs::write(&data_path, r#"[{"count":0}]"#).unwrap();

        let iterator = FileIterator {
            source_file: data_path.clone().into(),
        };
        let mut items = get_file_iterator_items(&iterator, &wf_path).await.unwrap();
        assert_eq!(items.len(), 1);

        // Update the item via context API
        let path: context::PropertyPath = vec!["count"];
        let updated = items[0].update(&path, &serde_json::json!(42)).unwrap();
        assert_eq!(updated.as_u64().unwrap(), 42);

        // Verify file was written back
        let content = std::fs::read_to_string(&data_path).unwrap();
        assert!(content.contains("42"));
    }

    #[tokio::test]
    async fn test_file_context_item_shared_array_visibility() {
        let tmp = TempDir::new().unwrap();
        let wf_path = tmp.path().join("workflow.json");
        std::fs::write(&wf_path, r#"{}"#).unwrap();

        let data_path = tmp.path().join("data.json");
        std::fs::write(&data_path, r#"[{"val":"a"},{"val":"b"}]"#).unwrap();

        let iterator = FileIterator {
            source_file: data_path.clone().into(),
        };
        let mut items = get_file_iterator_items(&iterator, &wf_path).await.unwrap();
        assert_eq!(items.len(), 2);

        // Mutate first item
        let path: context::PropertyPath = vec!["val"];
        items[0]
            .update(&path, &serde_json::json!("mutated"))
            .unwrap();

        // Second item should see the file contents still reflect its own original value
        // (only item 0 was mutated)
        assert_eq!(
            items[1].get_data().get("val").unwrap().as_str().unwrap(),
            "b"
        );
    }
}
