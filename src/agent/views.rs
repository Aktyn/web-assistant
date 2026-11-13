use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Navigate {
        url: String,
    },
    Click {
        index: u32,
    },
    Input {
        index: u32,
        text: String,
    },
    Scroll {
        amount: i32,
    },
    Wait {
        seconds: u32,
    },
    Extract {
        query: String,
    },
    Done {
        text: String,
        success: bool,
    },
    #[serde(untagged)]
    Custom(serde_json::Value),
}

impl Action {
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Action::Navigate { .. } => "navigate",
            Action::Click { .. } => "click",
            Action::Input { .. } => "input",
            Action::Scroll { .. } => "scroll",
            Action::Wait { .. } => "wait",
            Action::Extract { .. } => "extract",
            Action::Done { .. } => "done",
            Action::Custom(_) => "custom",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentOutput {
    #[serde(default)]
    pub thought: Option<String>,
    #[serde(default)]
    pub memory: Option<String>,
    #[serde(default)]
    pub next_goal: Option<String>,
    #[serde(default)]
    pub action: Vec<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ActionResult {
    #[serde(default)]
    pub is_done: bool,
    #[serde(default)]
    pub success: Option<bool>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub extracted_content: Option<String>,
}

impl ActionResult {
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            is_done: false,
            success: None,
            error: None,
            extracted_content: Some(content.into()),
        }
    }

    pub fn done(content: impl Into<String>, success: bool) -> Self {
        Self {
            is_done: true,
            success: Some(success),
            error: None,
            extracted_content: Some(content.into()),
        }
    }

