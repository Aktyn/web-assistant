mod agent_runner;
mod condition_runner;
mod context_runner;
mod iteration_runner;
mod misc_runner;
mod page_action_runner;
mod procedure_runner;
mod system_action_runner;

use anyhow::{Context as _, Result, bail};
use scopeguard::defer;
use std::mem;
use std::path::Path;
use tokio::sync::Mutex;

use crate::context;
use crate::workflow::{Workflow, WorkflowState};
use crate::workflow_data::{WorkflowData, step};
use browser::Browser;
use config::Config;

pub struct RunningWorkflow {
    pub data: WorkflowData,
    config: Box<Config>,
    pub(crate) browser: Mutex<Browser>,
    pub(crate) logger: Mutex<utils::logger::CliLogger>,
    pub(crate) context: Mutex<context::WorkflowContext>,
}

impl std::fmt::Debug for RunningWorkflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunningWorkflow")
            .field("data", &self.data)
            .finish_non_exhaustive()
    }
}

impl RunningWorkflow {
    fn new(data: WorkflowData, config: Box<Config>) -> Self {
        let browser = Mutex::new(Browser::new(&config.browser));
        let logger = utils::logger::CliLogger::default();

        Self {
            data,
            config,
            browser,
            logger: Mutex::new(logger),
            context: Mutex::new(super::context::WorkflowContext::new()),
        }
    }

    async fn run_steps(
        &self,
        steps: &Vec<step::WorkflowStepType>,
        workflow_file_path: &Path,
    ) -> Result<()> {
        for step in steps {
            match step {
                step::WorkflowStepType::PageAction(page_action_step) => {
                    self.browser.lock().await.start().await?;
                    self.run_page_action(page_action_step, workflow_file_path)
                        .await?;
                }
                step::WorkflowStepType::Agent(agent_step) => {
                    self.run_agent(agent_step, workflow_file_path).await?;
                }
                step::WorkflowStepType::SystemAction(system_action_step) => {
                    self.run_system_action(system_action_step).await?;
                }

                step::WorkflowStepType::Condition(condition_step) => {
                    self.check_condition(condition_step, workflow_file_path)
                        .await?;
                }
                step::WorkflowStepType::ForEach(for_each_step) => {
                    self.iterate_over_each(for_each_step, workflow_file_path)
                        .await?;
                }
                step::WorkflowStepType::RunProcedure(run_procedure_step) => {
                    self.run_procedure(run_procedure_step, workflow_file_path)
                        .await?;
                }

                step::WorkflowStepType::UpdateContext(update_context_step) => {
                    self.update_context(update_context_step).await?;
                }

                step::WorkflowStepType::Wait(wait_step) => {
                    self.wait(wait_step).await?;
                }
                step::WorkflowStepType::Abort(abort_step) => {
                    self.abort_workflow(abort_step).await?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct SuccessfullWorkflow {
    pub data: WorkflowData,
}

#[derive(Debug)]
pub struct FailedWorkflow {
    pub data: WorkflowData,
    pub error: anyhow::Error,
}

impl Workflow {
    /// # Errors
    /// This function returns an error if the workflow fails to run.
    pub async fn run(&mut self, config: Box<Config>) -> Result<()> {
        defer! {
            utils::logger::CliLogger::default().reset();
        }

        let get_running_state = |data: &mut WorkflowData| -> Result<WorkflowState> {
            let workflow_file_path = self
                .file_path
                .as_ref()
                .context("Cannot run a draft workflow")?;

            Ok(WorkflowState::Running(
                Box::new(RunningWorkflow::new(mem::take(data), config)),
                workflow_file_path.clone(),
            ))
        };

        self.state = match &mut self.state {
            WorkflowState::SuccessfulRun(run) => get_running_state(&mut run.data)?,
            WorkflowState::FailedRun(run) => get_running_state(&mut run.data)?,
            WorkflowState::Valid(data) => get_running_state(data)?,
            WorkflowState::Running(..) => bail!("Workflow is already running"),
            WorkflowState::Invalid(error) => bail!("Cannot run invalid Workflow: {error}"),
        };

        if let WorkflowState::Running(running_workflow, workflow_file_path) = &mut self.state {
            running_workflow.logger.lock().await.info(&format!(
                "Running workflow: \"{}\"",
                running_workflow.data.name
            ));

            match running_workflow
                .run_steps(&running_workflow.data.steps, workflow_file_path)
                .await
            {
                Ok(()) => {
                    self.state = WorkflowState::SuccessfulRun(SuccessfullWorkflow {
                        data: mem::take(&mut running_workflow.data),
                    });
                }
                Err(error) => {
                    self.state = WorkflowState::FailedRun(FailedWorkflow {
                        data: mem::take(&mut running_workflow.data),
                        error,
                    });
                }
            }
        } else {
            bail!("Workflow failed to start");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{Workflow, WorkflowState, workflow_loader};
    use workflow_loader::JsonFile;

    fn default_config() -> Config {
        Config::default()
    }

    /// Build a workflow whose state is Error (invalid JSON).
    fn workflow_with_error_state() -> Workflow {
        Workflow::new(JsonFile::from_raw(
            std::path::PathBuf::from("/tmp/invalid-for-run-test.json"),
            "{invalid json".to_string(),
        ))
    }

    #[tokio::test]
    async fn test_run_draft_workflow_bails() {
        let mut draft = Workflow::new_draft();
        let result = draft.run(Box::new(default_config())).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Cannot run a draft workflow")
        );
    }

    #[tokio::test]
    async fn test_run_invalid_state_bails() {
        let mut workflow = workflow_with_error_state();
        assert!(matches!(workflow.state, WorkflowState::Invalid(_)));
        let result = workflow.run(Box::new(default_config())).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Cannot run invalid Workflow")
        );
    }

    #[tokio::test]
    async fn test_run_steps_empty_returns_ok() {
        let runner = RunningWorkflow::new(
            WorkflowData {
                name: "Test".to_string(),
                description: None,
                procedures: vec![],
                steps: vec![],
            },
            Box::new(default_config()),
        );
        let result = runner
            .run_steps(&Vec::new(), Path::new("/tmp/test.json"))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_steps_with_navigate_attempts_browser_interaction() {
        // Test that workflow.run() with a navigate step attempts browser interaction
        let tmp = tempfile::tempdir().unwrap();
        let wf_path = tmp.path().join("navigate.json");
        std::fs::write(
            &wf_path,
            r#"{"name":"Test","steps":[{"type":"page-action","action":{"type":"navigate","url":"about:blank"}}]}"#,
        )
        .unwrap();

        let config = Box::new(crate::cli::cli_runner::test_headless_config());
        let mut workflow = workflow_loader::load_from_file(&wf_path).await.unwrap();
        let result = workflow.run(config).await;
        // The only acceptable failure is browser-related (no Chrome in CI)
        if let Err(e) = result {
            let err_str = e.to_string();
            assert!(
                err_str.contains("Chrome")
                    || err_str.contains("chromium")
                    || err_str.contains("DevTools")
                    || err_str.contains("browser"),
                "Unexpected error: {err_str}"
            );
        }
    }
}
