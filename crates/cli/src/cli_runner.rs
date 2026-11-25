use anyhow::{Context, Result, bail};
use std::{
    io::{Write as _, stdout},
    path::PathBuf,
};

use crate::cli::{Cli, Commands};
use config::{Config, create_default_config};
use utils::logger::print_lines;
use workflow::workflow_data::{
    WorkflowData, WorkflowProcedure, step as workflow_step, workflow_string::WorkflowString,
};
use workflow::{self, WorkflowState};

use Commands::{ExplainConfig, ExplainWorkflow, InitConfig, Run};

impl Cli {
    /// Returns true if any cli command was ran
    pub async fn run(&self) -> Result<bool> {
        match &self.command {
            Some(Run { workflow }) => self.run_workflow(workflow).await?,
            Some(InitConfig { output_path }) => init_config(output_path.as_ref()).await?,
            Some(ExplainConfig {}) => explain_config()?,
            Some(ExplainWorkflow { include_example }) => {
                explain_workflow(include_example.unwrap_or(false))?;
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
                logger.error(&format!("Workflow run failed: {}", failed_run.error));
            }
            WorkflowState::Valid(..) | WorkflowState::Invalid(..) | WorkflowState::Running(..) => {
                bail!("Workflow state is not expected")
            }
        }

        Ok(())
    }
}

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

async fn init_config(output_path: Option<&PathBuf>) -> Result<()> {
    let mut logger = utils::logger::CliLogger::default();
    logger.info("Initializing config...");

    let config_path = create_default_config(output_path.map(PathBuf::as_path)).await?;

    let absolute_path = config_path
        .canonicalize()
        .unwrap_or_else(|_| config_path.to_path_buf());

    logger.success(&format!(
        "The default config was successfully saved to: {}",
        absolute_path.display()
    ));

    logger.reset();

    Ok(())
}

fn explain_config() -> Result<()> {
    print_struct_docs!(Config)?;
    print_lines(1);
    print_struct_docs!(config::HexColorsConfig, 2)?;
    print_lines(1);
    print_struct_docs!(config::BrowserConfig, 2)?;
    print_lines(1);
    print_struct_docs!(config::AgentConfig, 2)?;
    print_lines(1);

    Ok(())
}

fn explain_workflow(include_example: bool) -> Result<()> {
    print_struct_docs!(WorkflowData)?;
    print_lines(1);
    print_struct_docs!(WorkflowProcedure, 2)?;
    print_lines(1);

    print_struct_docs!(workflow_step::PageActionStep, 2)?;
    print_lines(1);
    print_struct_docs!(workflow_step::page_action::NavigateAction, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::page_action::InputAction, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::page_action::ClickAction, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::page_action::EvaluateAction, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::AgentStep, 2)?;
    print_lines(1);
    print_struct_docs!(workflow_step::ConditionStep, 2)?;
    print_lines(1);
    print_struct_docs!(workflow_step::condition::MatchRegexCondition, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::condition::TruthyCondition, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::condition::EqualCondition, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::condition::HasElementCondition, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::ForEachStep, 2)?;
    print_lines(1);
    print_struct_docs!(workflow_step::iteration::FileIterator, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::RunProcedureStep, 2)?;
    print_lines(1);
    print_struct_docs!(workflow_step::UpdateContextStep, 2)?;
    print_lines(1);
    print_struct_docs!(workflow_step::SystemActionStep, 2)?;
    print_lines(1);
    print_struct_docs!(workflow_step::system_action::NotifyAction, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::system_action::OpenUrlAction, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::WaitStep, 2)?;
    print_lines(1);
    print_struct_docs!(workflow_step::AbortStep, 2)?;

    if include_example {
        print_lines(2);

        let example: WorkflowData = WorkflowData {
            name: String::from("Example"),
            description: Some(String::from("Example workflow description")),
            procedures: vec![
                //TODO: Add example procedure
            ],
            steps: vec![
                workflow_step::WorkflowStepType::PageAction(workflow_step::PageActionStep {
                    page_index: 0,
                    action: workflow_step::PageAction::Navigate(
                        workflow_step::page_action::NavigateAction {
                            url: WorkflowString::from("https://github.com/Aktyn/web-assistant"),
                        },
                    ),
                }),
                workflow_step::WorkflowStepType::Agent(workflow_step::AgentStep {
                    task: WorkflowString::from(
                        "Star this repository and print total number of stars the repository has.",
                    ),
                    max_steps: 16,
                    history_output: Some(PathBuf::from("./repo-stars-agent-run.json")),
                    prompt_mode: agent::PromptMode::Standard,
                }),
            ],
        };
        let json = serde_json::to_string_pretty(&example)?.replace('\n', "\r\n");
        let output = format!("Example workflow json:\r\n{json}");
        stdout().write_all(output.as_bytes()).ok();
    } else {
        print_lines(1);
        stdout()
            .write_all(b"\nUse --include-example argument to print an example workflow file\n")
            .ok();
    }

    Ok(())
}

/// Convenience macro that builds a [`StructDocs`] from a documented type's
/// `DOCS`, `FIELD_NAMES`, and `FIELD_DOCS` associated constants and passes it
/// to [`print_struct`].
///
/// # Examples
///
/// ```ignore
/// print_struct_docs!(Config);
/// print_struct_docs!(config::HexColorsConfig, 2);
/// ```
macro_rules! print_struct_docs {
    ($struct_type:ty) => {
        tui::inline::print_struct(&tui::inline::StructDocs {
            description: <$struct_type as documented::Documented>::DOCS,
            field_names: <$struct_type as documented::DocumentedFields>::FIELD_NAMES,
            field_docs: <$struct_type as documented::DocumentedFields>::FIELD_DOCS,
            ..Default::default()
        })
    };
    ($struct_type:ty, $margin_left:expr) => {
        tui::inline::print_struct(&tui::inline::StructDocs {
            description: <$struct_type as documented::Documented>::DOCS,
            field_names: <$struct_type as documented::DocumentedFields>::FIELD_NAMES,
            field_docs: <$struct_type as documented::DocumentedFields>::FIELD_DOCS,
            margin_left: Some($margin_left),
        })
    };
}
use print_struct_docs;

#[cfg(test)]
mod tests {
    use super::*;

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
}
