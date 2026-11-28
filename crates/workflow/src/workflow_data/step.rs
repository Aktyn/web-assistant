mod agent;
pub mod condition;
pub mod context;
pub mod iteration;
mod misc;
pub mod page_action;
mod procedure;
pub mod system_action;

use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIter};

pub(crate) use super::WorkflowString;
pub use agent::AgentStep;
pub use condition::{Condition, ConditionStep};
pub use context::UpdateContextStep;
pub use iteration::ForEachStep;
pub use misc::{AbortStep, WaitStep};
pub use page_action::{PageAction, PageActionStep};
pub use procedure::RunProcedureStep;
pub use system_action::{SystemAction, SystemActionStep};

#[derive(Debug, Deserialize, Serialize, Clone, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter))]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WorkflowStepType {
    #[serde(alias = "page-action", rename = "page-action")]
    PageAction(page_action::PageActionStep),
    Agent(agent::AgentStep),
    #[serde(alias = "system-action", rename = "system-action")]
    SystemAction(system_action::SystemActionStep),

    Condition(condition::ConditionStep),
    #[serde(alias = "for-each", rename = "for-each")]
    ForEach(iteration::ForEachStep),
    #[serde(alias = "run-procedure", rename = "run-procedure")]
    RunProcedure(procedure::RunProcedureStep),

    #[serde(alias = "update-context", rename = "update-context")]
    UpdateContext(context::UpdateContextStep),

    Wait(misc::WaitStep),
    Abort(misc::AbortStep),
}

#[cfg(test)]
mod tests {
    use super::super::workflow_string::WorkflowString;
    use super::*;

    #[test]
    fn test_navigate_step_serde_roundtrip() {
        let step = page_action::NavigateAction {
            url: "https://example.com".into(),
        };
        let json = serde_json::to_string(&step).unwrap();
        let parsed: page_action::NavigateAction = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.url, step.url);
    }

    #[test]
    fn test_workflow_step_navigate_deserialization() {
        let json = r#"{"type":"page-action","pageIndex":0,"action":{"type":"navigate","url":"https://example.com"}}"#;
        let step: WorkflowStepType = serde_json::from_str(json).unwrap();
        match step {
            WorkflowStepType::PageAction(page_action_step) => {
                assert_eq!(page_action_step.page_index, 0);
                assert!(matches!(page_action_step.action, PageAction::Navigate(_)));
            }
            _ => panic!("Expected PageAction variant"),
        }
    }

    #[test]
    fn test_workflow_step_unknown_type_fails() {
        let json = r#"{"type":"unknown","url":"https://example.com"}"#;
        let result: Result<WorkflowStepType, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_workflow_step_missing_type_fails() {
        let json = r#"{"url":"https://example.com"}"#;
        let result: Result<WorkflowStepType, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_step_type_serde_roundtrip() {
        let step = WorkflowStepType::PageAction(page_action::PageActionStep {
            page_index: 0,
            action: PageAction::Navigate(page_action::NavigateAction {
                url: "https://example.com".into(),
            }),
        });
        let json = serde_json::to_string(&step).unwrap();
        let parsed: WorkflowStepType = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, WorkflowStepType::PageAction(_)));
    }

    #[test]
    fn test_workflow_step_agent_deserialization() {
        let json = r#"{"type":"agent","task":"do something","maxSteps":8}"#;
        let step: WorkflowStepType = serde_json::from_str(json).unwrap();
        match step {
            WorkflowStepType::Agent(agent_step) => {
                assert_eq!(agent_step.task, WorkflowString::new("do something"));
                assert_eq!(agent_step.max_steps, 8);
            }
            _ => panic!("Expected Agent variant"),
        }
    }

    #[test]
    fn test_agent_step_default_max_steps() {
        let json = r#"{"type":"agent","task":"do something"}"#;
        let step: WorkflowStepType = serde_json::from_str(json).unwrap();
        match step {
            WorkflowStepType::Agent(agent_step) => {
                assert_eq!(agent_step.max_steps, 64);
            }
            _ => panic!("Expected Agent variant"),
        }
    }
}
