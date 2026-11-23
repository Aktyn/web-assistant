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
        //TODO: investigate whether this &**v pattern is not causing unnecessary clones
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
