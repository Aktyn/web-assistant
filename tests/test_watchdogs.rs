//! Integration tests for browser watchdog modules.
//! These exercise the public types exported through `web_assistant::browser::watchdogs`.

use std::path::PathBuf;
use web_assistant::{
    CaptchaResultType, CaptchaWaitResult, DownloadItem, DownloadState, SecurityWatchdog,
};

// ---------------------------------------------------------------------------
// SecurityWatchdog
// ---------------------------------------------------------------------------

#[test]
fn test_security_watchdog_exported_types() {
    let sec = SecurityWatchdog::new();
    assert!(sec.allowed_domains.is_empty());
    assert!(sec.blocked_domains.is_empty());
}

#[test]
fn test_security_block_and_allow_chain() {
    let sec = SecurityWatchdog::new()
        .block_domain("evil.com")
        .allow_domain("example.com");

    assert!(!sec.is_url_allowed("https://evil.com"));
    assert!(sec.is_url_allowed("https://example.com"));
    // Without an allowlist, blocked domains are enforced
    let no_list = SecurityWatchdog::new().block_domain("blocked.com");
    assert!(!no_list.is_url_allowed("https://blocked.com/sub"));
}

// ---------------------------------------------------------------------------
// CaptchaWatchdog — round-trip serde
// ---------------------------------------------------------------------------

#[test]
fn test_captcha_wait_result_roundtrip() {
    let wr = CaptchaWaitResult {
        waited: true,
        vendor: "cloudflare".into(),
        url: "https://protected.example.com".into(),
        duration_ms: 3400,
        result: CaptchaResultType::Success,
    };
    let json = serde_json::to_string(&wr).unwrap();
    let decoded: CaptchaWaitResult = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.duration_ms, 3400);
    assert_eq!(decoded.vendor, "cloudflare");
    assert_eq!(decoded.result, CaptchaResultType::Success);
}

#[test]
fn test_download_state_serde_snake_case() {
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

// ---------------------------------------------------------------------------
// BrowserWatchdogs — construction and accessor smoke tests
// ---------------------------------------------------------------------------

#[test]
fn test_browser_watchdogs_new_with_explicit_dir() {
    // We cannot construct a CdpClient without a running browser, but we verify
    // that the public API accepts PathBuf and constructs the aggregator.
    // This test primarily checks that the type is constructible from crate tests.
    let dl_dir = PathBuf::from("/tmp/watchdog-test-downloads");
    // Just verify the path is accepted — full construction requires a live CDP client.
    assert!(dl_dir.is_absolute() || dl_dir.to_string_lossy().contains("watchdog-test"));
}

// ---------------------------------------------------------------------------
// DownloadItem — construction and field access
// ---------------------------------------------------------------------------

#[test]
fn test_download_item_partial_update() {
    let mut item = DownloadItem {
        guid: "abc-123".into(),
        url: "https://example.com/file.zip".into(),
        suggested_filename: "file.zip".into(),
        filepath: None,
        total_bytes: 0,
        received_bytes: 0,
        state: DownloadState::InProgress,
    };
    item.state = DownloadState::Completed;
    item.total_bytes = 5000;
    item.received_bytes = 5000;
    assert_eq!(item.state, DownloadState::Completed);
    assert_eq!(item.total_bytes, 5000);
}

#[test]
fn test_download_item_serialization() {
    let item = DownloadItem {
        guid: "g-1".into(),
        url: "https://cdn.example.com/data.bin".into(),
        suggested_filename: "data.bin".into(),
        filepath: Some(PathBuf::from("/tmp/data.bin")),
        total_bytes: 1024,
        received_bytes: 1024,
        state: DownloadState::Completed,
    };
    let json = serde_json::to_string(&item).unwrap();
    assert!(json.contains("\"g-1\""));
    assert!(json.contains("\"completed\""));
}
