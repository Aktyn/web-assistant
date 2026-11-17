use anyhow::{Result, anyhow, bail};
use std::{
    io::{Write, stdout},
    path::{Path, PathBuf},
};
use tokio::fs::DirEntry;

//TODO: organize these utils into text utils and file utils

#[must_use]
pub fn truncate_text(text: &str, limit: usize, suffix: Option<&str>) -> String {
    truncate(text, limit, suffix, false)
}

#[must_use]
pub fn truncate_text_left(text: &str, limit: usize, suffix: Option<&str>) -> String {
    truncate(text, limit, suffix, true)
}

fn truncate(text: &str, limit: usize, suffix: Option<&str>, from_left: bool) -> String {
    let char_count = text.chars().count();
    if char_count <= limit {
        return text.to_string();
    }

    let suffix_ = suffix.unwrap_or("...");
    let suffix_char_count = suffix_.chars().count();
    // If the suffix itself is longer than or equal to the limit, just return the suffix.
    if suffix_char_count >= limit {
        return suffix_.to_string();
    }
    let keep = limit - suffix_char_count;
    if from_left {
        format!(
            "{}{}",
            suffix_,
            text.chars().skip(char_count - keep).collect::<String>(),
        )
    } else {
        format!("{}{}", text.chars().take(keep).collect::<String>(), suffix_)
    }
}

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

/// Returns the user's home directory across Linux, macOS, and Windows.
#[must_use]
pub fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}

/// Runs system command that opens given file in its default application
/// # Errors
/// Returns an error if the command fails
pub fn open_file(path: &Path) -> Result<std::process::Child> {
    let absolute_path = path
        .canonicalize()
        .map_err(|e| anyhow!("failed to get absolute path of \"{}\": {e}", path.display()))?;

    let mut command = if cfg!(target_os = "windows") {
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(["/C", "start", ""]);
        cmd.arg(&absolute_path);
        cmd
    } else {
        let program = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        let mut cmd = std::process::Command::new(program);
        cmd.arg(&absolute_path);
        cmd
    };

    command
        .stdout(std::process::Stdio::null())
        .spawn()
        .map_err(|e| anyhow!("failed to open \"{}\": {e}", absolute_path.display()))
}

