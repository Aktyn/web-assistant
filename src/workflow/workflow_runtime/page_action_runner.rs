use anyhow::{Context, Result};

use super::step;

impl super::RunningWorkflow {
    pub(super) async fn run_page_action(&self, step: &step::PageActionStep) -> Result<()> {
        let mut logger = self.logger.lock().await;
        log::info!("{step:#?}");

        //TODO: support multi-page workflows
        // step.page_index;

        match &step.action {
            step::PageAction::Navigate(navigate_action) => {
                let url = navigate_action.url.parse(self).await?;

                logger.info(&format!("Navigating to: \"{url}\""));

                self.browser
                    .lock()
                    .await
                    .navigate(&url)
                    .await
                    .context("Failed to navigate to the URL")?;
            }

            step::PageAction::Input(input_action) => {
                let value = input_action.value.parse(self).await?;
                let selector = input_action.selector.parse(self).await?;

                logger.info(&format!("Inputting: \"{value}\" into \"{selector}\""));

                //TODO: implement
            }

            step::PageAction::Click(click_action) => {
                let selector = click_action.selector.parse(self).await?;

                logger.info(&format!("Clicking: \"{selector}\""));

                //TODO: implement
            }

            step::PageAction::Evaluate(_evaluate_action) => {
                logger.info("Evaluating code");

                //TODO: implement
            }
        }

        Ok(())
    }
}
