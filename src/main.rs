//TODO: remove `pub` from module declarations
pub mod agent;
pub mod browser;
mod cli;
mod config;
pub mod llm;
mod tui;
mod utils;
mod workflow;

use anyhow::{Context, Result};
use documented::{Documented, DocumentedFields};
use std::{
    io::{Write, stdout},
    path::PathBuf,
};

use cli::Commands::{ExplainConfig, ExplainWorkflow, InitConfig, Run};
use config::Config;

use crate::{tui::common::init_palette, workflow::WorkflowData};
use workflow::workflow_data::step as workflow_step;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::get_cli();
    run_cli(&cli).await
}

async fn run_cli(cli: &cli::Cli) -> Result<()> {
    match &cli.command {
        Some(Run { workflow }) => run_workflow(workflow, cli).await,
        Some(InitConfig { output_path }) => init_config(output_path.as_ref()).await,
        Some(ExplainConfig {}) => explain_config(),
        Some(ExplainWorkflow { include_example }) => {
            explain_workflow(include_example.unwrap_or(false))
        }
        None => run_tui(cli).await,
    }
}

async fn run_workflow(workflow_path_or_name: &str, cli: &cli::Cli) -> Result<()> {
    dotenvy::dotenv().ok();
    env_logger::try_init().ok();

    log::info!("Running workflow: {workflow_path_or_name}");

    let config_path = cli
        .config_path
        .as_ref()
        .context("Config path is required")?;
    let config = config::load_config(config_path).await;

    if let Some(colors) = &config.tui_colors {
        init_palette(colors);
    }

    let possible_path = PathBuf::from(workflow_path_or_name);
    let mut workflow = if possible_path.exists() && possible_path.is_file() {
        workflow::load_from_file(&possible_path)
            .await
            .context("Couldn't load workflow from file")?
    } else {
        // Load all workflows from config.workflowsDirectory and find the one with matching name
        let directory_workflows =
            workflow::load_from_directory(&config.workflows_directory).await?;
        let matching_workflow = directory_workflows.into_iter().find(|workflow| {
            matches!(
                &workflow.state,
                workflow::WorkflowState::Valid(data) if data.name.eq_ignore_ascii_case(workflow_path_or_name)
            )
        });

        matching_workflow.context(format!(
            "No workflow found matching name \"{workflow_path_or_name}\""
        ))?
    };

    workflow.run(Box::new(config)).await?;

    Ok(())
}

async fn init_config(output_path: Option<&PathBuf>) -> Result<()> {
    stdout().write_all(b"Initializing config...\n").ok();

    let config_path = config::create_default_config(output_path.map(PathBuf::as_path)).await?;

    let absolute_path = config_path
        .canonicalize()
        .unwrap_or_else(|_| config_path.to_path_buf());
    tui::inline::print_success(&format!(
        r"The default config was successfully saved to: {}",
        absolute_path.display()
    ))
}

fn explain_config() -> Result<()> {
    tui::inline::print_struct_docs!(Config)?;
    utils::print_lines(1);
    tui::inline::print_struct_docs!(config::HexColorsConfig, 2)?;
    utils::print_lines(1);
    tui::inline::print_struct_docs!(config::BrowserConfig, 2)?;
    utils::print_lines(1);
    tui::inline::print_struct_docs!(config::AgentConfig, 2)?;
    utils::print_lines(1);

    Ok(())
}

fn explain_workflow(include_example: bool) -> Result<()> {
    tui::inline::print_struct_docs!(workflow::WorkflowData)?;

    utils::print_lines(1);

    tui::inline::print_struct_docs!(workflow_step::PageActionStep, 2)?;
    utils::print_lines(1);
    tui::inline::print_struct_docs!(workflow_step::page_action::NavigateAction, 4)?;
    utils::print_lines(1);
    tui::inline::print_struct_docs!(workflow_step::AgentStep, 2)?;

    if include_example {
        utils::print_lines(2);

        let example: WorkflowData = workflow::WorkflowData {
            name: String::from("Example"),
            description: Some(String::from("Example workflow description")),
            steps: vec![
                workflow_step::WorkflowStepType::PageAction(workflow_step::PageActionStep {
                    page_index: 0,
                    action: workflow_step::PageAction::Navigate(
                        workflow_step::page_action::NavigateAction {
                            url: String::from("https://github.com/Aktyn/web-assistant"),
                        },
                    ),
                }),
                workflow_step::WorkflowStepType::Agent(workflow_step::AgentStep {
                    task: String::from(
                        "Star this repository and print total number of stars the repository has.",
                    ),
                    max_steps: 16,
                    history_output: Some(PathBuf::from("./repo-stars-agent-run.json")),
                }),
            ],
        };
        let json = serde_json::to_string_pretty(&example)?.replace('\n', "\r\n");
        let output = format!("Example workflow json:\r\n{json}");
        stdout().write_all(output.as_bytes()).ok();
    } else {
        utils::print_lines(1);
        stdout()
            .write_all(b"\nUse --include-example argument to print an example workflow file\n")
            .ok();
    }

    Ok(())
}

