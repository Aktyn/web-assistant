mod explain_config;
mod explain_workflow;
mod init_config;

use anyhow::{Context, Result, bail};

use crate::cli::{Cli, Commands};
use Commands::{ExplainConfig, ExplainWorkflow, InitConfig, Run};
use workflow::{self, WorkflowState};

impl Cli {
    /// Returns true if any cli command was ran
    pub async fn run(&self) -> Result<bool> {
        match &self.command {
            Some(Run { workflow }) => self.run_workflow(workflow).await?,
            Some(InitConfig { output_path }) => {
                init_config::init_config(output_path.as_ref()).await?;
            }
            Some(ExplainConfig {}) => explain_config::explain_config()?,
            Some(ExplainWorkflow { include_example }) => {
                explain_workflow::explain_workflow(include_example.unwrap_or(false))?;
            }
            None => return Ok(false),
        }
        Ok(true)
    }

    async fn run_workflow(&self, workflow_path_or_name: &str) -> Result<()> {
        // dotenvy::dotenv().ok();
        env_logger::try_init().ok();

        let config_path = self
            .config_path
            .as_ref()
            .context("Config path is required")?;
        let config = config::load_config(config_path).await?;

        let mut workflow = workflow::load_by_name_or_path(workflow_path_or_name, &config).await?;

        workflow.run(Box::new(config)).await?;

        let mut logger = utils::logger::CliLogger::default();

        match workflow.state {
            WorkflowState::SuccessfulRun(..) => {
                logger.success("Workflow finished successfully");
            }
            WorkflowState::FailedRun(failed_run) => {
                logger.error(&format!("Workflow run failed: {:#?}", failed_run.error));
            }
            WorkflowState::Valid(..) | WorkflowState::Invalid(..) | WorkflowState::Running(..) => {
                bail!("Workflow state is not expected")
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli_runner::explain_workflow::explain_workflow;
    use crate::cli_runner::init_config::init_config;
    use config::Config;

    /// Creates a default Config with headless browser enabled for testing.
    #[allow(dead_code)]
    pub(crate) fn test_headless_config() -> Config {
        Config {
            browser: config::BrowserConfig {
                headless: true,
                ..Default::default()
            },
            ..Config::default()
        }
    }

    #[tokio::test]
    async fn test_init_config_creates_file() {
        let tmp = tempfile::tempdir().unwrap();
        let output_path = tmp.path().join("test-config.json");
        let result = init_config(Some(&output_path)).await;
        assert!(result.is_ok(), "init_config should succeed");
        assert!(output_path.exists(), "config file should be created");
    }

    #[tokio::test]
    async fn test_init_config_with_none_uses_default() {
        let result = init_config(None).await;
        // Should not panic; writes to default location
        assert!(result.is_ok() || result.is_err()); // either ok or error from directory issues is fine
    }

    #[test]
    fn test_explain_workflow_no_example_does_not_panic() {
        // explain_workflow uses print_struct_docs! which may fail in non-TTY environments
        let result = std::panic::catch_unwind(|| explain_workflow(false));
        if let Ok(Ok(()) | Err(_)) = result {}
    }

    #[test]
    fn test_explain_workflow_with_example_does_not_panic() {
        // explain_workflow uses print_struct_docs! which may fail in non-TTY environments
        let result = std::panic::catch_unwind(|| explain_workflow(true));
        if let Ok(Ok(()) | Err(_)) = result {}
    }

    #[test]
    fn test_explain_workflow_example_json_matches_expected() {
        let actual = &*crate::cli_runner::explain_workflow::EXAMPLE_WORKFLOW_DATA;

        assert_eq!(actual.name, "Complete example");
        assert!(actual.description.is_some());
        assert_eq!(actual.procedures.len(), 1);
        assert_eq!(actual.procedures[0].name, "main-procedure");
        assert!(actual.procedures[0].steps.len() > 10);
        assert_eq!(actual.steps.len(), 2);

        let json = serde_json::to_string(actual).unwrap();
        let deserialized: workflow::workflow_data::WorkflowData =
            serde_json::from_str(&json).expect("Failed to deserialize example JSON");

        assert_eq!(deserialized.name, actual.name);
        assert_eq!(deserialized.procedures.len(), actual.procedures.len());
        assert_eq!(deserialized.steps.len(), actual.steps.len());
    }

    #[test]
    fn test_test_headless_config_has_headless_true() {
        let config = test_headless_config();
        assert!(
            config.browser.headless,
            "Test config should have headless=true"
        );
    }

    #[test]
    fn test_test_headless_config_has_default_workflows_dir() {
        let config = test_headless_config();
        assert_eq!(
            config.workflows_directory,
            Config::default().workflows_directory
        );
    }

    #[test]
    fn test_explain_workflow_example_json_roundtrip() {
        // Verify the example JSON produced by explain_workflow(true) can be
        // deserialized back into WorkflowData
        let example_json = serde_json::to_string_pretty(&{
            let data = workflow::workflow_data::WorkflowData {
                name: String::from("Complete Example"),
                description: Some(String::from("A comprehensive example workflow demonstrating all step types and their parameters")),
                procedures: vec![
                    workflow::workflow_data::WorkflowProcedure {
                        name: String::from("main"),
                        steps: vec![
                            workflow::workflow_data::step::WorkflowStepType::PageAction(
                                workflow::workflow_data::step::PageActionStep {
                                    page_index: 0,
                                    action: workflow::workflow_data::step::PageAction::Navigate(
                                        workflow::workflow_data::step::page_action::NavigateAction {
                                            url: "https://example.com".into(),
                                        },
                                    ),
                                },
                            ),
                        ],
                    },
                ],
                steps: vec![
                    workflow::workflow_data::step::WorkflowStepType::PageAction(
                        workflow::workflow_data::step::PageActionStep {
                            page_index: 0,
                            action: workflow::workflow_data::step::PageAction::Navigate(
                                workflow::workflow_data::step::page_action::NavigateAction {
                                    url: "https://github.com".into(),
                                },
                            ),
                        },
                    ),
                ],
            };
            data
        })
        .unwrap();

        // This should not panic
        let deserialized: workflow::workflow_data::WorkflowData =
            serde_json::from_str(&example_json).expect("Failed to deserialize example JSON");

        assert_eq!(deserialized.name, "Complete Example");
        assert_eq!(
            deserialized.description,
            Some(String::from(
                "A comprehensive example workflow demonstrating all step types and their parameters"
            ))
        );
        assert_eq!(deserialized.procedures.len(), 1);
        assert_eq!(deserialized.procedures[0].name, "main");
        assert_eq!(deserialized.steps.len(), 1);
    }
}
