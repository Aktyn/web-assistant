use anyhow::Result;
use std::path::Path;

use super::step;

impl super::RunningWorkflow {
    pub(super) async fn check_condition(
        &self,
        step: &step::ConditionStep,
        workflow_file_path: &Path,
    ) -> Result<()> {
        log::info!("{step:#?}");
        // let mut logger = self.logger.lock().await;
        // logger.info(&format!("Waiting for {} milliseconds", step.duration));

        let is_met: bool = match &step.condition {
            step::Condition::MatchRegex(_match_regex_condition) => {
                //TODO:
                // match_regex_condition.pattern;
                // match_regex_condition.value;
                true
            }
            step::Condition::Truthy(_truthy_condition) => {
                //TODO:
                // truthy_condition.value;
                true
            }
            step::Condition::Equal(_equal_condition) => true,
        };

        //TODO: implement
        if is_met {
            Box::pin(self.run_steps(&step.then, workflow_file_path)).await?;
        } else {
            Box::pin(self.run_steps(&step.else_, workflow_file_path)).await?;
        }

        Ok(())
    }
}
