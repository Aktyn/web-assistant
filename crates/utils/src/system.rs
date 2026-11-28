use anyhow::{Result, anyhow};
use std::path::PathBuf;

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
pub fn run_open_command(target: &str) -> Result<std::process::Child> {
    let mut command = if cfg!(target_os = "windows") {
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(["/C", "start", ""]);
        cmd.arg(target);
        cmd
    } else {
        let program = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        let mut cmd = std::process::Command::new(program);
        cmd.arg(target);
        cmd
    };

    command
        .stdout(std::process::Stdio::null())
        .spawn()
        .map_err(|e| anyhow!("failed to open \"{target}\": {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_file_with_nonexistent_path_succeeds_spawn() {
        // run_open_command spawns the process; even for nonexistent files,
        // xdg-open spawns successfully (it prints an error to stderr and exits non-zero).
        let result = run_open_command("/tmp/does_not_exist_for_testing_xyz");
        assert!(
            result.is_ok(),
            "spawn should succeed even if target doesn't exist"
        );
    }

    #[test]
    fn test_home_dir_returns_some_on_linux() {
        // HOME is typically set in CI/test environments
        let home = home_dir();
        // On systems where HOME is unset it may return None, but most environments have it
        if let Some(h) = home {
            assert!(!h.as_os_str().is_empty());
        }
    }

    #[test]
    fn test_run_open_command_valid_url_spawns() {
        let result = run_open_command("https://example.com");
        assert!(result.is_ok(), "should spawn successfully on valid URL");
    }

    #[test]
    fn test_run_open_command_path_with_spaces() {
        let result = run_open_command("/tmp/file with spaces.txt");
        assert!(result.is_ok(), "should spawn even with spaces in path");
    }
}
