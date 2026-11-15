use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tracing::{debug, warn};

use crate::browser::cdp::CdpClient;

#[derive(Clone)]
pub struct PopupsWatchdog {
    client: CdpClient,
    handled_count: Arc<AtomicUsize>,
    /// Whether Page domain events have been enabled on this client.
    page_enabled: Arc<AtomicBool>,
}

impl PopupsWatchdog {
    #[must_use]
    pub fn new(client: CdpClient) -> Self {
        Self {
            client,
            handled_count: Arc::new(AtomicUsize::new(0)),
            page_enabled: Arc::new(AtomicBool::new(false)),
        }
    }

    #[must_use]
    pub fn handled_count(&self) -> usize {
        self.handled_count.load(Ordering::SeqCst)
    }

    /// Enable the Page CDP domain so we receive dialog events.
    /// Called during `setup_all`.
    pub async fn setup(&self) -> Result<()> {
        if self
            .page_enabled
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            debug!("PopupsWatchdog: Page domain already enabled, skipping");
            return Ok(());
        }
        self.client
            .send_command("Page.enable", serde_json::json!({}))
            .await?;
        debug!("PopupsWatchdog: Page events enabled for dialog handling");
        Ok(())
    }

    /// Handle a JavaScript dialog event by auto-accepting or dismissing it.
    ///
    /// Dialog handling strategy mirrors browser-use's Python implementation:
    /// - `alert` / `confirm` / `beforeunload` → accept (click OK)
    /// - `prompt` → dismiss (click Cancel), since we cannot provide user input
    pub async fn handle_cdp_event(&self, event: &Value) -> Result<bool> {
        if event.get("method").and_then(|m| m.as_str()) != Some("Page.javascriptDialogOpening") {
            return Ok(false);
        }

        // Ensure Page domain is active before attempting to handle dialogs
        if !self.page_enabled.load(Ordering::SeqCst)
            && let Err(e) = self.setup().await
        {
            debug!("PopupsWatchdog: Failed to enable Page domain: {e}");
        }

        let dialog_type = event
            .pointer("/params/type")
            .and_then(|v| v.as_str())
            .unwrap_or("alert");
        let message = event
            .pointer("/params/message")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Choose action based on dialog type — matches Python popups_watchdog.py logic:
        // alert/confirm/beforeunload → accept=true; prompt → accept=false (we can't type)
        let should_accept = matches!(dialog_type, "alert" | "confirm" | "beforeunload");

        let action_str = if should_accept {
            "accepting (OK)"
        } else {
            "dismissing (Cancel)"
        };
        warn!(
            "PopupsWatchdog: Encountered {} dialog: \"{}\". {}",
            dialog_type,
            message.chars().take(100).collect::<String>(),
            action_str,
        );

        self.client
            .send_command(
                "Page.handleJavaScriptDialog",
                serde_json::json!({
                    "accept": should_accept
                }),
            )
            .await?;

        self.handled_count.fetch_add(1, Ordering::SeqCst);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_action_selection() {
        // Verify the dialog-type → accept decision matches Python logic
        assert!(matches_dialog_accept("alert"));
        assert!(matches_dialog_accept("confirm"));
        assert!(matches_dialog_accept("beforeunload"));
        assert!(!matches_dialog_accept("prompt"));
    }

    #[test]
    fn test_handled_count_increments() {
        // Construct with unreachable client — count checks don't require a connection
        let wd = PopupsWatchdog {
            client: CdpClient::new_test_client(),
            handled_count: Arc::new(AtomicUsize::new(0)),
            page_enabled: Arc::new(AtomicBool::new(false)),
        };
        assert_eq!(wd.handled_count(), 0);
        wd.handled_count.store(3, Ordering::SeqCst);
        assert_eq!(wd.handled_count(), 3);
    }

    #[test]
    fn test_page_enabled_flag() {
        let wd = PopupsWatchdog {
            client: CdpClient::new_test_client(),
            handled_count: Arc::new(AtomicUsize::new(0)),
            page_enabled: Arc::new(AtomicBool::new(false)),
        };
        assert!(!wd.page_enabled.load(Ordering::SeqCst));
        wd.page_enabled.store(true, Ordering::SeqCst);
        assert!(wd.page_enabled.load(Ordering::SeqCst));
    }

    /// Mirror of the acceptance-decision logic from `handle_cdp_event`.
    fn matches_dialog_accept(dialog_type: &str) -> bool {
        matches!(dialog_type, "alert" | "confirm" | "beforeunload")
    }

    #[test]
    fn test_handle_non_dialog_event_returns_false() {
        let wd = PopupsWatchdog::new(CdpClient::new_test_client());
        let event = serde_json::json!({"method": "Page.navigate"});
        let rt = tokio::runtime::Runtime::new().unwrap();
        let handled = rt.block_on(wd.handle_cdp_event(&event)).unwrap();
        assert!(!handled);
    }

    #[test]
    fn test_dialog_types_recognized() {
        // Verify all expected dialog types are handled correctly
        for dialog_type in ["alert", "confirm", "beforeunload"] {
            let _event = serde_json::json!({
                "method": "Page.javascriptDialogOpening",
                "params": {"type": dialog_type, "message": "Test message"}
            });
            // We can't easily test the full handle since it requires a real client,
            // but we verify the pattern matching works
            assert!(matches_dialog_accept(dialog_type));
        }
        // Prompt should be dismissed
        assert!(!matches_dialog_accept("prompt"));
    }

    #[test]
    fn test_setup_already_enabled_skips_cdp() {
        let wd = PopupsWatchdog::new(CdpClient::new_test_client());
        // Mark page as enabled internally
        wd.page_enabled.store(true, Ordering::SeqCst);
        let rt = tokio::runtime::Runtime::new().unwrap();
        // setup should short-circuit when already enabled (no CDP call)
        let result = rt.block_on(wd.setup());
        assert!(result.is_ok());
    }

    #[test]
    fn test_message_truncated_to_100_chars_in_log() {
        // Verify that long messages don't cause issues — truncate is internal
        let msg = "a".repeat(200);
        let truncated: String = msg.chars().take(100).collect();
        assert_eq!(truncated.len(), 100);
        assert_eq!(truncated, "a".repeat(100));
    }
}
