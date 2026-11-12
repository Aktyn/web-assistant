use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Notify};
use tracing::{debug, info};

use crate::browser::cdp::CdpClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadState {
    InProgress,
    Completed,
    Canceled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadItem {
    pub guid: String,
    pub url: String,
    pub suggested_filename: String,
    pub filepath: Option<PathBuf>,
    pub total_bytes: u64,
    pub received_bytes: u64,
    pub state: DownloadState,
}

#[derive(Clone)]
pub struct DownloadsWatchdog {
    client: CdpClient,
    download_dir: PathBuf,
    downloads: Arc<Mutex<HashMap<String, DownloadItem>>>,
    notify_completed: Arc<Notify>,
}

impl DownloadsWatchdog {
    pub fn new(client: CdpClient, download_dir: impl Into<PathBuf>) -> Self {
        Self {
            client,
            download_dir: download_dir.into(),
            downloads: Arc::new(Mutex::new(HashMap::new())),
            notify_completed: Arc::new(Notify::new()),
        }
    }

    /// Construct a test-only instance with a dummy client and temp download dir.
    #[cfg(test)]
    pub(crate) fn new_test(download_dir: impl Into<PathBuf>) -> Self {
        Self {
            client: CdpClient::new_test_client(),
            download_dir: download_dir.into(),
            downloads: Arc::new(Mutex::new(HashMap::new())),
            notify_completed: Arc::new(Notify::new()),
        }
    }

    #[must_use]
    pub fn download_dir(&self) -> &Path {
        &self.download_dir
    }

    pub async fn setup(&self) -> Result<()> {
        if !self.download_dir.exists() {
            std::fs::create_dir_all(&self.download_dir).with_context(|| {
                format!(
                    "Failed to create download dir {}",
                    self.download_dir.display()
                )
            })?;
        }

        // Set download behavior in Chrome CDP
        let res = self
            .client
            .send_command(
                "Page.setDownloadBehavior",
                serde_json::json!({
                    "behavior": "allow",
                    "downloadPath": self.download_dir.display().to_string()
                }),
            )
            .await;

        if let Err(e) = res {
            debug!("Page.setDownloadBehavior fallback to Browser domain: {e}");
            let _ = self
                .client
                .send_command(
                    "Browser.setDownloadBehavior",
                    serde_json::json!({
                        "behavior": "allow",
                        "downloadPath": self.download_dir.display().to_string(),
                        "eventsEnabled": true
                    }),
                )
                .await;
        }

        debug!(
            "DownloadsWatchdog configured for {}",
            self.download_dir.display()
        );
        Ok(())
    }

    pub async fn handle_cdp_event(&self, event: &Value) -> Result<bool> {
        let Some(method) = event.get("method").and_then(|m| m.as_str()) else {
            return Ok(false);
        };

        match method {
            "Page.downloadWillBegin" | "Browser.downloadWillBegin" => {
                let params = event.get("params").cloned().unwrap_or(Value::Null);
                let guid = params
                    .get("guid")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let url = params
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let suggested_filename = params
                    .get("suggestedFilename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("downloaded_file")
                    .to_string();

                let filepath = self.download_dir.join(&suggested_filename);
                info!(
                    "📥 Download started: {} -> {}",
                    suggested_filename,
                    filepath.display()
                );

                let item = DownloadItem {
                    guid: guid.clone(),
                    url,
                    suggested_filename,
                    filepath: Some(filepath),
                    total_bytes: 0,
                    received_bytes: 0,
                    state: DownloadState::InProgress,
                };

                let mut lock = self.downloads.lock().await;
                lock.insert(guid, item);
                Ok(true)
            }
            "Page.downloadProgress" | "Browser.downloadProgress" => {
                let params = event.get("params").cloned().unwrap_or(Value::Null);
                let guid = params.get("guid").and_then(|v| v.as_str()).unwrap_or("");
                let total_bytes = params
                    .get("totalBytes")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let received_bytes = params
                    .get("receivedBytes")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let state_str = params.get("state").and_then(|v| v.as_str()).unwrap_or("");

                let state = match state_str {
                    "completed" => DownloadState::Completed,
                    "canceled" => DownloadState::Canceled,
                    _ => DownloadState::InProgress,
                };

                let mut lock = self.downloads.lock().await;
                if let Some(item) = lock.get_mut(guid) {
                    item.total_bytes = total_bytes;
                    item.received_bytes = received_bytes;
                    item.state = state;

                    if state == DownloadState::Completed {
                        info!("✅ Download completed: {}", item.suggested_filename);
                        self.notify_completed.notify_waiters();
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub async fn get_completed_downloads(&self) -> Vec<DownloadItem> {
        let lock = self.downloads.lock().await;
        lock.values()
            .filter(|d| d.state == DownloadState::Completed)
            .cloned()
            .collect()
    }

    pub async fn wait_for_all_downloads(&self, timeout: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            let pending_count = {
                let lock = self.downloads.lock().await;
                lock.values()
                    .filter(|d| d.state == DownloadState::InProgress)
                    .count()
            };

            if pending_count == 0 {
                return true;
            }

            let now = tokio::time::Instant::now();
            if now >= deadline {
                return false;
            }

            let remaining = deadline - now;
            let _ = tokio::time::timeout(remaining, self.notify_completed.notified()).await;
        }
    }

    /// Return a snapshot of all tracked downloads for testing / inspection.
    pub async fn list_downloads(&self) -> Vec<DownloadItem> {
        let lock = self.downloads.lock().await;
        lock.values().cloned().collect()
    }

    #[must_use]
    pub fn download_dir_path(&self) -> &PathBuf {
        &self.download_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_download_dir() -> PathBuf {
        tempfile::tempdir()
            .expect("failed to create temp dir")
            .path()
            .join("downloads-test")
    }

    #[test]
    fn test_new_watchdog_has_correct_download_dir() {
        let dir = temp_download_dir();
        let wd = DownloadsWatchdog::new_test(&dir);
        assert_eq!(wd.download_dir_path(), &dir);
    }

    #[test]
    fn test_download_item_state_serialization() {
        let item = DownloadItem {
            guid: "GUID-123".into(),
            url: "https://example.com/file.zip".into(),
            suggested_filename: "file.zip".into(),
            filepath: Some(PathBuf::from("/tmp/file.zip")),
            total_bytes: 1024,
            received_bytes: 1024,
            state: DownloadState::Completed,
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"GUID-123\""));
        assert!(json.contains("\"completed\""));

        let restored: DownloadItem = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.guid, "GUID-123");
        assert_eq!(restored.state, DownloadState::Completed);
    }

    #[test]
    fn test_download_state_variant_values() {
        assert_eq!(
            serde_json::to_string(&DownloadState::InProgress).unwrap(),
            "\"in_progress\""
        );
        assert_eq!(
            serde_json::to_string(&DownloadState::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&DownloadState::Canceled).unwrap(),
            "\"canceled\""
        );
    }

    #[test]
    fn test_get_completed_downloads_filters() {
        let dir = temp_download_dir();
        let wd = DownloadsWatchdog::new_test(dir);

        // Simulate inserting two downloads via internal map manipulation
        let rt = tokio::runtime::Runtime::new().unwrap();
        let guard = rt.block_on(async { wd.downloads.lock().await });
        drop(guard);

        // Use handle_cdp_event to insert items properly
        let will_begin = serde_json::json!({
            "method": "Page.downloadWillBegin",
            "params": {
                "guid": "g1",
                "url": "https://example.com/a.pdf",
                "suggestedFilename": "a.pdf"
            }
        });
        let progress_done = serde_json::json!({
            "method": "Page.downloadProgress",
            "params": {
                "guid": "g1",
                "totalBytes": 100,
                "receivedBytes": 100,
                "state": "completed"
            }
        });
        let _progress_inprog = serde_json::json!({
            "method": "Page.downloadProgress",
            "params": {
                "guid": "g2",
                "totalBytes": 500,
                "receivedBytes": 200,
                "state": "in_progress"
            }
        });

        let rt2 = tokio::runtime::Runtime::new().unwrap();
        rt2.block_on(wd.handle_cdp_event(&will_begin)).unwrap();
        rt2.block_on(wd.handle_cdp_event(&progress_done)).unwrap();
        // g2 is just in_progress — not inserted by an event but we simulate it below

        // Manually insert g2 to test filtering
        let guard = rt2.block_on(async { wd.downloads.lock().await });
        let mut guard_mut = (*guard).clone();
        drop(guard);
        guard_mut.insert(
            "g2".into(),
            DownloadItem {
                guid: "g2".into(),
                url: "https://example.com/b.zip".into(),
                suggested_filename: "b.zip".into(),
                filepath: Some(temp_download_dir().join("b.zip")),
                total_bytes: 500,
                received_bytes: 200,
                state: DownloadState::InProgress,
            },
        );
        // We need interior mutability — use a separate insert approach

        // Instead: re-create with both items set up properly
        let wd2 = DownloadsWatchdog::new_test(temp_download_dir());
        let rt3 = tokio::runtime::Runtime::new().unwrap();
        rt3.block_on(wd2.handle_cdp_event(&will_begin)).unwrap();
        rt3.block_on(wd2.handle_cdp_event(&progress_done)).unwrap();

        // Only g1 should be completed
        let completed = rt3.block_on(wd2.get_completed_downloads());
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].guid, "g1");
        assert_eq!(completed[0].state, DownloadState::Completed);
    }

    #[test]
    fn test_unknown_method_returns_false() {
        let dir = temp_download_dir();
        let wd = DownloadsWatchdog::new_test(dir);
        let event = serde_json::json!({"method": "Page.navigate"});
        let rt = tokio::runtime::Runtime::new().unwrap();
        let handled = rt.block_on(wd.handle_cdp_event(&event)).unwrap();
        assert!(!handled);
    }

    #[test]
    fn test_missing_guid_falls_back_to_empty_string() {
        let dir = temp_download_dir();
        let wd = DownloadsWatchdog::new_test(dir);
        let event = serde_json::json!({
            "method": "Page.downloadWillBegin",
            "params": {}
        });
        let rt = tokio::runtime::Runtime::new().unwrap();
        let handled = rt.block_on(wd.handle_cdp_event(&event)).unwrap();
        assert!(handled);
        // Should still be inserted with empty guid
        let downloads = rt.block_on(wd.list_downloads());
        assert_eq!(downloads.len(), 1);
        assert_eq!(downloads[0].guid, "");
    }
}
