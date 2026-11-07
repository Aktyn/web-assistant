pub mod prompts;
pub mod variable_detector;
pub mod views;

use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, warn};

pub use variable_detector::{detect_variables_in_history, substitute_variables_in_history};
pub use views::{
    Action, ActionResult, AgentHistory, AgentHistoryList, AgentOutput, AgentStepState,
    DetectedVariable, InteractedElementMetadata,
};

use crate::browser::Browser;
use crate::llm::{ChatMessage, ChatOpenAI};

pub struct Agent {
    pub task: String,
    pub llm: ChatOpenAI,
    pub browser: Browser,
    pub max_steps: usize,
    pub history: AgentHistoryList,
}

impl Agent {
    pub fn new(task: impl Into<String>, llm: ChatOpenAI) -> Self {
        Self {
            task: task.into(),
            llm,
            browser: Browser::new(),
            max_steps: 20,
            history: AgentHistoryList::new(),
        }
    }

    pub fn new_with_browser(task: impl Into<String>, llm: ChatOpenAI, browser: Browser) -> Self {
        Self {
            task: task.into(),
            llm,
            browser,
            max_steps: 20,
            history: AgentHistoryList::new(),
        }
    }

    pub fn max_steps(mut self, steps: usize) -> Self {
        self.max_steps = steps;
        self
    }

    pub async fn run(&mut self) -> Result<AgentHistoryList> {
        info!("Starting agent task: {}", self.task);
        self.browser.start().await?;

        let mut step = 0;
        let mut conversation: Vec<ChatMessage> = Vec::new();
        conversation.push(ChatMessage::system(prompts::get_system_prompt()));

        while step < self.max_steps {
            step += 1;
            info!("Agent step {}/{}", step, self.max_steps);

            let browser_state = self.browser.get_state().await?;
            let formatted_state = browser_state.format_for_prompt();

            let step_prompt = format!(
                "Task: {}\n\nCurrent Browser State:\n{}\n\nWhat is your next action?",
                self.task, formatted_state
            );

            conversation.push(ChatMessage::user(step_prompt));

            let response = self
                .llm
                .invoke(&conversation)
                .await
                .context("Failed to get completion from LLM")?;

            debug!("LLM raw response: {}", response);
            conversation.push(ChatMessage::assistant(&response));

            let agent_output = parse_agent_output(&response)?;
            let mut action_results = Vec::new();
            let mut interacted_elements = Vec::new();
            let mut is_done = false;

            for action in &agent_output.action {
                match action {
                    Action::Navigate { url } => {
                        info!("Navigating to: {}", url);
                        match self.browser.navigate(url).await {
                            Ok(_) => action_results
                                .push(ActionResult::success(format!("Navigated to {url}"))),
                            Err(e) => action_results
                                .push(ActionResult::error(format!("Navigation failed: {e}"))),
                        }
                    }
                    Action::Click { index } => {
                        info!("Clicking element [{}]", index);
                        if let Some(elem) = self.browser.get_element(*index) {
                            interacted_elements.push(InteractedElementMetadata {
                                index: *index,
                                tag: elem.tag,
                                attributes: elem.attributes,
                            });
                        }
                        match self.browser.click(*index).await {
                            Ok(_) => action_results
                                .push(ActionResult::success(format!("Clicked element [{index}]"))),
                            Err(e) => action_results
                                .push(ActionResult::error(format!("Click failed: {e}"))),
                        }
                    }
                    Action::Input { index, text } => {
                        info!("Inputting into element [{}]: {}", index, text);
                        if let Some(elem) = self.browser.get_element(*index) {
                            interacted_elements.push(InteractedElementMetadata {
                                index: *index,
                                tag: elem.tag,
                                attributes: elem.attributes,
                            });
                        }
                        match self.browser.input(*index, text).await {
                            Ok(_) => action_results.push(ActionResult::success(format!(
                                "Inputted text into element [{index}]"
                            ))),
                            Err(e) => action_results
                                .push(ActionResult::error(format!("Input failed: {e}"))),
                        }
                    }
                    Action::Scroll { amount } => {
                        info!("Scrolling by {}", amount);
                        match self.browser.scroll(*amount).await {
                            Ok(_) => action_results
                                .push(ActionResult::success(format!("Scrolled by {amount}"))),
                            Err(e) => action_results
                                .push(ActionResult::error(format!("Scroll failed: {e}"))),
                        }
                    }
                    Action::Wait { seconds } => {
                        info!("Waiting for {} seconds", seconds);
                        let _ = self.browser.wait(*seconds as u64).await;
                        action_results
                            .push(ActionResult::success(format!("Waited {seconds} seconds")));
                    }
                    Action::Extract { query } => {
                        info!("Extracting query: {}", query);
                        let text = self.browser.get_state().await?.text;
                        action_results.push(ActionResult::success(format!(
                            "Extracted content for {query}: {text}"
                        )));
                    }
                    Action::Done { text, success } => {
                        info!("Task completed: {} (success: {})", text, success);
                        action_results.push(ActionResult::done(text.clone(), *success));
                        is_done = true;
                    }
                    Action::Custom(val) => {
                        warn!("Unhandled custom action: {:?}", val);
                        action_results.push(ActionResult::error("Custom action not implemented"));
                    }
                }
            }

            let history_item = AgentHistory {
                model_output: Some(agent_output),
                result: action_results,
                state: Some(AgentStepState {
                    url: browser_state.url,
                    title: browser_state.title,
                    interacted_element: if interacted_elements.is_empty() {
                        None
                    } else {
                        Some(interacted_elements)
                    },
                }),
            };

            self.history.history.push(history_item);

            if is_done {
                break;
            }
        }

        Ok(self.history.clone())
    }

