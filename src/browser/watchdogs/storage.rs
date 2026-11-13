#![allow(clippy::missing_errors_doc)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

/// Cookie fingerprint key: (name, domain, path).
type CookieFingerprint = std::collections::HashSet<(String, String, String)>;

use crate::browser::cdp::CdpClient;

/// Storage item representing a single origin's local or session storage data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageItem {
    pub name: String,
    pub value: String,
}

/// Origin entry containing optional local and session storage items
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct OriginEntry {
    pub origin: String,
    #[serde(rename = "localStorage", skip_serializing_if = "Option::is_none")]
    pub local_storage: Option<Vec<StorageItem>>,
    #[serde(rename = "sessionStorage", skip_serializing_if = "Option::is_none")]
    pub session_storage: Option<Vec<StorageItem>>,
}

/// Full storage state combining cookies and origin entries
/// Matches the format used by `BrowserUse`'s Python `storage_state_watchdog.py`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct StorageState {
    #[serde(default)]
    pub cookies: Vec<Value>,
    #[serde(default)]
    pub origins: Vec<OriginEntry>,
}

#[derive(Clone)]
pub struct StorageStateWatchdog {
    client: CdpClient,
    last_cookie_snapshot: Arc<Mutex<Option<CookieFingerprint>>>,
}

impl StorageStateWatchdog {
    #[must_use]
    pub fn new(client: CdpClient) -> Self {
        Self {
            client,
            last_cookie_snapshot: Arc::new(Mutex::new(None)),
        }
    }

    /// Construct a placeholder watchdog for unit tests that only exercise
    /// pure methods (serialization, cookie normalization).
    #[cfg(test)]
    pub(crate) fn new_test() -> Self {
        Self {
            client: CdpClient::new_test_client(),
            last_cookie_snapshot: Arc::new(Mutex::new(None)),
        }
    }

    async fn get_cookies_raw(&self) -> Result<Vec<Value>> {
        let resp = self
            .client
            .send_command("Storage.getCookies", serde_json::json!({}))
            .await?;

        let cookies = resp
            .get("cookies")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(cookies)
    }

    pub async fn get_cookies(&self) -> Result<Vec<Value>> {
        let cookies = self.get_cookies_raw().await?;
        Ok(cookies.into_iter().map(normalize_cookie_expiry).collect())
    }

    pub async fn set_cookies(&self, cookies: &[Value]) -> Result<()> {
        // Build normalized cookies array (normalize expiry on the fly)
        let normalized: Vec<Value> = cookies
            .iter()
            .map(|c| normalize_cookie_expiry(c.clone()))
            .collect();

        self.client
            .send_command(
                "Storage.setCookies",
                serde_json::json!({
                    "cookies": normalized
                }),
            )
            .await?;

        debug!(
            "StorageStateWatchdog: Restored {} cookies",
            normalized.len()
        );
        Ok(())
    }

    /// Get current localStorage + sessionStorage content via JavaScript evaluation.
    /// Returns an array of origin entries with their storage contents.
    pub async fn get_all_storage_items(&self) -> Result<Vec<OriginEntry>> {
        let js = r"
        (() => {
            const entries = [];
            try {
                // Walk through each window in case of cross-origin iframes (best-effort)
                const currentOrigin = window.location.origin || 'current';
                const localItems = [];
                const sessionItems = [];
                for (let i = 0; i < window.localStorage.length; i++) {
                    const key = window.localStorage.key(i);
                    localItems.push({ name: key, value: window.localStorage.getItem(key) });
                }
                for (let i = 0; i < window.sessionStorage.length; i++) {
                    const key = window.sessionStorage.key(i);
                    sessionItems.push({ name: key, value: window.sessionStorage.getItem(key) });
                }
                if (localItems.length > 0 || sessionItems.length > 0) {
                    entries.push({ origin: currentOrigin, localStorage: localItems, sessionStorage: sessionItems });
                }
            } catch (e) {}
            return entries;
        })()
        ";

        let result = self.client.evaluate(js).await?;
        let items: Vec<OriginEntry> = serde_json::from_value(result).unwrap_or_default();
        Ok(items)
    }

