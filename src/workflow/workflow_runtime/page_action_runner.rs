use anyhow::{Context, Result};

use super::step;

impl super::RunningWorkflow {
    pub(super) async fn run_page_action_step(&self, step: &step::PageActionStep) -> Result<()> {
        let mut logger = self.logger.lock().await;
        log::info!("{step:#?}");

        //TODO: support multi-page workflows
        // step.page_index;

        match &step.action {
            step::PageAction::Navigate(navigate_action) => {
                logger.info(&format!("Navigating to: \"{}\"", navigate_action.url));

                let mut browser = self.browser.lock().await;

                browser
                    .navigate(&navigate_action.url)
                    .await
                    .context("Failed to navigate to the URL")?;
            }
        }

        Ok(())
    }
}