    pub fn save_history(&self, file_path: impl AsRef<Path>) -> Result<()> {
        self.history.save_to_file(file_path)
    }

    pub fn detect_variables(&self) -> HashMap<String, DetectedVariable> {
        detect_variables_in_history(&self.history)
    }

    pub async fn rerun_history(&mut self, history: &AgentHistoryList) -> Result<Vec<ActionResult>> {
        self.browser.start().await?;
        let mut results = Vec::new();

        for (i, step) in history.history.iter().enumerate() {
            let output = match &step.model_output {
                Some(out) => out,
                None => continue,
            };

            info!("Replaying step {}/{}", i + 1, history.history.len());
            for action in &output.action {
                match action {
                    Action::Navigate { url } => {
                        info!("Replay: Navigating to {}", url);
                        match self.browser.navigate(url).await {
                            Ok(_) => {
                                results.push(ActionResult::success(format!("Navigated to {url}")))
                            }
                            Err(e) => {
                                results.push(ActionResult::error(format!("Navigation failed: {e}")))
                            }
                        }
                    }
                    Action::Click { index } => {
                        info!("Replay: Clicking element [{}]", index);
                        match self.browser.click(*index).await {
                            Ok(_) => {
                                results.push(ActionResult::success(format!("Clicked [{index}]")))
                            }
                            Err(e) => {
                                results.push(ActionResult::error(format!("Click failed: {e}")))
                            }
                        }
                    }
                    Action::Input { index, text } => {
                        info!("Replay: Inputting [{}] -> {}", index, text);
                        match self.browser.input(*index, text).await {
                            Ok(_) => results.push(ActionResult::success(format!(
                                "Inputted text into [{index}]"
                            ))),
                            Err(e) => {
                                results.push(ActionResult::error(format!("Input failed: {e}")))
                            }
                        }
                    }
                    Action::Scroll { amount } => {
                        let _ = self.browser.scroll(*amount).await;
                        results.push(ActionResult::success(format!("Scrolled {amount}")));
                    }
                    Action::Wait { seconds } => {
                        let _ = self.browser.wait(*seconds as u64).await;
                        results.push(ActionResult::success(format!("Waited {seconds}s")));
                    }
                    Action::Extract { query } => {
                        let text = self.browser.get_state().await?.text;
                        results.push(ActionResult::success(format!(
                            "Extracted for {query}: {text}"
                        )));
                    }
                    Action::Done { text, success } => {
                        info!("Replay done: {}", text);
                        results.push(ActionResult::done(text.clone(), *success));
                    }
                    Action::Custom(_) => {}
                }
            }
        }

        Ok(results)
    }