    /// Save full storage state (cookies + localStorage/sessionStorage) to file.
    ///
    /// Format matches `BrowserUse`'s Python JSON output for easy interoperability.
    pub async fn save_storage_state(&self, path: impl AsRef<Path>) -> Result<()> {
        let cookies = self.get_cookies().await?;
        let origins = self.get_all_storage_items().await?;

        let state = StorageState { cookies, origins };
        let json_str = serde_json::to_string_pretty(&state)?;
        std::fs::write(path.as_ref(), json_str).with_context(|| {
            format!(
                "Failed to save storage state to {}",
                path.as_ref().display()
            )
        })?;
        Ok(())
    }

    /// Load full storage state (cookies + localStorage/sessionStorage) from file.
    ///
    /// Cookies are applied via CDP; localStorage/sessionStorage are injected via
    /// eval scripts scoped to the matching origin.
    pub async fn load_storage_state(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = std::fs::read_to_string(path.as_ref()).with_context(|| {
            format!(
                "Failed to read storage state from {}",
                path.as_ref().display()
            )
        })?;
        let state: StorageState = serde_json::from_str(&content)?;
        self.set_cookies(&state.cookies).await?;

        // Restore localStorage and sessionStorage via injected scripts per origin
        for origin in &state.origins {
            let mut scripts: Vec<String> = Vec::new();

            if let Some(ref local_items) = origin.local_storage {
                let lines: Vec<String> = local_items
                    .iter()
                    .map(|item| {
                        let n = serde_json::to_string(&item.name).unwrap_or_default();
                        let v = serde_json::to_string(&item.value).unwrap_or_default();
                        format!("try {{ window.localStorage.setItem({n}, {v}); }} catch(e) {{}}")
                    })
                    .collect();
                scripts.push(format!(
                    "(function(){{ if(window.location&&window.location.origin!=={})return;{} }})();",
                    serde_json::to_string(&origin.origin).unwrap_or_default(),
                    lines.join(" ")
                ));
            }

            if let Some(ref session_items) = origin.session_storage {
                let lines: Vec<String> = session_items
                    .iter()
                    .map(|item| {
                        let n = serde_json::to_string(&item.name).unwrap_or_default();
                        let v = serde_json::to_string(&item.value).unwrap_or_default();
                        format!("try {{ window.sessionStorage.setItem({n}, {v}); }} catch(e) {{}}")
                    })
                    .collect();
                scripts.push(format!(
                    "(function(){{ if(window.location&&window.location.origin!=={})return;{} }})();",
                    serde_json::to_string(&origin.origin).unwrap_or_default(),
                    lines.join(" ")
                ));
            }

            for script in scripts {
                let _ = self.client.evaluate(&script).await;
            }
        }

        debug!(
            "StorageStateWatchdog: Restored {} cookies, {} origins",
            state.cookies.len(),
            state.origins.len()
        );
        Ok(())
    }

    /// Compute a fingerprint of current cookies for change detection.
    pub async fn cookie_fingerprint(&self) -> Result<CookieFingerprint> {
        let cookies = self.get_cookies_raw().await?;
        let fp: CookieFingerprint = cookies
            .iter()
            .filter_map(|c| {
                let name = c
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(std::string::ToString::to_string)?;
                let domain = c
                    .get("domain")
                    .and_then(|d| d.as_str())
                    .map(std::string::ToString::to_string)?;
                let path = c
                    .get("path")
                    .and_then(|p| p.as_str())
                    .map(std::string::ToString::to_string)?;
                Some((name, domain, path))
            })
            .collect();
        Ok(fp)
    }

    /// Compare against last snapshot to detect changes.
    /// First call records the current state without reporting a change.
    pub async fn has_cookie_changes(&self) -> Result<bool> {
        let current = self.cookie_fingerprint().await?;
        let mut guard = self.last_cookie_snapshot.lock().await;
        if let Some(prev) = guard.as_ref() {
            let changed = prev != &current;
            *guard = Some(current);
            Ok(changed)
        } else {
            // First call — just record current state
            *guard = Some(current);
            Ok(false)
        }
    }

