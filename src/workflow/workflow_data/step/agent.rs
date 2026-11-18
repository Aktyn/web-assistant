use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::agent;

/// `AgentStep`
#[derive(Debug, Serialize, Deserialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct AgentStep {
    /// Prompt sent to the agent
    pub task: String,

    /// Maximum number of steps the agent is allowed to take
    /// Default is 64
    #[serde(default = "default_max_steps", alias = "maxSteps", rename = "maxSteps")]
    pub max_steps: u16,

    /// Path to where the steps that agent did will be saved
    /// Relative to workflow file
    #[serde(alias = "historyOutput", rename = "historyOutput")]
    pub history_output: Option<PathBuf>,

    /// Defines which prompt will be used during agent run
    /// Options: "standard", "no-thinking", "flash"
    #[serde(
        default = "agent::PromptMode::default",
        alias = "promptMode",
        rename = "promptMode"
    )]
    pub prompt_mode: agent::PromptMode,
    //TODO:
    // "judge": {
    //   "onCaptcha": "TODO",
    //   "onImpossibleTask": "TODO"
    // }
}

fn default_max_steps() -> u16 {
    64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_step_defaults_max_steps() {
        let json = r#"{"task":"do something"}"#;
        let step: AgentStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.task, "do something");
        assert_eq!(step.max_steps, 64);
        assert!(step.history_output.is_none());
    }

    #[test]
    fn test_agent_step_explicit_max_steps_and_history_output() {
        let json = r#"{"task":"go","maxSteps":16,"historyOutput":"./out.json"}"#;
        let step: AgentStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.task, "go");
        assert_eq!(step.max_steps, 16);
        assert_eq!(step.history_output, Some(PathBuf::from("./out.json")));
    }

    #[test]
    fn test_agent_step_serializes_with_camel_case() {
        let step = AgentStep {
            task: "task".to_string(),
            max_steps: 8,
            history_output: Some(PathBuf::from("./h.json")),
        };
        let json = serde_json::to_value(&step).unwrap();
        assert_eq!(json["task"], "task");
        assert_eq!(json["maxSteps"], 8);
        assert_eq!(json["historyOutput"], "./h.json");
    }

    #[test]
    fn test_agent_step_clone_and_debug() {
        let step = AgentStep {
            task: "clone me".to_string(),
            max_steps: 32,
            history_output: None,
        };
        let cloned = step.clone();
        assert_eq!(cloned.task, step.task);
        assert_eq!(cloned.max_steps, step.max_steps);
        assert!(format!("{step:?}").contains("AgentStep"));
    }
}
