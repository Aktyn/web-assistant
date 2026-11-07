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
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    pub fn len(&self) -> usize {
        self.history.len()
    }

    pub fn final_result(&self) -> Option<String> {
        if let Some(last_step) = self.history.last() {
            if let Some(last_res) = last_step.result.last() {
                if let Some(ref content) = last_res.extracted_content {
                    return Some(content.clone());
                }
            }
        }
        None
    }

    pub fn is_done(&self) -> bool {
        if let Some(last_step) = self.history.last() {
            if let Some(last_res) = last_step.result.last() {
                return last_res.is_done;
            }
        }
        false
    }

    pub fn is_successful(&self) -> Option<bool> {
        if let Some(last_step) = self.history.last() {
            if let Some(last_res) = last_step.result.last() {
                if last_res.is_done {
                    return last_res.success;
                }
            }
        }
        None
    }

    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory {}", parent.display()))?;
            }
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
