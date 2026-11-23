use anyhow::{Context, Result};
use std::path::Path;

use super::context;
use super::step;

impl super::RunningWorkflow {
    pub(super) async fn run_page_action(
        &self,
        step: &step::PageActionStep,
        workflow_file_path: &Path,
    ) -> Result<()> {
        let mut logger = self.logger.lock().await;

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

                self.browser
                    .lock()
                    .await
                    .input_selector(&selector, &value)
                    .await?;
            }

            step::PageAction::Click(click_action) => {
                let selector = click_action.selector.parse(self).await?;
                let mut browser = self.browser.lock().await;

                logger.info(&format!("Clicking: \"{selector}\""));

                browser
                    .click_selector(&selector, click_action.wait_for_navigation)
                    .await?;
                if click_action.wait_for_navigation {
                    browser.wait_for_navigation().await?;
                }
            }

            step::PageAction::Evaluate(evaluate_action) => {
                logger.info("Evaluating code");

                let code = step::page_action::get_script_content(
                    &evaluate_action.script,
                    workflow_file_path.parent().unwrap_or_else(|| Path::new("")),
                )
                .await?;

                let mut browser = self.browser.lock().await;

                match browser.run_script(&code).await {
                    Ok(evaluated_response) => {
                        logger.info(&format!("Evaluated response: \"{evaluated_response:#?}\""));
                        if let Some(context_name) = &evaluate_action.context_name {
                            let basic_context_item = Box::new(context::BasicContextItem::new(
                                evaluated_response.unwrap_or(serde_json::Value::Null),
                            ));
                            self.context
                                .lock()
                                .await
                                .set_item(context_name, basic_context_item);
                        }
                    }
                    Err(e) => {
                        logger.warning(&format!("Failed to evaluate code: \"{e}\""));
                    }
                }
            }
        }

        Ok(())
    }
}
