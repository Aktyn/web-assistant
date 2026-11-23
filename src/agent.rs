pub mod judge;
pub mod prompts;
mod variable_detector;
pub mod views;

use crate::llm::ChatMessage;
use crate::{browser::Browser, llm};
use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, warn};

pub use prompts::{PromptMode, SystemPrompt};
pub use variable_detector::{detect_variables_in_history, substitute_variables_in_history};
use views::{
    Action, ActionResult, AgentHistoryItem, AgentHistoryList, AgentOutput, AgentStepState,
    InteractedElementMetadata,
};

pub struct Agent {
    pub task: String,
    pub system_prompt: SystemPrompt,
    pub max_steps: u16,
}

impl Agent {
    pub fn new(task: impl Into<String>) -> Self {
        Self {
            task: task.into(),
            system_prompt: SystemPrompt::default(),
            max_steps: 20,
        }
    }

    #[must_use]
    pub fn max_steps(mut self, steps: u16) -> Self {
        self.max_steps = steps;
        self
    }

    #[must_use]
    pub fn prompt_mode(mut self, mode: PromptMode) -> Self {
        self.system_prompt.mode = mode;
        self
    }

    #[must_use]
    pub fn system_prompt(mut self, prompt: SystemPrompt) -> Self {
        self.system_prompt = prompt;
        self
    }

    #[allow(clippy::too_many_lines)]
    pub async fn run<LlmType: llm::LLM>(
        &mut self,
        browser: &mut Browser,
        llm: &LlmType,
    ) -> Result<AgentHistoryList> {
        info!("Starting agent task: {}", self.task);
        browser.start().await?;

        let mut step = 0;
        let mut conversation: Vec<ChatMessage> = Vec::new();
        conversation.push(ChatMessage::system(self.system_prompt.get_system_message()));

        let mut history = AgentHistoryList::new();

        while step < self.max_steps {
            step += 1;
            info!("Agent step {}/{}", step, self.max_steps);

            let browser_state = browser.get_state().await?;
            let formatted_state = browser_state.format_for_prompt();

            let step_prompt = format!(
                "Task: {}\n\nCurrent Browser State:\n{}\n\nWhat is your next action?",
                self.task, formatted_state
            );

            conversation.push(ChatMessage::user(step_prompt));

            let response = llm
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
                        match browser.navigate(url).await {
                            Ok(()) => action_results
                                .push(ActionResult::success(format!("Navigated to {url}"))),
                            Err(e) => action_results
                                .push(ActionResult::error(format!("Navigation failed: {e}"))),
                        }
                    }
                    Action::Click { index } => {
                        info!("Clicking element [{}]", index);
                        if let Some(elem) = browser.get_element(*index) {
                            interacted_elements.push(InteractedElementMetadata {
                                index: *index,
                                tag: elem.tag,
                                attributes: elem.attributes,
                            });
                        }
                        match browser.click(*index).await {
                            Ok(()) => action_results
                                .push(ActionResult::success(format!("Clicked element [{index}]"))),
                            Err(e) => action_results
                                .push(ActionResult::error(format!("Click failed: {e}"))),
                        }
                    }
                    Action::Input { index, text } => {
                        info!("Inputting into element [{}]: {}", index, text);
                        if let Some(elem) = browser.get_element(*index) {
                            interacted_elements.push(InteractedElementMetadata {
                                index: *index,
                                tag: elem.tag,
                                attributes: elem.attributes,
                            });
                        }
                        match browser.input(*index, text).await {
                            Ok(()) => action_results.push(ActionResult::success(format!(
                                "Inputted text into element [{index}]"
                            ))),
                            Err(e) => action_results
                                .push(ActionResult::error(format!("Input failed: {e}"))),
                        }
                    }
                    Action::Scroll { amount } => {
                        info!("Scrolling by {}", amount);
                        match browser.scroll(*amount).await {
                            Ok(()) => action_results
                                .push(ActionResult::success(format!("Scrolled by {amount}"))),
                            Err(e) => action_results
                                .push(ActionResult::error(format!("Scroll failed: {e}"))),
                        }
                    }
                    Action::Wait { seconds } => {
                        info!("Waiting for {} seconds", seconds);
                        let _ = browser.wait(u64::from(*seconds)).await;
                        action_results
                            .push(ActionResult::success(format!("Waited {seconds} seconds")));
                    }
                    Action::Extract { query } => {
                        info!("Extracting query: {}", query);
                        let text = browser.get_state().await?.text;
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

            history.items.push(AgentHistoryItem {
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
            });

            if is_done {
                break;
            }
        }

        Ok(history.clone())
    }

