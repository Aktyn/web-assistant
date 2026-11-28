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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_procedure_step_deserialize() {
        let json = r#"{"procedure":"my-proc"}"#;
        let step: RunProcedureStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.procedure, "my-proc");
    }

    #[test]
    fn test_run_procedure_step_serialize() {
        let step = RunProcedureStep {
            procedure: "login".to_string(),
        };
        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("my-proc") || json.contains("\"procedure\":\"login\""));
        // Actually verify it contains the expected procedure name
        let parsed: RunProcedureStep = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.procedure, "login");
    }

    #[test]
    fn test_run_procedure_step_clone() {
        let step = RunProcedureStep {
            procedure: "proc".to_string(),
        };
        let cloned = step.clone();
        assert_eq!(cloned.procedure, step.procedure);
    }

    #[test]
    fn test_run_procedure_step_debug() {
        let step = RunProcedureStep {
            procedure: "debug-test".to_string(),
        };
        let debug_str = format!("{step:?}");
        assert!(debug_str.contains("RunProcedureStep"));
        assert!(debug_str.contains("debug-test"));
    }

    #[test]
    fn test_run_procedure_step_empty_name() {
        let step = RunProcedureStep {
            procedure: "".to_string(),
        };
        let json = serde_json::to_string(&step).unwrap();
        let parsed: RunProcedureStep = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.procedure, "");
    }
}
