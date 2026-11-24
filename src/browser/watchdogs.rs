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

use super::CdpClient;

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
            crate::utils::home_dir().map_or_else(
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

    pub async fn setup_all(&self) -> Result<()> {
        self.popups.setup().await?;
        self.downloads.setup().await?;
        self.permissions.apply().await?;
        Ok(())
    }

    pub async fn handle_cdp_event(&self, event: &Value) -> Result<()> {
        let _ = self.captcha.handle_cdp_event(event).await;
        let _ = self.popups.handle_cdp_event(event).await;
        let _ = self.downloads.handle_cdp_event(event).await;
        let _ = self.crash.handle_cdp_event(event);
        Ok(())
    }

    pub fn on_before_navigate(&self, url: &str) -> Result<()> {
        self.security.validate_url(url)
    }

    pub async fn on_after_navigate(&self, current_url: &str) -> Result<()> {
        let _ = self
            .captcha
            .wait_if_captcha_solving(Duration::from_secs(30))
            .await;
        let _ = self.captcha.detect_on_page().await;
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
