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

use super::WorkflowState;
pub(crate) use super::context;
use super::workflow_data::step;
use crate::browser::Browser;
use crate::cli::CliLogger;
use crate::config::Config;
use crate::workflow::WorkflowData;

pub struct RunningWorkflow {
    pub data: super::WorkflowData,
    config: Box<Config>,
    pub(crate) browser: Mutex<Browser>,
    pub(crate) logger: Mutex<CliLogger>,
    pub(crate) context: Mutex<context::WorkflowContext>,
}

impl RunningWorkflow {
    fn new(data: super::WorkflowData, config: Box<Config>) -> Self {
        let browser = Mutex::new(Browser::new(&config.browser));
        let logger = CliLogger::default();

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

pub struct SuccessfullWorkflow {
    pub data: super::WorkflowData,
}

pub struct FailedWorkflow {
    pub data: super::WorkflowData,
    pub error: anyhow::Error,
}

impl super::Workflow {
    /// # Errors
    /// This function returns an error if the workflow fails to run.
    pub async fn run(&mut self, config: Box<Config>) -> Result<()> {
        defer! {
            CliLogger::default().reset();
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
    use crate::workflow::{JsonFile, Workflow, WorkflowState};

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
        let result = draft.run(&default_config()).await;
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
        let result = workflow.run(&default_config()).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Workflow is not valid")
        );
    }

    #[tokio::test]
    async fn test_run_steps_empty_returns_ok() {
        let mut browser = Browser::new(&crate::config::BrowserConfig::default());
        let result = run_steps(&Vec::new(), &mut browser).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_steps_with_navigate_uses_browser() {
        use crate::browser::MockCdp;

        let state_json = serde_json::json!({
            "url": "https://example.com",
            "title": "Example",
            "elements": [],
            "text": ""
        });
        let mock = MockCdp::new();
        mock.set(
            "Runtime.evaluate",
            serde_json::json!({"result": {"value": state_json}}),
        );
        let mut browser = Browser::new_with_client(mock.client);

        let steps = vec![step::WorkflowStepType::PageAction(step::PageActionStep {
            page_index: 0,
            action: step::PageAction::Navigate(step::page_action::NavigateAction {
                url: "https://example.com".to_string(),
            }),
        })];
        let result = run_steps(&steps, &mut browser).await;
        assert!(result.is_ok());
    }
}
