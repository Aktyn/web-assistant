use anyhow::Result;
use dotenvy::dotenv;
use std::path::Path;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use web_assistant::{Agent, ChatBrowserUse, ChatOpenAI};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Translated from python example:
    // llm = ChatOpenAI(model='auto')
    // agent = Agent(
    //     task="Find the number of stars of the browser-use repo",
    //     llm=llm,
    //     # browser=Browser(use_cloud=True),
    // )
    // history = await agent.run()
    // print(history.final_result())

    let llm = ChatOpenAI::new("auto");
    println!("OpenAI base URL: {}", llm.base_url);
    let mut agent = Agent::new("Find the number of stars of the browser-use repo", llm);

    let history = agent.run().await?;

    if let Some(ref result) = history.final_result() {
        println!(
            "\n================ FINAL RESULT ================\n{}\n==============================================",
            result
        );
    } else {
        println!("\nNo final result returned.");
    }

    // Save history to file
    let history_file = Path::new("agent_history.json");
    agent.save_history(history_file)?;
    println!("\nHistory saved to {}", history_file.display());

    // Variable detection capabilities on run history
    let variables = agent.detect_variables();
    println!("\nDetected {} variable(s) in task run:", variables.len());
    for (name, var) in &variables {
        println!(
            "  - {}: \"{}\" (format: {:?})",
            name, var.original_value, var.format
        );
    }

    // Rerun history capability
    println!("\nRerunning history from saved file...");
    let mut rerun_agent = Agent::new("", ChatBrowserUse::new("auto"));
    let rerun_results = rerun_agent.load_and_rerun(history_file, None).await?;
    println!(
        "Rerun finished successfully with {} action results.",
        rerun_results.len()
    );

    // Variable detection and substitution demonstration
    println!("\nDemonstrating variable detection & substitution on parameterized workflow:");
    use std::collections::HashMap;
    use web_assistant::agent::views::{
        Action, ActionResult, AgentHistory, AgentHistoryList, AgentOutput, AgentStepState,
        InteractedElementMetadata,
    };
    use web_assistant::agent::{detect_variables_in_history, substitute_variables_in_history};

    let mut email_attrs = HashMap::new();
    email_attrs.insert("type".into(), "email".into());
    email_attrs.insert("name".into(), "user_email".into());

    let mut name_attrs = HashMap::new();
    name_attrs.insert("name".into(), "first_name".into());

    let form_history = AgentHistoryList {
        history: vec![AgentHistory {
            model_output: Some(AgentOutput {
                thought: Some("Input email and name".into()),
                action: vec![
                    Action::Input {
                        index: 1,
                        text: "original.user@example.com".into(),
                    },
                    Action::Input {
                        index: 2,
                        text: "Alice".into(),
                    },
                    Action::Input {
                        index: 3,
                        text: "2026-11-07".into(),
                    },
                ],
                ..Default::default()
            }),
            state: Some(AgentStepState {
                url: "https://example.com/form".into(),
                title: "Registration".into(),
                interacted_element: Some(vec![
                    InteractedElementMetadata {
                        index: 1,
                        tag: "input".into(),
                        attributes: email_attrs,
                    },
                    InteractedElementMetadata {
                        index: 2,
                        tag: "input".into(),
                        attributes: name_attrs,
                    },
                    InteractedElementMetadata {
                        index: 3,
                        tag: "input".into(),
                        attributes: HashMap::new(),
                    },
                ]),
            }),
            result: vec![ActionResult::success("Inputs filled")],
        }],
    };

    let detected_vars = detect_variables_in_history(&form_history);
    println!(
        "Detected {} variable(s) from form history:",
        detected_vars.len()
    );
    for (name, var) in &detected_vars {
        println!(
            "  • {}: \"{}\" (format: {:?})",
            name, var.original_value, var.format
        );
    }

    let mut substitutions = HashMap::new();
    substitutions.insert("email".into(), "substituted.user@rust-lang.org".into());
    substitutions.insert("first_name".into(), "Ferris".into());
    substitutions.insert("date".into(), "2030-01-01".into());

    println!("Substituting with new values:");
    for (var, val) in &substitutions {
        println!("  • {} -> \"{}\"", var, val);
    }

    let substituted_history = substitute_variables_in_history(&form_history, &substitutions);
    let sub_actions = &substituted_history.history[0]
        .model_output
        .as_ref()
        .unwrap()
        .action;
    println!("Substituted actions for rerun:");
    for act in sub_actions {
        if let Action::Input { index, text } = act {
            println!("  • Element [{}]: \"{}\"", index, text);
        }
    }
    let param_history_file = Path::new("parameterized_history.json");
    substituted_history.save_to_file(param_history_file)?;
    println!(
        "Parameterized history saved to {}",
        param_history_file.display()
    );

    Ok(())
}
