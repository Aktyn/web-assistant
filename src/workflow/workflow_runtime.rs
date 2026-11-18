mod agent_runner;
mod page_action_runner;

use std::sync::Arc;

use anyhow::{Result, bail};
use tokio::sync::RwLock;

use super::WorkflowState;
use super::workflow_data::step;
use crate::browser::Browser;
use crate::config::Config;

pub struct RunningWorkflow {
    pub(crate) data: super::WorkflowData,
    browser: Arc<RwLock<Browser>>,
    config: Box<Config>,
}

impl RunningWorkflow {
    async fn run_steps(&mut self) -> Result<()> {
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

        log::info!("Running workflow: \"{}\"", data.name);

        let browser = Arc::new(RwLock::new(Browser::new(&config.browser)));

        self.state = WorkflowState::Running(RunningWorkflow {
            data: data.clone(),
            config,
            browser,
        });

        if let WorkflowState::Running(running_workflow) = &mut self.state {
            match running_workflow.run_steps().await {
                Ok(()) => {
                    //TODO: set state to SuccessfulRun
                    self.state = WorkflowState::Valid(running_workflow.data.clone());
                }
                Err(e) => {
                    //TODO: set state to FailedRun
                    log::error!("Workflow failed to run: {e:?}");
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
