use anyhow::Result;
use documented::{Documented, DocumentedFields};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use strum::{EnumDiscriminants, EnumIter};
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

#[derive(Debug, Deserialize, Serialize, Clone, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter))]
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
                url: "https://example.com".into(),
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
            url: "https://github.com".into(),
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
                url: "https://example.com".into(),
            }),
        };
        let cloned = step.clone();
        assert_eq!(cloned.page_index, step.page_index);
        assert!(format!("{step:?}").contains("PageActionStep"));
    }

    #[tokio::test]
    async fn test_get_script_content_inline_code_returns_as_is() {
        let root = PathBuf::from("/tmp");
        let content = get_script_content("alert('hi');", &root).await.unwrap();
        assert_eq!(content, "alert('hi');");
    }

    #[tokio::test]
    async fn test_get_script_content_from_file_reads_file() {
        let tmp = tempfile::tempdir().unwrap();
        let script_path = tmp.path().join("script.js");
        std::fs::write(&script_path, "console.log('loaded');").unwrap();

        let content = get_script_content("script.js", tmp.path()).await.unwrap();
        assert_eq!(content, "console.log('loaded');");
    }

    #[test]
    fn test_get_script_content_relative_vs_absolute_resolve_same() {
        // Both relative and absolute paths should resolve to same file
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = rt.block_on(async {
            let d = tempfile::tempdir().unwrap();
            let p = d.path().join("s.js");
            std::fs::write(&p, "x").unwrap();
            (d, p)
        });

        let abs_result = rt.block_on(async {
            get_script_content(tmp.1.to_str().unwrap(), Path::new("/nonexistent")).await
        });
        let rel_result = rt.block_on(async { get_script_content("s.js", tmp.0.path()).await });

        let abs_content = abs_result.unwrap();
        let rel_content = rel_result.unwrap();
        assert_eq!(abs_content, rel_content);
        assert_eq!(rel_content, "x");
    }

    #[tokio::test]
    async fn test_get_script_content_nonexistent_file_falls_back_to_inline() {
        let root = PathBuf::from("/tmp");
        // Non-existent file path — should be treated as inline code
        let content = get_script_content("/no/such/file.js", &root).await.unwrap();
        assert_eq!(content, "/no/such/file.js");
    }
}
