pub(super) mod step;

use serde::Deserialize;

/// Describes valid workflow data structure
#[derive(Debug, Deserialize, Clone)]
pub struct WorkflowData {
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<step::WorkflowStep>,
}
