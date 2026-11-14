pub mod agent;
pub mod browser;
mod cli;
mod config;
pub mod llm;
mod tui;
mod utils;
pub mod workflow;

use anyhow::Result;
use documented::DocumentedFields;
use std::io::{Write, stdout};

use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = cli::get_cli();

    // let config = browser::BrowserConfig {
    //     headless: false,
    //     ..Default::default()
    // };
    // let mut browser = browser::Browser::new(Some(config));
    // match browser.start().await {
    //     Ok(_) => println!("Browser started successfully"),
    //     Err(e) => return Err(anyhow::anyhow!("Browser failed to start: \"{e}\"")),
    // };
    //
    // match browser.navigate("https://example.com").await {
    //     Ok(_) => println!("Navigation successful"),
    //     Err(e) => println!("Navigation failed: {e}"),
    // };
    //
    // You can check the value provided by positional arguments, or option arguments
    // if let Some(name) = cli.name.as_deref() {
    //     println!("Value for name: {name}");
    // }

    match &cli.command {
        Some(cli::Commands::Run { workflow }) => {
            log::info!("Running workflow: {workflow}");
            //TODO: Implement inline workflow execution

            // let config_path = config_path.expect("Config path is required");
            // let config = Config::new(config_path).await;
        }

        Some(cli::Commands::InitConfig { output_path }) => {
            stdout().write_all(b"Initializing config...\n").ok();

            config::create_default_config(output_path.clone()).await;

            stdout()
                .write_all(
                    format!(r"The default config was successfully saved to {output_path:?}")
                        .as_bytes(),
                )
                .ok();
        }

        Some(cli::Commands::ExplainConfig {}) => {
            tui::inline::print_struct(&tui::inline::StructDocs {
                description: "Config must be a json file with the following fields:",
                field_names: Config::FIELD_NAMES,
                field_docs: Config::FIELD_DOCS,
            });

            stdout().write_all(b"\n\n").ok();

            tui::inline::print_struct(&tui::inline::StructDocs {
                description: "HexColorsConfig object. All fields are optional.",
                field_names: tui::common::HexColorsConfig::FIELD_NAMES,
                field_docs: tui::common::HexColorsConfig::FIELD_DOCS,
            });
        }

        None => {
            //NOTE: TUI logger is being initialized here to catch early core logs
            tui_logger::init_logger(tui_logger::LevelFilter::Trace).unwrap();
            tui_logger::set_default_level(tui_logger::LevelFilter::Trace);

            let config_path = cli.config_path.expect("Config path is required");
            let config = config::load_config(&config_path).await;

            return tui::render_tui(config).await;
        }
    }

    Ok(())
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
