use crate::dom::{BrowserState, ElementRect, EnhancedElement};
use crate::js;
use crate::utils::browser_info;
use crate::watchdogs::BrowserWatchdogs;
use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use playwright_rs::{
    BrowserContext, BrowserContextOptions, ClickOptions, LaunchOptions, Locator, Page, Playwright,
    WaitUntil,
};
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

use config::BrowserConfig;
use utils as project_utils;

/// Internal CDP client used when connecting to a remote browser via WebSocket URL.
/// When using Playwright's native launch mode, this field remains `None`.
#[derive(Clone)]
pub struct CdpClient {
    page: Option<Page>,
}

impl CdpClient {
    pub async fn connect(ws_url: &str) -> Result<Self> {
        let pw = Playwright::launch().await?;
        let browser = pw.chromium().connect_over_cdp(ws_url, None).await?;
        let context = browser.new_context().await?;
        let page = context.new_page().await?;
        Ok(Self { page: Some(page) })
    }

    #[allow(clippy::unused_async_trait_impl)]
    #[allow(clippy::unused_async)]
    pub async fn send_command(&self, _method: &str, _params: Value) -> Result<Value> {
        Err(anyhow!(
            "CDP command not available without a live connection"
        ))
    }

    pub async fn evaluate(&self, expression: &str) -> Result<Value> {
        if let Some(ref page) = self.page {
            let val_str = page.evaluate_value(expression).await?;
            Ok(project_utils::json::json_parse(&val_str))
        } else {
            Err(anyhow!("No page available for evaluation"))
        }
    }
}

pub struct Browser {
    config: BrowserConfig,
    client: Option<CdpClient>,
    watchdogs: Option<BrowserWatchdogs>,
    cached_state: Option<BrowserState>,
    playwright_page: Option<Page>,
    context: Option<BrowserContext>,
    playwright: Option<Playwright>,
}

