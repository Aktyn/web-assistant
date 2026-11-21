use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};

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
    pub condition: Condition,

    /// Steps to run if the condition is met
    pub then: Vec<super::WorkflowStepType>,

    /// Steps to run if the condition is not met
    #[serde(alias = "else", rename = "else")]
    pub else_: Vec<super::WorkflowStepType>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Condition {
    #[serde(rename = "match-regex")]
    MatchRegex(MatchRegexCondition),
    Truthy(TruthyCondition),
    Equal(EqualCondition),
}

/// `MatchRegexCondition`
#[derive(Debug, Deserialize, Serialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct MatchRegexCondition {
    /// Value to match against a regex
    pub value: String,

    /// String containing regex pattern
    pub pattern: String,
}

/// `TruthyCondition`
#[derive(Debug, Deserialize, Serialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct TruthyCondition {
    /// Value to check if it is truthy
    pub value: String,
}

/// `EqualCondition` - compares two values equality
#[derive(Debug, Deserialize, Serialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct EqualCondition {
    /// Left value to compare
    pub left: String,

    /// Right value to compare
    pub right: String,
}
