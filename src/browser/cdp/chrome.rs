use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct ChromeProfileInfo {
    pub name: String,
    pub directory: String,
}

pub struct Chrome;

impl Chrome {
    /// Find Chrome or Chromium executable on the system
    pub fn find_executable() -> Result<PathBuf> {
        if let Ok(path) = std::env::var("CHROME_PATH") {
            let p = PathBuf::from(path);
            if p.exists() {
                return Ok(p);
            }
        }

        #[cfg(target_os = "macos")]
        {
            let candidates = [
                "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
                "/Applications/Chromium.app/Contents/MacOS/Chromium",
                "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary",
                "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
            ];
            for candidate in candidates {
                let p = PathBuf::from(candidate);
                if p.exists() {
                    return Ok(p);
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            let candidates = [
                "/usr/bin/google-chrome",
                "/usr/bin/google-chrome-stable",
                "/usr/bin/chromium",
                "/usr/bin/chromium-browser",
                "/snap/bin/chromium",
                "/usr/bin/brave-browser",
            ];
            for candidate in candidates {
                let p = PathBuf::from(candidate);
                if p.exists() {
                    return Ok(p);
                }
            }

            for cmd in [
                "google-chrome",
                "google-chrome-stable",
                "chromium",
                "chromium-browser",
                "brave-browser",
            ] {
                if let Ok(output) = std::process::Command::new("which").arg(cmd).output()
                    && output.status.success()
                {
                    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !s.is_empty() && Path::new(&s).exists() {
                        return Ok(PathBuf::from(s));
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            let program_files =
                std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into());
            let program_files_x86 = std::env::var("ProgramFiles(x86)")
                .unwrap_or_else(|_| r"C:\Program Files (x86)".into());
            let local_app_data = std::env::var("LocalAppData")
                .unwrap_or_else(|_| r"C:\Users\Default\AppData\Local".into());

            let candidates = [
                format!(r"{program_files}\Google\Chrome\Application\chrome.exe"),
                format!(r"{program_files_x86}\Google\Chrome\Application\chrome.exe"),
                format!(r"{local_app_data}\Google\Chrome\Application\chrome.exe"),
            ];
            for candidate in candidates {
                let p = PathBuf::from(candidate);
                if p.exists() {
                    return Ok(p);
                }
            }
        }

        Err(anyhow!(
            "No Chrome or Chromium executable found. Please install Chromium or set CHROME_PATH."
        ))
    }

    /// Get user data directory for Chrome
    pub fn get_default_user_data_dir(executable: Option<&Path>) -> Option<PathBuf> {
        let home = dirs_home()?;

        #[cfg(target_os = "macos")]
        {
            let app_support = home.join("Library/Application Support");
            if let Some(exe) = executable {
                let exe_str = exe.to_string_lossy();
                if exe_str.contains("Chromium") {
                    return Some(app_support.join("Chromium"));
                }
            }
            Some(app_support.join("Google/Chrome"))
        }

        #[cfg(target_os = "linux")]
        {
            let config_dir = home.join(".config");
            if let Some(exe) = executable {
                let name = exe.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                if name.contains("chromium") && config_dir.join("chromium").is_dir() {
                    return Some(config_dir.join("chromium"));
                }
                if name.contains("google-chrome") && config_dir.join("google-chrome").is_dir() {
                    return Some(config_dir.join("google-chrome"));
                }
            }
            if config_dir.join("chromium").is_dir() {
                Some(config_dir.join("chromium"))
            } else if config_dir.join("google-chrome").is_dir() {
                Some(config_dir.join("google-chrome"))
            } else {
                Some(config_dir.join("chromium"))
            }
        }

        #[cfg(target_os = "windows")]
        {
            let local_app_data = std::env::var("LocalAppData").ok().map(PathBuf::from)?;
            Some(local_app_data.join(r"Google\Chrome\User Data"))
        }
    }

    /// Resolves a persistent user data directory.
    /// If default user directory is locked by a running non-debug Chrome instance,
    /// falls back to a dedicated persistent web-assistant user data directory so logins and cookies are preserved.
    pub fn resolve_user_data_dir(
        executable: Option<&Path>,
        requested_dir: Option<&Path>,
    ) -> Result<PathBuf> {
        if let Some(req) = requested_dir {
            return Ok(req.to_path_buf());
        }

        let home = dirs_home().ok_or_else(|| anyhow!("Could not resolve home directory"))?;
        let dedicated_dir = home
            .join(".config")
            .join("web-assistant")
            .join("chrome-profile");

        if let Some(default_dir) = Self::get_default_user_data_dir(executable) {
            // Check if default user data dir is currently locked
            if !Self::is_user_data_dir_locked(&default_dir) {
                return Ok(default_dir);
            }
        }

        // Use dedicated persistent directory to avoid lock contention
        fs::create_dir_all(&dedicated_dir)?;
        Ok(dedicated_dir)
    }

    /// Check if the user data directory has an active SingletonLock
    pub fn is_user_data_dir_locked(user_data_dir: &Path) -> bool {
        let lock_file = user_data_dir.join("SingletonLock");
        if !lock_file.exists() && fs::symlink_metadata(&lock_file).is_err() {
            return false;
        }

        // On Linux, check if /proc/<pid> exists
        #[cfg(target_os = "linux")]
        {
            if let Ok(target) = fs::read_link(&lock_file) {
                let target_str = target.to_string_lossy();
                if let Some(pid_str) = target_str.split('-').next_back()
                    && let Ok(pid) = pid_str.parse::<u32>()
                {
                    return Path::new(&format!("/proc/{pid}")).exists();
                }
            }
        }

        true
    }

    /// List available profiles inside user data directory
    pub fn list_profiles(user_data_dir: &Path) -> Vec<ChromeProfileInfo> {
        let mut profiles = Vec::new();
        if !user_data_dir.is_dir() {
            return profiles;
        }

        if user_data_dir.join("Default").is_dir() {
            profiles.push(ChromeProfileInfo {
                name: "Default".into(),
                directory: "Default".into(),
            });
        }

        if let Ok(entries) = fs::read_dir(user_data_dir) {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.starts_with("Profile ") && entry.path().is_dir() {
                    profiles.push(ChromeProfileInfo {
                        name: file_name.clone(),
                        directory: file_name,
                    });
                }
            }
        }

        profiles
    }

    /// Standard Chrome arguments optimized for automation and CDP
    pub fn default_args(port: u16, user_data_dir: &Path, headless: bool) -> Vec<String> {
        let mut args = vec![
            format!("--remote-debugging-port={port}"),
            format!("--user-data-dir={}", user_data_dir.display()),
            "--no-first-run".into(),
            "--no-default-browser-check".into(),
            "--disable-background-networking".into(),
            "--disable-background-timer-throttling".into(),
            "--disable-backgrounding-occluded-windows".into(),
            "--disable-breakpad".into(),
            "--disable-client-side-phishing-detection".into(),
            "--disable-default-apps".into(),
            "--disable-dev-shm-usage".into(),
            "--disable-extensions".into(),
            "--disable-features=Translate,BackForwardCache,AcceptCHFrame,MediaRouter,OptimizationHints".into(),
            "--disable-hang-monitor".into(),
            "--disable-ipc-flooding-protection".into(),
            "--disable-popup-blocking".into(),
            "--disable-prompt-on-repost".into(),
            "--disable-renderer-backgrounding".into(),
            "--disable-sync".into(),
            "--force-color-profile=srgb".into(),
            "--metrics-recording-only".into(),
            "--no-sandbox".into(),
        ];

        if headless {
            args.push("--headless=new".into());
        }

        args.push("about:blank".into());
        args
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_chrome_profile_info_eq() {
        let info1 = ChromeProfileInfo {
            name: "Default".into(),
            directory: "Default".into(),
        };
        let info2 = ChromeProfileInfo {
            name: "Default".into(),
            directory: "Default".into(),
        };
        assert_eq!(info1, info2);
    }

    #[test]
    fn test_chrome_profile_info_debug() {
        let info = ChromeProfileInfo {
            name: "Profile 1".into(),
            directory: "Profile 1".into(),
        };
        let debug_str = format!("{info:?}");
        assert!(debug_str.contains("Profile 1"));
    }

    #[test]
    fn test_list_profiles_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let profiles = Chrome::list_profiles(tmp.path());
        assert!(profiles.is_empty());
    }

    #[test]
    fn test_list_profiles_with_default() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("Default")).unwrap();
        let profiles = Chrome::list_profiles(tmp.path());
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].directory, "Default");
    }

    #[test]
    fn test_list_profiles_with_named_profile() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("Profile 1")).unwrap();
        std::fs::create_dir(tmp.path().join("Profile 2")).unwrap();
        let profiles = Chrome::list_profiles(tmp.path());
        assert_eq!(profiles.len(), 2);
    }

    #[test]
    fn test_default_args_contains_required_flags() {
        let args = Chrome::default_args(9222, Path::new("/tmp"), true);
        let args_str: Vec<String> = args.iter().cloned().collect();
        let combined = args_str.join(" ");
        assert!(combined.contains("--remote-debugging-port=9222"));
        assert!(combined.contains("--headless=new"));
        assert!(combined.contains("--no-sandbox"));
        assert!(combined.contains("about:blank"));
    }

    #[test]
    fn test_default_args_non_headless() {
        let args = Chrome::default_args(9223, Path::new("/tmp"), false);
        let args_str: Vec<String> = args.iter().cloned().collect();
        let combined = args_str.join(" ");
        assert!(!combined.contains("--headless"));
    }

    #[test]
    fn test_resolve_user_data_dir_with_explicit_path() {
        let tmp = TempDir::new().unwrap();
        let result = Chrome::resolve_user_data_dir(None, Some(tmp.path())).unwrap();
        assert_eq!(result, tmp.path());
    }

    #[test]
    fn test_is_user_data_dir_locked_nonexistent() {
        let tmp = TempDir::new().unwrap();
        let locked = Chrome::is_user_data_dir_locked(tmp.path());
        assert!(!locked);
    }
}
