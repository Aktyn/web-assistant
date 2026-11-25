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
