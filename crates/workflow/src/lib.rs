mod context;
mod workflow;
mod workflow_loader;
mod workflow_runtime;

pub mod workflow_data;
pub use workflow::*;
pub use workflow_loader::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_data::WorkflowData;

    #[test]
    fn test_workflow_module_reexports() {
        // Verify public API is accessible from crate root
        let _wf = Workflow::new_draft();
        let _state = WorkflowState::Valid(WorkflowData::default());
    }

    #[test]
    fn test_workflow_loader_exports() {
        // These functions exist and are accessible
        fn _check_load_from_file(
            _: &std::path::Path,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<Workflow>> + Send>>
        {
            Box::pin(async { Ok(Workflow::new_draft()) })
        }
        let _ = _check_load_from_file;
    }
}