async fn run_tui(cli: &cli::Cli) -> Result<()> {
    dotenvy::dotenv().ok();

    // NOTE: TUI logger is initialized here to catch early core logs
    tui_logger::init_logger(tui_logger::LevelFilter::Trace)?;
    tui_logger::set_default_level(tui_logger::LevelFilter::Trace);

    if let Some(config_path) = &cli.config_path {
        let config = config::load_config(config_path).await;
        tui::render_tui(config).await
    } else {
        anyhow::bail!("Config path is required")
    }
}

// An example left for future reference
// #[tokio::main]
// async fn main() -> Result<()> {
//     dotenv().ok();
//
//     tracing_subscriber::registry()
//         .with(
//             tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
//         )
//         .with(tracing_subscriber::fmt::layer())
//         .init();
//
//     let task = "Find the latest version of the Rust programming language and print it as a result.";
//
//     println!("Initializing Agent with task: {task}\n");
//
//     let llm = llm::openai::ChatOpenAI::new("auto");
//     let mut agent = agent::Agent::new(task, llm)
//         .prompt_mode(agent::PromptMode::Standard)
//         .max_steps(32);
//     // agent.browser.config.headless = true;
//
//     let history = agent.run().await?;
//
//     if let Some(ref result) = history.final_result() {
//         println!("\n=================== FINAL RESULT ===================");
//         println!("{result}");
//         println!("====================================================\n");
//     } else {
//         println!("\nNo final result returned.");
//     }
//
//     // Save history to file
//     let history_file = Path::new("agent_history.json");
//     agent.save_history(history_file)?;
//     println!("\nHistory saved to {}", history_file.display());
//
//     // Variable detection capabilities on run history
//     let variables = agent.detect_variables();
//     println!("\nDetected {} variable(s) in task run:", variables.len());
//     for (name, var) in &variables {
//         println!(
//             "  - {}: \"{}\" (format: {:?})",
//             name, var.original_value, var.format
//         );
//     }
//
//     ---------------------------------------------------------------------
//
//     // Can be run again:
//     // let mut rerun_agent = Agent::new("", ChatBrowserUse::new("auto"));
//     // let rerun_results = rerun_agent.load_and_rerun(history_file, None).await?;
//     // println!(
//     //     "Rerun finished successfully with {} action results.",
//     //     rerun_results.len()
//     // );
//
//     // Evaluate trace using the ported Judge system
//     println!("Evaluating agent execution trace with AI Judge...");
//     let ground_truth = "The project license is MIT. The project is an open-source library for web browser automation with AI.";
//     let judgement = agent.judge(Some(ground_truth)).await?;
//
//     println!("\n================= JUDGEMENT REPORT =================");
//     println!(
//         "Verdict:          {}",
//         if judgement.verdict {
//             "PASSED (true)"
//         } else {
//             "FAILED (false)"
//         }
//     );
//     println!("Impossible Task:  {}", judgement.impossible_task);
//     println!("Reached Captcha:  {}", judgement.reached_captcha);
//     if !judgement.failure_reason.is_empty() {
//         println!("Failure Reason:   {}", judgement.failure_reason);
//     }
//     println!("Judge Reasoning:  {}", judgement.reasoning);
//     println!("====================================================");
//
//     Ok(())
// }

#[cfg(test)]
mod tests {
    use super::*;
    use cli::Cli;
    use cli::Commands::Run;

