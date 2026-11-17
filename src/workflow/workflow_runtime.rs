use anyhow::{Context, Result, bail};

use super::workflow_data::step;
use crate::browser::Browser;
use crate::config::Config;

impl super::Workflow {
    /// # Errors
    /// This function returns an error if the workflow fails to run.
    pub async fn run(&mut self, config: &Config) -> Result<()> {
        if self.is_draft() {
            bail!("Cannot run a draft workflow");
        }

        let data = match &self.state {
            super::WorkflowState::Valid(data) => data,
            super::WorkflowState::Error(error) => bail!("Workflow is not valid: {error}"),
        };

        log::info!("Running workflow: \"{}\"", data.name);

        let mut browser = Browser::new(&config.browser);
        browser.start().await?;

        run_steps(&data.steps, &mut browser).await?;
        Ok(())
    }
}

async fn run_steps(steps: &Vec<step::WorkflowStepType>, browser: &mut Browser) -> Result<()> {
    for step in steps {
        match &step {
            step::WorkflowStepType::PageAction(page_action_step) => {
                run_page_action_step(page_action_step, browser).await?;
            }
            step::WorkflowStepType::Agent(agent_step) => {
                run_agent_step(agent_step, browser)?;
            }
        }
    }

    Ok(())
}

async fn run_page_action_step(step: &step::PageActionStep, browser: &mut Browser) -> Result<()> {
    log::info!("Running page action step: \"{step:?}\"");

    //TODO: support multi-page workflows
    // step.page_index;

    match &step.action {
        step::PageAction::Navigate(navigate_action) => {
            log::info!("Navigating to: \"{}\"", navigate_action.url);

            browser
                .navigate(&navigate_action.url)
                .await
                .context("Failed to navigate to the URL")?;
        }
    }

    Ok(())
}

fn run_agent_step(step: &step::AgentStep, _browser: &mut Browser) -> Result<()> {
    log::info!("Running agent step: \"{step:?}\"");

    //TODO: implement

    if 5 > 10 {
        bail!("5 is not greater than 10");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{Workflow, WorkflowState};

    fn default_config() -> Config {
        Config::default()
    }

    /// Build a workflow whose state is Error (invalid JSON).
    fn workflow_with_error_state() -> Workflow {
        Workflow::new(crate::workflow::JsonFile::from_raw(
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

        let steps = vec![step::WorkflowStep {
            step_type: step::StepType::Navigate(step::NavigateStep {
                url: "https://example.com".to_string(),
            }),
        }];
        let result = run_steps(&steps, &mut browser).await;
        assert!(result.is_ok());
    }
}
