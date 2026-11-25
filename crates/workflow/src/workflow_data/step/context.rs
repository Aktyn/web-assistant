use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};

use super::WorkflowString;

/// `UpdateContextStep`
#[derive(Debug, Serialize, Deserialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct UpdateContextStep {
    /// Path to given context property (e.g. "context1.property1")
    /// First segment is the context name
    /// Given context must be present (previously set),
    /// otherwise the operation will fail
    pub key: String,

    /// Value to set for given key
    /// None will be treated as null
    pub value: Option<WorkflowString>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_context_step_with_value() {
        let json = r#"{"key":"user.name","value":"Alice"}"#;
        let step: UpdateContextStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.key, "user.name");
        assert_eq!(step.value, Some(WorkflowString::new("Alice")));
    }

    #[test]
    fn test_update_context_step_without_value() {
        let json = r#"{"key":"counter"}"#;
        let step: UpdateContextStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.key, "counter");
        assert_eq!(step.value, None);
    }

    #[test]
    fn test_update_context_step_null_value() {
        let json = r#"{"key":"slot","value":null}"#;
        let step: UpdateContextStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.key, "slot");
        assert_eq!(step.value, None);
    }

    #[test]
    fn test_update_context_step_serialization_roundtrip() {
        let step = UpdateContextStep {
            key: "a.b.c".to_string(),
            value: Some(WorkflowString::new("deep")),
        };
        let json = serde_json::to_string(&step).unwrap();
        let parsed: UpdateContextStep = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.key, "a.b.c");
        assert_eq!(parsed.value, Some(WorkflowString::new("deep")));
    }

    #[test]
    fn test_update_context_step_clone() {
        let step = UpdateContextStep {
            key: "k".to_string(),
            value: Some(WorkflowString::new("v")),
        };
        let cloned = step.clone();
        assert_eq!(cloned.key, step.key);
        assert_eq!(cloned.value, step.value);
    }

    #[test]
    fn test_update_context_step_debug_output() {
        let step = UpdateContextStep {
            key: "test_key".to_string(),
            value: None,
        };
        let debug_str = format!("{step:?}");
        assert!(debug_str.contains("UpdateContextStep"));
        assert!(debug_str.contains("test_key"));
    }
}
