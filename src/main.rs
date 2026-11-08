use anyhow::Result;
use dotenvy::dotenv;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use web_assistant::{Agent, ChatOpenAI, PromptMode};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Complex multi-step task requiring navigation, parsing repository metrics and metadata
    let task = "Go to https://github.com/browser-use/browser-use and find: 1) the repository star count, 2) the software license, and 3) the main description of what the project does.";

    println!("Initializing Agent with task: {task}\n");

    let llm = ChatOpenAI::new("auto");
    let mut agent = Agent::new(task, llm)
        .prompt_mode(PromptMode::Standard)
        .max_steps(16);
    agent.browser.config.headless = true;

    let history = agent.run().await?;

    if let Some(ref result) = history.final_result() {
        println!("\n=================== FINAL RESULT ===================");
        println!("{result}");
        println!("====================================================\n");
    } else {
        println!("\nNo final result returned.");
    }

    // Evaluate trace using the ported Judge system
    println!("Evaluating agent execution trace with AI Judge...");
    let ground_truth = "The project license is MIT. The project is an open-source library for web browser automation with AI.";
    let judgement = agent.judge(Some(ground_truth)).await?;

    println!("\n================= JUDGEMENT REPORT =================");
    println!(
        "Verdict:          {}",
        if judgement.verdict {
            "PASSED (true)"
        } else {
            "FAILED (false)"
        }
    );
    println!("Impossible Task:  {}", judgement.impossible_task);
    println!("Reached Captcha:  {}", judgement.reached_captcha);
    if !judgement.failure_reason.is_empty() {
        println!("Failure Reason:   {}", judgement.failure_reason);
    }
    println!("Judge Reasoning:  {}", judgement.reasoning);
    println!("====================================================");

    Ok(())
}
