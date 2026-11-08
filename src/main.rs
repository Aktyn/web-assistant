//TODO: public modifier is only to suppress unused code warnings and should be removed in the future
pub mod agent;
pub mod browser;
pub mod llm;

use anyhow::Result;
use dotenvy::dotenv;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let config = browser::BrowserConfig {
        headless: false,
        ..Default::default()
    };
    let mut browser = browser::Browser::new(Some(config));
    match browser.start().await {
        Ok(_) => println!("Browser started successfully"),
        Err(e) => return Err(anyhow::anyhow!("Browser failed to start: \"{e}\"")),
    };

    match browser.navigate("https://example.com").await {
        Ok(_) => println!("Navigation successful"),
        Err(e) => println!("Navigation failed: {e}"),
    };

    //TODO: create configurable scrapper logic

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
