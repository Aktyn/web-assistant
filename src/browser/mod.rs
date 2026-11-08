pub mod cdp;
pub mod chrome;
pub mod dom;
pub mod watchdogs;

use anyhow::{Context, Result, anyhow};
use cdp::{BrowserProcess, CdpClient};
pub use chrome::Chrome;
pub use dom::{BrowserState, EnhancedElement};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::sleep;
pub use watchdogs::BrowserWatchdogs;

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
    pub fn new() -> Self {
        Self {
            config: BrowserConfig::default(),
            process: None,
            client: None,
            watchdogs: None,
            cached_state: None,
        }
    }

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

        let click_js = format!(
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
        );

        let res = self.client()?.evaluate(&click_js).await?;
        if res.as_bool() != Some(true) {
            return Err(anyhow!("Element with index {index} not found to click"));
        }

        sleep(Duration::from_millis(500)).await;
        self.refresh_state().await?;
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
                el.focus();
                el.value = {escaped_text};
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return true;
            }})()
            "#,
            index = index,
            escaped_text = escaped_text
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

impl Default for Browser {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
pub struct BrowserBuilder {
    config: BrowserConfig,
}

impl BrowserBuilder {
    pub fn headless(mut self, headless: bool) -> Self {
        self.config.headless = headless;
        self
    }

    pub fn executable_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.executable_path = Some(path.into());
        self
    }

    pub fn user_data_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.user_data_dir = Some(path.into());
        self
    }

    pub fn downloads_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.downloads_dir = Some(path.into());
        self
    }

    pub fn cdp_url(mut self, url: impl Into<String>) -> Self {
        self.config.cdp_url = Some(url.into());
        self
    }

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

/// Compatibility alias matching python browser_use
pub type BrowserSession = Browser;
