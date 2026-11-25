use anyhow::Result;
use std::time::Duration;

use super::step;

impl super::RunningWorkflow {
    pub(super) async fn wait(&self, step: &step::WaitStep) -> Result<()> {
        let mut logger = self.logger.lock().await;
        logger.info(&format!("Waiting for {} milliseconds", step.duration));

        tokio::time::sleep(Duration::from_millis(step.duration.into())).await;

        Ok(())
    }

    #[allow(clippy::unused_self)]
    pub(super) async fn abort_workflow(&self, step: &step::AbortStep) -> Result<()> {
        Err(anyhow::anyhow!(
            "Aborting workflow. {}",
            step.message.parse(self).await?
        ))
    }
}
