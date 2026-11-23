use anyhow::{Result, anyhow};
use std::path::Path;

use super::step;

impl super::RunningWorkflow {
    pub(super) async fn iterate_over_each(
        &self,
        for_each_step: &step::ForEachStep,
        workflow_file_path: &Path,
    ) -> Result<()> {
        let items = step::get_iteration_items(&for_each_step.items, workflow_file_path).await?;

        if items.is_empty() {
            return Err(anyhow!("No items found to iterate over"));
        }

        let total_items = items.len();

        //TODO: option for reverse and random order iterations
        for (index, item) in items.into_iter().enumerate() {
            self.logger.lock().await.info(&format!(
                "Starting iteration {} of {}",
                index.saturating_add(1),
                total_items,
            ));

            // Closure for temporary context access
            {
                let mut context = self.context.lock().await;
                if !for_each_step.preserve_context && index > 0 {
                    context.reset();
                }
                context.set_item(&for_each_step.context_name, item);
            }

            let run_result =
                Box::pin(self.run_steps(&for_each_step.steps, workflow_file_path)).await;

            if let Err(error) = run_result {
                if let Some(steps_on_error) = &for_each_step.on_error {
                    Box::pin(self.run_steps(steps_on_error, workflow_file_path)).await?;
                } else {
                    self.logger.lock().await.error(&format!(
                        "Error running iteration {}: {}",
                        index.saturating_add(1),
                        error
                    ));
                }
            }
        }

        Ok(())
    }
}
