use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};
use tokio::sync::{Mutex, Notify};
use tracing::{info, warn};

use crate::browser::cdp::CdpClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptchaResultType {
    Success,
    Failed,
    Timeout,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaWaitResult {
    pub waited: bool,
    pub vendor: String,
    pub url: String,
    pub duration_ms: u64,
    pub result: CaptchaResultType,
}

#[derive(Debug, Clone, Default)]
pub struct CaptchaInfo {
    pub vendor: String,
    pub url: String,
    pub target_id: String,
    /// Unix timestamp (seconds since epoch) when captcha solving started.
    /// Stored as f64 so it can be serialized/deserialized.
    pub started_at_epoch: f64,
}

impl Serialize for CaptchaInfo {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("CaptchaInfo", 4)?;
        state.serialize_field("vendor", &self.vendor)?;
        state.serialize_field("url", &self.url)?;
        state.serialize_field("target_id", &self.target_id)?;
        state.serialize_field("started_at_epoch", &self.started_at_epoch)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for CaptchaInfo {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[allow(dead_code)]
        struct Helper {
            vendor: String,
            url: String,
            target_id: String,
            started_at_epoch: f64,
        }
        let h = Helper::deserialize(deserializer)?;
        Ok(CaptchaInfo {
            vendor: h.vendor,
            url: h.url,
            target_id: h.target_id,
            started_at_epoch: h.started_at_epoch,
        })
    }
}

impl CaptchaInfo {
    pub fn now() -> Self {
        let started_at_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        Self {
            vendor: String::new(),
            url: String::new(),
            target_id: String::new(),
            started_at_epoch,
        }
    }
}

#[derive(Clone)]
pub struct CaptchaWatchdog {
    client: CdpClient,
    solving: Arc<AtomicBool>,
    detected: Arc<AtomicBool>,
    info: Arc<Mutex<Option<CaptchaInfo>>>,
    notify_finished: Arc<Notify>,
    last_result: Arc<Mutex<Option<CaptchaWaitResult>>>,
}

impl CaptchaWatchdog {
    pub fn new(client: CdpClient) -> Self {
        Self {
            client,
            solving: Arc::new(AtomicBool::new(false)),
            detected: Arc::new(AtomicBool::new(false)),
            info: Arc::new(Mutex::new(None)),
            notify_finished: Arc::new(Notify::new()),
            last_result: Arc::new(Mutex::new(None)),
        }
    }

    pub fn is_solving(&self) -> bool {
        self.solving.load(Ordering::SeqCst)
    }

    pub fn was_detected(&self) -> bool {
        self.detected.load(Ordering::SeqCst)
    }

    /// Handles CDP events from BrowserUse captcha solving proxy (BrowserUse.captchaSolverStarted / Finished)
    pub async fn handle_cdp_event(&self, event: &Value) -> Result<bool> {
        let method = match event.get("method").and_then(|m| m.as_str()) {
            Some(m) => m,
            None => return Ok(false),
        };

        match method {
            "BrowserUse.captchaSolverStarted" => {
                let params = event.get("params").cloned().unwrap_or(Value::Null);
                let vendor = params
                    .get("vendor")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let url = params
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let target_id = params
                    .get("targetId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                info!("🔒 Captcha solving started: {} on {}", vendor, url);
                self.solving.store(true, Ordering::SeqCst);
                self.detected.store(true, Ordering::SeqCst);

                let mut lock = self.info.lock().await;
                *lock = Some(CaptchaInfo {
                    vendor,
                    url,
                    target_id,
                    started_at_epoch: SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .map(|d| d.as_secs_f64())
                        .unwrap_or(0.0),
                });
                Ok(true)
            }
            "BrowserUse.captchaSolverFinished" => {
                let params = event.get("params").cloned().unwrap_or(Value::Null);
                let success = params
                    .get("success")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let duration_ms = params
                    .get("durationMs")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let vendor = params
                    .get("vendor")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let url = params
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let result = if success {
                    CaptchaResultType::Success
                } else {
                    CaptchaResultType::Failed
                };

                info!(
                    "🔓 Captcha solving finished: {:?} — {} on {} ({}ms)",
                    result, vendor, url, duration_ms
                );
                self.solving.store(false, Ordering::SeqCst);

                let wait_result = CaptchaWaitResult {
                    waited: true,
                    vendor,
                    url,
                    duration_ms,
                    result,
                };

                let mut res_lock = self.last_result.lock().await;
                *res_lock = Some(wait_result);

                self.notify_finished.notify_waiters();
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Wait if an automated captcha solve is in-flight
    pub async fn wait_if_captcha_solving(&self, timeout: Duration) -> Option<CaptchaWaitResult> {
        if !self.is_solving() {
            return None;
        }

        let info_clone = {
            let lock = self.info.lock().await;
            lock.clone()
        };

        let vendor = info_clone
            .as_ref()
            .map(|i| i.vendor.clone())
            .unwrap_or_else(|| "unknown".into());
        let url = info_clone
            .as_ref()
            .map(|i| i.url.clone())
            .unwrap_or_default();

        info!(
            "⏳ Waiting for {} captcha to be solved on {} (timeout={:?})...",
            vendor, url, timeout
        );

        tokio::select! {
            _ = self.notify_finished.notified() => {
                let lock = self.last_result.lock().await;
                lock.clone()
            }
            _ = tokio::time::sleep(timeout) => {
                warn!("⏰ Captcha wait timed out after {:?} for {} on {}", timeout, vendor, url);
                self.solving.store(false, Ordering::SeqCst);
                Some(CaptchaWaitResult {
                    waited: true,
                    vendor,
                    url,
                    duration_ms: timeout.as_millis() as u64,
                    result: CaptchaResultType::Timeout,
                })
            }
        }
    }

    /// Inspect page for bot detection and captcha challenges
    pub async fn detect_on_page(&self) -> Result<bool> {
        let check_js = r#"
        (() => {
            const text = (document.body ? document.body.innerText : '').toLowerCase();
            const captchaKeywords = [
                'verify you are human', 'turnstile', 'recaptcha',
                'hcaptcha', 'security check', 'cf-challenge',
                'bot detection', 'please complete the security check'
            ];
            const hasKeyword = captchaKeywords.some(kw => text.includes(kw));
            const hasFrame = !!document.querySelector('iframe[src*="recaptcha"], iframe[src*="turnstile"], iframe[src*="hcaptcha"]');
            return hasKeyword || hasFrame;
        })()
        "#;

        if let Ok(val) = self.client.evaluate(check_js).await
            && val.as_bool() == Some(true)
        {
            self.detected.store(true, Ordering::SeqCst);
            return Ok(true);
        }
        Ok(false)
    }

    pub fn reset(&self) {
        self.solving.store(false, Ordering::SeqCst);
        self.detected.store(false, Ordering::SeqCst);
        self.notify_finished.notify_waiters();
    }

    #[cfg(test)]
    pub(crate) fn new_test() -> Self {
        Self {
            client: CdpClient::new_test_client(),
            solving: Arc::new(AtomicBool::new(false)),
            detected: Arc::new(AtomicBool::new(false)),
            info: Arc::new(Mutex::new(None)),
            notify_finished: Arc::new(Notify::new()),
            last_result: Arc::new(Mutex::new(None)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_not_solving() {
        let wd = CaptchaWatchdog::new_test();
        assert!(!wd.is_solving());
        assert!(!wd.was_detected());
    }

    #[test]
    fn test_reset_clears_flags() {
        let wd = CaptchaWatchdog::new_test();
        wd.solving.store(true, Ordering::SeqCst);
        wd.detected.store(true, Ordering::SeqCst);
        wd.reset();
        assert!(!wd.is_solving());
        assert!(!wd.was_detected());
    }

    #[test]
    fn test_handle_captcha_solver_started_event() {
        let wd = CaptchaWatchdog::new_test();
        let event = serde_json::json!({
            "method": "BrowserUse.captchaSolverStarted",
            "params": {
                "vendor": "cloudflare",
                "url": "https://example.com/captcha",
                "targetId": "TAB-123"
            }
        });
        let rt = tokio::runtime::Runtime::new().unwrap();
        let handled = rt.block_on(wd.handle_cdp_event(&event)).unwrap();
        assert!(handled);
        assert!(wd.is_solving());
        assert!(wd.was_detected());

        // Verify info was stored correctly
        let info_opt = rt.block_on(async { wd.info.lock().await.clone() });
        assert_eq!(info_opt.as_ref().unwrap().vendor, "cloudflare");
        assert_eq!(
            info_opt.as_ref().unwrap().url,
            "https://example.com/captcha"
        );
    }

    #[test]
    fn test_handle_unknown_method_returns_false() {
        let wd = CaptchaWatchdog::new_test();
        let event = serde_json::json!({"method": "Page.loadEventFired"});
        let rt = tokio::runtime::Runtime::new().unwrap();
        let handled = rt.block_on(wd.handle_cdp_event(&event)).unwrap();
        assert!(!handled);
    }

    #[test]
    fn test_info_starts_default_empty() {
        let wd = CaptchaWatchdog::new_test();
        let info = wd.info.try_lock().unwrap();
        assert!(info.is_none());
    }

    #[test]
    fn test_duration_ms_preserved_in_wait_result() {
        let result = CaptchaWaitResult {
            waited: true,
            vendor: "hCaptcha".into(),
            url: "https://site.com".into(),
            duration_ms: 7500,
            result: CaptchaResultType::Success,
        };
        assert_eq!(result.duration_ms, 7500);
        assert_eq!(result.vendor, "hCaptcha");
        assert_eq!(result.result, CaptchaResultType::Success);
    }

    #[test]
    fn test_captcha_info_serialization_roundtrip() {
        let info = CaptchaInfo {
            vendor: "cloudflare".into(),
            url: "https://example.com".into(),
            target_id: "abc-123".into(),
            started_at_epoch: 1700000000.0,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"cloudflare\""));
        assert!(json.contains("\"started_at_epoch\""));
        let restored: CaptchaInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.vendor, "cloudflare");
        assert_eq!(restored.started_at_epoch, 1700000000.0);
    }

    #[test]
    fn test_captcha_result_type_deserialize_snake_case() {
        let success: CaptchaResultType = serde_json::from_str("\"success\"").unwrap();
        assert_eq!(success, CaptchaResultType::Success);

        let failed: CaptchaResultType = serde_json::from_str("\"failed\"").unwrap();
        assert_eq!(failed, CaptchaResultType::Failed);

        let timeout: CaptchaResultType = serde_json::from_str("\"timeout\"").unwrap();
        assert_eq!(timeout, CaptchaResultType::Timeout);
    }

    #[test]
    fn test_captcha_wait_result_all_fields() {
        let wr = CaptchaWaitResult {
            waited: true,
            vendor: "hCaptcha".into(),
            url: "https://site.com".into(),
            duration_ms: 5200,
            result: CaptchaResultType::Success,
        };
        let json = serde_json::to_string(&wr).unwrap();
        let roundtrip: CaptchaWaitResult = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.duration_ms, 5200);
        assert!(roundtrip.waited);
    }
}
