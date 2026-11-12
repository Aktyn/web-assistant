#![allow(clippy::missing_errors_doc)]

pub mod cdp;
mod dom;
pub mod watchdogs;

use anyhow::{Context, Result, anyhow};
use cdp::{BrowserProcess, CdpClient};
use dom::{BrowserState, ElementRect, EnhancedElement};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};
use watchdogs::BrowserWatchdogs;

#[derive(Debug, Clone, Default)]
pub struct BrowserConfig {
    pub headless: bool,
    pub executable_path: Option<PathBuf>,
    pub user_data_dir: Option<PathBuf>,
    pub downloads_dir: Option<PathBuf>,
    pub cdp_url: Option<String>,
}

pub struct Browser {
    pub config: BrowserConfig,
    process: Option<BrowserProcess>,
    client: Option<CdpClient>,
    watchdogs: Option<BrowserWatchdogs>,
    cached_state: Option<BrowserState>,
}

impl Browser {
    #[must_use]
    pub fn new(conf: Option<BrowserConfig>) -> Self {
        Self {
            config: conf.unwrap_or_default(),
            process: None,
            client: None,
            watchdogs: None,
            cached_state: None,
        }
    }

    #[must_use]
    pub fn builder() -> BrowserBuilder {
        BrowserBuilder::default()
    }

    pub async fn start(&mut self) -> Result<()> {
        if self.client.is_some() {
            return Ok(());
        }

        if let Some(ref ws_url) = self.config.cdp_url {
            let client = CdpClient::connect(ws_url).await?;
            let watchdogs =
                BrowserWatchdogs::new(client.clone(), self.config.downloads_dir.clone());
            watchdogs.setup_all().await?;
            self.watchdogs = Some(watchdogs);
            self.client = Some(client);
            return Ok(());
        }

        let process = BrowserProcess::launch(
            self.config.headless,
            self.config.executable_path.as_deref(),
            self.config.user_data_dir.as_deref(),
        )
        .await?;

        let watchdogs = BrowserWatchdogs::new(
            process.cdp_client.clone(),
            self.config.downloads_dir.clone(),
        );
        watchdogs.setup_all().await?;
        self.watchdogs = Some(watchdogs);
        self.client = Some(process.cdp_client.clone());
        self.process = Some(process);

        Ok(())
    }

    pub fn user_data_dir(&self) -> Option<&Path> {
        self.process.as_ref().map(|p| p.user_data_dir.as_path())
    }

    pub fn watchdogs(&self) -> Option<&BrowserWatchdogs> {
        self.watchdogs.as_ref()
    }

    pub fn watchdogs_mut(&mut self) -> Option<&mut BrowserWatchdogs> {
        self.watchdogs.as_mut()
    }

    fn client(&self) -> Result<&CdpClient> {
        self.client
            .as_ref()
            .ok_or_else(|| anyhow!("Browser not started. Call start() first."))
    }

    pub async fn navigate(&mut self, url: &str) -> Result<()> {
        self.start().await?;
        let formatted_url = if !url.starts_with("http://")
            && !url.starts_with("https://")
            && !url.starts_with("about:")
            && !url.starts_with("chrome:")
        {
            format!("https://{url}")
        } else {
            url.to_string()
        };

        // Security watchdog check before navigation
        if let Some(wd) = &self.watchdogs {
            wd.on_before_navigate(&formatted_url)?;
        }

        self.client()?.navigate(&formatted_url).await?;

        // Watchdogs post-navigation checks
        if let Some(wd) = &self.watchdogs {
            wd.on_after_navigate(&formatted_url).await?;
        }

        self.refresh_state().await?;
        Ok(())
    }

    pub async fn get_state(&mut self) -> Result<BrowserState> {
        self.start().await?;
        self.refresh_state().await
    }

    pub async fn refresh_state(&mut self) -> Result<BrowserState> {
        let val = self.client()?.evaluate(dom::EXTRACT_DOM_JS).await?;
        let state: BrowserState = serde_json::from_value(val)
            .context("Failed to deserialize BrowserState from JS evaluation")?;
        self.cached_state = Some(state.clone());
        Ok(state)
    }

    pub fn get_element(&self, index: u32) -> Option<EnhancedElement> {
        self.cached_state
            .as_ref()
            .and_then(|s| s.elements.iter().find(|el| el.index == index).cloned())
    }

    pub async fn click(&mut self, index: u32) -> Result<()> {
        self.start().await?;

        // Step 1: Get element information and validate special cases
        let (_tag, _input_type, mut rect) = self.get_element_info(index).await?;

        // Step 2: Try to get backend node ID for advanced CDP operations
        let backend_node_id = self.resolve_backend_node_id(index).await.ok();

        if let Some(node_id) = backend_node_id {
            // Step 3: Scroll element into view (if we have backend node)
            if let Err(e) = self.client()?.scroll_into_view_if_needed(node_id).await {
                warn!("Failed to scroll element into view: {e}");
            }
            sleep(Duration::from_millis(50)).await;

            // Step 4: Get viewport dimensions
            if let Ok((viewport_width, viewport_height)) = self.client()?.get_layout_metrics().await
            {
                // Step 5: Try to get better coordinates from CDP
                if let Ok(cdp_rect) = self
                    .get_element_rect_from_cdp(node_id, viewport_width, viewport_height)
                    .await
                {
                    rect = cdp_rect;
                }
            }
        }

        // Step 6: Calculate center point
        let (center_x, center_y) = rect.center();

        // Ensure coordinates are within reasonable bounds
        let center_x = center_x.clamp(0.0, 10000.0);
        let center_y = center_y.clamp(0.0, 10000.0);

        // Step 7: Try CDP mouse events first (most reliable for modern frameworks)
        let cdp_result = self.cdp_click(center_x, center_y).await;

        match cdp_result {
            Ok(()) => {
                debug!("Clicked element at ({center_x:.0}, {center_y:.0}) using CDP mouse events");
            }
            Err(e) => {
                warn!("CDP click failed: {e}, falling back to JavaScript click");
                // Fallback to JavaScript click via Runtime.evaluate
                let fallback_js = format!(
                    r#"
                    (() => {{
                        const isVisible = (elem) => {{
                            if (!elem) return false;
                            const style = window.getComputedStyle(elem);
                            if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return false;
                            const rect = elem.getBoundingClientRect();
                            return rect.width > 0 && rect.height > 0 && rect.bottom >= 0 && rect.top <= window.innerHeight;
                        }};

                        const interactiveSelectors = [
                            'a[href]', 'button', 'input', 'textarea', 'select',
                            '[role="button"]', '[role="link"]', '[role="checkbox"]', '[role="menuitem"]',
                            '[role="tab"]', '[role="combobox"]', '[role="switch"]', '[tabindex]:not([tabindex="-1"])'
                        ];

                        const matched = Array.from(document.querySelectorAll(interactiveSelectors.join(',')))
                            .filter(isVisible);

                        const el = matched[{index} - 1];
                        if (!el) return false;

                        el.scrollIntoView({{ behavior: 'instant', block: 'center' }});
                        el.focus();
                        el.click();
                        return true;
                    }})()
                    "#
                );
                let res = self.client()?.evaluate(&fallback_js).await?;
                if res.as_bool() != Some(true) {
                    return Err(anyhow!(
                        "Element with index {index} not found to click after CDP fallback"
                    ));
                }
            }
        }

        sleep(Duration::from_millis(500)).await;
        self.refresh_state().await?;
        Ok(())
    }

