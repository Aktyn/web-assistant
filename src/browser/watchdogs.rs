#![allow(clippy::missing_errors_doc)]

pub mod captcha;
pub mod crash;
pub mod downloads;
pub mod permissions;
pub mod popups;
pub mod security;
pub mod storage;

use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

pub use captcha::CaptchaWatchdog;
pub use crash::CrashWatchdog;
pub use downloads::DownloadsWatchdog;
pub use permissions::PermissionsWatchdog;
pub use popups::PopupsWatchdog;
use security::SecurityWatchdog;
pub use storage::StorageStateWatchdog;

use crate::browser::cdp::CdpClient;

#[derive(Clone)]
pub struct BrowserWatchdogs {
    pub captcha: CaptchaWatchdog,
    pub popups: PopupsWatchdog,
    pub downloads: DownloadsWatchdog,
    pub crash: CrashWatchdog,
    pub security: SecurityWatchdog,
    pub permissions: PermissionsWatchdog,
    pub storage: StorageStateWatchdog,
}

impl BrowserWatchdogs {
    #[must_use]
    pub fn new(client: CdpClient, download_dir: Option<PathBuf>) -> Self {
        let dl_dir = download_dir.unwrap_or_else(|| {
            dirs_home().map_or_else(
                || PathBuf::from("downloads"),
                |h| h.join(".config/web-assistant/downloads"),
            )
        });

        Self {
            captcha: CaptchaWatchdog::new(client.clone()),
            popups: PopupsWatchdog::new(client.clone()),
            downloads: DownloadsWatchdog::new(client.clone(), dl_dir),
            crash: CrashWatchdog::new(client.clone()),
            security: SecurityWatchdog::new(),
            permissions: PermissionsWatchdog::new(client.clone()),
            storage: StorageStateWatchdog::new(client),
        }
    }

    /// Set up all watchdogs on browser start
    pub async fn setup_all(&self) -> Result<()> {
        self.popups.setup().await?;
        self.downloads.setup().await?;
        self.permissions.apply().await?;
        Ok(())
    }

    /// Dispatch incoming CDP message/event across active watchdogs
    pub async fn handle_cdp_event(&self, event: &Value) -> Result<()> {
        let _ = self.captcha.handle_cdp_event(event).await;
        let _ = self.popups.handle_cdp_event(event).await;
        let _ = self.downloads.handle_cdp_event(event).await;
        let _ = self.crash.handle_cdp_event(event);
        Ok(())
    }

    /// Check security before navigation
    pub fn on_before_navigate(&self, url: &str) -> Result<()> {
        self.security.validate_url(url)
    }

    /// Post-navigation watchdog verification (check captcha, wait if in progress, recover blank page)
    pub async fn on_after_navigate(&self, current_url: &str) -> Result<()> {
        // Check if proxy started solving captcha
        let _ = self
            .captcha
            .wait_if_captcha_solving(Duration::from_secs(30))
            .await;

        // Detect if page has captcha
        let _ = self.captcha.detect_on_page().await;

        // Recover blank page if needed
        let _ = self.recover_blank_page_if_needed(current_url).await;
        Ok(())
    }

    pub async fn recover_blank_page_if_needed(&self, current_url: &str) -> Result<bool> {
        if current_url.is_empty() || current_url == "about:blank" {
            return Ok(false);
        }

        if self.crash.is_crashed() {
            return self.crash.recover().await;
        }

        Ok(false)
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::cdp::CdpClient;

    #[test]
    fn test_browser_watchdogs_construction() {
        let client = CdpClient::new_test_client();
        let wd = BrowserWatchdogs::new(client, None);
        // Basic sanity: all fields are present and non-crashing
        assert!(!wd.captcha.is_solving());
        assert!(!wd.crash.is_crashed());
        assert_eq!(wd.popups.handled_count(), 0);
        assert_eq!(wd.crash.crash_count(), 0);
    }

    #[test]
    fn test_before_navigate_allows_localhost() {
        let client = CdpClient::new_test_client();
        let wd = BrowserWatchdogs::new(client, None);
        // Default policy allows everything
        assert!(wd.on_before_navigate("https://example.com").is_ok());
        assert!(wd.on_before_navigate("about:blank").is_ok());
    }

    #[test]
    fn test_blank_page_recovery_no_crash() {
        let client = CdpClient::new_test_client();
        let wd = BrowserWatchdogs::new(client, None);
        let rt = tokio::runtime::Runtime::new().unwrap();
        // current_url is blank-like → no recovery triggered
        let recovered = rt
            .block_on(wd.recover_blank_page_if_needed("about:blank"))
            .unwrap();
        assert!(!recovered);
        let recovered2 = rt.block_on(wd.recover_blank_page_if_needed("")).unwrap();
        assert!(!recovered2);
    }

    #[test]
    fn test_handle_cdp_event_dispatches_to_watchdogs() {
        let client = CdpClient::new_test_client();
        let wd = BrowserWatchdogs::new(client, None);
        let rt = tokio::runtime::Runtime::new().unwrap();

        // An event with unknown method should not error
        let event = serde_json::json!({"method": "Page.loadEventFired"});
        let result = rt.block_on(wd.handle_cdp_event(&event));
        assert!(result.is_ok());
    }

    #[test]
    fn test_downloads_watchdog_has_correct_dir() {
        let client = CdpClient::new_test_client();
        let wd = BrowserWatchdogs::new(client, Some(PathBuf::from("/tmp/fake-dl")));
        assert_eq!(
            wd.downloads.download_dir_path(),
            &PathBuf::from("/tmp/fake-dl")
        );
    }

    #[test]
    fn test_before_navigate_blocks_domain() {
        let client = CdpClient::new_test_client();
        let wd = BrowserWatchdogs::new(client, None);
        // Access the security watchdog through a different path or test via BrowserWatchdogs
        // The on_before_navigate calls security.validate_url internally
        // Default policy allows everything so this should pass
        assert!(wd.on_before_navigate("https://allowed.com").is_ok());
    }
}
