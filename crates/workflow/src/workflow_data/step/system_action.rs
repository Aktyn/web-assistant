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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_action_notify_deserialize() {
        let json = r#"{"type":"notify","message":"Hello world"}"#;
        let action: SystemAction = serde_json::from_str(json).unwrap();
        assert!(matches!(action, SystemAction::Notify(_)));
    }

    #[test]
    fn test_system_action_open_url_deserialize() {
        let json = r#"{"type":"open-url","url":"https://example.com"}"#;
        let action: SystemAction = serde_json::from_str(json).unwrap();
        assert!(matches!(action, SystemAction::OpenUrl(_)));
    }

    #[test]
    fn test_system_action_open_url_alias_deserialize() {
        let json = r#"{"type":"open-url","url":"https://example.com"}"#;
        let action: SystemAction = serde_json::from_str(json).unwrap();
        assert!(matches!(action, SystemAction::OpenUrl(_)));
    }

    #[test]
    fn test_system_action_serialization_roundtrip() {
        let action = SystemAction::Notify(NotifyAction {
            message: WorkflowString::new("Test notification"),
        });
        let json = serde_json::to_string(&action).unwrap();
        let parsed: SystemAction = serde_json::from_str(&json).unwrap();
        match &parsed {
            SystemAction::Notify(n) => {
                assert_eq!(n.message, WorkflowString::new("Test notification"))
            }
            _ => panic!("Expected Notify"),
        }
    }

    #[test]
    fn test_system_action_step_deserialization() {
        let json = r#"{"action":{"type":"notify","message":"remind me"}}"#;
        let step: SystemActionStep = serde_json::from_str(json).unwrap();
        assert!(matches!(step.action, SystemAction::Notify(_)));
    }

    #[test]
    fn test_system_action_unknown_type_fails() {
        let json = r#"{"type":"unknown-action","data":"x"}"#;
        let result: Result<SystemAction, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_open_url_action_roundtrip() {
        let action = OpenUrlAction {
            url: WorkflowString::new("https://rust-lang.org"),
        };
        let json = serde_json::to_string(&action).unwrap();
        let parsed: OpenUrlAction = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.url, WorkflowString::new("https://rust-lang.org"));
    }

    #[test]
    fn test_notify_action_roundtrip() {
        let action = NotifyAction {
            message: WorkflowString::new("Click here!"),
        };
        let json = serde_json::to_string(&action).unwrap();
        let parsed: NotifyAction = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.message, WorkflowString::new("Click here!"));
    }

    #[test]
    fn test_system_action_clone() {
        let action = SystemAction::OpenUrl(OpenUrlAction {
            url: WorkflowString::new("https://example.com"),
        });
        let cloned = action.clone();
        assert!(matches!(cloned, SystemAction::OpenUrl(_)));
    }
}
