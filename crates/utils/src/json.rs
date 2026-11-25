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
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_read_json_files_nonexistent_directory_returns_empty() {
        let nonexistent = PathBuf::from("/tmp/does_not_exist_xyz_12345");
        let result = read_json_files(&nonexistent).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_json_files_empty_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let result = read_json_files(tmp.path()).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_read_json_files_finds_json_files() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.json"), "{}").unwrap();
        std::fs::write(tmp.path().join("b.jsonc"), "{}").unwrap();
        std::fs::write(tmp.path().join("c.txt"), "hello").unwrap();

        let result = read_json_files(tmp.path()).await.unwrap();
        // Only .json files should be returned (not .txt or directories)
        let names: Vec<_> = result.iter().map(|e| e.file_name()).collect();
        assert!(
            names
                .iter()
                .any(|n| n.to_string_lossy().as_bytes() == b"a.json")
        );
        assert!(
            names
                .iter()
                .any(|n| n.to_string_lossy().as_bytes() == b"b.jsonc")
        );
        assert!(
            !names
                .iter()
                .any(|n| n.to_string_lossy().as_bytes() == b"c.txt")
        );
    }

    #[test]
    fn test_json_parse_valid_json_object() {
        let v = json_parse(r#"{"key":"value"}"#);
        assert!(v.is_object());
        assert_eq!(v.get("key").unwrap().as_str().unwrap(), "value");
    }

    #[test]
    fn test_json_parse_valid_json_array() {
        let v = json_parse("[1, 2, 3]");
        assert!(v.is_array());
        assert_eq!(v[0].as_u64().unwrap(), 1);
    }

    #[test]
    fn test_json_parse_string_fallback_for_invalid_json() {
        let v = json_parse("not valid json at all");
        assert!(v.is_string());
        assert_eq!(v.as_str().unwrap(), "not valid json at all");
    }

    #[test]
    fn test_json_parse_numeric_string() {
        let v = json_parse("42");
        assert!(v.is_number());
        assert_eq!(v.as_i64().unwrap(), 42);
    }

    #[test]
    fn test_json_parse_bool_string() {
        let v = json_parse("true");
        assert!(v.is_boolean());
        assert_eq!(v.as_bool().unwrap(), true);
    }
}
