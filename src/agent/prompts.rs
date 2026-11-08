#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PromptMode {
    #[default]
    Standard,
    NoThinking,
    Flash,
    BrowserUseModel,
    BrowserUseNoThinking,
    BrowserUseFlash,
}

#[derive(Debug, Clone)]
pub struct SystemPrompt {
    pub mode: PromptMode,
    pub max_actions_per_step: usize,
    pub override_system_message: Option<String>,
    pub extend_system_message: Option<String>,
}

impl Default for SystemPrompt {
    fn default() -> Self {
        Self {
            mode: PromptMode::Standard,
            max_actions_per_step: 3,
            override_system_message: None,
            extend_system_message: None,
        }
    }
}

impl SystemPrompt {
    pub fn new(mode: PromptMode) -> Self {
        Self {
            mode,
            max_actions_per_step: 3,
            override_system_message: None,
            extend_system_message: None,
        }
    }

    pub fn builder() -> SystemPromptBuilder {
        SystemPromptBuilder::default()
    }

    pub fn get_system_message(&self) -> String {
        if let Some(ref override_msg) = self.override_system_message {
            return override_msg.clone();
        }

        let mut base = match self.mode {
            PromptMode::Standard => self.prompt_standard(),
            PromptMode::NoThinking => self.prompt_no_thinking(),
            PromptMode::Flash => self.prompt_flash(),
            PromptMode::BrowserUseModel => self.prompt_browser_use(),
            PromptMode::BrowserUseNoThinking => self.prompt_browser_use_no_thinking(),
            PromptMode::BrowserUseFlash => self.prompt_browser_use_flash(),
        };

        if let Some(ref ext) = self.extend_system_message {
            base.push_str("\n\n");
            base.push_str(ext);
        }

        base
    }

    fn prompt_standard(&self) -> String {
        format!(
            r#"You are an AI agent designed to operate in an iterative loop to automate browser tasks. Your ultimate goal is accomplishing the task provided in <user_request>.

<intro>
You excel at:
1. Navigating complex websites and extracting precise information
2. Automating form submissions and interactive web actions
3. Gathering and saving information
4. Operating effectively in an agent loop
</intro>

<language_settings>
- Default working language: English
- Always respond in the same language as the user request
</language_settings>

<browser_state>
Interactive elements are indexed as [index]<tag attr="val">text</tag>.
- Only elements with numeric indices in [] are interactive.
- Indentation indicates child elements.
</browser_state>

<browser_rules>
- Only interact with elements that have a numeric [index] assigned.
- If the page has not fully loaded, use the wait action.
- When you find the requested information, call "done" immediately with "success": true.
- Before calling "done" with success=true: verify all criteria are met and no information is fabricated.
- Maximum actions per step: {max_actions}
</browser_rules>

<response_format>
You must respond with a valid JSON in this exact structure:
{{
  "thought": "Detailed reasoning about current state and strategy",
  "memory": "Key facts or progress to remember for future steps",
  "next_goal": "Immediate next goal in one sentence",
  "action": [
    {{ "navigate": {{ "url": "https://..." }} }}
  ]
}}
Available action types:
- {{"navigate": {{"url": "https://..."}}}}
- {{"click": {{"index": 1}}}}
- {{"input": {{"index": 2, "text": "value"}}}}
- {{"scroll": {{"amount": 500}}}}
- {{"wait": {{"seconds": 3}}}}
- {{"extract": {{"query": "what to extract"}}}}
- {{"done": {{"text": "Final answer or summary", "success": true}}}}
</response_format>
"#,
            max_actions = self.max_actions_per_step
        )
    }

    fn prompt_no_thinking(&self) -> String {
        format!(
            r#"You are an AI agent designed to automate browser tasks without extended chain-of-thought.
Ultimate goal: accomplish the user request.

<browser_state>
Elements: [index]<tag attr="val">text</tag>. Only elements with numeric [index] can be clicked or filled.
</browser_state>

<rules>
- Max actions per step: {max_actions}
- Grounding: Only report data observed in page state. Do NOT fabricate information.
- Call "done" as soon as task is accomplished.
</rules>

<response_format>
Respond ONLY with JSON:
{{
  "action": [
    {{ "action_name": {{ ...params... }} }}
  ]
}}
</response_format>
"#,
            max_actions = self.max_actions_per_step
        )
    }