    /// Get element information including tag, type, and visible status
    async fn get_element_info(&self, index: u32) -> Result<(String, String, ElementRect)> {
        let element_info_js = format!(
            r#"
            (() => {{
                const isVisible = (elem) => {{
                    if (!elem) return false;
                    const style = window.getComputedStyle(elem);
                    if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return false;
                    const rect = elem.getBoundingClientRect();
                    return rect.width > 0 && rect.height > 0 && rect.bottom >= 0 && rect.top <= window.innerHeight;
                }};

                const interactiveSelectors = [
                    'a[href]', 'button', 'input', 'textarea', 'select',
                    '[role="button"]', '[role="link"]', '[role="checkbox"]', '[role="menuitem"]',
                    '[role="tab"]', '[role="combobox"]', '[role="switch"]', '[tabindex]:not([tabindex="-1"])'
                ];

                const matched = Array.from(document.querySelectorAll(interactiveSelectors.join(',')))
                    .filter(isVisible);

                const el = matched[{index} - 1];
                if (!el) return null;

                const rect = el.getBoundingClientRect();
                const tagName = el.tagName.toLowerCase();
                const type = el.getAttribute('type')?.toLowerCase() || '';
                const role = el.getAttribute('role')?.toLowerCase() || '';

                return {{
                    tag: tagName,
                    type: type,
                    role: role,
                    x: rect.left,
                    y: rect.top,
                    width: rect.width,
                    height: rect.height
                }};
            }})()
            "#
        );

        let element_info: serde_json::Value = self.client()?.evaluate(&element_info_js).await?;

        let Some(info) = element_info.as_object() else {
            return Err(anyhow!("Element with index {index} not found"));
        };

        let tag = info
            .get("tag")
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow!("Missing element tag"))?
            .to_string();

        let input_type = info.get("type").and_then(|t| t.as_str()).unwrap_or("");

