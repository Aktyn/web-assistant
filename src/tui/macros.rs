#[macro_export]
macro_rules! workflows_count {
    ($core:expr) => {
        $core.read().map(|core| core.workflows.len()).unwrap_or(0)
    };
}