    fn prompt_flash(&self) -> String {
        format!(
            r#"You are an AI agent operating in flash mode for fast browser automation.
<user_request>Accomplish task accurately and concisely.</user_request>
<browser_state>Interactive elements: [index]<type>text</type>.</browser_state>
<action_rules>Max {max_actions} actions per step. Call done immediately when goal is reached.</action_rules>
<output>Respond with JSON:
{{
  "memory": "Brief status and next action",
  "action": [{{ "action_name": {{ ...params... }} }}]
}}
</output>
"#,
            max_actions = self.max_actions_per_step
        )
    }

    fn prompt_browser_use(&self) -> String {
        format!(
            r#"You are a browser-use agent operating in thinking mode. You automate browser tasks by outputting structured JSON actions.

<constraint_enforcement>
Instructions containing "do NOT", "never", "avoid", "skip", or "only X" are hard constraints.
</constraint_enforcement>

<rules>
- Max {max_actions} actions per step.
- Verify actions against page state.
- Call done with success=true when finished.
</rules>

<output>
Respond with JSON:
{{
  "thinking": "Structured reasoning analyzing current state and next actions",
  "evaluation_previous_goal": "Concise analysis of last action result",
  "memory": "Key memory tracking progress",
  "next_goal": "Next immediate goal in one clear sentence",
  "action": [{{ "action_name": {{ ...params... }} }}]
}}
</output>
"#,
            max_actions = self.max_actions_per_step
        )
    }

    fn prompt_browser_use_no_thinking(&self) -> String {
        format!(
            r#"You are a browser-use agent. Max {max_actions} actions per step.
Respond with JSON:
{{
  "next_goal": "Next immediate goal",
  "action": [{{ "action_name": {{ ...params... }} }}]
}}
"#,
            max_actions = self.max_actions_per_step
        )
    }

    fn prompt_browser_use_flash(&self) -> String {
        format!(
            r#"Browser-use flash mode. Max {max_actions} actions per step.
Respond with JSON:
{{
  "action": [{{ "action_name": {{ ...params... }} }}]
}}
"#,
            max_actions = self.max_actions_per_step
        )
    }
}

#[derive(Default)]
pub struct SystemPromptBuilder {
    mode: PromptMode,
    max_actions_per_step: Option<usize>,
    override_system_message: Option<String>,
    extend_system_message: Option<String>,
}

impl SystemPromptBuilder {
    pub fn mode(mut self, mode: PromptMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn max_actions_per_step(mut self, max: usize) -> Self {
        self.max_actions_per_step = Some(max);
        self
    }

    pub fn override_system_message(mut self, msg: impl Into<String>) -> Self {
        self.override_system_message = Some(msg.into());
        self
    }

    pub fn extend_system_message(mut self, msg: impl Into<String>) -> Self {
        self.extend_system_message = Some(msg.into());
        self
    }

    pub fn build(self) -> SystemPrompt {
        SystemPrompt {
            mode: self.mode,
            max_actions_per_step: self.max_actions_per_step.unwrap_or(3),
            override_system_message: self.override_system_message,
            extend_system_message: self.extend_system_message,
        }
    }
}

pub fn get_system_prompt() -> String {
    SystemPrompt::default().get_system_message()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_prompt_modes() {
        let modes = [
            PromptMode::Standard,
            PromptMode::NoThinking,
            PromptMode::Flash,
            PromptMode::BrowserUseModel,
            PromptMode::BrowserUseNoThinking,
            PromptMode::BrowserUseFlash,
        ];

        for mode in modes {
            let prompt = SystemPrompt::new(mode);
            let msg = prompt.get_system_message();
            assert!(!msg.is_empty(), "Mode {:?} produced empty message", mode);
            assert!(msg.contains("JSON"), "Mode {:?} should mention JSON", mode);
        }
    }

    #[test]
    fn test_prompt_customization() {
        let prompt = SystemPrompt::builder()
            .mode(PromptMode::Flash)
            .max_actions_per_step(5)
            .extend_system_message("SPECIAL INSTRUCTION")
            .build();

        let msg = prompt.get_system_message();
        assert!(msg.contains("Max 5 actions"));
        assert!(msg.contains("SPECIAL INSTRUCTION"));
    }

    #[test]
    fn test_prompt_override() {
        let prompt = SystemPrompt::builder()
            .override_system_message("Completely custom prompt")
            .build();

        let msg = prompt.get_system_message();
        assert_eq!(msg, "Completely custom prompt");
    }
}
