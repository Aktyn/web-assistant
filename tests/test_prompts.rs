use web_assistant::agent::prompts::{PromptMode, SystemPrompt};

#[test]
fn test_all_system_prompt_modes_non_empty() {
    let modes = [
        PromptMode::Standard,
        PromptMode::NoThinking,
        PromptMode::Flash,
        PromptMode::BrowserUseModel,
        PromptMode::BrowserUseNoThinking,
        PromptMode::BrowserUseFlash,
    ];

    for mode in modes {
        let sp = SystemPrompt::new(mode);
        let content = sp.get_system_message();
        assert!(!content.is_empty(), "Mode {:?} should not be empty", mode);
        assert!(content.contains("JSON"));
    }
}

#[test]
fn test_system_prompt_builder_options() {
    let prompt = SystemPrompt::builder()
        .mode(PromptMode::Flash)
        .max_actions_per_step(7)
        .extend_system_message("EXTENDED_INSTRUCTIONS_FOR_TEST")
        .build();

    let msg = prompt.get_system_message();
    assert!(msg.contains("Max 7 actions"));
    assert!(msg.contains("EXTENDED_INSTRUCTIONS_FOR_TEST"));
}

#[test]
fn test_system_prompt_override() {
    let prompt = SystemPrompt::builder()
        .override_system_message("CUSTOM_OVERRIDE_PROMPT")
        .build();

    assert_eq!(prompt.get_system_message(), "CUSTOM_OVERRIDE_PROMPT");
}