    //TODO: adjust this implementation
    // pub async fn judge(&self, _ground_truth: Option<&str>) -> Result<JudgementResult> {
    //     let judge = Judge::new(self.llm.clone());
    //     let final_res = self.history.final_result().unwrap_or_default();
    //     judge
    //         .judge(&self.task, &final_res, &self.history, ground_truth)
    //         .await
    // }

    #[allow(unused)] //TODO:
    pub async fn rerun_history(
        &mut self,
        history: &AgentHistoryList,
        browser: &mut Browser,
    ) -> Result<Vec<ActionResult>> {
        browser.start().await?;
        let mut results = Vec::new();

        for (i, step) in history.items.iter().enumerate() {
            let Some(output) = &step.model_output else {
                continue;
            };

            info!("Replaying step {}/{}", i + 1, history.items.len());
            for action in &output.action {
                match action {
                    Action::Navigate { url } => {
                        info!("Replay: Navigating to {}", url);
                        match browser.navigate(url).await {
                            Ok(()) => {
                                results.push(ActionResult::success(format!("Navigated to {url}")));
                            }
                            Err(e) => {
                                results
                                    .push(ActionResult::error(format!("Navigation failed: {e}")));
                            }
                        }
                    }
                    Action::Click { index } => {
                        info!("Replay: Clicking element [{}]", index);
                        match browser.click(*index).await {
                            Ok(()) => {
                                results.push(ActionResult::success(format!("Clicked [{index}]")));
                            }
                            Err(e) => {
                                results.push(ActionResult::error(format!("Click failed: {e}")));
                            }
                        }
                    }
                    Action::Input { index, text } => {
                        info!("Replay: Inputting [{}] -> {}", index, text);
                        match browser.input(*index, text).await {
                            Ok(()) => results.push(ActionResult::success(format!(
                                "Inputted text into [{index}]"
                            ))),
                            Err(e) => {
                                results.push(ActionResult::error(format!("Input failed: {e}")));
                            }
                        }
                    }
                    Action::Scroll { amount } => {
                        let _ = browser.scroll(*amount).await;
                        results.push(ActionResult::success(format!("Scrolled {amount}")));
                    }
                    Action::Wait { seconds } => {
                        let _ = browser.wait(u64::from(*seconds)).await;
                        results.push(ActionResult::success(format!("Waited {seconds}s")));
                    }
                    Action::Extract { query } => {
                        let text = browser.get_state().await?.text;
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

    #[allow(unused)] //TODO:
    pub async fn load_and_rerun(
        &mut self,
        file_path: impl AsRef<Path>,
        variables: Option<HashMap<String, String>>,
        browser: &mut Browser,
    ) -> Result<Vec<ActionResult>> {
        let mut history = AgentHistoryList::load_from_file(file_path).await?;
        if let Some(ref vars) = variables {
            history = substitute_variables_in_history(&history, vars);
        }
        self.rerun_history(&history, browser).await
    }
}

pub fn parse_agent_output(raw_text: &str) -> Result<AgentOutput> {
    let clean_json = extract_json_str(raw_text);
    let val: serde_json::Value = serde_json::from_str(clean_json)
        .with_context(|| format!("Failed to parse JSON from:\n{clean_json}"))?;

    let thought = val
        .get("thought")
        .and_then(|t| t.as_str())
        .map(std::string::ToString::to_string);
    let memory = val
        .get("memory")
        .and_then(|t| t.as_str())
        .map(std::string::ToString::to_string);
    let next_goal = val
        .get("next_goal")
        .and_then(|t| t.as_str())
        .map(std::string::ToString::to_string);

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

fn extract_json_str(raw: &str) -> &str {
    let trimmed = raw.trim();

    if let Some(start_idx) = trimmed.find("```json") {
        let after_start = &trimmed[start_idx + 7..];
        if let Some(end_idx) = after_start.find("```") {
            return after_start[..end_idx].trim();
        }
    } else if let Some(start_idx) = trimmed.find("```") {
        let after_start = &trimmed[start_idx + 3..];
        if let Some(end_idx) = after_start.find("```") {
            return after_start[..end_idx].trim();
        }
    }

    if let (Some(first_brace), Some(last_brace)) = (trimmed.find('{'), trimmed.rfind('}'))
        && first_brace < last_brace
    {
        return &trimmed[first_brace..=last_brace];
    }

    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::openai::{ChatOpenAI, ChatOpenAIConfig};

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
                assert_eq!(url, "https://github.com/browser-use/browser-use");
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

    #[test]
    fn test_parse_agent_output_multiple_actions() {
        let raw = r#"{
            "action": [
                {"click": {"index": 1}},
                {"input": {"index": 2, "text": "hello"}}
            ]
        }"#;

        let output = parse_agent_output(raw).unwrap();
        assert_eq!(output.action.len(), 2);
        match &output.action[0] {
            Action::Click { index } => assert_eq!(*index, 1),
            _ => panic!("Expected Click action"),
        }
        match &output.action[1] {
            Action::Input { index, text } => {
                assert_eq!(*index, 2);
                assert_eq!(text, "hello");
            }
            _ => panic!("Expected Input action"),
        }
    }

    #[test]
    fn test_parse_agent_output_single_action_object() {
        let raw = r#"{
            "action": {"scroll": {"amount": 500}}
        }"#;

        let output = parse_agent_output(raw).unwrap();
        assert_eq!(output.action.len(), 1);
        match &output.action[0] {
            Action::Scroll { amount } => assert_eq!(*amount, 500),
            _ => panic!("Expected Scroll action"),
        }
    }

    #[test]
    fn test_parse_agent_output_wait_action() {
        let raw = r#"{
            "action": [{"wait": {"seconds": 5}}]
        }"#;

        let output = parse_agent_output(raw).unwrap();
        match &output.action[0] {
            Action::Wait { seconds } => assert_eq!(*seconds, 5),
            _ => panic!("Expected Wait action"),
        }
    }

    #[test]
    fn test_parse_agent_output_extract_action() {
        let raw = r#"{
            "action": [{"extract": {"query": "price"}}]
        }"#;

        let output = parse_agent_output(raw).unwrap();
        match &output.action[0] {
            Action::Extract { query } => assert_eq!(query, "price"),
            _ => panic!("Expected Extract action"),
        }
    }

    #[test]
    fn test_parse_agent_output_custom_action() {
        let raw = r#"{
            "action": {"custom": {"key": "value"}}
        }"#;

        let output = parse_agent_output(raw).unwrap();
        match &output.action[0] {
            Action::Custom(val) => {
                // Custom actions are stored as-is, the key should be there
                assert_eq!(val["custom"]["key"], "value");
            }
            _ => panic!("Expected Custom action"),
        }
    }

    #[test]
    fn test_extract_json_str_plain_json() {
        let raw = r#"{"key": "value"}"#;
        let result = extract_json_str(raw);
        assert_eq!(result, "{\"key\": \"value\"}");
    }

    #[test]
    fn test_extract_json_str_with_codeblock() {
        let raw = "```json\n{\"key\": \"value\"}\n```";
        let result = extract_json_str(raw);
        assert_eq!(result, "{\"key\": \"value\"}");
    }

    #[test]
    fn test_extract_json_str_with_text_before_json() {
        let raw = "Some text\n{\"key\": \"value\"}";
        let result = extract_json_str(raw);
        assert_eq!(result, "{\"key\": \"value\"}");
    }

    #[test]
    fn test_extract_json_str_returns_trimmed() {
        let raw = "  \n  {\"key\": \"value\"}  \n  ";
        let result = extract_json_str(raw);
        assert_eq!(result, "{\"key\": \"value\"}");
    }

    #[test]
    fn test_chat_message_system() {
        let msg = ChatMessage::system("You are helpful");
        assert_eq!(msg.role, "system");
        assert_eq!(msg.content, "You are helpful");
    }

    #[test]
    fn test_chat_message_user() {
        let msg = ChatMessage::user("Hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_chat_message_assistant() {
        let msg = ChatMessage::assistant("Hi there");
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "Hi there");
    }

    #[test]
    fn test_chat_openai_new_with_explicit_model() {
        let config = crate::llm::ChatOpenAIConfig {
            model: "custom-model".into(),
            api_key: String::new(),
            base_url: "http://localhost:8080".into(),
            temperature: None,
            max_completion_tokens: None,
        };
        let llm = ChatOpenAI::new(config);
        // Access via the config field
        assert_eq!(llm.config.model, "custom-model");
    }

    #[test]
    fn test_chat_openai_default_config_uses_env_fallback() {
        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
            std::env::remove_var("OPENAI_BASE_URL");
        }
        let llm = ChatOpenAI::new(ChatOpenAIConfig::default());
        assert_eq!(llm.config.model, "auto");
        assert_eq!(llm.config.api_key, "");
        assert_eq!(llm.config.base_url, "https://api.openai.com/v1");
        assert_eq!(llm.config.temperature, Some(0.2));
        assert_eq!(llm.config.max_completion_tokens, Some(4096));
    }