impl Browser {
    #[must_use]
    pub fn new(config: &BrowserConfig) -> Self {
        Self {
            config: config.clone(),
            client: None,
            watchdogs: None,
            cached_state: None,
            playwright_page: None,
            context: None,
            playwright: None,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        if self.client.is_some() || self.playwright_page.is_some() {
            return Ok(());
        }

        let browser_info = browser_info::get_browser_info(&self.config)?;

        // If a CDP URL is provided, connect to it (for external browser control)
        if let Some(ref ws_url) = self.config.cdp_url {
            let client = CdpClient::connect(ws_url).await?;
            self.client = Some(client);
            self.watchdogs = None;
            return Ok(());
        }

        // Launch locally with Playwright
        let playwright = Playwright::launch().await?;
        let browser_type = match browser_info.browser_type {
            browser_info::BrowserType::Chrome => playwright.chromium(),
            browser_info::BrowserType::Firefox => playwright.firefox(),
        };

        let headless = self.config.headless;
        let mut context_options = BrowserContextOptions::builder()
            .args(browser_info.default_args.clone())
            .headless(headless)
            .no_viewport(!headless)
            .executable_path(browser_info.executable_path.display().to_string());
        if let Some(downloads_dir) = &self.config.downloads_dir {
            context_options = context_options.downloads_path(downloads_dir.display().to_string());
        }
        log::info!(
            "Starting browser {:#?}",
            (
                &browser_info.executable_path,
                &browser_info.user_data_dir,
                &headless
            )
        );

        if let Some(data_dir) = browser_info.user_data_dir {
            let context = browser_type
                .launch_persistent_context_with_options(
                    data_dir.to_str().unwrap_or_default(),
                    context_options.build(),
                )
                .await
                .with_context(|| "Failed to launch persistent context")?;

            let page = context.new_page().await?;
            self.context = Some(context);
            self.playwright_page = Some(page);
        } else {
            let mut options = LaunchOptions::new()
                .args(browser_info.default_args.clone())
                .headless(headless)
                .executable_path(browser_info.executable_path.display().to_string());

            if let Some(downloads_dir) = &self.config.downloads_dir {
                options = options.downloads_path(downloads_dir.display().to_string());
            }

            let browser = browser_type
                .launch_with_options(options)
                .await
                .context("Failed to launch browser with options")?;

            let context = browser
                .new_context_with_options(context_options.build())
                .await?;
            let page = context.new_page().await?;

            self.context = Some(context);
            self.playwright_page = Some(page);
        }
        self.playwright = Some(playwright);

        Ok(())
    }

    pub fn watchdogs(&self) -> Option<&BrowserWatchdogs> {
        self.watchdogs.as_ref()
    }

    pub fn watchdogs_mut(&mut self) -> Option<&mut BrowserWatchdogs> {
        self.watchdogs.as_mut()
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

        if let Some(wd) = &self.watchdogs {
            wd.on_before_navigate(&formatted_url)?;
        }

        let page = self
            .playwright_page
            .as_ref()
            .ok_or_else(|| anyhow!("No page available for navigation"))?;
        page.goto(
            &formatted_url,
            None,
            // GotoOptions::new()
            //     .timeout(Duration::from_secs(20))
            //     .wait_until(WaitUntil::NetworkIdle),
        )
        .await
        .with_context(|| format!("Failed to goto {formatted_url}"))?;
        self.wait(1).await?;

        if let Some(wd) = &self.watchdogs {
            wd.on_after_navigate(&formatted_url).await?;
        }

        self.refresh_state().await?;
        Ok(())
    }
    pub async fn wait_for_navigation(&mut self) -> Result<()> {
        let page = self
            .playwright_page
            .as_ref()
            .ok_or_else(|| anyhow!("No page available"))?;
        page.wait_for_load_state(Some(WaitUntil::NetworkIdle))
            .await?;
        Ok(())
    }

    pub async fn get_state(&mut self) -> Result<BrowserState> {
        self.start().await?;
        self.refresh_state().await
    }

    pub async fn refresh_state(&mut self) -> Result<BrowserState> {
        let page = self
            .playwright_page
            .as_ref()
            .ok_or_else(|| anyhow!("Cannot refresh state: no page available"))?;
        let state: BrowserState = page
            .evaluate(js::EXTRACT_DOM_JS.as_str(), None::<&()>)
            .await
            .context("Failed to extract DOM from page")?;
        self.cached_state = Some(state.clone());
        Ok(state)
    }

    pub fn get_element(&self, index: u32) -> Option<EnhancedElement> {
        self.cached_state
            .as_ref()
            .and_then(|s| s.elements.iter().find(|el| el.index == index).cloned())
    }
    pub fn get_selector(&self, selector: &str) -> Result<Locator> {
        let page = self
            .playwright_page
            .as_ref()
            .ok_or_else(|| anyhow!("No page available"))?;

        Ok(page.locator(selector))
    }

    pub async fn click(&mut self, index: u32) -> Result<()> {
        self.start().await?;
        let (_tag, _input_type, rect) = self.get_element_info(index).await?;
        let (center_x, center_y) = rect.center();
        let center_x = center_x.clamp(0.0, 10000.0);
        let center_y = center_y.clamp(0.0, 10000.0);
        self.playwright_click(center_x, center_y).await?;
        sleep(Duration::from_millis(500)).await;
        self.refresh_state().await?;
        Ok(())
    }
    pub async fn click_selector(
        &mut self,
        selector: &str,
        wait_for_navigation: bool,
    ) -> Result<()> {
        let element = self.get_selector(selector)?;
        element
            .click(
                ClickOptions::builder()
                    .no_wait_after(!wait_for_navigation)
                    .build(),
            )
            .await?;
        self.wait(1).await?;

        Ok(())
    }

    async fn get_element_info(&self, index: u32) -> Result<(String, String, ElementRect)> {
        #[derive(serde::Deserialize)]
        struct ElementInfo {
            tag: String,
            #[serde(rename = "type")]
            input_type: String,
            x: f64,
            y: f64,
            width: f64,
            height: f64,
        }

        let element_info_js = js::build_element_info_js(index);

        let page = self
            .playwright_page
            .as_ref()
            .ok_or_else(|| anyhow!("No page available"))?;
        let info: Option<ElementInfo> = page.evaluate(&element_info_js, None::<&()>).await?;
        let info = info.ok_or_else(|| anyhow!("Element with index {index} not found"))?;

        let tag = &info.tag;
        let input_type = &info.input_type;

        // Special case: file inputs should not be clicked directly
        if *tag == "input" && input_type == "file" {
            return Err(anyhow!(
                "Cannot click on file input element at index {index}. File uploads must be handled using upload_file action."
            ));
        }

        // Special case: select elements should not be clicked
        if *tag == "select" {
            return Err(anyhow!(
                "Cannot click on <select> elements. Use dropdown selection actions instead."
            ));
        }

        Ok((
            info.tag.clone(),
            info.input_type.clone(),
            ElementRect {
                x: info.x,
                y: info.y,
                width: info.width,
                height: info.height,
            },
        ))
    }

    async fn playwright_click(&self, x: f64, y: f64) -> Result<()> {
        let page = self
            .playwright_page
            .as_ref()
            .ok_or_else(|| anyhow!("Page not set"))?;
        page.mouse().click(x, y, None).await?;
        Ok(())
    }

    pub async fn input(&mut self, index: u32, text: &str) -> Result<()> {
        self.start().await?;
        let escaped_text = serde_json::to_string(text)?;
        let input_js = js::build_input_js(index, &escaped_text);

        let page = self
            .playwright_page
            .as_ref()
            .ok_or_else(|| anyhow!("No page available for input"))?;
        let res_str = page.evaluate_value(&input_js).await?;
        if !res_str.parse::<bool>().unwrap_or(false) {
            return Err(anyhow!(
                "Element with index {index} not found to input text"
            ));
        }

        sleep(Duration::from_millis(300)).await;
        self.refresh_state().await?;
        Ok(())
    }
    pub async fn input_selector(&self, selector: &str, text: &str) -> Result<()> {
        let element = self.get_selector(selector)?;
        element.fill(text, None).await?;

        Ok(())
    }

    pub async fn scroll(&mut self, amount: i32) -> Result<()> {
        self.start().await?;
        let script = format!("window.scrollBy(0, {amount});");
        let page = self
            .playwright_page
            .as_ref()
            .ok_or_else(|| anyhow!("No page available for scrolling"))?;
        page.evaluate_value(&script).await?;
        sleep(Duration::from_millis(300)).await;
        self.refresh_state().await?;
        Ok(())
    }

    pub async fn wait(&mut self, seconds: u64) -> Result<()> {
        sleep(Duration::from_secs(seconds)).await;
        if self.playwright_page.is_some() {
            self.refresh_state().await?;
        }
        Ok(())
    }

    /// Executes javascript code in the page and returns the script's output as serde value
    pub async fn run_script(&mut self, script: &str) -> Result<Option<serde_json::Value>> {
        self.start().await?;
        let page = self
            .playwright_page
            .as_ref()
            .ok_or_else(|| anyhow!("No page available for evaluation"))?;
        let js_expression = format!(r"() => {{ {script} }}");
        let response: Option<serde_json::Value> =
            page.evaluate(&js_expression, None::<&()>).await?;
        Ok(response)
    }

    pub async fn screenshot(&mut self) -> Result<String> {
        self.start().await?;
        let page = self
            .playwright_page
            .as_ref()
            .ok_or_else(|| anyhow!("No page available for screenshot"))?;
        let bytes = page.screenshot(None).await?;
        Ok(general_purpose::STANDARD.encode(&bytes))
    }

    pub fn get_url(&self) -> Option<String> {
        self.playwright_page.as_ref().map(playwright_rs::Page::url)
    }

    pub async fn close(&mut self) -> Result<()> {
        if let Some(page) = self.playwright_page.take() {
            let _ = page.close().await;
        }
        if let Some(ctx) = self.context.take() {
            let _ = ctx.close().await;
        }
        self.playwright.take();
        self.client = None;
        self.watchdogs = None;
        self.cached_state = None;
        Ok(())
    }
}

impl Default for Browser {
    fn default() -> Self {
        Self::new(&BrowserConfig::default())
    }
}

#[cfg(test)]
impl CdpClient {
    /// Constructs a disconnected client for unit tests that only exercise the
    /// watchdog decision logic (no live CDP connection is required).
    #[must_use]
    pub(crate) fn new_test_client() -> Self {
        Self { page: None }
    }
}
