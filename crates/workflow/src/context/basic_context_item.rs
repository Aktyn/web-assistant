#[derive(Debug, Clone)]
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

#[cfg(test)]
mod tests {
    use super::super::WorkflowContextItem;
    use super::*;
    use crate::context::utils::PropertyPath;

    #[test]
    fn test_basic_context_item_new_with_object() {
        let item = BasicContextItem::new(serde_json::json!({"key": "value"}));
        assert_eq!(item.get_data(), &serde_json::json!({"key": "value"}));
    }

    #[test]
    fn test_basic_context_item_new_with_null() {
        let item = BasicContextItem::new(serde_json::Value::Null);
        assert_eq!(item.get_data(), &serde_json::Value::Null);
    }

    #[test]
    fn test_basic_context_item_update_simple_path() {
        let mut item = BasicContextItem::new(serde_json::json!({"a": 1}));
        let path: PropertyPath = vec!["a"];
        let _result = item.update(&path, &serde_json::json!(42)).unwrap();
        assert_eq!(item.get_data(), &serde_json::json!({"a": 42}));
    }

    #[test]
    fn test_basic_context_item_update_nested_path() {
        let mut item = BasicContextItem::new(serde_json::json!({"user": {"name": "old"}}));
        let path: PropertyPath = vec!["user", "name"];
        let _result = item.update(&path, &serde_json::json!("new")).unwrap();
        assert_eq!(
            item.get_data(),
            &serde_json::json!({"user": {"name": "new"}})
        );
    }

    #[test]
    fn test_basic_context_item_get_value_by_path() {
        let item = BasicContextItem::new(serde_json::json!({"a": {"b": {"c": 42}}}));
        let path: PropertyPath = vec!["a", "b", "c"];
        assert_eq!(item.get_value(&path), Some(serde_json::json!(42)));
        let path2: PropertyPath = vec!["a", "b", "z"];
        // Missing keys return Some(Null), not None
        assert_eq!(item.get_value(&path2), Some(serde_json::Value::Null));
    }

    #[test]
    fn test_basic_context_item_get_value_array_index() {
        let item = BasicContextItem::new(serde_json::json!({"items": [10, 20, 30]}));
        let path: PropertyPath = vec!["items", "1"];
        assert_eq!(item.get_value(&path), Some(serde_json::json!(20)));
        let path2: PropertyPath = vec!["items", "99"];
        assert_eq!(item.get_value(&path2), Some(serde_json::Value::Null));
    }

    #[test]
    fn test_basic_context_item_clone() {
        let item1 = BasicContextItem::new(serde_json::json!({"x": true}));
        let item2 = item1.clone();
        assert_eq!(item1.get_data(), item2.get_data());
    }

    #[test]
    fn test_basic_context_item_debug_output() {
        let item = BasicContextItem::new(serde_json::json!({"key": "val"}));
        let debug_str = format!("{item:?}");
        assert!(debug_str.contains("BasicContextItem"));
        assert!(debug_str.contains("key"));
    }
}
