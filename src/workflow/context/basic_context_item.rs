#[derive(Debug)]
pub(crate) struct BasicContextItem {
    data: serde_json::Value,
}

impl super::WorkflowContextItem for BasicContextItem {
    fn get_data(&self) -> &serde_json::Value {
        &self.data
    }

    fn update(
        &mut self,
        path: &super::PropertyPath,
        value: &serde_json::Value,
    ) -> anyhow::Result<&serde_json::Value> {
        self.data = super::set_nested_property(&self.data, path, value.clone());
        Ok(self.get_data())
    }
}

impl BasicContextItem {
    pub(crate) fn new(data: serde_json::Value) -> Self {
        Self { data }
    }
}
