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

async fn run_steps(steps: &Vec<step::WorkflowStep>, browser: &mut Browser) -> Result<()> {
    for step in steps {
        match &step.step_type {
            step::StepType::Navigate(navigate_step) => {
                log::info!("Navigating to: \"{}\"", navigate_step.url);

                browser
                    .navigate(&navigate_step.url)
                    .await
                    .context("Failed to navigate to the URL")?;
            }
        }
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
}
