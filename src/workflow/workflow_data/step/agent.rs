use std::path::PathBuf;

use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};

/// `AgentStep`
#[derive(Debug, Serialize, Deserialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct AgentStep {
    /// Prompt sent to the agent
    task: String,

    /// Maximum number of steps the agent is allowed to take
    /// Default is 64
    #[serde(default = "default_max_steps", alias = "maxSteps", rename = "maxSteps")]
    max_steps: u16,

    /// Path to where the steps agent decided to run will be saved
    #[serde(alias = "historyOutput", rename = "historyOutput")]
    history_output: Option<PathBuf>,
    //TODO:
    // promptMode: "standard",
    // "judge": {
    //   "onCaptcha": "TODO",
    //   "onImpossibleTask": "TODO"
    // }
}

fn default_max_steps() -> u16 {
    64
}