    pub async fn load_and_rerun(
        &mut self,
        file_path: impl AsRef<Path>,
        variables: Option<HashMap<String, String>>,
    ) -> Result<Vec<ActionResult>> {
        let mut history = AgentHistoryList::load_from_file(file_path)?;
        if let Some(ref vars) = variables {
            history = substitute_variables_in_history(&history, vars);
        }
        self.rerun_history(&history).await
    }
}

pub fn parse_agent_output(raw_text: &str) -> Result<AgentOutput> {
    let clean_json = extract_json_str(raw_text)?;
    let val: serde_json::Value = serde_json::from_str(&clean_json)
        .with_context(|| format!("Failed to parse JSON from:\n{clean_json}"))?;

    let thought = val
        .get("thought")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());
    let memory = val
        .get("memory")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());
    let next_goal = val
        .get("next_goal")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());

    let mut actions = Vec::new();

    if let Some(action_val) = val.get("action") {
        if let Some(arr) = action_val.as_array() {
            for item in arr {
                actions.push(serde_json::from_value::<Action>(item.clone())?);
            }
        } else {
            actions.push(serde_json::from_value::<Action>(action_val.clone())?);
        }
    } else {
        // Fallback: check if the root object has action keys directly
        for key in [
            "navigate", "click", "input", "scroll", "wait", "done", "extract",
        ] {
            if let Some(sub) = val.get(key) {
                let single_action_obj = serde_json::json!({ key: sub });
                if let Ok(act) = serde_json::from_value::<Action>(single_action_obj) {
                    actions.push(act);
                    break;
                }
            }
        }
    }

    if actions.is_empty() {
        return Err(anyhow!("No valid actions found in LLM output:\n{raw_text}"));
    }

    Ok(AgentOutput {
        thought,
        memory,
        next_goal,
        action: actions,
    })
}

fn extract_json_str(raw: &str) -> Result<String> {
    let trimmed = raw.trim();

    // Check for markdown code fences
    if let Some(start_idx) = trimmed.find("```json") {
        let after_start = &trimmed[start_idx + 7..];
        if let Some(end_idx) = after_start.find("```") {
            return Ok(after_start[..end_idx].trim().to_string());
        }
    } else if let Some(start_idx) = trimmed.find("```") {
        let after_start = &trimmed[start_idx + 3..];
        if let Some(end_idx) = after_start.find("```") {
            return Ok(after_start[..end_idx].trim().to_string());
        }
    }

    // Try finding outer curly braces
    if let (Some(first_brace), Some(last_brace)) = (trimmed.find('{'), trimmed.rfind('}')) && first_brace < last_brace {
        return Ok(trimmed[first_brace..=last_brace].to_string());
    }

    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_agent_output_standard() {
        let raw = r#"{
            "thought": "I need to navigate to github",
            "action": {
                "navigate": {
                    "url": "https://github.com/browser-use/browser-use"
                }
            }
        }"#;

        let output = parse_agent_output(raw).unwrap();
        assert_eq!(
            output.thought.as_deref(),
            Some("I need to navigate to github")
        );
        assert_eq!(output.action.len(), 1);
        match &output.action[0] {
            Action::Navigate { url } => {
                assert_eq!(url, "https://github.com/browser-use/browser-use")
            }
            _ => panic!("Expected Navigate action"),
        }
    }

    #[test]
    fn test_parse_agent_output_done() {
        let raw = r#"```json
        {
            "thought": "Found stars",
            "action": {
                "done": {
                    "text": "55.4k stars",
                    "success": true
                }
            }
        }
        ```"#;

        let output = parse_agent_output(raw).unwrap();
        assert_eq!(output.action.len(), 1);
        match &output.action[0] {
            Action::Done { text, success } => {
                assert_eq!(text, "55.4k stars");
                assert!(success);
            }
            _ => panic!("Expected Done action"),
        }
    }
}
