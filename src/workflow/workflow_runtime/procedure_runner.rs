use anyhow::Result;
use std::path::Path;

use super::step;

impl super::RunningWorkflow {
    #[allow(clippy::unused_self)]
    pub(super) async fn run_procedure(
        &self,
        step: &step::RunProcedureStep,
        workflow_file_path: &Path,
    ) -> Result<()> {
        if let Some(procedure) = self
            .data
            .procedures
            .iter()
            .find(|p| p.name == step.procedure)
        {
            return Box::pin(self.run_steps(&procedure.steps, workflow_file_path)).await;
        }

        Err(anyhow::anyhow!(
            "Procedure not found: \"{}\"",
            step.procedure
        ))
    }
}
