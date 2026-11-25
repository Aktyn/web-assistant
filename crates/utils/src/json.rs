use anyhow::{Result, bail};
use std::path::Path;
use tokio::fs::DirEntry;

/// Returns list of JSON files found in the given directory
/// # Errors
/// This function will return an error if the directory does not exist or is not a directory.
pub async fn read_json_files(directory_path: &Path) -> Result<Vec<DirEntry>> {
    if !directory_path.exists() {
        bail!("directory does not exist: {}", directory_path.display());
    }

    let mut dir = tokio::fs::read_dir(directory_path).await?;

    let mut entries = Vec::new();
    loop {
        match dir.next_entry().await {
            Ok(Some(entry)) => {
                if filter_entry_by_extension(&entry, "json").await {
                    entries.push(entry);
                }
            }
            Ok(None) => break,
            Err(e) => {
                log::error!("error reading directory entry: {e}");
            }
        }
    }
    Ok(entries)
}

async fn filter_entry_by_extension(entry: &DirEntry, extension: &str) -> bool {
    let is_file = match entry.file_type().await {
        Ok(file_type) => file_type.is_file(),
        Err(_) => return false,
    };

    is_file && entry.path().extension().and_then(|ext| ext.to_str()) == Some(extension)
}

#[must_use]
pub fn json_parse(json_string: &str) -> serde_json::Value {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_string) {
        return v;
    }

    // Fall back to string
    serde_json::Value::String(json_string.to_string())
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_read_json_files_nonexistent_directory_returns_empty() {
        let nonexistent = PathBuf::from("/tmp/does_not_exist_xyz_12345");
        let result = read_json_files(&nonexistent).await;
        assert!(result.is_err());
    }
}
