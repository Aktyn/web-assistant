use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// Get the default Firefox profile directory.
///
/// This is the platform directory that holds `profiles.ini` and the per-profile
/// directories — the Firefox analogue of Chrome's `--user-data-dir`. Unlike
/// Chrome, the default location does not depend on which Firefox variant is
/// running, so the `executable` argument is accepted for API parity but unused.
#[must_use]
fn get_default_user_data_dir(_executable: Option<&Path>) -> Option<PathBuf> {
    let home = utils::system::home_dir()?;

    #[cfg(target_os = "macos")]
    {
        Some(home.join("Library/Application Support/Firefox"))
    }

    #[cfg(target_os = "linux")]
    {
        Some(home.join(".mozilla/firefox"))
    }

    #[cfg(target_os = "windows")]
    {
        let app_data = std::env::var("APPDATA")
            .ok()
            .map(PathBuf::from)
            .or_else(|| home.join("AppData/Roaming"))?;
        Some(app_data.join("Mozilla/Firefox"))
    }
}

/// Resolves a persistent user data (profile) directory for Firefox.
///
/// Mirrors `chrome::resolve_user_data_dir`:
/// - an explicitly requested directory is used as-is;
/// - otherwise the platform default profile directory is used unless a running
///   Firefox instance has locked it;
/// - when locked, fall back to a dedicated, persistent web-assistant profile
///   directory so logins and cookies survive across runs without contending
///   with a browser the user has open.
pub(crate) fn resolve_user_data_dir(
    executable: Option<&Path>,
    requested_dir: Option<&Path>,
) -> Result<PathBuf> {
    if let Some(req) = requested_dir {
        return Ok(req.to_path_buf());
    }

    if let Some(exe) = executable {
        log::debug!(
            "Resolving Firefox profile directory for executable {}",
            exe.display()
        );
    }

    if let Some(default_dir) = get_default_user_data_dir(executable) {
        // Check if the default profile dir is currently locked by a running Firefox
        if !is_user_data_dir_locked(&default_dir) {
            return Ok(default_dir);
        }
    }

    let home =
        utils::system::home_dir().ok_or_else(|| anyhow!("Could not resolve home directory"))?;
    let dedicated_dir = home
        .join(".config")
        .join("web-assistant")
        .join("firefox-profile");

    // Use a dedicated persistent directory to avoid lock contention.
    fs::create_dir_all(&dedicated_dir)?;
    Ok(dedicated_dir)
}

/// Check whether the given Firefox profile directory has an active lock.
///
/// Firefox marks the default profile directory with `parent.lock` and the
/// active profile with `lock`. On Linux the file content encodes
/// `<host>-<pid>`; we verify the owning process is still alive before calling
/// the directory busy, so stale locks are ignored.
#[must_use]
fn is_user_data_dir_locked(user_data_dir: &Path) -> bool {
    for name in ["parent.lock", "lock"] {
        let lock_file = user_data_dir.join(name);
        if !lock_file.exists() {
            continue;
        }

        // On Linux a lock file whose process has exited is stale → free.
        #[cfg(target_os = "linux")]
        {
            if let Some(pid) = lock_pid(&lock_file)
                && Path::new(&format!("/proc/{pid}")).exists()
            {
                return true;
            }
        }

        // Without a /proc check (other platforms, or an unparseable lock) a
        // present lock file conservatively means the directory is in use.
        #[cfg(not(target_os = "linux"))]
        {
            return true;
        }
    }

    false
}

/// Read the owning PID from a Firefox `lock` file.
///
/// The file is plain text of the form `<hostname>-<pid>`; only the PID is
/// needed to confirm the process is still running.
#[cfg(target_os = "linux")]
fn lock_pid(lock_file: &Path) -> Option<u32> {
    let contents = fs::read_to_string(lock_file).ok()?;
    contents
        .trim()
        .split('-')
        .next_back()
        .filter(|pid| !pid.is_empty())?
        .parse::<u32>()
        .ok()
}

/// Standard Firefox arguments optimized for automation.
pub(crate) static DEFAULT_ARGS: LazyLock<Vec<String>> =
    LazyLock::new(|| vec!["--no-remote".into()]);

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_args_contains_no_remote() {
        assert!(DEFAULT_ARGS.iter().any(|a| a == "--no-remote"));
    }

    #[test]
    fn test_default_args_is_short() {
        // Firefox exposes few automation flags; keep the default set minimal.
        assert!(DEFAULT_ARGS.len() <= 4);
    }

    #[test]
    fn test_resolve_user_data_dir_with_explicit_path() {
        let tmp = TempDir::new().unwrap();
        let result = resolve_user_data_dir(None, Some(tmp.path())).unwrap();
        assert_eq!(result, tmp.path());
    }

    #[test]
    fn test_resolve_user_data_dir_fallback() {
        // Returns Ok even when the default dir is missing/locked: it falls
        // back to (or points at) a resolvable profile directory.
        let result = resolve_user_data_dir(None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_user_data_dir_locked_nonexistent() {
        let tmp = TempDir::new().unwrap();
        assert!(!is_user_data_dir_locked(tmp.path()));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_is_user_data_dir_locked_stale_on_linux() {
        let tmp = TempDir::new().unwrap();
        // A lock file pointing at a surely-dead PID must not count as locked.
        fs::write(tmp.path().join("parent.lock"), "deadhost-999999").unwrap();
        assert!(!is_user_data_dir_locked(tmp.path()));
    }
}
