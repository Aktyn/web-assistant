use anyhow::{Result, bail};

use super::step;
use crate::agent;
use crate::llm;

impl super::RunningWorkflow {
    pub(super) fn run_agent_step(&self, step: &step::AgentStep) -> Result<()> {
        log::info!("Running agent step: \"{step:?}\"");

        // LLM inference interface
        let llm = llm::openai::ChatOpenAI::new(&self.config.agent.model);

        //TODO: remove
        if 5 > 10 {
            bail!("Lolz");
        }

        //TODO: implement browser support
        // let browser = self.browser.write().await;
        // let _agent = agent::Agent::new_with_browser(&step.task, llm, self.browser)
        let _agent = agent::Agent::new(&step.task, llm)
            .prompt_mode(agent::PromptMode::Standard)
            .max_steps(32);

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

        Ok(())
    }
}