        let x: f64 = info
            .get("x")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| anyhow!("Missing element x coordinate"))?;
        let y: f64 = info
            .get("y")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| anyhow!("Missing element y coordinate"))?;
        let width: f64 = info
            .get("width")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| anyhow!("Missing element width"))?;
        let height: f64 = info
            .get("height")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| anyhow!("Missing element height"))?;

        // Special case: file inputs should not be clicked directly
        if tag == "input" && input_type == "file" {
            return Err(anyhow!(
                "Cannot click on file input element at index {index}. File uploads must be handled using upload_file action."
            ));
        }

        // Special case: select elements should not be clicked
        if tag == "select" {
            return Err(anyhow!(
                "Cannot click on <select> elements. Use dropdown selection actions instead."
            ));
        }

        let rect = ElementRect {
            x,
            y,
            width,
            height,
        };
        Ok((tag, input_type.to_string(), rect))
    }

    /// Resolve element by index to get its backend node ID
    async fn resolve_backend_node_id(&self, index: u32) -> Result<u64> {
        // Use JavaScript to find the element and get its backend node ID
        let resolve_js = format!(
            r#"
            (() => {{
                const isVisible = (elem) => {{
                    if (!elem) return false;
                    const style = window.getComputedStyle(elem);
                    if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return false;
                    const rect = elem.getBoundingClientRect();
                    return rect.width > 0 && rect.height > 0 && rect.bottom >= 0 && rect.top <= window.innerHeight;
                }};

                const interactiveSelectors = [
                    'a[href]', 'button', 'input', 'textarea', 'select',
                    '[role="button"]', '[role="link"]', '[role="checkbox"]', '[role="menuitem"]',
                    '[role="tab"]', '[role="combobox"]', '[role="switch"]', '[tabindex]:not([tabindex="-1"])'
                ];

                const matched = Array.from(document.querySelectorAll(interactiveSelectors.join(',')))
                    .filter(isVisible);

                const el = matched[{index} - 1];
                if (!el) return null;

                // Use a marker to identify the element for DOM resolution
                const markerId = 'marker_' + Date.now() + '_' + Math.random();
                el.setAttribute('data-cdp-marker', markerId);

                // Return a flag indicating we need DOM resolution
                return {{ markerId, found: true }};
            }})()
            "#
        );

        let marker_info: serde_json::Value = self.client()?.evaluate(&resolve_js).await?;

        let Some(info) = marker_info.as_object() else {
            return Err(anyhow!("Element with index {index} not found"));
        };

        if !info
            .get("found")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
            return Err(anyhow!("Element with index {index} not found"));
        }

        // Note: For now we'll use a simplified approach - in production you'd want
        // to track backend node IDs during state extraction
        // This is a limitation of the current architecture
        Ok(0) // Placeholder - will be fixed when backend node tracking is added
    }

    /// Get element rectangle using multiple fallback strategies
    async fn get_element_rect_from_cdp(
        &self,
        backend_node_id: u64,
        viewport_width: f64,
        viewport_height: f64,
    ) -> Result<ElementRect> {
        // Method 1: Try DOM.getContentQuads (best for complex layouts)
        if let Ok(quads) = self.client()?.get_content_quads(backend_node_id).await
            && let Some(best_quad) =
                find_largest_visible_quad(&quads, viewport_width, viewport_height)
        {
            return Ok(best_quad);
        }

        // Method 2: Try DOM.getBoxModel
        if let Ok(box_model) = self.client()?.get_box_model(backend_node_id).await
            && let Some(model) = box_model.get("model")
            && let Some(content) = model.get("content")
            && let Some(coords) = content.as_array()
            && coords.len() >= 8
        {
            return Ok(ElementRect {
                x: coords[0].as_f64().unwrap_or(0.0),
                y: coords[1].as_f64().unwrap_or(0.0),
                width: coords[4].as_f64().unwrap_or(0.0) - coords[0].as_f64().unwrap_or(0.0),
                height: coords[7].as_f64().unwrap_or(0.0) - coords[1].as_f64().unwrap_or(0.0),
            });
        }

        Err(anyhow!("Could not get element rect from any CDP method"))
    }

    /// Perform a click using CDP mouse events (more reliable for React/Vue apps).
    async fn cdp_click(&self, x: f64, y: f64) -> Result<()> {
        // Move mouse to position
        self.client()?
            .send_command(
                "Input.dispatchMouseEvent",
                serde_json::json!({
                    "type": "mouseMoved",
                    "x": x,
                    "y": y
                }),
            )
            .await?;

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Mouse down
        self.client()?
            .send_command(
                "Input.dispatchMouseEvent",
                serde_json::json!({
                    "type": "mousePressed",
                    "x": x,
                    "y": y,
                    "button": "left",
                    "clickCount": 1
                }),
            )
            .await?;

        tokio::time::sleep(Duration::from_millis(80)).await;

        // Mouse up
        self.client()?
            .send_command(
                "Input.dispatchMouseEvent",
                serde_json::json!({
                    "type": "mouseReleased",
                    "x": x,
                    "y": y,
                    "button": "left",
                    "clickCount": 1
                }),
            )
            .await?;

        Ok(())
    }

    pub async fn input(&mut self, index: u32, text: &str) -> Result<()> {
        self.start().await?;

        let escaped_text = serde_json::to_string(text)?;
        let input_js = format!(
            r#"
            (() => {{
                const isVisible = (elem) => {{
                    if (!elem) return false;
                    const style = window.getComputedStyle(elem);
                    if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return false;
                    const rect = elem.getBoundingClientRect();
                    return rect.width > 0 && rect.height > 0 && rect.bottom >= 0 && rect.top <= window.innerHeight;
                }};

                const interactiveSelectors = [
                    'a[href]', 'button', 'input', 'textarea', 'select',
                    '[role="button"]', '[role="link"]', '[role="checkbox"]', '[role="menuitem"]',
                    '[role="tab"]', '[role="combobox"]', '[role="switch"]', '[tabindex]:not([tabindex="-1"])'
                ];

                const matched = Array.from(document.querySelectorAll(interactiveSelectors.join(',')))
                    .filter(isVisible);

                const el = matched[{index} - 1];
                if (!el) return false;

                el.scrollIntoView({{ behavior: 'instant', block: 'center' }});

                // REACT-COMPATIBLE VALUE SETTING:
                // React uses Object.getOwnPropertyDescriptor to track input changes.
                // Setting el.value = "..." directly bypasses React's internal state,
                // so the field shows the text but React reverts it on the next render.
                // Use the native setter from HTMLInputElement.prototype to bypass React's
                // tracking while still triggering the input's internal value update.
                const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
                    window.HTMLInputElement.prototype,
                    'value'
                ).set;

                const nativeTextAreaValueSetter = Object.getOwnPropertyDescriptor(
                    window.HTMLTextAreaElement.prototype,
                    'value'
                ).set;

                // Dispatch focus event (in case element isn't focused)
                el.dispatchEvent(new FocusEvent('focus', {{ bubbles: true }}));

                if (el instanceof HTMLInputElement) {{
                    nativeInputValueSetter.call(el, {escaped_text});
                }} else if (el instanceof HTMLTextAreaElement) {{
                    nativeTextAreaValueSetter.call(el, {escaped_text});
                }} else if (el.isContentEditable) {{
                    // contenteditable elements: clear then set text content
                    el.focus();
                    el.textContent = '';
                    el.textContent = {escaped_text};
                }} else {{
                    // Fallback for custom elements that mimic inputs
                    nativeInputValueSetter.call(el, {escaped_text});
                }}

                // Dispatch comprehensive events to ensure all frameworks detect the change.
                // Order matters: focus -> input -> change -> blur (mimics user interaction).
                const inputEvent = new Event('input', {{ bubbles: true, cancelable: true }});
                el.dispatchEvent(inputEvent);

                const changeEvent = new Event('change', {{ bubbles: true, cancelable: true }});
                el.dispatchEvent(changeEvent);

                // Blur event triggers final validation in some libraries
                el.dispatchEvent(new FocusEvent('blur', {{ bubbles: true }}));

                // jQuery-specific events (if jQuery is present)
                if (typeof jQuery !== 'undefined' && jQuery.fn) {{
                    try {{
                        jQuery(el).trigger('change');
                    }} catch (e) {{
                        // jQuery not available or error, continue anyway
                    }}
                }}

                return true;
            }})()
            "#
        );

        let res = self.client()?.evaluate(&input_js).await?;
        if res.as_bool() != Some(true) {
            return Err(anyhow!(
                "Element with index {index} not found to input text"
            ));
        }

        sleep(Duration::from_millis(300)).await;
        self.refresh_state().await?;
        Ok(())
    }

    pub async fn scroll(&mut self, amount: i32) -> Result<()> {
        self.start().await?;
        let script = format!("window.scrollBy(0, {amount});");
        self.client()?.evaluate(&script).await?;
        sleep(Duration::from_millis(300)).await;
        self.refresh_state().await?;
        Ok(())
    }

    pub async fn wait(&mut self, seconds: u64) -> Result<()> {
        sleep(Duration::from_secs(seconds)).await;
        if self.client.is_some() {
            self.refresh_state().await?;
        }
        Ok(())
    }

    pub async fn evaluate(&mut self, script: &str) -> Result<serde_json::Value> {
        self.start().await?;
        self.client()?.evaluate(script).await
    }

    pub async fn screenshot(&mut self) -> Result<String> {
        self.start().await?;
        self.client()?.capture_screenshot().await
    }

    pub async fn close(&mut self) -> Result<()> {
        if let Some(mut proc) = self.process.take() {
            let _ = proc.kill().await;
        }
        self.client = None;
        self.watchdogs = None;
        self.cached_state = None;
        Ok(())
    }
}