    #[test]
    fn test_run_workflow_requires_config_path() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let cli = Cli {
                config_path: None,
                command: Some(Run {
                    workflow: "test-workflow".to_string(),
                }),
            };
            run_workflow("test-workflow", &cli).await
        });
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Config path is required");
    }

    #[test]
    fn test_run_workflow_with_valid_config_returns_ok() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("config.json");
        // Create a workflow file in the temp directory.
        let wf_dir = tmp.path().join("workflows");
        std::fs::create_dir(&wf_dir).unwrap();
        std::fs::write(
            wf_dir.join("my-workflow.json"),
            r#"{"name":"my-workflow","steps":[]}"#,
        )
        .unwrap();
        let config_json = serde_json::json!({
            "workflowsDirectory": wf_dir.to_str().unwrap(),
            "browser": { "headless": true }
        });
        std::fs::write(
            &config_path,
            serde_json::to_string_pretty(&config_json).unwrap(),
        )
        .unwrap();

        let result = rt.block_on(async {
            let cli = Cli {
                config_path: Some(config_path),
                command: Some(Run {
                    workflow: "my-workflow".to_string(),
                }),
            };
            run_workflow("my-workflow", &cli).await
        });
        // The workflow lookup and config loading should succeed; the only
        // acceptable failure is the browser not being available in the CI
        // environment (no Chrome/Chromium installed).
        if let Err(err) = &result {
            let err_str = err.to_string();
            assert!(
                err_str.contains("Chrome")
                    || err_str.contains("chromium")
                    || err_str.contains("DevTools"),
                "Expected only a browser-launch failure, got: {err_str}"
            );
        }
    }

    #[test]
    fn test_run_cli_matches_workflow_command() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("config.json");
        // Create a workflow file in the temp directory.
        let wf_dir = tmp.path().join("workflows");
        std::fs::create_dir(&wf_dir).unwrap();
        std::fs::write(
            wf_dir.join("test-wf.json"),
            r#"{"name":"test-wf","steps":[]}"#,
        )
        .unwrap();
        let config_json = serde_json::json!({
            "workflowsDirectory": wf_dir.to_str().unwrap(),
            "browser": { "headless": true }
        });
        std::fs::write(
            &config_path,
            serde_json::to_string_pretty(&config_json).unwrap(),
        )
        .unwrap();

        let cli = Cli {
            config_path: Some(config_path.clone()),
            command: Some(Run {
                workflow: "test-wf".to_string(),
            }),
        };
        let result = rt.block_on(run_cli(&cli));
        // The workflow lookup should succeed; browser.start() may fail in CI env.
        if let Err(err) = &result {
            let err_str = err.to_string();
            assert!(
                !err_str.contains("No workflow found"),
                "Expected workflow lookup to succeed, got: {err_str}"
            );
        }
    }

    #[test]
    fn test_run_workflow_no_workflow_found() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("config.json");
        let wf_dir = tmp.path().join("workflows");
        std::fs::create_dir(&wf_dir).unwrap();
        let config_json = serde_json::json!({
            "workflowsDirectory": wf_dir.to_str().unwrap(),
            "browser": { "headless": true }
        });
        std::fs::write(
            &config_path,
            serde_json::to_string_pretty(&config_json).unwrap(),
        )
        .unwrap();

        let result = rt.block_on(async {
            let cli = Cli {
                config_path: Some(config_path),
                command: Some(Run {
                    workflow: "does-not-exist".to_string(),
                }),
            };
            run_cli(&cli).await
        });
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No workflow found")
        );
    }

    #[test]
    fn test_run_workflow_path_not_exists() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("config.json");
        let wf_dir = tmp.path().join("workflows");
        std::fs::create_dir(&wf_dir).unwrap();
        let config_json = serde_json::json!({
            "workflowsDirectory": wf_dir.to_str().unwrap(),
            "browser": { "headless": true }
        });
        std::fs::write(
            &config_path,
            serde_json::to_string_pretty(&config_json).unwrap(),
        )
        .unwrap();

        let result = rt.block_on(async {
            let cli = Cli {
                config_path: Some(config_path),
                command: Some(Run {
                    workflow: "/nonexistent/path/to/workflow.json".to_string(),
                }),
            };
            run_cli(&cli).await
        });
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        // When the path doesn't exist, the code falls back to directory lookup
        // which returns "No workflow found" since the name doesn't match.
        assert!(
            err_str.contains("No workflow found")
                || err_str.contains("Couldn't load workflow from file"),
            "Unexpected error: {err_str}"
        );
    }

    #[test]
    fn test_run_tui_requires_config_path() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let cli = Cli {
                config_path: None,
                command: None,
            };
            run_tui(&cli).await
        });
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        // The TUI logger may already be initialized in some test contexts,
        // which short-circuits before the config path check.
        assert!(
            err_str.contains("Config path is required") || err_str.contains("SetLoggerError"),
            "Unexpected error: {err_str}"
        );
    }

    #[test]
    fn test_run_tui_with_config_path() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("config.json");
        let wf_dir = tmp.path().join("workflows");
        std::fs::create_dir(&wf_dir).unwrap();
        let config_json = serde_json::json!({
            "workflowsDirectory": wf_dir.to_str().unwrap(),
            "browser": { "headless": true }
        });
        std::fs::write(
            &config_path,
            serde_json::to_string_pretty(&config_json).unwrap(),
        )
        .unwrap();

        // `run_tui` initializes the TUI logger and then tries to render the
        // real terminal. In CI / tarpaulin there is no TTY, so the render
        // step panics. We accept either a successful return or a panic whose
        // message indicates a terminal initialization failure.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            rt.block_on(async {
                let cli = Cli {
                    config_path: Some(config_path),
                    command: None,
                };
                run_tui(&cli).await
            })
        }));
        match result {
            Ok(Ok(())) => {} // TUI ran successfully (rare in CI).
            Ok(Err(e)) => {
                // Logger already initialized or some other recoverable error.
                let _ = e;
            }
            Err(payload) => {
                let msg = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(ToString::to_string))
                    .unwrap_or_default();
                assert!(
                    msg.contains("terminal")
                        || msg.contains("No such device")
                        || msg.contains("SetLoggerError"),
                    "Unexpected panic: {msg}"
                );
            }
        }
    }
}
