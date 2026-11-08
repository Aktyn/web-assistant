use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use tracing::{error, warn};

use crate::browser::cdp::CdpClient;

#[derive(Clone)]
pub struct CrashWatchdog {
    client: CdpClient,
    crashed: Arc<AtomicBool>,
    crash_count: Arc<AtomicUsize>,
    recovery_count: Arc<AtomicUsize>,
}

impl CrashWatchdog {
    pub fn new(client: CdpClient) -> Self {
        Self {
            client,
            crashed: Arc::new(AtomicBool::new(false)),
            crash_count: Arc::new(AtomicUsize::new(0)),
            recovery_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Construct a test-only instance with a dummy client (no real connection).
    #[cfg(test)]
    pub(crate) fn new_test() -> Self {
        Self {
            client: CdpClient::new_test_client(),
            crashed: Arc::new(AtomicBool::new(false)),
            crash_count: Arc::new(AtomicUsize::new(0)),
            recovery_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn crash_count(&self) -> usize {
        self.crash_count.load(Ordering::SeqCst)
    }

    pub fn is_crashed(&self) -> bool {
        self.crashed.load(Ordering::SeqCst)
    }

    pub fn recovery_count(&self) -> usize {
        self.recovery_count.load(Ordering::SeqCst)
    }

    pub async fn handle_cdp_event(&self, event: &Value) -> Result<bool> {
        let method = match event.get("method").and_then(|m| m.as_str()) {
            Some(m) => m,
            None => return Ok(false),
        };

        if method == "Inspector.targetCrashed" || method == "Target.targetCrashed" {
            error!("🚨 CrashWatchdog: Target crashed!");
            self.crashed.store(true, Ordering::SeqCst);
            self.crash_count.fetch_add(1, Ordering::SeqCst);
            return Ok(true);
        }

        Ok(false)
    }

    /// Heartbeat ping to check if page is responsive
    pub async fn check_responsive(&self, timeout: Duration) -> bool {
        let ping_future = self.client.evaluate("1 + 1");
        match tokio::time::timeout(timeout, ping_future).await {
            Ok(Ok(val)) => val.as_i64() == Some(2),
            _ => {
                warn!(
                    "CrashWatchdog: Page failed to respond to heartbeat ping within {:?}",
                    timeout
                );
                false
            }
        }
    }

    /// Attempt reload recovery
    pub async fn recover(&self) -> Result<bool> {
        warn!("CrashWatchdog: Attempting recovery via Page.reload...");
        self.recovery_count.fetch_add(1, Ordering::SeqCst);
        let res = self
            .client
            .send_command("Page.reload", serde_json::json!({}))
            .await;
        if res.is_ok() {
            self.crashed.store(false, Ordering::SeqCst);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_healthy() {
        let wd = CrashWatchdog::new_test();
        assert!(!wd.is_crashed());
        assert_eq!(wd.crash_count(), 0);
        assert_eq!(wd.recovery_count(), 0);
    }

    #[test]
    fn test_target_crashed_event_sets_flag() {
        let wd = CrashWatchdog::new_test();
        let event = serde_json::json!({
            "method": "Target.targetCrashed",
            "params": {}
        });
        let rt = tokio::runtime::Runtime::new().unwrap();
        let handled = rt.block_on(wd.handle_cdp_event(&event)).unwrap();
        assert!(handled);
        assert!(wd.is_crashed());
        assert_eq!(wd.crash_count(), 1);
    }

    #[test]
    fn test_inspector_target_crashed_event_sets_flag() {
        let wd = CrashWatchdog::new_test();
        let event = serde_json::json!({
            "method": "Inspector.targetCrashed",
            "params": {}
        });
        let rt = tokio::runtime::Runtime::new().unwrap();
        let handled = rt.block_on(wd.handle_cdp_event(&event)).unwrap();
        assert!(handled);
        assert!(wd.is_crashed());
    }

    #[test]
    fn test_unrelated_event_returns_false() {
        let wd = CrashWatchdog::new_test();
        let event = serde_json::json!({
            "method": "Page.loadEventFired",
            "params": {}
        });
        let rt = tokio::runtime::Runtime::new().unwrap();
        let handled = rt.block_on(wd.handle_cdp_event(&event)).unwrap();
        assert!(!handled);
        assert!(!wd.is_crashed());
    }

    #[test]
    fn test_missing_method_field_returns_false() {
        let wd = CrashWatchdog::new_test();
        let event = serde_json::json!({"params": {}});
        let rt = tokio::runtime::Runtime::new().unwrap();
        let handled = rt.block_on(wd.handle_cdp_event(&event)).unwrap();
        assert!(!handled);
    }

    #[test]
    fn test_recovery_count_increments() {
        let wd = CrashWatchdog::new_test();
        wd.recovery_count.store(5, Ordering::SeqCst);
        assert_eq!(wd.recovery_count(), 5);
    }
}
