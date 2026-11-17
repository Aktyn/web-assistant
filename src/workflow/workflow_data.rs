pub(crate) mod step;

use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};

/// Workflow must be a json file with the following fields:
#[derive(Debug, Default, Serialize, Deserialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct WorkflowData {
    /// Name of the workflow
    pub name: String,
    /// Description of the workflow
    pub description: Option<String>,
    /// List of steps to be performed
    /// Each step is an object that contains "type" property (enum)
    /// See specific fields for each type:
    /// "page-action" --> `PageActionStep`
    /// "agent" --> `AgentStep`
    pub steps: Vec<step::WorkflowStepType>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_data_deserialize_minimal() {
        let json = r#"{"name":"Minimal","steps":[]}"#;
        let data: WorkflowData = serde_json::from_str(json).unwrap();
        assert_eq!(data.name, "Minimal");
        assert!(data.description.is_none());
        assert!(data.steps.is_empty());
    }

    #[test]
    fn test_workflow_data_deserialize_with_steps() {
        let json = r#"{
            "name":"Full",
            "description":"desc",
            "steps":[{"type":"navigate","url":"https://example.com"}]
        }"#;
        let data: WorkflowData = serde_json::from_str(json).unwrap();
        assert_eq!(data.name, "Full");
        assert_eq!(data.description.as_deref(), Some("desc"));
        assert_eq!(data.steps.len(), 1);
    }

    #[test]
    fn test_workflow_data_missing_name_fails() {
        let json = r#"{"steps":[]}"#;
        let result: Result<WorkflowData, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_workflow_data_clone() {
        let data = WorkflowData {
            name: "clone".to_string(),
            description: Some("desc".to_string()),
            steps: vec![],
        };
        let cloned = data.clone();
        assert_eq!(cloned.name, data.name);
        assert_eq!(cloned.description, data.description);
    }
}
