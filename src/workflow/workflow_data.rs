pub(crate) mod step;
mod workflow_string;

use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};

pub(crate) use workflow_string::WorkflowString;

/// Workflow must be a json file with the following fields:
#[derive(Debug, Default, Serialize, Deserialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct WorkflowData {
    /// Name of the workflow
    pub name: String,

    /// Description of the workflow
    pub description: Option<String>,

    /// Named group of steps for easier workflow management
    /// See `WorkflowProcedure` for more info
    /// Can be thought of as functions in programming
    pub procedures: Vec<WorkflowProcedure>,

    /// Main list of steps to be performed
    /// Each step is an object that contains "type" property (enum)
    /// See specific fields for each type:
    /// "page-action" --> `PageActionStep`
    /// "agent" --> `AgentStep`
    /// "system-action" --> `SystemActionStep`
    /// "condition" --> `ConditionStep`
    /// "for-each" --> `ForEachStep`
    /// "update-context" --> `UpdateContextStep`
    /// "wait" --> `WaitStep`
    /// "abort" --> `AbortStep`
    pub steps: Vec<step::WorkflowStepType>,
}

/// `WorkflowProcedure`
#[derive(Debug, Default, Serialize, Deserialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct WorkflowProcedure {
    /// Name of the procedure
    /// Works as identifier
    pub name: String,

    /// List of steps to be performed (same as in `WorkflowData`)
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
            "steps":[
                {"type":"page-action","pageIndex":0,"action":{"type":"navigate","url":"https://example.com"}},
                {"type":"agent","task":"do something"}
            ]
        }"#;
        let data: WorkflowData = serde_json::from_str(json).unwrap();
        assert_eq!(data.name, "Full");
        assert_eq!(data.description.as_deref(), Some("desc"));
        assert_eq!(data.steps.len(), 2);
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
            procedures: vec![],
            steps: vec![],
        };
        let cloned = data.clone();
        assert_eq!(cloned.name, data.name);
        assert_eq!(cloned.description, data.description);
    }
}
