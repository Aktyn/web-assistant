use std::path::Path;

use anyhow::Result;

use super::step;
use crate::agent;
use crate::llm;

impl super::RunningWorkflow {
    pub(super) async fn run_agent_step(
        &self,
        step: &step::AgentStep,
        workflow_path: impl AsRef<Path>,
    ) -> Result<()> {
        let mut logger = self.logger.lock().await;
        logger.info("Running agent");
        log::info!("{step:#?}");

        let mut browser = self.browser.lock().await;

        // LLM inference interface
        let llm = llm::openai::ChatOpenAI::new(llm::ChatOpenAIConfig {
            model: self.config.agent.model.clone(),
            base_url: self.config.agent.base_url.clone(),
            api_key: self.config.agent.api_key.clone(),
            ..llm::ChatOpenAIConfig::default()
        });
        let mut agent = agent::Agent::new(&step.task)
            .prompt_mode(step.prompt_mode)
            .max_steps(step.max_steps);

        let history = agent.run(&mut browser, &llm).await?;

        if let Some(ref result) = history.final_result() {
            logger.success("=================== AGENT RESULT ===================");
            logger.success(result);
            logger.success("====================================================");
        } else {
            logger.warning("No final result returned.");
        }

        // Save history to file
        if let Some(output_path) = &step.history_output {
            if output_path.is_relative() {
                let combined_path = workflow_path
                    .as_ref()
                    .parent()
                    .unwrap_or(Path::new(""))
                    .join(output_path);

                history.save_to_file(combined_path).await?;
            } else {
                history.save_to_file(output_path).await?;
            }
            logger.info(&format!("\nHistory saved to \"{}\"", output_path.display()));

            // Variable detection capabilities on run history
            // let variables = agent.detect_variables();
            let variables = history.detect_variables();
            logger.info(&format!(
                "\nDetected {} variable(s) in task run:",
                variables.len()
            ));
            for (name, var) in &variables {
                logger.info(&format!(
                    "  - {}: \"{}\" (format: {:?})",
                    name, var.original_value, var.format
                ));
            }
        }

        // Can be run again:
        // let mut rerun_agent = Agent::new("", ChatBrowserUse::new("auto"));
        // let rerun_results = rerun_agent.load_and_rerun(history_file, None).await?;
        // println!(
        //     "Rerun finished successfully with {} action results.",
        //     rerun_results.len()
        // );

        // Evaluate trace using the ported Judge system
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

        Ok(())
    }
}
