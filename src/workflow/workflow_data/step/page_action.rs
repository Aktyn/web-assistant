use anyhow::Result;
use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

use super::WorkflowString;

/// `PageActionStep` - interacts with the page
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
    /// "input" --> `InputAction`
    /// "click" --> `ClickAction`
    /// "evaluate" --> `EvaluateAction`
    pub action: PageAction,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PageAction {
    Navigate(NavigateAction),
    Input(InputAction),
    Click(ClickAction),
    Evaluate(EvaluateAction),
}

/// `NavigateAction`
#[derive(Debug, Deserialize, Serialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct NavigateAction {
    /// URL to navigate to
    pub url: WorkflowString,
}

/// `InputAction`
#[derive(Debug, Deserialize, Serialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct InputAction {
    /// CSS selector to find the input element
    pub selector: WorkflowString,

    /// Value to type inside the input element
    /// Input field will be cleared beforehand
    pub value: WorkflowString,
}

/// `ClickAction`
#[derive(Debug, Deserialize, Serialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct ClickAction {
    /// CSS selector to find the click element
    pub selector: WorkflowString,

    /// Expects the page to redirect after clicking an element
    /// If the navigation won't occur, workflow will fail
    #[serde(
        default = "bool::default",
        alias = "waitForNavigation",
        rename = "waitForNavigation"
    )]
    pub wait_for_navigation: bool,
}

/// `EvaluateAction`
#[derive(Debug, Deserialize, Serialize, Clone, DocumentedFields, Documented)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct EvaluateAction {
    /// Either path to JavaScript file or code to evaluate on the page
    pub script: String,

    /// Upserts context with the value returned from evaluated code
    /// If not provided, the context will not be created/updated
    #[serde(default, alias = "contextName", rename = "contextName")]
    pub context_name: Option<String>,
}

pub(crate) async fn get_script_content(script: &str, root_path: &Path) -> Result<String> {
    let mut possible_script_path = PathBuf::from(script);
    if possible_script_path.is_relative() {
        possible_script_path = root_path.join(possible_script_path);
    }

    if possible_script_path.exists() && possible_script_path.is_file() {
        log::info!(
            "Reading script content from {}",
            possible_script_path.display()
        );
        let code = fs::read_to_string(possible_script_path).await?;
        return Ok(code);
    }

    Ok(script.to_string())
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
