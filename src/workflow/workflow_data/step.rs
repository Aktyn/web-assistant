use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StepType {
    Navigate(NavigateStep),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WorkflowStep {
    #[serde(flatten)]
    pub step_type: StepType,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NavigateStep {
    pub url: String,
}
