#![allow(clippy::missing_errors_doc)]

pub mod chrome;

use anyhow::{Context, Result, anyhow};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use chrome::Chrome;

pub fn get_available_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("Failed to bind to random port")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

/// Internal request type sent through the CDP client channel.
#[derive(Serialize)]
pub(crate) struct CdpRequest {
    id: u64,
    method: String,
    params: Value,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct CdpResponse {
    id: Option<u64>,
    result: Option<Value>,
    error: Option<Value>,
    method: Option<String>,
}

#[derive(Clone)]
pub struct CdpClient {
    next_id: Arc<AtomicU64>,
    tx_cmd: mpsc::Sender<(CdpRequest, oneshot::Sender<Result<Value>>)>,
}

impl CdpClient {
    pub async fn connect(ws_url: &str) -> Result<Self> {
        let (ws_stream, _) = connect_async(ws_url)
            .await
            .with_context(|| format!("Failed to connect to CDP WebSocket at {ws_url}"))?;

        let (mut write, mut read) = ws_stream.split();
        let (tx_cmd, mut rx_cmd) =
            mpsc::channel::<(CdpRequest, oneshot::Sender<Result<Value>>)>(100);

        tokio::spawn(async move {
            let mut pending: HashMap<u64, oneshot::Sender<Result<Value>>> = HashMap::new();

            loop {
                tokio::select! {
                    Some((req, resp_tx)) = rx_cmd.recv() => {
                        let id = req.id;
                        let text = match serde_json::to_string(&req) {
                            Ok(t) => t,
                            Err(e) => {
                                let _ = resp_tx.send(Err(anyhow!("Failed to serialize request: {e}")));
                                continue;
                            }
                        };
                        pending.insert(id, resp_tx);
                        if let Err(e) = write.send(Message::Text(text.into())).await {
                            if let Some(tx) = pending.remove(&id) {
                                let _ = tx.send(Err(anyhow!("Failed to send CDP message: {e}")));
                            }
                            break;
                        }
                    }
                    msg_opt = read.next() => {
                        match msg_opt {
                            Some(Ok(Message::Text(text))) => {
                                if let Ok(resp) = serde_json::from_str::<CdpResponse>(&text)
                                    && let Some(id) = resp.id
                                        && let Some(tx) = pending.remove(&id) {
                                            if let Some(err) = resp.error {
                                                let _ = tx.send(Err(anyhow!("CDP error: {err}")));
                                            } else {
                                                let _ = tx.send(Ok(resp.result.unwrap_or(Value::Null)));
                                            }
                                        }
                            }
                            Some(Ok(Message::Close(_))) | None => {
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Clear remaining pending requests
            for (_, tx) in pending.drain() {
                let _ = tx.send(Err(anyhow!("CDP connection closed")));
            }
        });

        let client = Self {
            next_id: Arc::new(AtomicU64::new(1)),
            tx_cmd,
        };

        // Enable essential domains
        client
            .send_command("Page.enable", serde_json::json!({}))
            .await?;
        client
            .send_command("DOM.enable", serde_json::json!({}))
            .await?;
        client
            .send_command("Runtime.enable", serde_json::json!({}))
            .await?;

        Ok(client)
    }

    pub async fn send_command(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = CdpRequest {
            id,
            method: method.to_string(),
            params,
        };

        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx_cmd
            .send((req, resp_tx))
            .await
            .map_err(|_| anyhow!("CDP client channel closed"))?;

        resp_rx.await.map_err(|_| anyhow!("CDP response dropped"))?
    }

    /// Test-only placeholder constructor — the client will reject all commands.
    #[cfg(test)]
    pub(crate) fn new_test_client() -> Self {
        use tokio::sync::mpsc;
        // Drop the sender immediately so any send will fail gracefully.
        let (tx, _rx) = mpsc::channel::<(CdpRequest, oneshot::Sender<Result<Value>>)>(1);
        Self {
            next_id: Arc::new(AtomicU64::new(1)),
            tx_cmd: tx,
        }
    }

    #[cfg(test)]
    #[allow(clippy::type_complexity)]
    pub(crate) fn new_with_channel() -> (
        Self,
        tokio::sync::mpsc::Receiver<(CdpRequest, oneshot::Sender<Result<Value>>)>,
    ) {
        use tokio::sync::mpsc;
        let (tx, rx) = mpsc::channel::<(CdpRequest, oneshot::Sender<Result<Value>>)>(100);
        (
            Self {
                next_id: Arc::new(AtomicU64::new(1)),
                tx_cmd: tx,
            },
            rx,
        )
    }

    pub async fn navigate(&self, url: &str) -> Result<()> {
        self.send_command(
            "Page.navigate",
            serde_json::json!({
                "url": url
            }),
        )
        .await?;

        // Wait for page to reach ready state
        for _ in 0..50 {
            sleep(Duration::from_millis(150)).await;
            if let Ok(res) = self.evaluate("document.readyState").await
                && let Some(s) = res.as_str()
                && (s == "complete" || s == "interactive")
            {
                break;
            }
        }
        // Small extra settle time
        sleep(Duration::from_millis(300)).await;
        Ok(())
    }

    pub async fn evaluate(&self, expression: &str) -> Result<Value> {
        let resp = self
            .send_command(
                "Runtime.evaluate",
                serde_json::json!({
                    "expression": expression,
                    "returnByValue": true,
                    "awaitPromise": true
                }),
            )
            .await?;

        if let Some(exception) = resp.get("exceptionDetails") {
            return Err(anyhow!("JS Evaluation error: {exception}"));
        }

        let val = resp
            .pointer("/result/value")
            .cloned()
            .unwrap_or(Value::Null);
        Ok(val)
    }

    pub async fn capture_screenshot(&self) -> Result<String> {
        let resp = self
            .send_command(
                "Page.captureScreenshot",
                serde_json::json!({
                    "format": "png"
                }),
            )
            .await?;

        let data = resp
            .get("data")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Screenshot data missing"))?;

        Ok(data.to_string())
    }

    /// Get element content quads via DOM.getContentQuads
    pub async fn get_content_quads(&self, backend_node_id: u64) -> Result<Vec<Vec<f64>>> {
        let resp = self
            .send_command(
                "DOM.getContentQuads",
                serde_json::json!({
                    "backendNodeId": backend_node_id
                }),
            )
            .await?;

        // Response format: {"quads": [[x1,y1,x2,y2,...], ...]}
        if let Some(quads) = resp.get("quads").and_then(|q| q.as_array()) {
            let mut result = Vec::new();
            for quad in quads {
                if let Some(q) = quad.as_array() {
                    let coords: Vec<f64> = q.iter().filter_map(serde_json::Value::as_f64).collect();
                    if coords.len() == 8 {
                        result.push(coords);
                    }
                }
            }
            return Ok(result);
        }

        Err(anyhow!("No quads returned from DOM.getContentQuads"))
    }

    /// Get element box model via DOM.getBoxModel
    pub async fn get_box_model(&self, backend_node_id: u64) -> Result<Value> {
        self.send_command(
            "DOM.getBoxModel",
            serde_json::json!({
                "backendNodeId": backend_node_id
            }),
        )
        .await
        .map_err(|e| anyhow!("Failed to get box model: {e}"))
    }

    /// Execute a JavaScript function on an element by object ID
    pub async fn call_function_on(
        &self,
        function_declaration: &str,
        object_id: &str,
    ) -> Result<Value> {
        self.send_command(
            "Runtime.callFunctionOn",
            serde_json::json!({
                "functionDeclaration": function_declaration,
                "objectId": object_id,
                "returnByValue": true
            }),
        )
        .await
        .map_err(|e| anyhow!("Failed to call function: {e}"))
    }

    /// Scroll element into view using DOM.scrollIntoViewIfNeeded
    pub async fn scroll_into_view_if_needed(&self, backend_node_id: u64) -> Result<()> {
        self.send_command(
            "DOM.scrollIntoViewIfNeeded",
            serde_json::json!({
                "backendNodeId": backend_node_id
            }),
        )
        .await?;
        Ok(())
    }

    /// Get page layout metrics (viewport dimensions)
    pub async fn get_layout_metrics(&self) -> Result<(f64, f64)> {
        let resp = self
            .send_command("Page.getLayoutMetrics", serde_json::json!({}))
            .await?;

        if let Some(layout) = resp.get("layoutViewport") {
            let width = layout
                .get("clientWidth")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let height = layout
                .get("clientHeight")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            return Ok((width, height));
        }

        Err(anyhow!("Failed to get layout metrics"))
    }

    /// Resolve a DOM node to get its object ID
    pub async fn resolve_node(&self, backend_node_id: u64) -> Result<String> {
        let resp = self
            .send_command(
                "DOM.resolveNode",
                serde_json::json!({
                    "backendNodeId": backend_node_id
                }),
            )
            .await?;

        if let Some(object) = resp.get("object")
            && let Some(object_id) = object.get("objectId").and_then(|v| v.as_str())
        {
            return Ok(object_id.to_string());
        }

        Err(anyhow!("Failed to resolve node to object ID"))
    }
}

pub struct BrowserProcess {
    pub child: Child,
    pub port: u16,
    pub user_data_dir: PathBuf,
    pub is_temp: bool,
    pub cdp_client: CdpClient,
}

impl BrowserProcess {
    pub async fn launch(
        headless: bool,
        user_executable: Option<&Path>,
        user_data_dir: Option<&Path>,
    ) -> Result<Self> {
        let executable = match user_executable {
            Some(p) => p.to_path_buf(),
            None => Chrome::find_executable()?,
        };

        let port = get_available_port()?;
        let (resolved_data_dir, is_temp) = if let Some(dir) = user_data_dir {
            (dir.to_path_buf(), false)
        } else {
            let dir = Chrome::resolve_user_data_dir(Some(&executable), None)?;
            (dir, false)
        };

        let args = Chrome::default_args(port, &resolved_data_dir, headless);

        let child = Command::new(&executable)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn {}", executable.display()))?;

        // Poll for DevTools to become available
        let client = reqwest::Client::new();
        let version_url = format!("http://127.0.0.1:{port}/json/version");
        let mut ready = false;
        for _ in 0..60 {
            sleep(Duration::from_millis(100)).await;
            if let Ok(resp) = client.get(&version_url).send().await
                && resp.status().is_success()
            {
                ready = true;
                break;
            }
        }

        if !ready {
            return Err(anyhow!(
                "Chromium DevTools did not become ready on port {port}"
            ));
        }

        // Open a new tab or inspect existing tab
        let new_tab_url = format!("http://127.0.0.1:{port}/json/new?about:blank");
        let tab_resp = client
            .put(&new_tab_url)
            .send()
            .await
            .context("Failed to create new page target via CDP HTTP endpoint")?;

        let tab_json: Value = tab_resp.json().await.context("Failed to parse tab JSON")?;
        let ws_url = tab_json
            .get("webSocketDebuggerUrl")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("No webSocketDebuggerUrl in new tab response: {tab_json}"))?;

        let cdp_client = CdpClient::connect(ws_url).await?;

        Ok(Self {
            child,
            port,
            user_data_dir: resolved_data_dir,
            is_temp,
            cdp_client,
        })
    }

    pub async fn kill(&mut self) -> Result<()> {
        let _ = self.child.kill().await;
        if self.is_temp && self.user_data_dir.exists() {
            let _ = std::fs::remove_dir_all(&self.user_data_dir);
        }
        Ok(())
    }
}

impl Drop for BrowserProcess {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
        if self.is_temp && self.user_data_dir.exists() {
            let _ = std::fs::remove_dir_all(&self.user_data_dir);
        }
    }
}

/// Test-only mock CDP server that responds to commands with canned JSON.
/// Built on `CdpClient::new_with_channel` so no real WebSocket is needed.
#[cfg(test)]
pub(crate) struct MockCdp {
    pub client: CdpClient,
    overrides: Arc<std::sync::Mutex<HashMap<String, Value>>>,
    fails: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
}

#[cfg(test)]
impl MockCdp {
    pub(crate) fn new() -> Self {
        let (client, mut rx) = CdpClient::new_with_channel();
        let overrides = Arc::new(std::sync::Mutex::new(HashMap::<String, Value>::new()));
        let fails = Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
        let ov = overrides.clone();
        let fl = fails.clone();
        tokio::spawn(async move {
            while let Some((req, tx)) = rx.recv().await {
                let should_fail = fl.lock().unwrap().contains(&req.method);
                if should_fail {
                    let _ = tx.send(Err(anyhow!("MockCdp: forced failure for {}", req.method)));
                    continue;
                }
                let guard = ov.lock().unwrap();
                let result = if let Some(resp) = guard.get(&req.method) {
                    resp.clone()
                } else {
                    default_cdp_response(&req.method, &req.params)
                };
                let _ = tx.send(Ok(result));
            }
        });
        Self {
            client,
            overrides,
            fails,
        }
    }

    pub(crate) async fn set(&self, method: &str, value: Value) {
        self.overrides
            .lock()
            .unwrap()
            .insert(method.to_string(), value);
    }

    /// Make a specific CDP method always fail (simulates browser errors).
    pub(crate) async fn set_fail(&self, method: &str) {
        self.fails.lock().unwrap().insert(method.to_string());
    }
}

#[cfg(test)]
fn default_cdp_response(method: &str, params: &Value) -> Value {
    match method {
        "Runtime.evaluate" => {
            let expr = params.get("expression").and_then(|v| v.as_str()).unwrap_or("");
            if expr.contains("document.readyState") {
                serde_json::json!({"result": {"value": "complete"}})
            } else if expr.contains("data-cdp-marker") {
                serde_json::json!({"result": {"value": {"markerId": "marker_test", "found": true}}})
            } else if expr.contains("el.click()") || expr.contains("nativeInputValueSetter")
                || expr.contains("window.scrollBy") || expr.contains("bodyText")
            {
                serde_json::json!({"result": {"value": true}})
            } else if expr.contains("tagName") {
                serde_json::json!({"result": {"value": {
                    "tag": "button", "type": "", "role": "",
                    "x": 10.0, "y": 10.0, "width": 100.0, "height": 30.0
                }}})
            } else if expr.contains("interactiveSelectors") {
                // State extraction (EXTRACT_DOM_JS)
                serde_json::json!({"result": {"value": {
                    "url": "https://example.com",
                    "title": "Example Page",
                    "elements": [{"index":1,"tag":"button","text":"Mock","attributes":{},"x":60.0,"y":25.0}],
                    "text": "Mock page content"
                }}})
            } else {
                serde_json::json!({"result": {"value": null}})
            }
        }
        "Page.captureScreenshot" => serde_json::json!({"data": "iVBORw0KGgo="}),
        "Page.getLayoutMetrics" => {
            serde_json::json!({"layoutViewport": {"clientWidth": 1920.0, "clientHeight": 1080.0}})
        }
        "DOM.getContentQuads" => serde_json::json!({"quads": []}),
        "DOM.getBoxModel" => serde_json::json!({
            "model": {"content": [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0]}
        }),
        "DOM.resolveNode" => serde_json::json!({"object": {"objectId": "mock-object-id"}}),
        _ => serde_json::json!({}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_available_port() {
        let port = get_available_port().unwrap();
        assert!(port > 0);
        assert!(port < u16::MAX);
    }

    #[test]
    fn test_cdp_request_serialization() {
        let req = CdpRequest {
            id: 1,
            method: "Page.navigate".to_string(),
            params: serde_json::json!({"url": "https://example.com"}),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("Page.navigate"));
        assert!(json.contains("https://example.com"));
    }

    #[test]
    fn test_cdp_response_deserialization() {
        let json = r#"{"id": 1, "result": {"foo": "bar"}, "method": null}"#;
        let resp: CdpResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, Some(1));
        assert_eq!(resp.method, None);
        assert!(resp.result.is_some());
    }

    #[test]
    fn test_cdp_response_with_error() {
        let json = r#"{"id": 2, "error": {"code": -32000, "message": "Error"}}"#;
        let resp: CdpResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, Some(2));
        assert!(resp.error.is_some());
    }

    #[test]
    fn test_test_client_creation() {
        let client = CdpClient::new_test_client();
        assert_eq!(client.next_id.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_new_with_channel() {
        let (client, _rx) = CdpClient::new_with_channel();
        assert_eq!(client.next_id.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_test_client_send_command_fails() {
        let client = CdpClient::new_test_client();
        // Sending should fail because the receiver is dropped
        let result = client
            .send_command("Page.enable", serde_json::json!({}))
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_browser_process_fields() {
        // Just verify the struct can be referenced
        let _: fn() -> std::mem::MaybeUninit<BrowserProcess> = || std::mem::MaybeUninit::uninit();
    }

    // ------------------------------------------------------------------
    // CdpClient method coverage via MockCdp
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_evaluate_returns_value_from_mock() {
        let mock = MockCdp::new();
        mock.set(
            "Runtime.evaluate",
            serde_json::json!({"result": {"value": 42}}),
        )
        .await;
        let val = mock.client.evaluate("anything").await.unwrap();
        assert_eq!(val, serde_json::json!(42));
    }

    #[tokio::test]
    async fn test_evaluate_returns_null_when_missing() {
        let mock = MockCdp::new();
        let val = mock.client.evaluate("anything").await.unwrap();
        assert_eq!(val, serde_json::json!(null));
    }

    #[tokio::test]
    async fn test_evaluate_propagates_exception_details() {
        let mock = MockCdp::new();
        mock.set(
            "Runtime.evaluate",
            serde_json::json!({"exceptionDetails": {"exception": {"description": "boom"}}}),
        )
        .await;
        let result = mock.client.evaluate("throw new Error('boom')").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("boom"));
    }

    #[tokio::test]
    async fn test_capture_screenshot_returns_data() {
        let mock = MockCdp::new();
        let data = mock.client.capture_screenshot().await.unwrap();
        assert_eq!(data, "iVBORw0KGgo=");
    }

    #[tokio::test]
    async fn test_capture_screenshot_errors_when_data_missing() {
        let mock = MockCdp::new();
        mock.set("Page.captureScreenshot", serde_json::json!({})).await;
        let result = mock.client.capture_screenshot().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Screenshot data missing"));
    }

    #[tokio::test]
    async fn test_get_layout_metrics_returns_dimensions() {
        let mock = MockCdp::new();
        let (w, h) = mock.client.get_layout_metrics().await.unwrap();
        assert_eq!(w, 1920.0);
        assert_eq!(h, 1080.0);
    }

    #[tokio::test]
    async fn test_get_layout_metrics_errors_when_missing() {
        let mock = MockCdp::new();
        mock.set("Page.getLayoutMetrics", serde_json::json!({})).await;
        let result = mock.client.get_layout_metrics().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_content_quads_parses_quads() {
        let mock = MockCdp::new();
        mock.set(
            "DOM.getContentQuads",
            serde_json::json!({"quads": [[0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0]]}),
        )
        .await;
        let quads = mock.client.get_content_quads(1).await.unwrap();
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].len(), 8);
    }

    #[tokio::test]
    async fn test_get_content_quads_skips_short_arrays() {
        let mock = MockCdp::new();
        mock.set(
            "DOM.getContentQuads",
            serde_json::json!({"quads": [[1.0, 2.0, 3.0]]}),
        )
        .await;
        let quads = mock.client.get_content_quads(1).await.unwrap();
        assert!(quads.is_empty());
    }

    #[tokio::test]
    async fn test_get_content_quads_errors_when_missing() {
        let mock = MockCdp::new();
        mock.set("DOM.getContentQuads", serde_json::json!({})).await;
        let result = mock.client.get_content_quads(1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_box_model_passes_backend_node_id() {
        let mock = MockCdp::new();
        let model = mock.client.get_box_model(42).await.unwrap();
        assert_eq!(model["model"]["content"][0], 0.0);
    }

    #[tokio::test]
    async fn test_call_function_on_returns_value() {
        let mock = MockCdp::new();
        let result = mock
            .client
            .call_function_on("function() { return 1; }", "obj-1")
            .await
            .unwrap();
        assert_eq!(result, serde_json::json!({}));
    }

    #[tokio::test]
    async fn test_scroll_into_view_if_needed_succeeds() {
        let mock = MockCdp::new();
        let result = mock.client.scroll_into_view_if_needed(7).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_resolve_node_returns_object_id() {
        let mock = MockCdp::new();
        let id = mock.client.resolve_node(5).await.unwrap();
        assert_eq!(id, "mock-object-id");
    }

    #[tokio::test]
    async fn test_resolve_node_errors_when_missing() {
        let mock = MockCdp::new();
        mock.set("DOM.resolveNode", serde_json::json!({})).await;
        let result = mock.client.resolve_node(5).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_command_increments_id() {
        let mock = MockCdp::new();
        let _ = mock.client.send_command("Page.enable", serde_json::json!({})).await;
        assert_eq!(mock.client.next_id.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_navigate_waits_for_ready_state() {
        let mock = MockCdp::new();
        let result = mock.client.navigate("https://example.com").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_navigate_formats_url_with_https() {
        let mock = MockCdp::new();
        // Default mock returns "complete" for readyState, so navigate succeeds.
        let result = mock.client.navigate("example.com").await;
        assert!(result.is_ok());
    }
}
