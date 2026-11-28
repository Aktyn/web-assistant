use anyhow::{Result, bail};
use config::BrowserConfig;
use std::path::{Path, PathBuf};

use super::chrome;
use super::firefox;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowserType {
    Chrome,
    Firefox,
}

pub(crate) struct BrowserInfo {
    pub browser_type: BrowserType,
    pub executable_path: PathBuf,
    pub user_data_dir: Option<PathBuf>,
    pub default_args: &'static Vec<String>,
}

pub(crate) fn get_browser_info(config: &BrowserConfig) -> Result<BrowserInfo> {
    let executable_path = if let Some(path) = &config.executable_path {
        path.clone()
    } else if let Ok(p) = chrome::find_executable() {
        p
    } else {
        bail!("Could not determine browser executable path")
    };

    if !executable_path.exists() || !executable_path.is_file() {
        bail!("Browser executable path does not exist or is not a file");
    }

    match determine_browser_type(&executable_path)? {
        BrowserType::Chrome => {
            let user_data_dir: Option<PathBuf> = match chrome::resolve_user_data_dir(
                Some(&executable_path),
                config.user_data_dir.as_deref(),
            ) {
                Ok(dir) => Some(dir),
                Err(error) => {
                    log::error!("{error}");
                    None
                }
            };

            Ok(BrowserInfo {
                browser_type: BrowserType::Chrome,
                executable_path,
                user_data_dir,
                default_args: &chrome::DEFAULT_ARGS,
            })
        }
        BrowserType::Firefox => {
            let user_data_dir: Option<PathBuf> = match firefox::resolve_user_data_dir(
                Some(&executable_path),
                config.user_data_dir.as_deref(),
            ) {
                Ok(dir) => Some(dir),
                Err(error) => {
                    log::error!("{error}");
                    None
                }
            };

            Ok(BrowserInfo {
                browser_type: BrowserType::Firefox,
                executable_path,
                user_data_dir,
                default_args: &firefox::DEFAULT_ARGS,
            })
        }
    }
}

/// Determines whether an executable is a Chrome/Chromium or Firefox build.
///
/// Detection is attempted in two steps:
/// 1. A fast, dependency-free heuristic based on the executable file name
///    (covers the well-known distribution names on every platform).
/// 2. A fallback that asks the executable for its own version string via
///    `--version` and inspects the output, for custom or renamed builds.
///
/// Returns an error if the executable cannot be identified as either.
fn determine_browser_type(executable_path: &Path) -> Result<BrowserType> {
    if let Some(kind) = browser_type_from_version(executable_path) {
        return Ok(kind);
    }

    if let Some(kind) = browser_type_from_name(executable_path) {
        return Ok(kind);
    }

    bail!(
        "Could not determine browser type for '{}': not a recognized Chrome/Chromium or Firefox executable",
        executable_path.display()
    )
}

#[must_use]
fn browser_type_from_name(executable_path: &Path) -> Option<BrowserType> {
    let name = executable_path
        .file_name()
        .and_then(|n| n.to_str())?
        .to_lowercase();

    // "chrom" matches both "chrome" and "chromium"; "brave" is Chromium-based.
    if name.contains("chrom") || name.contains("brave") {
        Some(BrowserType::Chrome)
    } else if name.contains("firefox") {
        Some(BrowserType::Firefox)
    } else {
        None
    }
}

fn browser_type_from_version(executable_path: &Path) -> Option<BrowserType> {
    let output = match std::process::Command::new(executable_path)
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            log::debug!(
                "Browser version check '{} --version' failed to run: {error}",
                executable_path.display()
            );
            return None;
        }
    };

    let text = format!(
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .to_lowercase();

    if text.contains("firefox") || text.contains("mozilla") {
        Some(BrowserType::Firefox)
    } else if text.contains("chrom") {
        Some(BrowserType::Chrome)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_heuristic_detects_chromium_family() {
        for name in [
            "google-chrome",
            "google-chrome-stable",
            "chromium",
            "chromium-browser",
            "brave-browser",
        ] {
            assert_eq!(
                browser_type_from_name(Path::new(name)),
                Some(BrowserType::Chrome),
                "expected Chrome/Chromium for '{name}'"
            );
        }
    }

    #[test]
    fn name_heuristic_detects_firefox() {
        for name in ["firefox", "firefox-esr", "firefox.exe"] {
            assert_eq!(
                browser_type_from_name(Path::new(name)),
                Some(BrowserType::Firefox),
                "expected Firefox for '{name}'"
            );
        }
    }

    #[test]
    fn name_heuristic_is_case_insensitive_and_ignores_extension() {
        // macOS ships the binary as "Google Chrome" (with a space, no extension)
        // and "Firefox"; lowercase matching must still classify them.
        assert_eq!(
            browser_type_from_name(Path::new("Google Chrome")),
            Some(BrowserType::Chrome)
        );
        assert_eq!(
            browser_type_from_name(Path::new("Firefox")),
            Some(BrowserType::Firefox)
        );
    }

    #[test]
    fn name_heuristic_is_inconclusive_for_unknown_names() {
        assert_eq!(
            browser_type_from_name(Path::new("/opt/some/unknown-browser")),
            None,
            "an unrecognized name should defer to the version check"
        );
    }

    // These fast-path tests never reach the `--version` fallback because the
    // file-name heuristic resolves them first, so no process is spawned.
    #[test]
    fn determine_browser_type_classifies_known_paths() {
        assert_eq!(
            determine_browser_type(Path::new("/usr/bin/google-chrome")).unwrap(),
            BrowserType::Chrome
        );
        assert_eq!(
            determine_browser_type(Path::new("/usr/bin/firefox")).unwrap(),
            BrowserType::Firefox
        );
    }
}
