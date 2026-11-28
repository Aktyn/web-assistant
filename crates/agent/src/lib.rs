pub(crate) mod agent;
pub(crate) mod judge;
pub(crate) mod prompts;
pub(crate) mod variable_detector;
pub(crate) mod views;

pub use agent::Agent;
pub use judge::{construct_judge_messages, parse_judgement_response};
pub use prompts::{PromptMode, SystemPrompt};
pub use variable_detector::substitute_variables_in_history;
pub use views::{
    Action, ActionResult, AgentHistoryItem, AgentHistoryList, AgentOutput, AgentStepState,
    DetectedVariable, InteractedElementMetadata,
};
