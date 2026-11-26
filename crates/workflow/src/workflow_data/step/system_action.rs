use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIter};

use super::WorkflowString;

/// `SystemActionStep` - performs system actions
#[derive(Debug, Serialize, Deserialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct SystemActionStep {
    /// Object describing the system action
    /// Contains "type" property (enum)
    /// See specific fields for each type:
    /// "notify" --> `NotifyAction`
    /// "open-url" --> `OpenUrlAction`
    pub action: SystemAction,
}

#[derive(Debug, Deserialize, Serialize, Clone, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter))]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SystemAction {
    #[serde(alias = "open-url", rename = "open-url")]
    OpenUrl(OpenUrlAction),
    Notify(NotifyAction),
}

/// `NotifyAction` - sends system notification
#[derive(Debug, Deserialize, Serialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct NotifyAction {
    /// Message to notify
    pub message: WorkflowString,
}

/// `OpenUrlAction` - opens given url in default browser
#[derive(Debug, Deserialize, Serialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct OpenUrlAction {
    /// URL to open
    pub url: WorkflowString,
}
