use anyhow::Result;
use std::collections::HashMap;

// use jsonc_parser::{JsonObject, JsonValue};

//TODO: it's possible that this struct is redundant and using HashMap would be enough
pub(crate) struct WorkflowContext {
    store: HashMap<String, Box<dyn WorkflowContextItem>>,
}

/// Dot path to specific object property
/// e.g. "user.name" will get "John" from the following object:
/// ```json
/// {
///     "user": {
///         "name": "John"
///     }
/// }
/// ```
pub(crate) type PropertyPath<'a> = Vec<&'a str>;

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

    pub fn get_item(&self, name: &str) -> Option<&dyn WorkflowContextItem> {
        //TODO: investigate whether this &**v pattern is not causing unnecessary clones
        self.store.get(name).map(|v| &**v)
    }
}

pub(crate) trait WorkflowContextItem {
    /// Returns reference to the whole object data
    fn get_data(&self) -> &serde_json::Value;

    /// Returns reference to specific value located by path
    fn get_value(&self, path: &PropertyPath) -> Option<serde_json::Value>;

    /// Updates specific context object value
    /// path is string of dot-joined properties (e.g. "user.name")
    /// Returns reference to whole object data after update
    fn update(&self, path: &PropertyPath, value: serde_json::Value) -> Result<&serde_json::Value>;
}
