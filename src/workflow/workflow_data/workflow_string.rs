use anyhow::{Context, Result, bail};
use documented::{Documented, DocumentedFields};
use regex::{Match, Regex};
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::LazyLock;

use super::super::context::{PropertyPath, WorkflowContextItem};
use super::super::workflow_runtime::RunningWorkflow;

static SPECIAL_VALUE_GROUP_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{[^}]*\}\}").expect("valid regex"));
static INNER_GROUP_VALUE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{\s*(?P<value>[^}]*)\s*\}\}").expect("valid regex"));

/// String value containing special code that unwraps into dynamic value
/// Examples:
/// - `{{ context.foo.property }}` -- retrieves specific context value
/// - `{{url.pathname}}` -- returns the pathname of the current URL
/// ---
/// TODO: add to explain-workflow cli command output
/// `WorkflowString`
#[derive(Debug, Serialize, Clone, DocumentedFields, Documented, PartialEq)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct WorkflowString {
    raw_value: String,
}

impl WorkflowString {
    pub fn new(raw_value: &str) -> Self {
        Self {
            raw_value: raw_value.to_string(),
        }
    }

    pub(crate) async fn parse(&self, workflow: &RunningWorkflow) -> Result<String> {
        let matches_vec: Vec<Match<'_>> = SPECIAL_VALUE_GROUP_REGEX
            .find_iter(&self.raw_value)
            .collect();

        let mut parsed = self.raw_value.clone();

        for result in matches_vec.into_iter().rev() {
            let group = &self.raw_value[result.start()..result.end()];
            if let Some(captures) = INNER_GROUP_VALUE_REGEX.captures(group) {
                let value = &captures["value"];
                if !value.is_empty() {
                    let parsed_value = parse_special_value(value, workflow).await?;
                    parsed = parsed.replace(group, &parsed_value);
                }
            }
        }

        Ok(parsed)
    }
}

async fn parse_special_value(special_value: &str, workflow: &RunningWorkflow) -> Result<String> {
    let mut segments = special_value.split('.');

    let root_segment = segments
        .next()
        .with_context(|| format!("No root segment found in the special value: {special_value}"))?;

    match root_segment {
        "context" => {
            let context = workflow.context.lock().await;

            let context_name = segments.next().with_context(|| {
                format!("No context name found in the special value: {special_value}")
            })?;
            let item = context.get_item(context_name);
            if let Some(item) = item {
                let value = get_context_item_value(item, &segments.collect::<PropertyPath>());
                Ok(value)
            } else {
                log::warn!("No context with given name found: {context_name}");
                Ok(String::new())
            }
        }
        "url" => {
            let browser = workflow.browser.lock().await;
            let url = browser.get_url().unwrap_or_default();

            if !url.is_empty()
                && let Some(url_property) = segments.next()
                && let Ok(url_object) = url::Url::parse(&url)
            {
                let property_value = match url_property {
                    "pathname" | "path" => url_object.path().to_string(),
                    "hostname" | "host" => url_object
                        .host()
                        .unwrap_or(url::Host::Domain(""))
                        .to_string(),
                    // "origin" => return url_object.origin().into(),
                    "protocol" | "scheme" => url_object.scheme().to_string(),
                    "domain" => url_object.domain().unwrap_or_default().to_string(),
                    "port" => url_object.port().unwrap_or_default().to_string(),
                    "querystring" | "query" => url_object.query().unwrap_or_default().to_string(),
                    _ => bail!("Invalid URL property: {url_property}"),
                };

                return Ok(property_value);
            }

            Ok(url)
        }
        _ => Err(anyhow::anyhow!("Invalid root segment: {root_segment}")),
    }
}

fn get_context_item_value(context_item: &dyn WorkflowContextItem, path: &PropertyPath) -> String {
    if let Some(value) = context_item.get_value(path) {
        match value {
            serde_json::Value::String(s) => s,
            serde_json::Value::Null => String::new(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(number) => number.to_string(),
            serde_json::Value::Array(values) => serde_json::to_string(&values).unwrap_or_default(),
            serde_json::Value::Object(map) => serde_json::to_string(&map).unwrap_or_default(),
        }
    } else {
        String::new()
    }
}

impl<'de> Deserialize<'de> for WorkflowString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let deserialized_string = String::deserialize(deserializer)?;
        Ok(Self::new(&deserialized_string))
    }
}

impl From<&str> for WorkflowString {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}