    #[test]
    fn test_parse_agent_output_fallback_single_key() {
        // When action is not an array, try individual keys
        let raw = r#"{
            "click": {"index": 3}
        }"#;

        let output = parse_agent_output(raw).unwrap();
        assert_eq!(output.action.len(), 1);
        match &output.action[0] {
            Action::Click { index } => assert_eq!(*index, 3),
            _ => panic!("Expected Click action from fallback"),
        }
    }

    #[test]
    fn test_parse_agent_output_invalid_json() {
        let result = parse_agent_output("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_agent_output_no_actions() {
        let raw = r#"{"thought": "no actions here"}"#;
        let result = parse_agent_output(raw);
        assert!(result.is_err());
    }

    // =========================================================================
    // Agent struct construction tests
    // =========================================================================

    #[test]
    fn test_agent_new_construction() {
        let agent = Agent::new("Test task");
        assert_eq!(agent.task, "Test task");
        assert_eq!(agent.max_steps, 20);
        assert_eq!(agent.system_prompt.mode, PromptMode::Standard);
    }

    #[test]
    fn test_agent_max_steps_fluent_builder() {
        let agent = Agent::new("Task").max_steps(5);
        assert_eq!(agent.max_steps, 5);
    }

    #[test]
    fn test_agent_prompt_mode_builder() {
        let agent = Agent::new("Task").prompt_mode(PromptMode::Flash);
        assert_eq!(agent.system_prompt.mode, PromptMode::Flash);
    }

    #[test]
    fn test_agent_builder_pattern() {
        let agent = Agent::new("Complex task")
            .max_steps(10)
            .prompt_mode(PromptMode::Flash);

        assert_eq!(agent.max_steps, 10);
        assert_eq!(agent.system_prompt.mode, PromptMode::Flash);
    }

    #[test]
    fn test_agent_system_prompt_builder() {
        let custom_prompt = SystemPrompt::builder().mode(PromptMode::NoThinking).build();
        let agent = Agent::new("Task").system_prompt(custom_prompt);
        assert_eq!(agent.system_prompt.mode, PromptMode::NoThinking);
    }

    #[test]
    fn test_agent_default_history_is_empty() {
        let history = AgentHistoryList::new();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
    }

    // =========================================================================
    // extract_json_str edge cases
    // =========================================================================

    #[test]
    fn test_extract_json_str_without_braces_returns_trimmed() {
        let raw = "just some text";
        let result = extract_json_str(raw);
        assert_eq!(result, "just some text");
    }

    #[test]
    fn test_extract_json_str_nested_codeblocks() {
        let raw = "```json\n{\"a\": 1}\n``` more text";
        let result = extract_json_str(raw);
        assert_eq!(result, "{\"a\": 1}");
    }

    #[test]
    fn test_extract_json_str_plain_backtick_block() {
        let raw = "```\n{\"a\": 2}\n```";
        let result = extract_json_str(raw);
        assert_eq!(result, "{\"a\": 2}");
    }

    #[test]
    fn test_extract_json_str_multiple_braces_picks_outermost() {
        let raw = "prefix { inner } middle { \"key\": \"value\" } suffix";
        // First brace at position 8, last at position 43 → returns everything between them
        let result = extract_json_str(raw);
        assert!(result.contains("inner"));
        assert!(result.contains("\"key\": \"value\""));
    }

    // =========================================================================
    // Action parsing completeness
    // =========================================================================

    #[test]
    fn test_parse_all_action_types() {
        let all_actions_raw = r#"{
            "action": [
                {"navigate": {"url": "https://example.com"}},
                {"click": {"index": 1}},
                {"input": {"index": 2, "text": "hello"}},
                {"scroll": {"amount": 100}},
                {"wait": {"seconds": 2}},
                {"extract": {"query": "price"}},
                {"done": {"text": "finished", "success": true}}
            ]
        }"#;

        let output = parse_agent_output(all_actions_raw).unwrap();
        assert_eq!(output.action.len(), 7);

        match &output.action[0] {
            Action::Navigate { url } => assert_eq!(url, "https://example.com"),
            _ => panic!("Expected Navigate"),
        }
        match &output.action[1] {
            Action::Click { index } => assert_eq!(*index, 1),
            _ => panic!("Expected Click"),
        }
        match &output.action[2] {
            Action::Input { index, text } => {
                assert_eq!(*index, 2);
                assert_eq!(text, "hello");
            }
            _ => panic!("Expected Input"),
        }
        match &output.action[3] {
            Action::Scroll { amount } => assert_eq!(*amount, 100),
            _ => panic!("Expected Scroll"),
        }
        match &output.action[4] {
            Action::Wait { seconds } => assert_eq!(*seconds, 2),
            _ => panic!("Expected Wait"),
        }
        match &output.action[5] {
            Action::Extract { query } => assert_eq!(query, "price"),
            _ => panic!("Expected Extract"),
        }
        match &output.action[6] {
            Action::Done { text, success } => {
                assert_eq!(text, "finished");
                assert!(*success);
            }
            _ => panic!("Expected Done"),
        }
    }

    #[test]
    fn test_parse_output_with_thought_and_memory() {
        let raw = r#"{
            "thought": "Navigating first",
            "memory": "User is logged in",
            "next_goal": "Fill form",
            "action": [{"navigate": {"url": "https://form.example.com"}}]
        }"#;

        let output = parse_agent_output(raw).unwrap();
        assert_eq!(output.thought.as_deref(), Some("Navigating first"));
        assert_eq!(output.memory.as_deref(), Some("User is logged in"));
        assert_eq!(output.next_goal.as_deref(), Some("Fill form"));
    }

    #[test]
    fn test_single_action_as_object_parsed_correctly() {
        // When action is a single object (not array)
        let raw = r#"{
            "action": {"click": {"index": 5}}
        }"#;

        let output = parse_agent_output(raw).unwrap();
        assert_eq!(output.action.len(), 1);
        match &output.action[0] {
            Action::Click { index } => assert_eq!(*index, 5),
            _ => panic!("Expected Click"),
        }
    }

    // =========================================================================
    // Agent lifecycle and history management tests
    // =========================================================================

    #[tokio::test]
    async fn test_save_history_to_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("history.json");

        // Build some history manually
        let history = AgentHistoryList {
            items: vec![AgentHistoryItem {
                model_output: Some(AgentOutput {
                    thought: Some("Start".into()),
                    memory: None,
                    next_goal: None,
                    action: vec![Action::Navigate {
                        url: "https://example.com".into(),
                    }],
                }),
                result: vec![ActionResult::success("Navigated")],
                state: Some(AgentStepState {
                    url: "https://example.com".into(),
                    title: "Example".into(),
                    interacted_element: None,
                }),
            }],
        };

        let result = history.save_to_file(&file_path).await;
        assert!(result.is_ok());
        assert!(file_path.exists());

        // Verify we can load it back
        let loaded = AgentHistoryList::load_from_file(&file_path).await.unwrap();
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn test_detect_variables_empty_history() {
        let history = AgentHistoryList::new();
        let vars = history.detect_variables();
        assert!(vars.is_empty());
    }

    #[test]
    fn test_agent_default_system_prompt_standard() {
        let agent = Agent::new("Task");
        assert_eq!(agent.system_prompt.mode, PromptMode::Standard);
        assert_eq!(agent.system_prompt.max_actions_per_step, 3);
    }

    #[test]
    fn test_parse_agent_output_done_with_success_false() {
        let raw = r#"{
            "action": [{"done": {"text": "failed", "success": false}}]
        }"#;
        let output = parse_agent_output(raw).unwrap();
        match &output.action[0] {
            Action::Done { text, success } => {
                assert_eq!(text, "failed");
                assert!(!success);
            }
            _ => panic!("Expected Done"),
        }
    }

    #[test]
    fn test_extract_json_str_with_only_backticks_no_content() {
        let raw = "```\n```";
        let result = extract_json_str(raw);
        assert_eq!(result, "");
    }

    #[test]
    fn test_extract_json_str_first_brace_before_last_returns_inner() {
        let raw = "{a} middle {b}";
        let result = extract_json_str(raw);
        // Should find first '{' at 0 and last '}' at end → whole string
        assert!(result.contains("{a}"));
        assert!(result.contains("{b}"));
    }

    #[test]
    fn test_extract_json_str_nested_braces() {
        let raw = "prefix { outer { inner } middle } suffix";
        let result = extract_json_str(raw);
        assert!(result.contains("outer"));
        assert!(result.contains("inner"));
    }

    #[test]
    fn test_extract_json_str_malformed_json_still_finds_braces() {
        let raw = "bad {key: value} stuff";
        let result = extract_json_str(raw);
        assert!(result.contains("key"));
        assert!(result.contains("value"));
    }

    #[test]
    fn test_agent_builder_chained_fluent() {
        let agent = Agent::new("Final task")
            .max_steps(42)
            .prompt_mode(PromptMode::NoThinking);
        assert_eq!(agent.task, "Final task");
        assert_eq!(agent.max_steps, 42);
        assert_eq!(agent.system_prompt.mode, PromptMode::NoThinking);
    }
}
