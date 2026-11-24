use anyhow::Result;
use regex::Regex;
use std::path::Path;

use super::step;

impl super::RunningWorkflow {
    pub(super) async fn check_condition(
        &self,
        step: &step::ConditionStep,
        workflow_file_path: &Path,
    ) -> Result<()> {
        let is_met: bool = match &step.condition {
            step::Condition::MatchRegex(match_regex_condition) => {
                let regex = Regex::new(&match_regex_condition.pattern).unwrap();
                let value = match_regex_condition.value.parse(self).await?;
                let result = regex.is_match(&value);
                log::info!(
                    "Matching \"{}\" against {:?} --> {}",
                    value,
                    match_regex_condition.pattern,
                    result
                );
                result
            }
            step::Condition::Truthy(truthy_condition) => {
                !truthy_condition.value.parse(self).await?.is_empty()
            }
            step::Condition::Equal(equal_condition) => {
                let left_value = equal_condition.left.parse(self).await?;
                let right_value = equal_condition.right.parse(self).await?;
                log::info!("Comparing {left_value} == {right_value}");
                left_value == right_value
            }
            step::Condition::HasElement(has_element_condition) => {
                let selector = has_element_condition.selector.parse(self).await?;

                let browser = self.browser.lock().await;
                let locator = browser.get_selector(&selector)?;
                locator.is_visible().await?
            }
        };

        if is_met {
            Box::pin(self.run_steps(&step.then, workflow_file_path)).await?;
        } else {
            Box::pin(self.run_steps(&step.else_, workflow_file_path)).await?;
        }

        Ok(())
    }
}
