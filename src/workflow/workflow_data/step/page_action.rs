use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};

/// `PageActionStep`
#[derive(Debug, Serialize, Deserialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct PageActionStep {
    /// Index of the page to perform the action on
    /// Needed for multi-page workflows
    #[serde(default = "u16::default", alias = "pageIndex", rename = "pageIndex")]
    pub page_index: u16,

    /// Object describing the page action
    /// Contains "type" property (enum)
    /// See specific fields for each type:
    /// "navigate" --> `NavigateAction`
    pub action: PageAction,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PageAction {
    Navigate(NavigateAction),
}

/// `NavigateAction`
#[derive(Debug, Deserialize, Serialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct NavigateAction {
    /// URL to navigate to
    pub url: String,
}
