mod basic_context_item;
mod utils;

use anyhow::{Context as _, Result};
pub(crate) use basic_context_item::*;
use std::collections::HashMap;
pub(crate) use utils::*;

pub(crate) struct WorkflowContext {
    store: HashMap<String, Box<dyn WorkflowContextItem>>,
}

/// Stores all context objects used throughout the workflow run
impl WorkflowContext {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    /// Removes all stored context items
    pub fn reset(&mut self) {
        log::info!("Resetting context");
        self.store.clear();
    }

    pub fn set_item(&mut self, name: &str, item: Box<dyn WorkflowContextItem>) {
        self.store.insert(name.to_string(), item);
    }

    pub fn get_item(&self, context_name: &str) -> Option<&dyn WorkflowContextItem> {
        self.store.get(context_name).map(|v| &**v)
    }

    pub fn update_item_value(
        &mut self,
        context_name: &str,
        path: &PropertyPath,
        value: &serde_json::Value,
    ) -> Result<&dyn WorkflowContextItem> {
        let item = self
            .store
            .get_mut(context_name)
            .with_context(|| format!("No context is set with name: {context_name}"))?;

        item.update(path, value)?;
        Ok(&**item)
    }
}

pub(crate) trait WorkflowContextItem: std::fmt::Debug {
    /// Returns reference to the whole object data
    fn get_data(&self) -> &serde_json::Value;

    /// Returns reference to specific value located by path
    /// None is returned if specific value couldn't be located by the path
    fn get_value(&self, path: &PropertyPath) -> Option<serde_json::Value> {
        let mut result = self.get_data();
        for property_name in path {
            match result {
                // Return None for non-object values
                serde_json::Value::Null
                | serde_json::Value::Bool(..)
                | serde_json::Value::Number(..)
                | serde_json::Value::String(..) => return None,

                serde_json::Value::Array(values) => {
                    if let Ok(index) = property_name.parse::<usize>() {
                        result = values.get(index).unwrap_or(&serde_json::Value::Null);
                    } else {
                        // Not a valid index
                        return None;
                    }
                }
                serde_json::Value::Object(map) => {
                    result = map.get(*property_name).unwrap_or(&serde_json::Value::Null);
                }
            }
        }

        Some(result.clone())
    }

    /// Updates specific context object value
    /// path is string of dot-joined properties (e.g. "user.name")
    /// Returns reference to whole object data after update
    fn update(
        &mut self,
        path: &PropertyPath,
        value: &serde_json::Value,
    ) -> Result<&serde_json::Value>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::basic_context_item::BasicContextItem;
    use super::WorkflowContextItem;
    use crate::context::utils::PropertyPath;

    #[test]
    fn test_workflow_context_new_is_empty() {
        let ctx = WorkflowContext::new();
        assert!(ctx.get_item("any").is_none());
    }

    #[test]
    fn test_workflow_context_set_and_get() {
        let mut ctx = WorkflowContext::new();
        ctx.set_item(
            "user",
            Box::new(BasicContextItem::new(serde_json::json!({"name": "Alice"}))),
        );
        let item = ctx.get_item("user").expect("item should exist");
        let path: PropertyPath = vec!["name"];
        assert_eq!(item.get_value(&path), Some(serde_json::json!("Alice")));
    }

    #[test]
    fn test_workflow_context_overwrite_existing_item() {
        let mut ctx = WorkflowContext::new();
        ctx.set_item("k", Box::new(BasicContextItem::new(serde_json::json!(1))));
        ctx.set_item("k", Box::new(BasicContextItem::new(serde_json::json!(2))));
        let item = ctx.get_item("k").expect("item should exist");
        assert_eq!(item.get_data(), &serde_json::json!(2));
    }

    #[test]
    fn test_workflow_context_reset_clears_all() {
        let mut ctx = WorkflowContext::new();
        ctx.set_item("a", Box::new(BasicContextItem::new(serde_json::json!(1))));
        ctx.set_item("b", Box::new(BasicContextItem::new(serde_json::json!(2))));
        ctx.reset();
        assert!(ctx.get_item("a").is_none());
        assert!(ctx.get_item("b").is_none());
    }

    #[test]
    fn test_workflow_context_update_item_value_success() {
        let mut ctx = WorkflowContext::new();
        ctx.set_item(
            "data",
            Box::new(BasicContextItem::new(serde_json::json!({"count": 0}))),
        );
        let path: PropertyPath = vec!["count"];
        let result = ctx
            .update_item_value("data", &path, &serde_json::json!(99))
            .unwrap();
        let path2: PropertyPath = vec!["count"];
        assert_eq!(result.get_value(&path2), Some(serde_json::json!(99)));
    }

    #[test]
    fn test_workflow_context_update_item_value_missing_context_errors() {
        let mut ctx = WorkflowContext::new();
        let path: PropertyPath = vec!["key"];
        let result = ctx.update_item_value("missing", &path, &serde_json::json!("val"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No context is set"));
    }

    #[test]
    fn test_workflow_context_item_trait_get_value_on_primitive_returns_none() {
        let item = BasicContextItem::new(serde_json::json!("string_value"));
        let path: PropertyPath = vec!["nested"];
        assert!(item.get_value(&path).is_none());
    }

    #[test]
    fn test_workflow_context_item_trait_get_value_deep_path() {
        let item = BasicContextItem::new(serde_json::json!({
            "level1": {"level2": {"level3": "deep_value"}}
        }));
        let path: PropertyPath = vec!["level1", "level2", "level3"];
        assert_eq!(
            item.get_value(&path),
            Some(serde_json::json!("deep_value"))
        );
        let path2: PropertyPath = vec!["level1", "nonexistent"];
        // Missing keys return Some(Null), not None
        assert_eq!(item.get_value(&path2), Some(serde_json::Value::Null));
    }
}
