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
use documented::DocumentedFields;
use std::{
    io::{Write, stdout},
    path::PathBuf,
};

use cli::Commands::{ExplainConfig, InitConfig, Run};
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = cli::get_cli();
    run_cli(&cli).await
}

async fn run_cli(cli: &cli::Cli) -> Result<()> {
    match &cli.command {
        Some(Run { workflow }) => run_workflow(workflow, cli).await,
        Some(InitConfig { output_path }) => init_config(output_path.as_ref()).await,
        Some(ExplainConfig {}) => explain_config(),
        None => run_tui(cli).await,
    }
}

async fn run_workflow(workflow_path_or_name: &str, cli: &cli::Cli) -> Result<()> {
    let _ = env_logger::try_init();

    log::info!("Running workflow: {workflow_path_or_name}");

    let config_path = cli
        .config_path
        .as_ref()
        .context("Config path is required")?;
    let config = config::load_config(config_path).await;

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

    workflow.run(&config).await?;

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
    tui::inline::print_struct(&tui::inline::StructDocs {
        description: "Config must be a json file with the following fields:",
        field_names: Config::FIELD_NAMES,
        field_docs: Config::FIELD_DOCS,
    })?;

    stdout().write_all(b"\n\n").ok();

    tui::inline::print_struct(&tui::inline::StructDocs {
        description: "HexColorsConfig. All fields are optional.",
        field_names: config::HexColorsConfig::FIELD_NAMES,
        field_docs: config::HexColorsConfig::FIELD_DOCS,
    })?;

    stdout().write_all(b"\n\n").ok();

    tui::inline::print_struct(&tui::inline::StructDocs {
        description: "BrowserConfig. Used when starting the browser",
        field_names: config::BrowserConfig::FIELD_NAMES,
        field_docs: config::BrowserConfig::FIELD_DOCS,
    })?;

    Ok(())
}

async fn run_tui(cli: &cli::Cli) -> Result<()> {
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
}
