mod agent_runner;
mod page_action_runner;

use anyhow::{Result, bail};
use scopeguard::defer;
use std::mem;
use tokio::sync::Mutex;

use super::WorkflowState;
use super::workflow_data::step;
use crate::browser::Browser;
use crate::cli::CliLogger;
use crate::config::Config;
use crate::workflow::WorkflowData;

pub struct RunningWorkflow {
    pub data: super::WorkflowData,
    config: Box<Config>,
    browser: Mutex<Browser>,
    logger: Mutex<CliLogger>,
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
        }
    }

    async fn run_steps(&self) -> Result<()> {
        for step in &self.data.steps {
            match step {
                step::WorkflowStepType::PageAction(page_action_step) => {
                    self.run_page_action_step(page_action_step).await?;
                }
                step::WorkflowStepType::Agent(agent_step) => {
                    self.run_agent_step(agent_step).await?;
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
        if self.is_draft() {
            bail!("Cannot run a draft workflow");
        }

        defer! {
            CliLogger::default().reset();
        }

        let get_running_state = |data: &mut WorkflowData| {
            WorkflowState::Running(Box::new(RunningWorkflow::new(mem::take(data), config)))
        };

        self.state = match &mut self.state {
            WorkflowState::SuccessfulRun(run) => get_running_state(&mut run.data),
            WorkflowState::FailedRun(run) => get_running_state(&mut run.data),
            WorkflowState::Valid(data) => get_running_state(data),
            WorkflowState::Running(_) => bail!("Workflow is already running"),
            WorkflowState::Invalid(error) => bail!("Cannot run invalid Workflow: {error}"),
        };

        if let WorkflowState::Running(running_workflow) = &mut self.state {
            running_workflow.logger.lock().await.info(&format!(
                "Running workflow: \"{}\"",
                running_workflow.data.name
            ));

            match running_workflow.run_steps().await {
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
        use crate::browser::cdp::MockCdp;

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
