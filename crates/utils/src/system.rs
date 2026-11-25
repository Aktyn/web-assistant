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
}
