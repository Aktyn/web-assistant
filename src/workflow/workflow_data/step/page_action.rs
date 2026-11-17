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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_action_step_defaults_page_index() {
        let json = r#"{"action":{"type":"navigate","url":"https://example.com"}}"#;
        let step: PageActionStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.page_index, 0);
        assert!(matches!(step.action, PageAction::Navigate(_)));
    }

    #[test]
    fn test_page_action_step_explicit_page_index() {
        let json = r#"{"pageIndex":2,"action":{"type":"navigate","url":"https://example.com"}}"#;
        let step: PageActionStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.page_index, 2);
    }

    #[test]
    fn test_page_action_step_serializes_with_camel_case() {
        let step = PageActionStep {
            page_index: 1,
            action: PageAction::Navigate(NavigateAction {
                url: "https://example.com".to_string(),
            }),
        };
        let json = serde_json::to_value(&step).unwrap();
        assert_eq!(json["pageIndex"], 1);
        assert_eq!(json["action"]["type"], "navigate");
        assert_eq!(json["action"]["url"], "https://example.com");
    }

    #[test]
    fn test_navigate_action_roundtrip() {
        let action = NavigateAction {
            url: "https://github.com".to_string(),
        };
        let json = serde_json::to_string(&action).unwrap();
        let parsed: NavigateAction = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.url, action.url);
    }

    #[test]
    fn test_page_action_unknown_action_type_fails() {
        let json = r#"{"action":{"type":"unknown","url":"https://example.com"}}"#;
        let result: Result<PageActionStep, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_page_action_step_clone_and_debug() {
        let step = PageActionStep {
            page_index: 3,
            action: PageAction::Navigate(NavigateAction {
                url: "https://example.com".to_string(),
            }),
        };
        let cloned = step.clone();
        assert_eq!(cloned.page_index, step.page_index);
        assert!(format!("{step:?}").contains("PageActionStep"));
    }
}
