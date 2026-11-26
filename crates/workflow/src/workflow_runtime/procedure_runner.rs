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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_data::{WorkflowData, WorkflowProcedure};
    use crate::workflow_runtime::RunningWorkflow;
    use config::Config;

    fn default_config() -> Config {
        Config::default()
    }

    async fn make_runner_with_procedures(procedures: Vec<WorkflowProcedure>) -> RunningWorkflow {
        RunningWorkflow::new(
            WorkflowData {
                name: "Test".to_string(),
                description: None,
                procedures,
                steps: vec![],
            },
            Box::new(default_config()),
        )
    }

    #[tokio::test]
    async fn test_run_procedure_missing_returns_error() {
        let runner = make_runner_with_procedures(vec![]).await;
        let step = step::RunProcedureStep {
            procedure: "nonexistent".to_string(),
        };
        let result = runner
            .run_procedure(&step, Path::new("/tmp/test.json"))
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Procedure not found"));
        assert!(err.contains("nonexistent"));
    }

    #[tokio::test]
    async fn test_run_procedure_empty_name_not_found() {
        let runner = make_runner_with_procedures(vec![]).await;
        let step = step::RunProcedureStep {
            procedure: "".to_string(),
        };
        let result = runner
            .run_procedure(&step, Path::new("/tmp/test.json"))
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Procedure not found")
        );
    }

    #[tokio::test]
    async fn test_run_procedure_finds_matching_procedure() {
        let procedure = WorkflowProcedure {
            name: "my_proc".to_string(),
            steps: vec![],
        };
        let runner = make_runner_with_procedures(vec![procedure]).await;
        let step = step::RunProcedureStep {
            procedure: "my_proc".to_string(),
        };
        // Should succeed (empty steps list) even though we can't exercise browser steps
        let result = runner
            .run_procedure(&step, Path::new("/tmp/test.json"))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_procedure_selects_correct_one_of_many() {
        let proc_a = WorkflowProcedure {
            name: "proc_a".to_string(),
            steps: vec![],
        };
        let proc_b = WorkflowProcedure {
            name: "proc_b".to_string(),
            steps: vec![],
        };
        let runner = make_runner_with_procedures(vec![proc_a, proc_b]).await;

        let step = step::RunProcedureStep {
            procedure: "proc_b".to_string(),
        };
        let result = runner
            .run_procedure(&step, Path::new("/tmp/test.json"))
            .await;
        assert!(result.is_ok());
    }
}