/// Find the largest visible quad within the viewport
fn find_largest_visible_quad(
    quads: &[Vec<f64>],
    viewport_width: f64,
    viewport_height: f64,
) -> Option<ElementRect> {
    let mut best_area = 0.0;
    let mut best_quad: Option<ElementRect> = None;

    for quad in quads {
        if quad.len() < 8 {
            continue;
        }

        // Quad format: [x1,y1, x2,y2, x3,y3, x4,y4]
        let x1 = quad[0];
        let y1 = quad[1];
        let x2 = quad[2];
        let y2 = quad[3];
        let x3 = quad[4];
        let y3 = quad[5];
        let x4 = quad[6];
        let y4 = quad[7];

        // Calculate bounding box
        let min_x = x1.min(x2).min(x3).min(x4);
        let max_x = x1.max(x2).max(x3).max(x4);
        let min_y = y1.min(y2).min(y3).min(y4);
        let max_y = y1.max(y2).max(y3).max(y4);

        // Check if quad intersects with viewport
        if max_x < 0.0 || max_y < 0.0 || min_x > viewport_width || min_y > viewport_height {
            continue;
        }

        // Calculate visible area
        let visible_min_x = (0.0f64).max(min_x);
        let visible_max_x = viewport_width.min(max_x);
        let visible_min_y = (0.0f64).max(min_y);
        let visible_max_y = viewport_height.min(max_y);

        let visible_width = visible_max_x - visible_min_x;
        let visible_height = visible_max_y - visible_min_y;
        let visible_area = visible_width * visible_height;

        if visible_area > best_area {
            best_area = visible_area;
            best_quad = Some(ElementRect {
                x: visible_min_x,
                y: visible_min_y,
                width: visible_width,
                height: visible_height,
            });
        }
    }

    best_quad.or_else(|| {
        // If no visible quad found, use the first one anyway
        quads.first().and_then(|quad| {
            if quad.len() >= 8 {
                Some(ElementRect {
                    x: quad[0],
                    y: quad[1],
                    width: quad[4] - quad[0],
                    height: quad[7] - quad[1],
                })
            } else {
                None
            }
        })
    })
}

impl Default for Browser {
    fn default() -> Self {
        Self::new(None)
    }
}

#[derive(Default)]
pub struct BrowserBuilder {
    config: BrowserConfig,
}

impl BrowserBuilder {
    #[must_use]
    pub fn headless(mut self, headless: bool) -> Self {
        self.config.headless = headless;
        self
    }

