use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::views::AgentHistoryList;
use crate::llm::{ChatMessage, ChatOpenAI};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JudgementResult {
    pub reasoning: String,
    pub verdict: bool,
    pub failure_reason: String,
    pub impossible_task: bool,
    pub reached_captcha: bool,
}

pub struct Judge {
    pub llm: ChatOpenAI,
}

impl Judge {
    pub fn new(llm: ChatOpenAI) -> Self {
        Self { llm }
    }

    pub async fn judge(
        &self,
        task: &str,
        final_result: &str,
        history: &AgentHistoryList,
        ground_truth: Option<&str>,
    ) -> Result<JudgementResult> {
        info!("Running AI trace judgement for task: {}", task);

        let mut step_descriptions = Vec::new();
        for (i, item) in history.history.iter().enumerate() {
            let step_num = i + 1;
            let thought = item
                .model_output
                .as_ref()
                .and_then(|o| o.thought.as_deref())
                .unwrap_or("None");

            let mut actions_str = String::new();
            if let Some(out) = &item.model_output {
                for act in &out.action {
                    actions_str.push_str(&format!("{:?}; ", act));
                }
            }

            let mut results_str = String::new();
            for res in &item.result {
                if let Some(err) = &res.error {
                    results_str.push_str(&format!("Error: {err}; "));
                } else if let Some(content) = &res.extracted_content {
                    results_str.push_str(&format!("Output: {content}; "));
                }
            }

            step_descriptions.push(format!(
                "Step {step_num}:\nThought: {thought}\nActions: {actions_str}\nResults: {results_str}"
            ));
        }

        let messages =
            construct_judge_messages(task, final_result, &step_descriptions, ground_truth);
        let resp = self
            .llm
            .invoke(&messages)
            .await
            .context("Failed to get judgement from LLM")?;

        parse_judgement_response(&resp)
    }
}

pub fn construct_judge_messages(
    task: &str,
    final_result: &str,
    steps: &[String],
    ground_truth: Option<&str>,
) -> Vec<ChatMessage> {
    let ground_truth_section = if ground_truth.is_some() {
        "\n**GROUND TRUTH VALIDATION (HIGHEST PRIORITY):**\n\
            The <ground_truth> section contains verified criteria for this task.\n\
            The ground truth takes ABSOLUTE precedence. If not satisfied, the verdict MUST be false.\n".to_string()
    } else {
        String::new()
    };

    let system_prompt = format!(
        r#"You are an expert judge evaluating browser automation agent performance.

<evaluation_framework>
{ground_truth_section}
**PRIMARY EVALUATION CRITERIA:**
1. Task Satisfaction (Most Important): Did the agent accomplish what the user asked for?
2. Output Quality: Is the final result in the correct format, complete, and accurate?
3. Tool Effectiveness: Did the browser interactions work as expected?
4. Agent Reasoning: Quality of decision-making and planning throughout trajectory.
5. Browser Handling: Navigation stability, error recovery, and technical execution.

**VERDICT GUIDELINES:**
- true: Task completed as requested, all user criteria met, no fabricated info.
- false: Task not completed or only partially completed.

**FAILURE CONDITIONS (automatically set verdict to false):**
- Blocked by captcha or missing authentication
- Output format completely wrong or missing
- Severe technical failures or infinite loops
- Critical user requirements ignored
- Agent fabricated information not on page

**IMPOSSIBLE TASK DETECTION:**
Set `impossible_task` to true when task fundamentally could not be completed (broken site, requires credentials not provided).

**CAPTCHA DETECTION:**
Set `reached_captcha` to true if captcha or bot verification blocked the execution.
</evaluation_framework>

<response_format>
Respond with EXACTLY this JSON structure:
{{
  "reasoning": "Detailed analysis covering what went well, what failed, and user satisfaction prediction.",
  "verdict": true or false,
  "failure_reason": "Max 5 sentences explaining failure reason (empty string if verdict is true).",
  "impossible_task": true or false,
  "reached_captcha": true or false
}}
</response_format>
"#
    );

    let ground_truth_block = if let Some(gt) = ground_truth {
        format!("\n<ground_truth>\n{gt}\n</ground_truth>\n")
    } else {
        String::new()
    };

    let steps_text = steps.join("\n\n");
    let user_prompt = format!(
        r#"
<task>
{task}
</task>
{ground_truth_block}
<agent_trajectory>
{steps_text}
</agent_trajectory>

<final_result>
{final_result}
</final_result>

Evaluate this agent execution given the criteria and respond with the exact JSON structure requested."#
    );

    vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(user_prompt),
    ]
}

pub fn parse_judgement_response(raw: &str) -> Result<JudgementResult> {
    let trimmed = raw.trim();
    let json_str = if let Some(start) = trimmed.find("```json") {
        let after = &trimmed[start + 7..];
        if let Some(end) = after.find("```") {
            after[..end].trim()
        } else {
            after.trim()
        }
    } else if let Some(start) = trimmed.find("```") {
        let after = &trimmed[start + 3..];
        if let Some(end) = after.find("```") {
            after[..end].trim()
        } else {
            after.trim()
        }
    } else if let (Some(first), Some(last)) = (trimmed.find('{'), trimmed.rfind('}')) {
        &trimmed[first..=last]
    } else {
        trimmed
    };

    let result: JudgementResult = serde_json::from_str(json_str)
        .with_context(|| format!("Failed to parse JudgementResult from JSON:\n{json_str}"))?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_judgement_success() {
        let raw = r#"{
            "reasoning": "Task was completed accurately and repository stars were found.",
            "verdict": true,
            "failure_reason": "",
            "impossible_task": false,
            "reached_captcha": false
        }"#;

        let result = parse_judgement_response(raw).unwrap();
        assert!(result.verdict);
        assert_eq!(result.failure_reason, "");
        assert!(!result.impossible_task);
        assert!(!result.reached_captcha);
    }

    #[test]
    fn test_parse_judgement_failure_with_codeblock() {
        let raw = r#"```json
        {
            "reasoning": "Could not access required credentials.",
            "verdict": false,
            "failure_reason": "Authentication required but missing credentials.",
            "impossible_task": true,
            "reached_captcha": false
        }
        ```"#;

        let result = parse_judgement_response(raw).unwrap();
        assert!(!result.verdict);
        assert!(result.impossible_task);
        assert_eq!(
            result.failure_reason,
            "Authentication required but missing credentials."
        );
    }
}
