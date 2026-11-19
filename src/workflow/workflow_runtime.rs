mod agent_runner;
mod page_action_runner;

use anyhow::{Result, bail};
use scopeguard::defer;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use super::WorkflowState;
use super::workflow_data::step;
use crate::browser::Browser;
use crate::cli::CliLogger;
use crate::config::Config;

pub struct RunningWorkflow {
    pub(crate) data: Box<super::WorkflowData>,
    config: Box<Config>,
    //TODO: try simplify this with just Mutex
    browser: Arc<RwLock<Browser>>,
    logger: Mutex<CliLogger>,
}

impl RunningWorkflow {
    fn new(data: Box<super::WorkflowData>, config: Box<Config>) -> Self {
        let browser = Arc::new(RwLock::new(Browser::new(&config.browser)));
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

impl super::Workflow {
    /// # Errors
    /// This function returns an error if the workflow fails to run.
    pub async fn run(&mut self, config: Box<Config>) -> Result<()> {
        if self.is_draft() {
            bail!("Cannot run a draft workflow");
        }

        let data = match &self.state {
            WorkflowState::Valid(data) => data,
            WorkflowState::Running(_) => bail!("Workflow is already running"),
            WorkflowState::Error(error) => bail!("Workflow is not valid: {error}"),
        };

        //TODO: use it throughout the workflow run
        // let mut logger = CliLogger::default();
        // logger.info(&format!("Running workflow: {}", data.name));

        defer! {
            // logger.reset();
            CliLogger::default().reset();
        }

        self.state = WorkflowState::Running(RunningWorkflow::new(Box::new(data.clone()), config));

        if let WorkflowState::Running(running_workflow) = &mut self.state {
            match running_workflow.run_steps().await {
                Ok(()) => {
                    //TODO: set state to SuccessfulRun
                    self.state = WorkflowState::Valid(*running_workflow.data.clone());
                }
                Err(e) => {
                    running_workflow
                        .logger
                        .lock()
                        .await
                        .error(&format!("Workflow failed to run: {e:?}"));
                    //TODO: set state to FailedRun
                    self.state = WorkflowState::Valid(*running_workflow.data.clone());
                    // self.state = state_backup;
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
        assert!(matches!(workflow.state, WorkflowState::Error(_)));
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
