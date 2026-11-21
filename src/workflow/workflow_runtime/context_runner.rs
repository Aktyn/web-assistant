use anyhow::Result;
use std::time::Duration;

use super::step;

impl super::RunningWorkflow {
    pub(super) async fn update_context(&self, _step: &step::UpdateContextStep) -> Result<()> {
        //TODO: implement

        tokio::time::sleep(Duration::from_secs(1)).await;

        Ok(())
    }
}
