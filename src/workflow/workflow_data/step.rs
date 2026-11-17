mod agent;
pub(crate) mod page_action;

use serde::{Deserialize, Serialize};

pub(crate) use agent::AgentStep;
pub(crate) use page_action::{PageAction, PageActionStep};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WorkflowStepType {
    #[serde(alias = "page-action", rename = "page-action")]
    PageAction(page_action::PageActionStep),
    Agent(agent::AgentStep),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigate_step_serde_roundtrip() {
        let step = NavigateStep {
            url: "https://example.com".to_string(),
        };
        let json = serde_json::to_string(&step).unwrap();
        let parsed: NavigateStep = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.url, step.url);
    }

    #[test]
    fn test_workflow_step_navigate_deserialization() {
        let json = r#"{"type":"navigate","url":"https://example.com"}"#;
        let step: WorkflowStep = serde_json::from_str(json).unwrap();
        match step.step_type {
            WorkflowStepType::Navigate(navigate) => assert_eq!(navigate.url, "https://example.com"),
        }
    }

    #[test]
    fn test_workflow_step_unknown_type_fails() {
        let json = r#"{"type":"unknown","url":"https://example.com"}"#;
        let result: Result<WorkflowStep, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_workflow_step_missing_type_fails() {
        let json = r#"{"url":"https://example.com"}"#;
        let result: Result<WorkflowStep, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_step_type_serde_roundtrip() {
        let step = WorkflowStep {
            step_type: WorkflowStepType::Navigate(NavigateStep {
                url: "https://example.com".to_string(),
            }),
        };
        let json = serde_json::to_string(&step).unwrap();
        let parsed: WorkflowStep = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed.step_type, WorkflowStepType::Navigate(_)));
    }
}