    pub fn error(err: impl Into<String>) -> Self {
        Self {
            is_done: false,
            success: Some(false),
            error: Some(err.into()),
            extracted_content: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct InteractedElementMetadata {
    pub index: u32,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentStepState {
    pub url: String,
    pub title: String,
    #[serde(default)]
    pub interacted_element: Option<Vec<InteractedElementMetadata>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentHistory {
    pub model_output: Option<AgentOutput>,
    #[serde(default)]
    pub result: Vec<ActionResult>,
    #[serde(default)]
    pub state: Option<AgentStepState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentHistoryList {
    #[serde(default)]
    pub history: Vec<AgentHistory>,
}

impl AgentHistoryList {
    #[must_use]
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn final_result(&self) -> Option<String> {
        if let Some(last_step) = self.history.last()
            && let Some(last_res) = last_step.result.last()
            && let Some(ref content) = last_res.extracted_content
        {
            return Some(content.clone());
        }
        None
    }

    #[must_use]
    pub fn is_done(&self) -> bool {
        if let Some(last_step) = self.history.last()
            && let Some(last_res) = last_step.result.last()
        {
            return last_res.is_done;
        }
        false
    }

    #[must_use]
    pub fn is_successful(&self) -> Option<bool> {
        if let Some(last_step) = self.history.last()
            && let Some(last_res) = last_step.result.last()
            && last_res.is_done
        {
            return last_res.success;
        }
        None
    }

    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let p = path.as_ref();
        if let Some(parent) = p.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }
        let json_str = serde_json::to_string_pretty(self)
            .context("Failed to serialize AgentHistoryList to JSON")?;
        fs::write(p, json_str)
            .with_context(|| format!("Failed to write history to {}", p.display()))?;
        Ok(())
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let p = path.as_ref();
        let content = fs::read_to_string(p)
            .with_context(|| format!("Failed to read history from {}", p.display()))?;
        let history = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON history from {}", p.display()))?;
        Ok(history)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectedVariable {
    pub name: String,
    pub original_value: String,
    pub r#type: String,
    #[serde(default)]
    pub format: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_action_name_all_variants() {
        assert_eq!(
            Action::Navigate {
                url: "https://example.com".into()
            }
            .name(),
            "navigate"
        );
        assert_eq!(Action::Click { index: 1 }.name(), "click");
        assert_eq!(
            Action::Input {
                index: 1,
                text: "hi".into()
            }
            .name(),
            "input"
        );
        assert_eq!(Action::Scroll { amount: 10 }.name(), "scroll");
        assert_eq!(Action::Wait { seconds: 2 }.name(), "wait");
        assert_eq!(
            Action::Extract {
                query: "price".into()
            }
            .name(),
            "extract"
        );
        assert_eq!(
            Action::Done {
                text: "done".into(),
                success: true
            }
            .name(),
            "done"
        );
        assert_eq!(Action::Custom(serde_json::json!({"k": "v"})).name(), "custom");
    }

    #[test]
    fn test_action_serde_roundtrip() {
        let actions = [
            Action::Navigate {
                url: "https://example.com".into(),
            },
            Action::Click { index: 3 },
            Action::Input {
                index: 1,
                text: "value".into(),
            },
            Action::Scroll { amount: -50 },
            Action::Wait { seconds: 1 },
            Action::Extract {
                query: "title".into(),
            },
            Action::Done {
                text: "finished".into(),
                success: false,
            },
            Action::Custom(serde_json::json!({"custom": true})),
        ];

        for action in &actions {
            let json = serde_json::to_string(action).unwrap();
            let restored: Action = serde_json::from_str(&json).unwrap();
            assert_eq!(action, &restored);
        }
    }

    #[test]
    fn test_action_result_success() {
        let res = ActionResult::success("Navigated to page");
        assert!(!res.is_done);
        assert_eq!(res.success, None);
        assert_eq!(res.error, None);
        assert_eq!(res.extracted_content.as_deref(), Some("Navigated to page"));
    }

    #[test]
    fn test_action_result_done_success_and_failure() {
        let ok = ActionResult::done("55.4k stars", true);
        assert!(ok.is_done);
        assert_eq!(ok.success, Some(true));
        assert_eq!(ok.extracted_content.as_deref(), Some("55.4k stars"));

        let failed = ActionResult::done("partial", false);
        assert!(failed.is_done);
        assert_eq!(failed.success, Some(false));
    }

    #[test]
    fn test_action_result_error() {
        let res = ActionResult::error("Navigation failed: timeout");
        assert!(!res.is_done);
        assert_eq!(res.success, Some(false));
        assert_eq!(res.error.as_deref(), Some("Navigation failed: timeout"));
        assert_eq!(res.extracted_content, None);
    }

    #[test]
    fn test_action_result_default_is_empty() {
        let res = ActionResult::default();
        assert!(!res.is_done);
        assert_eq!(res.success, None);
        assert_eq!(res.error, None);
        assert_eq!(res.extracted_content, None);
    }

    #[test]
    fn test_agent_output_serde_defaults() {
        // Missing fields should all default to empty values.
        let output: AgentOutput = serde_json::from_str("{}").unwrap();
        assert_eq!(output.thought, None);
        assert_eq!(output.memory, None);
        assert_eq!(output.next_goal, None);
        assert_eq!(output.action, [] as [Action; 0]);
    }

    #[test]
    fn test_interacted_element_metadata_default() {
        let meta = InteractedElementMetadata::default();
        assert_eq!(meta.index, 0);
        assert_eq!(meta.tag, "");
        assert!(meta.attributes.is_empty());
    }

    #[test]
    fn test_agent_history_list_empty() {
        let list = AgentHistoryList::new();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
        assert_eq!(list.final_result(), None);
        assert!(!list.is_done());
        assert_eq!(list.is_successful(), None);
    }

    #[test]
    fn test_agent_history_list_is_done_but_not_successful() {
        let list = AgentHistoryList {
            history: vec![AgentHistory {
                model_output: None,
                result: vec![ActionResult::error("Something failed")],
                state: None,
            }],
        };
        // Last result is not a done result → is_done is false.
        assert!(!list.is_done());
        assert_eq!(list.is_successful(), None);
    }

    #[test]
    fn test_agent_history_list_done_with_no_extracted_content() {
        // A done result without extracted_content → final_result is None.
        let list = AgentHistoryList {
            history: vec![AgentHistory {
                model_output: None,
                result: vec![ActionResult {
                    is_done: true,
                    success: Some(true),
                    error: None,
                    extracted_content: None,
                }],
                state: None,
            }],
        };
        assert!(list.is_done());
        assert_eq!(list.is_successful(), Some(true));
        assert_eq!(list.final_result(), None);
    }

    #[test]
    fn test_agent_history_list_load_from_missing_file_errors() {
        let result = AgentHistoryList::load_from_file("/nonexistent/history/path.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_history_list_save_creates_parent_directories() {
        let temp_dir = tempfile::tempdir().unwrap();
        let nested = temp_dir.path().join("a/b/history.json");

        let list = AgentHistoryList {
            history: vec![AgentHistory {
                model_output: Some(AgentOutput {
                    thought: Some("thinking".into()),
                    memory: None,
                    next_goal: None,
                    action: vec![Action::Done {
                        text: "result".into(),
                        success: true,
                    }],
                }),
                result: vec![ActionResult::done("result", true)],
                state: None,
            }],
        };

        list.save_to_file(&nested).unwrap();
        assert!(nested.exists());

        let loaded = AgentHistoryList::load_from_file(&nested).unwrap();
        assert_eq!(loaded.len(), list.len());
        assert_eq!(loaded.is_done(), list.is_done());
        assert_eq!(loaded.is_successful(), list.is_successful());
        assert_eq!(loaded.final_result(), list.final_result());
    }

    #[test]
    fn test_agent_history_list_load_from_invalid_json_errors() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), "{not valid json").unwrap();
        let result = AgentHistoryList::load_from_file(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_detected_variable_serde_roundtrip() {
        let var = DetectedVariable {
            name: "email".into(),
            original_value: "user@example.com".into(),
            r#type: "string".into(),
            format: Some("email".into()),
        };
        let json = serde_json::to_string(&var).unwrap();
        let restored: DetectedVariable = serde_json::from_str(&json).unwrap();
        assert_eq!(var, restored);

        // Format is optional in deserialization.
        let no_format: DetectedVariable =
            serde_json::from_str(r#"{"name":"n","original_value":"v","type":"string"}"#).unwrap();
        assert_eq!(no_format.format, None);
    }

    #[test]
    fn test_agent_step_state_serde_roundtrip() {
        let state = AgentStepState {
            url: "https://example.com".into(),
            title: "Example".into(),
            interacted_element: Some(vec![InteractedElementMetadata {
                index: 1,
                tag: "input".into(),
                attributes: HashMap::from([("name".to_string(), "field".to_string())]),
            }]),
        };
        let json = serde_json::to_string(&state).unwrap();
        let restored: AgentStepState = serde_json::from_str(&json).unwrap();
        assert_eq!(
            restored.interacted_element.as_ref().unwrap()[0].attributes,
            state.interacted_element.as_ref().unwrap()[0].attributes
        );
    }

    #[test]
    fn test_save_to_file_with_bare_filename() {
        // A path without a parent directory component should still work.
        let history = AgentHistoryList::default();
        // Use a path in the current directory that has no parent dir.
        let filename = "web_assistant_test_history_no_parent.json";
        history.save_to_file(filename).unwrap();
        let _ = std::fs::remove_file(PathBuf::from(filename));
    }
}
