use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};

use super::WorkflowString;

/// `UpdateContextStep`
#[derive(Debug, Serialize, Deserialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct UpdateContextStep {
    /// Context key (e.g. "context1.property1")
    pub key: String,

    /// Value to set for given key
    pub value: Option<WorkflowString>,
}
