use anyhow::{Context, Result};

use super::step;

impl super::RunningWorkflow {
    pub(super) async fn run_page_action_step(&self, step: &step::PageActionStep) -> Result<()> {
        log::info!("Running page action step: \"{step:?}\"");

        //TODO: support multi-page workflows
        // step.page_index;

        match &step.action {
            step::PageAction::Navigate(navigate_action) => {
                log::info!("Navigating to: \"{}\"", navigate_action.url);

                let mut browser = self.browser.write().await;

                browser
                    .navigate(&navigate_action.url)
                    .await
                    .context("Failed to navigate to the URL")?;
            }
        }

        Ok(())
    }
}