pub fn print_lines(count: u16) {
    stdout().write_all(&b"\n".repeat(count.into())).ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_text_short_text_unchanged() {
        let text = "hello";
        assert_eq!(truncate_text(text, 10, None), "hello");
    }

    #[test]
    fn test_truncate_text_exact_limit_unchanged() {
        let text = "hello";
        assert_eq!(truncate_text(text, 5, None), "hello");
    }

    #[test]
    fn test_truncate_text_truncates_with_default_suffix() {
        let text = "hello world this is long";
        let result = truncate_text(text, 10, None);
        assert!(result.ends_with("..."));
        assert_eq!(result.chars().count(), 10);
        assert!(result.starts_with("hello w"));
    }

    #[test]
    fn test_truncate_text_custom_suffix() {
        let text = "hello world this is long";
        let result = truncate_text(text, 15, Some("[..]"));
        assert_eq!(result, "hello world[..]");
    }

    #[test]
    fn test_truncate_text_empty_input() {
        assert_eq!(truncate_text("", 5, None), "");
    }

    #[test]
    fn test_truncate_text_empty_input_with_limit_zero() {
        // Empty text with any limit returns the empty text unchanged.
        assert_eq!(truncate_text("", 0, None), "");
    }

    #[test]
    fn test_truncate_text_unicode_chars_counted_correctly() {
        // Each emoji is one char; limit is in chars, not bytes.
        let text = "😀😀😀😀😀";
        let result = truncate_text(text, 3, None);
        // keep = 3 - 3 = 0, so only the suffix remains.
        assert_eq!(result, "...");
    }

    #[test]
    fn test_truncate_text_left_short_text_unchanged() {
        assert_eq!(truncate_text_left("hello", 10, None), "hello");
    }

    #[test]
    fn test_truncate_text_left_exact_limit_unchanged() {
        assert_eq!(truncate_text_left("hello", 5, None), "hello");
    }

    #[test]
    fn test_truncate_text_left_truncates_left_side() {
        let text = "hello world this is long";
        let result = truncate_text_left(text, 10, None);
        assert!(result.starts_with("..."));
        assert_eq!(result.chars().count(), 10);
        assert!(result.ends_with("s long"));
    }

    #[test]
    fn test_truncate_text_left_custom_suffix() {
        let text = "hello world this is long";
        let result = truncate_text_left(text, 18, Some("[..]"));
        assert_eq!(result, "[..]d this is long");
    }

    #[test]
    fn test_truncate_text_left_empty_input() {
        assert_eq!(truncate_text_left("", 5, None), "");
    }

    #[test]
    fn test_truncate_text_left_unicode_chars_counted_correctly() {
        let text = "😀😀😀😀😀";
        let result = truncate_text_left(text, 3, None);
        // keep = 3 - 3 = 0, so only the suffix remains.
        assert_eq!(result, "...");
    }

    #[test]
    fn test_truncate_text_no_suffix_when_fits() {
        let text = "exactly10chars";
        // 14 chars > 13 limit, so it truncates with the custom suffix.
        let result = truncate_text(text, 13, Some("XYZ"));
        assert_eq!(result, "exactly10cXYZ");
    }

    #[test]
    fn test_truncate_text_suffix_longer_than_limit_returns_suffix() {
        // Regression test: previously `limit - suffix_.len()` underflowed in debug.
        let text = "hello world this is long";
        let result = truncate_text(text, 3, Some("VERY_LONG_SUFFIX"));
        assert_eq!(result, "VERY_LONG_SUFFIX");
    }

    #[test]
    fn test_truncate_text_left_suffix_longer_than_limit_returns_suffix() {
        let text = "hello world this is long";
        let result = truncate_text_left(text, 3, Some("VERY_LONG_SUFFIX"));
        assert_eq!(result, "VERY_LONG_SUFFIX");
    }

    #[test]
    fn test_truncate_text_default_suffix_at_limit_3() {
        let text = "hello world";
        // limit 3, default suffix "..." (len 3) → keep = 0, only suffix returned.
        let result = truncate_text(text, 3, None);
        assert_eq!(result, "...");
    }

    #[test]
    fn test_truncate_text_left_default_suffix_at_limit_3() {
        let text = "hello world";
        let result = truncate_text_left(text, 3, None);
        assert_eq!(result, "...");
    }

    /// Test that truncation preserves ASCII character boundaries.
    #[test]
    fn test_truncate_text_boundary_preservation() {
        let text = "abcdef";
        // limit=4, suffix=".." → keep=2 chars + ".."
        let result = truncate_text(text, 4, Some(".."));
        assert_eq!(result, "ab..");
    }

    /// Test left-truncation boundary preservation.
    #[test]
    fn test_truncate_text_left_boundary_preservation() {
        let text = "abcdef";
        let result = truncate_text_left(text, 4, Some(".."));
        assert_eq!(result, "..ef");
    }

    /// Edge case: limit equals suffix length returns just suffix.
    #[test]
    fn test_truncate_text_limit_equals_suffix_len() {
        let text = "longer than suffix";
        let result = truncate_text(text, 4, Some("----"));
        assert_eq!(result, "----");
    }

    /// Empty text should always return empty regardless of settings.
    #[test]
    fn test_truncate_empty_text_any_settings() {
        assert_eq!(truncate_text("", 5, Some("x")), "");
        assert_eq!(truncate_text_left("", 5, Some("x")), "");
    }

    /// Single char with limit 1 returns the char unchanged.
    #[test]
    fn test_single_char_returns_as_is() {
        assert_eq!(truncate_text("a", 1, None), "a");
        assert_eq!(truncate_text_left("a", 1, None), "a");
    }

    /// Large limit beyond string length returns original.
    #[test]
    fn test_large_limit_returns_original() {
        let text = "short";
        assert_eq!(truncate_text(text, 100, None), text);
        assert_eq!(truncate_text_left(text, 100, None), text);
    }

    #[tokio::test]
    async fn test_read_json_files_nonexistent_directory_returns_empty() {
        let nonexistent = PathBuf::from("/tmp/does_not_exist_xyz_12345");
        let result = read_json_files(&nonexistent).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_open_file_with_nonexistent_path_fails() {
        let result = open_file(Path::new("/tmp/does_not_exist_for_testing_xyz"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("failed to get absolute path")
                || err.to_string().contains("No such file")
        );
    }
}
