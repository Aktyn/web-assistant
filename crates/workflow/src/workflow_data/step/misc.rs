use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};

use super::WorkflowString;

/// `WaitStep` - pauses the workflow for given time
#[derive(Debug, Serialize, Deserialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct WaitStep {
    /// Number of milliseconds to wait
    #[serde(default = "u16::default")]
    pub duration: u16,
}

/// `AbortStep` - aborts the workflow with error message
#[derive(Debug, Serialize, Deserialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct AbortStep {
    /// Error message to display
    pub message: WorkflowString,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wait_step_default_duration() {
        let json = r#"{}"#;
        let step: WaitStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.duration, 0);
    }

    #[test]
    fn test_wait_step_explicit_duration() {
        let json = r#"{"duration":5000}"#;
        let step: WaitStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.duration, 5000);
    }

    #[test]
    fn test_wait_step_serde_roundtrip() {
        let step = WaitStep { duration: 1000 };
        let json = serde_json::to_string(&step).unwrap();
        let parsed: WaitStep = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.duration, step.duration);
    }

    #[test]
    fn test_abort_step_message() {
        let step = AbortStep {
            message: WorkflowString::new("Workflow aborted"),
        };
        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("Workflow aborted"));
    }

    #[test]
    fn test_abort_step_clone() {
        let step = AbortStep {
            message: WorkflowString::new("abort msg"),
        };
        let cloned = step.clone();
        assert_eq!(cloned.message, step.message);
    }

    #[test]
    fn test_wait_step_max_u16() {
        let step = WaitStep { duration: u16::MAX };
        let json = serde_json::to_string(&step).unwrap();
        let parsed: WaitStep = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.duration, u16::MAX);
    }
}
