pub(super) mod step;

use serde::Deserialize;

/// Describes valid workflow data structure
#[derive(Debug, Deserialize, Clone)]
pub struct WorkflowData {
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<step::WorkflowStep>,
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
