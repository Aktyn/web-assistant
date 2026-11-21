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
                logger.info(&format!("Navigating to: \"{}\"", navigate_action.url));

                self.browser
                    .lock()
                    .await
                    .navigate(&navigate_action.url)
                    .await
                    .context("Failed to navigate to the URL")?;
            }

            step::PageAction::Input(input_action) => {
                logger.info(&format!(
                    "Inputting: \"{}\" into \"{}\"",
                    input_action.value, input_action.selector
                ));

                //TODO: implement
            }

            step::PageAction::Click(click_action) => {
                logger.info(&format!("Clicking: \"{}\"", click_action.selector));

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