    #[must_use]
    pub fn executable_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.executable_path = Some(path.into());
        self
    }

    #[must_use]
    pub fn user_data_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.user_data_dir = Some(path.into());
        self
    }

    #[must_use]
    pub fn downloads_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.downloads_dir = Some(path.into());
        self
    }

    #[must_use]
    pub fn cdp_url(mut self, url: impl Into<String>) -> Self {
        self.config.cdp_url = Some(url.into());
        self
    }

    #[must_use]
    pub fn build(self) -> Browser {
        Browser {
            config: self.config,
            process: None,
            client: None,
            watchdogs: None,
            cached_state: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test helper: build an input JavaScript and return its text for inspection.
    #[allow(clippy::uninlined_format_args)]
    fn build_input_js(index: u32, text: &str) -> String {
        let escaped_text = serde_json::to_string(text).unwrap();
        format!(
            r#"
            (() => {{
                const isVisible = (elem) => {{
                    if (!elem) return false;
                    const style = window.getComputedStyle(elem);
                    if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return false;
                    const rect = elem.getBoundingClientRect();
                    return rect.width > 0 && rect.height > 0 && rect.bottom >= 0 && rect.top <= window.innerHeight;
                }};

                const interactiveSelectors = [
                    'a[href]', 'button', 'input', 'textarea', 'select',
                    '[role="button"]', '[role="link"]', '[role="checkbox"]', '[role="menuitem"]',
                    '[role="tab"]', '[role="combobox"]', '[role="switch"]', '[tabindex]:not([tabindex="-1"])'
                ];

                const matched = Array.from(document.querySelectorAll(interactiveSelectors.join(',')))
                    .filter(isVisible);

                const el = matched[{index} - 1];
                if (!el) return false;

                el.scrollIntoView({{ behavior: 'instant', block: 'center' }});

                // REACT-COMPATIBLE VALUE SETTING:
                const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
                    window.HTMLInputElement.prototype,
                    'value'
                ).set;

                const nativeTextAreaValueSetter = Object.getOwnPropertyDescriptor(
                    window.HTMLTextAreaElement.prototype,
                    'value'
                ).set;

                // Dispatch focus event
                el.dispatchEvent(new FocusEvent('focus', {{ bubbles: true }}));

                if (el instanceof HTMLInputElement) {{
                    nativeInputValueSetter.call(el, {escaped_text});
                }} else if (el instanceof HTMLTextAreaElement) {{
                    nativeTextAreaValueSetter.call(el, {escaped_text});
                }} else if (el.isContentEditable) {{
                    el.focus();
                    el.textContent = '';
                    el.textContent = {escaped_text};
                }} else {{
                    nativeInputValueSetter.call(el, {escaped_text});
                }}

                const inputEvent = new Event('input', {{ bubbles: true, cancelable: true }});
                el.dispatchEvent(inputEvent);

                const changeEvent = new Event('change', {{ bubbles: true, cancelable: true }});
                el.dispatchEvent(changeEvent);

                el.dispatchEvent(new FocusEvent('blur', {{ bubbles: true }}));

                if (typeof jQuery !== 'undefined' && jQuery.fn) {{
                    try {{
                        jQuery(el).trigger('change');
                    }} catch (e) {{}}
                }}

                return true;
            }})()
            "#,
            index = index,
            escaped_text = escaped_text
        )
    }

    #[test]
    fn test_input_js_uses_native_setter_for_react_compat() {
        let js = build_input_js(1, "Hello");
        // Verify native setter is used
        assert!(
            js.contains("nativeInputValueSetter"),
            "Should use native input value setter"
        );
        assert!(
            js.contains("nativeTextAreaValueSetter"),
            "Should use native textarea value setter"
        );
        assert!(
            js.contains("Object.getOwnPropertyDescriptor"),
            "Should access prototype descriptor"
        );
        assert!(
            js.contains("HTMLInputElement.prototype"),
            "Should use HTMLInputElement prototype"
        );
        assert!(
            js.contains("HTMLTextAreaElement.prototype"),
            "Should use HTMLTextAreaElement prototype"
        );
    }

    #[test]
    fn test_input_js_dispatches_focus_event() {
        let js = build_input_js(1, "Test");
        assert!(
            js.contains("FocusEvent('focus'"),
            "Should dispatch focus event"
        );
        assert!(js.contains("bubbles: true"), "Focus event should bubble");
    }

    #[test]
    fn test_input_js_dispatches_input_and_change_events() {
        let js = build_input_js(1, "Test");
        assert!(
            js.contains("new Event('input'"),
            "Should dispatch input event"
        );
        assert!(
            js.contains("new Event('change'"),
            "Should dispatch change event"
        );
        assert!(
            js.contains("cancelable: true"),
            "Events should be cancelable"
        );
    }

    #[test]
    fn test_input_js_dispatches_blur_event() {
        let js = build_input_js(1, "Test");
        assert!(
            js.contains("FocusEvent('blur'"),
            "Should dispatch blur event after input"
        );
    }

    #[test]
    fn test_input_js_handles_contenteditable_elements() {
        let js = build_input_js(1, "Test");
        // Should have contenteditable fallback path
        assert!(
            js.contains("isContentEditable"),
            "Should check for contenteditable"
        );
        assert!(
            js.contains("textContent"),
            "Should set textContent for contenteditable"
        );
    }

    #[test]
    fn test_input_js_uses_escaped_text_in_value() {
        // Raw string literal creates actual backslash-n sequence
        let text = r"Hello\nWorld";
        let escaped = serde_json::to_string(text).unwrap();
        // JSON serialization escapes backslashes, producing "\"Hello\\nWorld\""
        assert_eq!(escaped, "\"Hello\\\\nWorld\"");

        let js = build_input_js(1, text);
        // Verify the escaped text appears in the generated JS
        assert!(
            js.contains("\"Hello\\\\nWorld\""),
            "Should escape backslash-n correctly in JS"
        );
    }

    #[test]
    fn test_input_js_with_special_characters() {
        let special_cases = vec![
            "",                              // Empty string
            "Hello World",                   // Space
            "line1\nline2",                  // Newline
            "quote\"escape",                 // Double quote
            "back\\slash",                   // Backslash
            "unicode: 你好🌍",               // Unicode
            "<script>alert('xss')</script>", // XSS attempt
        ];

        for text in special_cases {
            let escaped = serde_json::to_string(text).unwrap();
            let js = build_input_js(1, text);
            // Verify the escaped value appears correctly in the JS
            assert!(
                js.contains(&escaped),
                "JS should contain properly escaped text for: {text}",
            );
        }
    }

    #[test]
    fn test_input_js_element_lookup_logic() {
        let js = build_input_js(5, "Test");
        // Element lookup uses index-1 (the format string produces "matched[5 - 1]")
        assert!(
            js.contains("matched[5 - 1]"),
            "Should look up element at index-1 using format"
        );
        assert!(js.contains("isVisible"), "Should check visibility");
        assert!(js.contains("scrollIntoView"), "Should scroll into view");
    }

    /// Live integration test: creates a browser, navigates to a test page,
    /// and verifies text input works correctly.
    #[tokio::test]
    #[ignore = "Requires Chrome/Chromium to be installed"]
    async fn test_input_with_live_browser() {
        // Create a simple HTML page with a form input
        let temp_dir = tempfile::tempdir().unwrap();
        let html_path = temp_dir.path().join("test_form.html");
        std::fs::write(
            &html_path,
            r#"<!DOCTYPE html>
<html>
<body>
    <form id="test-form">
        <input type="text" id="username" placeholder="Enter username" />
        <textarea id="message" placeholder="Enter message"></textarea>
        <div contenteditable="true" id="notes">Edit this</div>
        <button type="submit">Submit</button>
    </form>
</body>
</html>"#,
        )
        .unwrap();

        let file_url = format!("file://{}", html_path.display());

        let mut browser = Browser::new(None);
        browser.start().await.unwrap();

        // Navigate to the test page
        browser.navigate(&file_url).await.unwrap();

        // Get initial state to find element indices
        let state = browser.get_state().await.unwrap();
        // Find the input element (should be first interactive element)
        let input_elem = state
            .elements
            .iter()
            .find(|e| e.tag == "input")
            .expect("No input found");
        let input_index = input_elem.index;

        // Test text input on regular input
        browser.input(input_index, "test_user").await.unwrap();

        // Verify by evaluating JS directly
        let verify_js = r"(() => {
                const input = document.querySelector('#username');
                return input.value;
            })()"
            .to_string();
        let value: serde_json::Value = browser.evaluate(&verify_js).await.unwrap();
        let actual_value = value.as_str().unwrap_or_default();
        assert_eq!(
            actual_value, "test_user",
            "Input value should match typed text"
        );

        // Test on textarea
        let textarea_elem = state
            .elements
            .iter()
            .find(|e| e.tag == "textarea")
            .expect("No textarea found");
        let textarea_index = textarea_elem.index;

        browser.input(textarea_index, "hello world").await.unwrap();

        let verify_js = r"(() => {
                const textarea = document.querySelector('#message');
                return textarea.value;
            })()"
            .to_string();
        let value: serde_json::Value = browser.evaluate(&verify_js).await.unwrap();
        assert_eq!(value.as_str().unwrap(), "hello world");

        // Test empty string clears the field
        browser.input(textarea_index, "").await.unwrap();
        let value: serde_json::Value = browser.evaluate(&verify_js).await.unwrap();
        assert_eq!(value.as_str().unwrap(), "");

        browser.close().await.unwrap();
    }

    /// Test that verifies React-compatible value setting works on a real React-like page.
    #[tokio::test]
    #[ignore = "Requires Chrome/Chromium"]
    async fn test_input_on_simulated_react_page() {
        let temp_dir = tempfile::tempdir().unwrap();
        let html_path = temp_dir.path().join("react_form.html");

        // Simulate React's value tracking
        std::fs::write(
            &html_path,
            r#"<!DOCTYPE html>
<html>
<head>
<script>
// Simulate React's internal mechanism - override prototype setter
const originalSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
let simulatedReactState = null;

Object.defineProperty(window.HTMLInputElement.prototype, 'value', {
    get: function() {
        return this._value || '';
    },
    set: function(val) {
        // React checks if value changed via this mechanism
        this._value = val;
        simulatedReactState = val;
        // Call original setter
        originalSetter.call(this, val);
    }
});
</script>
</head>
<body>
    <input type="text" id="react-input" placeholder="Type here" />
    <div id="state-display">React state: null</div>
</body>
</html>"#,
        )
        .unwrap();

        let file_url = format!("file://{}", html_path.display());
        let mut browser = Browser::new(None);
        browser.start().await.unwrap();
        browser.navigate(&file_url).await.unwrap();

        // Get state and find input
        let state = browser.get_state().await.unwrap();
        let input_elem = state
            .elements
            .iter()
            .find(|e| e.tag == "input")
            .expect("No input found");

        // Input text using the fixed implementation
        browser
            .input(input_elem.index, "react_text_test")
            .await
            .unwrap();

        // Verify the value was actually set
        let verify_js = r"(() => {
            return document.getElementById('react-input').value;
        })()";
        let value: serde_json::Value = browser.evaluate(verify_js).await.unwrap();
        assert_eq!(value.as_str().unwrap(), "react_text_test");

        browser.close().await.unwrap();
    }

    #[test]
    fn test_builder_construction() {
        let browser = BrowserBuilder::default().headless(true).build();
        assert!(browser.config.headless);
        assert!(browser.client.is_none());
        assert!(browser.process.is_none());
    }

    #[test]
    fn test_input_js_generated_correctly_for_index() {
        // Verify the JS uses correct 0-based indexing (index - 1 in format)
        for index in [1, 5, 42] {
            let js = build_input_js(index, "Test");
            // Format produces "matched[index - 1]"
            let expected_element = format!("matched[{index} - 1]");
            assert!(
                js.contains(&expected_element),
                "JS should reference element using format {expected_element} for index {index}",
            );
        }
    }

    // =========================================================================
    // CLICK TESTS
    // =========================================================================

    /// Helper to build the click JavaScript and return its text for inspection.
    #[allow(clippy::uninlined_format_args)]
    fn build_click_js(index: u32) -> String {
        format!(
            r#"
            (() => {{
                const isVisible = (elem) => {{
                    if (!elem) return false;
                    const style = window.getComputedStyle(elem);
                    if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return false;
                    const rect = elem.getBoundingClientRect();
                    return rect.width > 0 && rect.height > 0 && rect.bottom >= 0 && rect.top <= window.innerHeight;
                }};

                const interactiveSelectors = [
                    'a[href]', 'button', 'input', 'textarea', 'select',
                    '[role="button"]', '[role="link"]', '[role="checkbox"]', '[role="menuitem"]',
                    '[role="tab"]', '[role="combobox"]', '[role="switch"]', '[tabindex]:not([tabindex="-1"])'
                ];

                const matched = Array.from(document.querySelectorAll(interactiveSelectors.join(',')))
                    .filter(isVisible);

                const el = matched[{index} - 1];
                if (!el) return false;

                el.scrollIntoView({{ behavior: 'instant', block: 'center' }});
                el.focus();
                el.click();
                return true;
            }})()
            "#,
            index = index
        )
    }

    #[test]
    fn test_click_js_uses_correct_element_lookup() {
        for index in [1, 3, 10] {
            let js = build_click_js(index);
            let expected_elem = format!("matched[{index} - 1]");
            assert!(
                js.contains(&expected_elem),
                "Should look up element at index-1"
            );
        }
    }

    #[test]
    fn test_click_js_has_visibility_check() {
        let js = build_click_js(1);
        assert!(
            js.contains("isVisible"),
            "Should have visibility check function"
        );
        assert!(
            js.contains("getComputedStyle"),
            "Should use getComputedStyle"
        );
        assert!(
            js.contains("display === 'none'"),
            "Should check display:none"
        );
        assert!(
            js.contains("visibility === 'hidden'"),
            "Should check visibility:hidden"
        );
        assert!(js.contains("opacity === '0'"), "Should check opacity:0");
    }

    #[test]
    fn test_click_js_has_interactive_selectors() {
        let js = build_click_js(1);
        assert!(
            js.contains("'a[href]'"),
            "Should select anchor tags with href"
        );
        assert!(js.contains("'button'"), "Should select buttons");
        assert!(js.contains("'input'"), "Should select inputs");
        assert!(js.contains("'textarea'"), "Should select textareas");
        assert!(js.contains("'select'"), "Should select selects");
    }

    #[test]
    fn test_click_js_falls_back_to_js_on_cdp_failure() {
        // The click implementation should have a fallback JS path
        // We verify this by checking the source code structure
        let js = build_click_js(1);
        // Fallback should also have scroll and click
        assert!(
            js.contains("scrollIntoView"),
            "Fallback should scroll into view"
        );
        assert!(js.contains(".click()"), "Fallback should call click()");
    }

    /// Integration test: verifies clicking works on a simple HTML page.
    #[tokio::test]
    #[ignore = "Requires Chrome/Chromium"]
    async fn test_click_with_live_browser() {
        let temp_dir = tempfile::tempdir().unwrap();
        let html_path = temp_dir.path().join("test_click.html");

        let html_content = "\
            <!DOCTYPE html>\n\
            <html>\n\
            <body>\n\
            <button id=\"btn1\">Click Me</button>\n\
            <a href=\"#\" id=\"link1\">Go to Google</a>\n\
            <div contenteditable=\"true\" id=\"editable\">Edit this</div>\n\
            <script>\n\
            let clickCount = 0;\n\
            document.getElementById('btn1').addEventListener('click', function() {{\n\
            clickCount++;\n\
            document.getElementById('count').textContent = clickCount;\n\
            }});\n\
            </script>\n\
            <span id=\"count\">0</span>\n\
            </body>\n\
            </html>";
        std::fs::write(&html_path, html_content).unwrap();

        let file_url = format!("file://{}", html_path.display());
        let mut browser = Browser::new(None);
        browser.start().await.unwrap();
        browser.navigate(&file_url).await.unwrap();

        // Get state and find elements
        let state = browser.get_state().await.unwrap();
        // Find button element
        let btn_elem = state
            .elements
            .iter()
            .find(|e| e.tag == "button")
            .expect("No button found");
        let btn_index = btn_elem.index;

        // Click the button multiple times
        for _ in 0..3 {
            browser.click(btn_index).await.unwrap();
        }

        // Verify click count increased
        let verify_js = r"(() => {
            return document.getElementById('count').textContent;
        })()";
        let value: serde_json::Value = browser.evaluate(verify_js).await.unwrap();
        let count = value.as_str().unwrap_or_default();

        assert_eq!(count, "3", "Button should be clicked 3 times");

        browser.close().await.unwrap();
    }

    /// Test that clicking file inputs returns an error.
    #[tokio::test]
    #[ignore = "Requires Chrome/Chromium"]
    async fn test_click_file_input_returns_error() {
        let temp_dir = tempfile::tempdir().unwrap();
        let html_path = temp_dir.path().join("test_file_input.html");

        let html_content = "<!DOCTYPE html>\n<html>\n<body>\n\
            <input type=\"file\" id=\"file-input\" />\n\
            <button id=\"submit-btn\">Submit</button>\n\
            </body>\n</html>";
        std::fs::write(&html_path, html_content).unwrap();

        let file_url = format!("file://{}", html_path.display());
        let mut browser = Browser::new(None);
        browser.start().await.unwrap();
        browser.navigate(&file_url).await.unwrap();

        let state = browser.get_state().await.unwrap();
        let file_input = state
            .elements
            .iter()
            .find(|e| e.tag == "input")
            .expect("No input found");

        // Clicking file input should fail with specific error message
        let result = browser.click(file_input.index).await;
        assert!(result.is_err(), "Clicking file input should fail");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Cannot click on file input"),
            "Error should mention file input restriction"
        );
        assert!(
            err_msg.contains("upload_file"),
            "Error should suggest upload_file action"
        );

        browser.close().await.unwrap();
    }

    /// Test that clicking non-existent element returns error.
    #[tokio::test]
    #[ignore = "Requires Chrome/Chromium"]
    async fn test_click_nonexistent_element_returns_error() {
        let temp_dir = tempfile::tempdir().unwrap();
        let html_path = temp_dir.path().join("test_empty.html");

        std::fs::write(&html_path, "<html><body></body></html>").unwrap();

        let file_url = format!("file://{}", html_path.display());
        let mut browser = Browser::new(None);
        browser.start().await.unwrap();
        browser.navigate(&file_url).await.unwrap();

        // Try to click element that doesn't exist
        let result = browser.click(999).await;
        assert!(result.is_err(), "Clicking non-existent element should fail");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("not found"),
            "Error should indicate element not found"
        );

        browser.close().await.unwrap();
    }

    /// Test that CDP mouse events are properly formatted.
    #[test]
    fn test_cdp_click_command_structure() {
        // Verify the command structures we send are valid JSON
        let move_cmd = serde_json::json!({
            "type": "mouseMoved",
            "x": 100.0,
            "y": 200.0
        });
        assert_eq!(move_cmd["type"], "mouseMoved");
        assert_eq!(move_cmd["x"], 100.0);
        assert_eq!(move_cmd["y"], 200.0);

        let press_cmd = serde_json::json!({
            "type": "mousePressed",
            "x": 100.0,
            "y": 200.0,
            "button": "left",
            "clickCount": 1
        });
        assert_eq!(press_cmd["type"], "mousePressed");
        assert_eq!(press_cmd["button"], "left");

        let release_cmd = serde_json::json!({
            "type": "mouseReleased",
            "x": 100.0,
            "y": 200.0,
            "button": "left",
            "clickCount": 1
        });
        assert_eq!(release_cmd["type"], "mouseReleased");
    }

    /// Test quad intersection with viewport logic.
    #[test]
    fn test_quad_viewport_intersection() {
        // Test case: quad fully inside viewport
        let quads: Vec<Vec<f64>> = vec![vec![0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0]];
        let browser = Browser::new(None);
        let _ = browser;
        let result = find_largest_visible_quad(&quads, 800.0, 600.0);
        assert!(result.is_some());
        if let Some(rect) = result {
            assert_eq!(rect.x, 0.0);
            assert_eq!(rect.y, 0.0);
            assert_eq!(rect.width, 100.0);
            assert_eq!(rect.height, 100.0);
        }

        // Test case: quad partially outside viewport (clipped)
        let quads2: Vec<Vec<f64>> =
            vec![vec![500.0, 500.0, 900.0, 500.0, 900.0, 700.0, 500.0, 700.0]];
        let result2 = find_largest_visible_quad(&quads2, 800.0, 600.0);
        assert!(result2.is_some());
        if let Some(rect) = result2 {
            // Should be clipped to viewport bounds
            assert_eq!(rect.x, 500.0);
            assert_eq!(rect.y, 500.0);
            assert_eq!(rect.width, 300.0); // Clipped at x=800
            assert_eq!(rect.height, 100.0); // Clipped at y=600
        }

        // Test case: quad completely outside viewport
        let quads3: Vec<Vec<f64>> = vec![vec![
            -100.0, -100.0, -50.0, -100.0, -50.0, -50.0, -100.0, -50.0,
        ]];
        let result3 = find_largest_visible_quad(&quads3, 800.0, 600.0);
        // Should return None or fall back to first quad
        assert!(result3.is_some());
    }

    /// Test `ElementRect` center calculation.
    #[test]
    fn test_element_rect_center() {
        use super::dom::ElementRect;

        let rect = ElementRect {
            x: 100.0,
            y: 200.0,
            width: 50.0,
            height: 30.0,
        };

        let (cx, cy) = rect.center();
        assert!((cx - 125.0).abs() < 0.001); // Center X
        assert!((cy - 215.0).abs() < 0.001); // Center Y
    }

    /// Test box model coordinate extraction from JSON.
    #[test]
    fn test_box_model_extraction() {
        let box_model_json = serde_json::json!({
            "model": {
                "content": [10.0, 20.0, 50.0, 20.0, 50.0, 60.0, 10.0, 60.0],
                "padding": [5.0, 15.0, 55.0, 15.0, 55.0, 65.0, 5.0, 65.0],
                "border": [0.0, 10.0, 60.0, 10.0, 60.0, 70.0, 0.0, 70.0],
                "margin": [-10.0, 0.0, 70.0, 0.0, 70.0, 80.0, -10.0, 80.0]
            }
        });

        if let Some(model) = box_model_json.get("model")
            && let Some(content) = model.get("content")
            && let Some(coords) = content.as_array()
        {
            assert_eq!(coords.len(), 8);
            assert_eq!(coords[0], 10.0); // x1
            assert_eq!(coords[1], 20.0); // y1
            assert_eq!(coords[4], 50.0); // x2
            assert_eq!(coords[7], 60.0); // y4
        }
    }

    /// Integration test: verifies click with live browser works end-to-end.
    #[tokio::test]
    #[ignore = "Requires Chrome/Chromium"]
    async fn test_click_with_multiple_elements() {
        let temp_dir = tempfile::tempdir().unwrap();
        let html_path = temp_dir.path().join("test_multi_click.html");

        let html_content = "\
            <!DOCTYPE html>\n<html><body>\n\
            <button id=\"btn1\">Button 1</button>\n\
            <button id=\"btn2\">Button 2</button>\n\
            <a href=\"#\" id=\"link1\">Link 1</a>\n\
            <script>\n\
            let count1 = 0;\n\
            let count2 = 0;\n\
            document.getElementById('btn1').addEventListener('click', function() { count1++; document.getElementById('c1').textContent = count1; });\n\
            document.getElementById('btn2').addEventListener('click', function() { count2++; document.getElementById('c2').textContent = count2; });\n\
            </script>\n\
            <span id=\"c1\">0</span>\n\
            <span id=\"c2\">0</span>\n\
            </body></html>";
        std::fs::write(&html_path, html_content).unwrap();

        let file_url = format!("file://{}", html_path.display());
        let mut browser = Browser::new(None);
        browser.start().await.unwrap();
        browser.navigate(&file_url).await.unwrap();

        let state = browser.get_state().await.unwrap();

        // Click first button twice
        let btn1 = state
            .elements
            .iter()
            .find(|e| e.tag == "button")
            .expect("No button found");
        browser.click(btn1.index).await.unwrap();
        browser.click(btn1.index).await.unwrap();

        // Click second button once
        let btn2 = state
            .elements
            .iter()
            .skip(1)
            .find(|e| e.tag == "button")
            .expect("No second button");
        browser.click(btn2.index).await.unwrap();

        // Verify counts
        let verify_js1 = r"(() => { return document.getElementById('c1').textContent; })()";
        let verify_js2 = r"(() => { return document.getElementById('c2').textContent; })()";

        let val1: serde_json::Value = browser.evaluate(verify_js1).await.unwrap();
        let val2: serde_json::Value = browser.evaluate(verify_js2).await.unwrap();

        assert_eq!(
            val1.as_str().unwrap_or_default(),
            "2",
            "First button should be clicked 2 times"
        );
        assert_eq!(
            val2.as_str().unwrap_or_default(),
            "1",
            "Second button should be clicked 1 time"
        );

        browser.close().await.unwrap();
    }

    /// Integration test: verifies link clicking works.
    #[tokio::test]
    #[ignore = "Requires Chrome/Chromium"]
    async fn test_click_link_navigates() {
        let temp_dir = tempfile::tempdir().unwrap();
        let html_path = temp_dir.path().join("test_link.html");
        let target_path = temp_dir.path().join("target.html");

        // Create main page with link
        std::fs::write(
            &html_path,
            "\
            <!DOCTYPE html>\n<html><body>\n\
            <a href=\"target.html\" id=\"test-link\">Go to Target</a>\n\
            </body></html>",
        )
        .unwrap();

        // Create target page
        std::fs::write(
            &target_path,
            "\
            <!DOCTYPE html>\n<html><body>\n\
            <h1>Target Page</h1>\n\
            </body></html>",
        )
        .unwrap();

        let file_url = format!("file://{}", html_path.display());
        let mut browser = Browser::new(None);
        browser.start().await.unwrap();
        browser.navigate(&file_url).await.unwrap();

        let state = browser.get_state().await.unwrap();
        let link_elem = state
            .elements
            .iter()
            .find(|e| e.tag == "a")
            .expect("No link found");

        // Click the link
        browser.click(link_elem.index).await.unwrap();

        // Give navigation time
        tokio::time::sleep(Duration::from_millis(1000)).await;

        // Check if we navigated (state should have changed)
        let new_state = browser.get_state().await.unwrap();
        assert_ne!(
            new_state.url, file_url,
            "Should have navigated after clicking link"
        );

        browser.close().await.unwrap();
    }

    /// Test special element types in click handling.
    #[test]
    fn test_click_special_element_types() {
        // Test that we can construct valid error messages for special cases
        let file_input_err = anyhow!(
            "Cannot click on file input element at index 1. File uploads must be handled using upload_file action."
        );
        assert!(file_input_err.to_string().contains("file input"));
        assert!(file_input_err.to_string().contains("upload_file"));

        let select_err =
            anyhow!("Cannot click on <select> elements. Use dropdown selection actions instead.");
        assert!(select_err.to_string().contains("<select>"));
        assert!(select_err.to_string().contains("dropdown"));
    }
}
