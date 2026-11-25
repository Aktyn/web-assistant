use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};

/// `RunProcedureStep` - runs procedure by name
#[derive(Debug, Serialize, Deserialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct RunProcedureStep {
    /// Name of the procedure to run
    /// Workflow will fail if the procedure is not defined
    pub procedure: String,
}
