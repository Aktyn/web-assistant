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

use super::chrome::Chrome;

pub fn find_chrome_executable() -> Result<PathBuf> {
    Chrome::find_executable()
}

pub fn get_available_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("Failed to bind to random port")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

#[derive(Serialize)]
struct CdpRequest {
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
        let (_tx, _rx) = mpsc::channel::<(CdpRequest, oneshot::Sender<Result<Value>>)>(1);
        Self {
            next_id: Arc::new(AtomicU64::new(1)),
            tx_cmd: _tx,
        }
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
            None => find_chrome_executable()?,
        };

        let port = get_available_port()?;
        let (resolved_data_dir, is_temp) = match user_data_dir {
            Some(dir) => (dir.to_path_buf(), false),
            None => {
                let dir = Chrome::resolve_user_data_dir(Some(&executable), None)?;
                (dir, false)
            }
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
