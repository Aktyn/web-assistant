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
