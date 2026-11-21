use anyhow::Result;

use super::step;

impl super::RunningWorkflow {
    #[allow(clippy::unused_self)]
    pub(super) async fn run_procedure(&self, step: &step::RunProcedureStep) -> Result<()> {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        //TODO: implement

        Err(anyhow::anyhow!(
            "Procedure not found: \"{}\"",
            step.procedure
        ))
    }
}
