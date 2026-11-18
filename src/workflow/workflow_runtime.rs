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

impl std::fmt::Debug for RunningWorkflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunningWorkflow")
            .field("data", &self.data)
            .finish_non_exhaustive()
    }
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
    use crate::workflow::{JsonFile, Workflow, WorkflowState, load_from_file};

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
        let result = draft.run(Box::new(Config::default())).await;
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
        let result = workflow.run(Box::new(Config::default())).await;
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
        let tmp = tempfile::tempdir().unwrap();
        let wf_path = tmp.path().join("wf.json");
        std::fs::write(&wf_path, r#"{"name":"Empty","steps":[]}"#).unwrap();

        let config = Box::new(Config::default());
        let mut workflow = load_from_file(&wf_path).await.unwrap();
        let result = workflow.run(config).await;
        // Should succeed with empty steps (only browser might fail in CI)
        if let Err(e) = result {
            let err_str = e.to_string();
            assert!(
                err_str.contains("Chrome")
                    || err_str.contains("chromium")
                    || err_str.contains("DevTools"),
                "Unexpected error: {err_str}"
            );
        }
    }

    #[tokio::test]
    async fn test_run_steps_with_navigate_uses_browser() {
        // This test verifies workflow.run() with a valid step.
        let tmp = tempfile::tempdir().unwrap();
        let wf_path = tmp.path().join("wf.json");
        std::fs::write(
            &wf_path,
            r#"{"name":"Test","steps":[{"type":"page-action","action":{"type":"navigate","url":"about:blank"}}]}"#,
        )
        .unwrap();

        let config = Box::new(Config::default());
        let mut workflow = load_from_file(&wf_path).await.unwrap();
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
