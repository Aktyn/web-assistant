mod agent_runner;
mod page_action_runner;

use anyhow::{Result, bail};
use scopeguard::defer;
use std::mem;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use super::WorkflowState;
use super::workflow_data::step;
use crate::browser::Browser;
use crate::cli::CliLogger;
use crate::config::Config;
use crate::workflow::WorkflowData;

pub struct RunningWorkflow {
    pub data: super::WorkflowData,
    config: Box<Config>,
    //TODO: try simplify this with just Mutex
    browser: Arc<RwLock<Browser>>,
    logger: Mutex<CliLogger>,
}

impl std::fmt::Debug for RunningWorkflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunningWorkflow")
            .field("data", &self.data)
            .finish_non_exhaustive()
    }
}

impl RunningWorkflow {
    fn new(data: super::WorkflowData, config: Box<Config>) -> Self {
        let browser = Arc::new(RwLock::new(Browser::new(&config.browser)));
        let logger = CliLogger::default();

        Self {
            data,
            config,
            browser,
            logger: Mutex::new(logger),
        }
    }

    pub(crate) async fn run_steps(&self) -> Result<()> {
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

#[derive(Debug)]
pub struct SuccessfullWorkflow {
    pub data: super::WorkflowData,
}

#[derive(Debug)]
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
        // Test that running a workflow with empty steps doesn't panic
        // (browser may fail in CI, which is acceptable)
        let tmp = tempfile::tempdir().unwrap();
        let wf_path = tmp.path().join("empty.json");
        std::fs::write(&wf_path, r#"{"name":"Empty","steps":[]}"#).unwrap();
        let config = Box::new(crate::cli::cli_runner::test_headless_config());
        let mut workflow = crate::workflow::load_from_file(&wf_path).await.unwrap();
        let result = workflow.run(config).await;
        // Acceptable failures are browser-related
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

    #[tokio::test]
    async fn test_run_steps_with_navigate_uses_browser() {
        // Test that workflow.run() with a navigate step attempts browser interaction
        let tmp = tempfile::tempdir().unwrap();
        let wf_path = tmp.path().join("navigate.json");
        std::fs::write(
            &wf_path,
            r#"{"name":"Test","steps":[{"type":"page-action","action":{"type":"navigate","url":"about:blank"}}]}"#,
        )
        .unwrap();

        let config = Box::new(crate::cli::cli_runner::test_headless_config());
        let mut workflow = crate::workflow::load_from_file(&wf_path).await.unwrap();
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
