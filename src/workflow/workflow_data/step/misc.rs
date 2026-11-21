use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};

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
    pub message: String,
}
