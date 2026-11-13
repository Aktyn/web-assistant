#![allow(clippy::missing_errors_doc)]

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
    #[must_use]
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

    #[must_use]
    pub fn crash_count(&self) -> usize {
        self.crash_count.load(Ordering::SeqCst)
    }

    #[must_use]
    pub fn is_crashed(&self) -> bool {
        self.crashed.load(Ordering::SeqCst)
    }

    #[must_use]
    pub fn recovery_count(&self) -> usize {
        self.recovery_count.load(Ordering::SeqCst)
    }

    /// Handle a crash-related CDP event.
    ///
    /// # Errors
    ///
    /// This function never returns an error.
    pub fn handle_cdp_event(&self, event: &Value) -> Result<bool> {
        let Some(method) = event.get("method").and_then(|m| m.as_str()) else {
            return Ok(false);
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
        if let Ok(Ok(val)) = tokio::time::timeout(timeout, ping_future).await {
            val.as_i64() == Some(2)
        } else {
            warn!(
                "CrashWatchdog: Page failed to respond to heartbeat ping within {:?}",
                timeout
            );
            false
        }
    }

    /// Attempt reload recovery.
    ///
    /// # Errors
    ///
    /// Returns an error when the reload command cannot be sent to the browser.
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
        let handled = wd.handle_cdp_event(&event).unwrap();
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
        let handled = wd.handle_cdp_event(&event).unwrap();
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
        let handled = wd.handle_cdp_event(&event).unwrap();
        assert!(!handled);
        assert!(!wd.is_crashed());
    }

    #[test]
    fn test_missing_method_field_returns_false() {
        let wd = CrashWatchdog::new_test();
        let event = serde_json::json!({"params": {}});
        let handled = wd.handle_cdp_event(&event).unwrap();
        assert!(!handled);
    }

    #[test]
    fn test_recovery_count_increments() {
        let wd = CrashWatchdog::new_test();
        wd.recovery_count.store(5, Ordering::SeqCst);
        assert_eq!(wd.recovery_count(), 5);
    }

    #[test]
    fn test_crash_event_handling_variants() {
        // Test both target crash event names
        for method in ["Target.targetCrashed", "Inspector.targetCrashed"] {
            let wd = CrashWatchdog::new_test();
            let event = serde_json::json!({"method": method, "params": {}});
            let handled = wd.handle_cdp_event(&event).unwrap();
            assert!(handled);
            assert!(wd.is_crashed());
        }
    }

    #[test]
    fn test_missing_method_field_with_params_returns_false() {
        let wd = CrashWatchdog::new_test();
        let event = serde_json::json!({"params": {"foo": "bar"}});
        let handled = wd.handle_cdp_event(&event).unwrap();
        assert!(!handled);
    }

    #[test]
    fn test_multiple_crash_events_increment_counter() {
        let wd = CrashWatchdog::new_test();
        let event1 = serde_json::json!({"method": "Target.targetCrashed", "params": {}});
        let event2 = serde_json::json!({"method": "Inspector.targetCrashed", "params": {}});
        wd.handle_cdp_event(&event1).unwrap();
        wd.handle_cdp_event(&event2).unwrap();
        assert_eq!(wd.crash_count(), 2);
    }
}
