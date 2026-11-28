use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIter};

use super::WorkflowString;

/// `ConditionStep` - controls the workflow flow based on given condition
#[derive(Debug, Serialize, Deserialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct ConditionStep {
    /// Object describing the logic of the condition
    /// Contains "type" property (enum)
    /// See specific fields for each type:
    /// "match-regex" --> `MatchRegexCondition`
    /// "truthy" --> `TruthyCondition`
    /// "equal" --> `EqualCondition`
    /// "has-element" --> `HasElementCondition`
    pub condition: Condition,

    /// Steps to run if the condition is met
    pub then: Vec<super::WorkflowStepType>,

    /// Steps to run if the condition is not met
    #[serde(default = "Vec::new", alias = "else", rename = "else")]
    pub else_: Vec<super::WorkflowStepType>,
}

#[derive(Debug, Deserialize, Serialize, Clone, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter))]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Condition {
    #[serde(rename = "match-regex")]
    MatchRegex(MatchRegexCondition),
    Truthy(TruthyCondition),
    Equal(EqualCondition),
    #[serde(rename = "has-element")]
    HasElement(HasElementCondition),
}

/// `MatchRegexCondition`
#[derive(Debug, Deserialize, Serialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct MatchRegexCondition {
    /// Value to match against a regex
    pub value: WorkflowString,

    /// String containing regex pattern
    pub pattern: String,
}

/// `TruthyCondition`
#[derive(Debug, Deserialize, Serialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct TruthyCondition {
    /// Value to check if it is truthy
    pub value: WorkflowString,
}

/// `EqualCondition` - compares two values equality
#[derive(Debug, Deserialize, Serialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct EqualCondition {
    /// Left value to compare
    pub left: WorkflowString,

    /// Right value to compare
    pub right: WorkflowString,
}

/// `HasElementCondition` - checks if the element is present in the page
#[derive(Debug, Deserialize, Serialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct HasElementCondition {
    /// Selector to check if the element is present
    pub selector: WorkflowString,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_match_regex_deserialize() {
        let json = r#"{"type":"match-regex","value":"hello","pattern":"hel.*"}"#;
        let cond: Condition = serde_json::from_str(json).unwrap();
        match &cond {
            Condition::MatchRegex(mr) => {
                assert_eq!(mr.value, WorkflowString::new("hello"));
                assert_eq!(mr.pattern, "hel.*");
            }
            _ => panic!("Expected MatchRegex"),
        }
    }

    #[test]
    fn test_condition_truthy_deserialize() {
        let json = r#"{"type":"truthy","value":"some_value"}"#;
        let cond: Condition = serde_json::from_str(json).unwrap();
        assert!(matches!(cond, Condition::Truthy(_)));
    }

    #[test]
    fn test_condition_equal_deserialize() {
        let json = r#"{"type":"equal","left":"a","right":"b"}"#;
        let cond: Condition = serde_json::from_str(json).unwrap();
        match &cond {
            Condition::Equal(eq) => {
                assert_eq!(eq.left, WorkflowString::new("a"));
                assert_eq!(eq.right, WorkflowString::new("b"));
            }
            _ => panic!("Expected Equal"),
        }
    }

    #[test]
    fn test_condition_has_element_deserialize() {
        let json = r##"{"type":"has-element","selector":"#submit"}"##;
        let cond: Condition = serde_json::from_str(json).unwrap();
        assert!(matches!(cond, Condition::HasElement(_)));
    }

    #[test]
    fn test_condition_serialization_roundtrip() {
        let cond = Condition::MatchRegex(MatchRegexCondition {
            value: WorkflowString::new("test"),
            pattern: "^t".to_string(),
        });
        let json = serde_json::to_string(&cond).unwrap();
        let parsed: Condition = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, Condition::MatchRegex(_)));
    }

    #[test]
    fn test_condition_unknown_type_fails() {
        let json = r#"{"type":"unknown-cond","value":"x"}"#;
        let result: Result<Condition, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_condition_clone() {
        let cond = Condition::Truthy(TruthyCondition {
            value: WorkflowString::new("v"),
        });
        let cloned = cond.clone();
        assert!(matches!(cloned, Condition::Truthy(_)));
    }

    #[test]
    fn test_condition_step_deserialization_full() {
        let json = r#"{
            "condition": {"type":"truthy","value":"flag"},
            "then": [{"type":"agent","task":"done"}],
            "else": []
        }"#;
        let step: ConditionStep = serde_json::from_str(json).unwrap();
        assert!(matches!(step.condition, Condition::Truthy(_)));
        assert_eq!(step.then.len(), 1);
        assert!(step.else_.is_empty());
    }

    #[test]
    fn test_condition_step_else_alias_deserialization() {
        // "else" should deserialize into the else_ field
        let json = r#"{"condition":{"type":"truthy","value":"x"},"then":[],"else":[{"type":"abort","message":"fail"}]}"#;
        let step: ConditionStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.else_.len(), 1);
    }

    #[test]
    fn test_condition_step_default_else_is_empty() {
        let json = r#"{"condition":{"type":"truthy","value":"x"},"then":[]}"#;
        let step: ConditionStep = serde_json::from_str(json).unwrap();
        assert!(step.else_.is_empty());
    }
}