    /// Persist storage state to disk when cookies have changed since last check.
    pub async fn maybe_save_on_change(&self, path: &Path) -> Result<bool> {
        if !self.has_cookie_changes().await? {
            return Ok(false);
        }
        self.save_storage_state(path).await?;
        Ok(true)
    }
}

/// Normalize cookie expiry values for compatibility with different formats.
///
/// Python/BrowserUse normalizes session cookies (expires=0, -1, etc.) by omitting them
/// because CDP treats expires=0 as already-expired.
fn normalize_cookie_expiry(mut cookie: Value) -> Value {
    if let Some(obj) = cookie.as_object_mut()
        && let Some(expires) = obj.get("expires")
        && (expires.is_i64() || expires.is_f64())
        && let Some(val) = expires.as_f64()
        && val <= 0.0
    {
        obj.remove("expires");
    }
    cookie
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::cdp::MockCdp;
    use tempfile::tempdir;

    fn mock_storage() -> (StorageStateWatchdog, MockCdp) {
        let mock = MockCdp::new();
        let wd = StorageStateWatchdog::new(mock.client.clone());
        (wd, mock)
    }

    #[test]
    fn test_origin_entry_serialization() {
        let entry = OriginEntry {
            origin: "https://example.com".into(),
            local_storage: Some(vec![
                StorageItem {
                    name: "theme".into(),
                    value: "dark".into(),
                },
                StorageItem {
                    name: "lastVisit".into(),
                    value: "1700000000000".into(),
                },
            ]),
            session_storage: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"theme\""));
        assert!(json.contains("\"dark\""));

        let restored: OriginEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.origin, "https://example.com");
        assert_eq!(restored.local_storage.as_ref().unwrap().len(), 2);
        assert!(restored.session_storage.is_none());
    }

    #[test]
    fn test_storage_state_roundtrip() {
        let state = StorageState {
            cookies: vec![serde_json::json!({
                "name": "session_id",
                "value": "abc123",
                "domain": "example.com",
                "path": "/"
            })],
            origins: vec![OriginEntry {
                origin: "https://app.example.com".into(),
                local_storage: Some(vec![StorageItem {
                    name: "token".into(),
                    value: "xyz".into(),
                }]),
                session_storage: None,
            }],
        };

        let json = serde_json::to_string_pretty(&state).unwrap();
        let restored: StorageState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.cookies.len(), 1);
        assert_eq!(restored.origins.len(), 1);
        assert_eq!(
            restored.origins[0].local_storage.as_ref().unwrap()[0].name,
            "token"
        );
    }

    #[test]
    fn test_cookie_expiry_normalization_session() {
        // Session cookies with expires=0 or -1 should be stripped
        // normalize_cookie_expiry is pure; we only need the type for type-checking.
        let _wd = StorageStateWatchdog::new_test();

        let mut cookie = serde_json::json!({"name":"a","value":"b","domain":"x","expires":0});
        cookie = normalize_cookie_expiry(cookie);
        assert!(
            cookie.get("expires").is_none(),
            "expires=0 should be removed"
        );

        // Future-expiry cookie should be kept
        let mut cookie2 =
            serde_json::json!({"name":"a","value":"b","domain":"x","expires":9_999_999_999.0});
        cookie2 = normalize_cookie_expiry(cookie2);
        assert!(
            cookie2.get("expires").is_some(),
            "future expires should be kept"
        );

        // Negative expiry should be stripped
        let mut cookie3 = serde_json::json!({"name":"a","value":"b","domain":"x","expires":-1});
        cookie3 = normalize_cookie_expiry(cookie3);
        assert!(
            cookie3.get("expires").is_none(),
            "negative expires should be removed"
        );
    }

    #[test]
    fn test_origin_entry_equality() {
        let a = OriginEntry {
            origin: "https://ex.com".into(),
            local_storage: Some(vec![StorageItem {
                name: "k".into(),
                value: "v".into(),
            }]),
            session_storage: None,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn test_storage_item_serialization() {
        let item = StorageItem {
            name: "myKey".into(),
            value: "myValue".into(),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert_eq!(json, "{\"name\":\"myKey\",\"value\":\"myValue\"}");
        let restored: StorageItem = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, item);
    }

    #[test]
    fn test_empty_origin_entry_deserialize() {
        // Origin entry without any storage items should deserialize cleanly
        let json = "{\"origin\":\"https://empty.example.com\"}";
        let entry: OriginEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.origin, "https://empty.example.com");
        assert!(entry.local_storage.is_none());
        assert!(entry.session_storage.is_none());
    }

    #[test]
    fn test_full_storage_state_format_matches_python_output() {
        // The JSON format must match BrowserUse's Python output for cross-language interoperability
        let py_style_json = r#"
        {
          "cookies": [
            {"name": "token", "value": "secret", "domain": "example.com", "path": "/"}
          ],
          "origins": [
            {
              "origin": "https://example.com",
              "localStorage": [{"name": "key1", "value": "val1"}],
              "sessionStorage": [{"name": "sess", "value": "sval"}]
            }
          ]
        }
        "#;
        let state: StorageState = serde_json::from_str(py_style_json).unwrap();
        assert_eq!(state.cookies.len(), 1);
        assert_eq!(state.origins.len(), 1);
        assert_eq!(
            state.origins[0].local_storage.as_ref().unwrap()[0].name,
            "key1"
        );
        assert_eq!(
            state.origins[0].session_storage.as_ref().unwrap()[0].value,
            "sval"
        );
    }

    // ------------------------------------------------------------------
    // StorageStateWatchdog CDP-backed methods via MockCdp
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_get_cookies_returns_normalized() {
        let (wd, mock) = mock_storage();
        mock.set(
            "Storage.getCookies",
            serde_json::json!({"cookies": [
                {"name": "a", "value": "1", "domain": "x", "path": "/", "expires": -1},
                {"name": "b", "value": "2", "domain": "x", "path": "/", "expires": 9_999_999_999.0}
            ]}),
        )
        .await;
        let cookies = wd.get_cookies().await.unwrap();
        assert_eq!(cookies.len(), 2);
        // Session cookie expires stripped
        assert!(cookies[0].get("expires").is_none());
        // Future cookie expires preserved
        assert!(cookies[1].get("expires").is_some());
    }

    #[tokio::test]
    async fn test_get_cookies_empty_when_missing() {
        let (wd, mock) = mock_storage();
        mock.set("Storage.getCookies", serde_json::json!({})).await;
        let cookies = wd.get_cookies().await.unwrap();
        assert!(cookies.is_empty());
    }

    #[tokio::test]
    async fn test_set_cookies_sends_normalized() {
        let (wd, mock) = mock_storage();
        mock.set("Storage.setCookies", serde_json::json!({})).await;
        let result = wd
            .set_cookies(&[serde_json::json!({"name": "a", "expires": 0})])
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_all_storage_items_parses_origins() {
        let (wd, mock) = mock_storage();
        mock.set(
            "Runtime.evaluate",
            serde_json::json!({"result": {"value": [
                {"origin": "https://ex.com", "localStorage": [{"name": "k", "value": "v"}]}
            ]}}),
        )
        .await;
        let items = wd.get_all_storage_items().await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].origin, "https://ex.com");
        assert_eq!(
            items[0].local_storage.as_ref().unwrap()[0].name,
            "k"
        );
    }

    #[tokio::test]
    async fn test_get_all_storage_items_invalid_json_returns_empty() {
        let (wd, mock) = mock_storage();
        mock.set(
            "Runtime.evaluate",
            serde_json::json!({"result": {"value": "not json"}}),
        )
        .await;
        let items = wd.get_all_storage_items().await.unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_save_storage_state_writes_file() {
        let (wd, mock) = mock_storage();
        mock.set(
            "Storage.getCookies",
            serde_json::json!({"cookies": [{"name": "c", "value": "v", "domain": "d", "path": "/"}]}),
        )
        .await;
        mock.set(
            "Runtime.evaluate",
            serde_json::json!({"result": {"value": []}}),
        )
        .await;
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("state.json");
        wd.save_storage_state(&path).await.unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let state: StorageState = serde_json::from_str(&content).unwrap();
        assert_eq!(state.cookies.len(), 1);
        assert!(state.origins.is_empty());
    }

    #[tokio::test]
    async fn test_load_storage_state_applies_cookies_and_scripts() {
        let (wd, mock) = mock_storage();
        mock.set("Storage.setCookies", serde_json::json!({})).await;
        mock.set(
            "Runtime.evaluate",
            serde_json::json!({"result": {"value": true}}),
        )
        .await;

        let tmp = tempdir().unwrap();
        let path = tmp.path().join("state.json");
        let state = StorageState {
            cookies: vec![serde_json::json!({
                "name": "tok", "value": "s", "domain": "ex.com", "path": "/"
            })],
            origins: vec![OriginEntry {
                origin: "https://ex.com".into(),
                local_storage: Some(vec![StorageItem {
                    name: "k".into(),
                    value: "v".into(),
                }]),
                session_storage: Some(vec![StorageItem {
                    name: "sk".into(),
                    value: "sv".into(),
                }]),
            }],
        };
        std::fs::write(&path, serde_json::to_string_pretty(&state).unwrap()).unwrap();
        let result = wd.load_storage_state(&path).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_load_storage_state_missing_file_errors() {
        let (wd, _mock) = mock_storage();
        let result = wd
            .load_storage_state("/nonexistent/path/state.json")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_load_storage_state_invalid_json_errors() {
        let (wd, _mock) = mock_storage();
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("bad.json");
        std::fs::write(&path, "{not json").unwrap();
        let result = wd.load_storage_state(&path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cookie_fingerprint_builds_set() {
        let (wd, mock) = mock_storage();
        mock.set(
            "Storage.getCookies",
            serde_json::json!({"cookies": [
                {"name": "a", "domain": "x", "path": "/"},
                {"name": "b", "domain": "y", "path": "/"},
                {"no_name": true}
            ]}),
        )
        .await;
        let fp = wd.cookie_fingerprint().await.unwrap();
        assert_eq!(fp.len(), 2);
        assert!(fp.contains(&("a".into(), "x".into(), "/".into())));
        assert!(fp.contains(&("b".into(), "y".into(), "/".into())));
    }

    #[tokio::test]
    async fn test_has_cookie_changes_first_call_no_change() {
        let (wd, mock) = mock_storage();
        mock.set(
            "Storage.getCookies",
            serde_json::json!({"cookies": []}),
        )
        .await;
        assert!(!wd.has_cookie_changes().await.unwrap());
    }

    #[tokio::test]
    async fn test_has_cookie_changes_detects_change() {
        let (wd, mock) = mock_storage();
        mock.set(
            "Storage.getCookies",
            serde_json::json!({"cookies": []}),
        )
        .await;
        assert!(!wd.has_cookie_changes().await.unwrap());

        // Simulate cookie change
        mock.set(
            "Storage.getCookies",
            serde_json::json!({"cookies": [
                {"name": "new", "domain": "d", "path": "/"}
            ]}),
        )
        .await;
        assert!(wd.has_cookie_changes().await.unwrap());

        // Same state again → no change
        assert!(!wd.has_cookie_changes().await.unwrap());
    }

    #[tokio::test]
    async fn test_maybe_save_on_change_no_change_returns_false() {
        let (wd, mock) = mock_storage();
        mock.set("Storage.getCookies", serde_json::json!({"cookies": []})).await;
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("state.json");
        assert!(!wd.maybe_save_on_change(&path).await.unwrap());
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn test_maybe_save_on_change_with_change_returns_true() {
        let (wd, mock) = mock_storage();
        mock.set("Storage.getCookies", serde_json::json!({"cookies": []})).await;
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("state.json");
        assert!(!wd.maybe_save_on_change(&path).await.unwrap());

        // Change cookies, second call should save
        mock.set(
            "Storage.getCookies",
            serde_json::json!({"cookies": [
                {"name": "new", "domain": "d", "path": "/"}
            ]}),
        )
        .await;
        let saved = wd.maybe_save_on_change(&path).await.unwrap();
        assert!(saved);
        assert!(path.exists());
    }
}
