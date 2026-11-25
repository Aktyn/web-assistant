use anyhow::{Context, Result, bail};
use documented::{Documented, DocumentedFields};
use regex::{Match, Regex};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::LazyLock;

use super::super::context::{PropertyPath, WorkflowContextItem};
use super::super::workflow_runtime::RunningWorkflow;

#[allow(clippy::expect_used)]
static SPECIAL_VALUE_GROUP_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{[^}]*\}\}").expect("valid regex"));
#[allow(clippy::expect_used)]
static INNER_GROUP_VALUE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{\s*(?P<value>[^}]*)\s*\}\}").expect("valid regex"));

/// String value containing special code that unwraps into dynamic value
/// Examples:
/// - `{{ context.foo.property }}` -- retrieves specific context value
/// - `{{url.pathname}}` -- returns the pathname of the current URL
/// ---
/// TODO: add to explain-workflow cli command output
/// `WorkflowString`
#[derive(Debug, Clone, DocumentedFields, Documented, PartialEq)]
#[documented_fields(default = "Missing documentation", rename_all = "camelCase")]
pub struct WorkflowString {
    raw_value: String,
}

impl WorkflowString {
    #[must_use]
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

impl Serialize for WorkflowString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.raw_value)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{BasicContextItem, WorkflowContext};
    use crate::workflow_runtime::RunningWorkflow;
    use config::Config;
    use std::path::Path;

    #[test]
    fn test_new_creates_exact_value() {
        let s = WorkflowString::new("hello world");
        assert_eq!(s.raw_value, "hello world");
    }

    #[test]
    fn test_from_str() {
        let s: WorkflowString = "foo bar".into();
        assert_eq!(s.raw_value, "foo bar");
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let original = WorkflowString::new("{{ context.foo }}");
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(json, "\"{{ context.foo }}\"");
        let restored: WorkflowString = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.raw_value, "{{ context.foo }}");
    }

    #[test]
    fn test_empty_raw_value() {
        let s = WorkflowString::new("");
        assert_eq!(s.raw_value, "");
    }

    #[test]
    fn test_no_placeholders_returns_as_is() {
        // parse requires RunningWorkflow; we test the raw_value here instead
        let s = WorkflowString::new("plain text without placeholders");
        assert_eq!(s.raw_value, "plain text without placeholders");
    }

    #[tokio::test]
    async fn test_parse_context_missing_name_returns_empty() {
        let runner = make_test_runner().await;
        let s = WorkflowString::new("{{ context.missing_key.property }}");
        let result = s.parse(&runner).await.unwrap();
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn test_parse_context_simple_value() {
        let mut runner = make_test_runner().await;
        {
            let mut ctx = runner.context.lock().await;
            ctx.set_item(
                "myctx",
                Box::new(BasicContextItem::new(serde_json::json!({"name": "Alice"}))),
            );
        }
        let s = WorkflowString::new("{{ context.myctx.name }}");
        let result = s.parse(&runner).await.unwrap();
        assert_eq!(result, "Alice");
    }

    #[tokio::test]
    async fn test_parse_invalid_root_segment_fails() {
        let runner = make_test_runner().await;
        let s = WorkflowString::new("{{ unknown.root.value }}");
        let result = s.parse(&runner).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid root segment")
        );
    }

    #[tokio::test]
    async fn test_parse_url_property_hostname() {
        let runner = make_test_runner_with_url("https://example.com/path?q=1").await;
        let s = WorkflowString::new("{{ url.hostname }}");
        let result = s.parse(&runner).await.unwrap();
        assert_eq!(result, "example.com");
    }

    #[tokio::test]
    async fn test_parse_url_property_pathname() {
        let runner = make_test_runner_with_url("https://example.com/foo/bar").await;
        let s = WorkflowString::new("{{ url.pathname }}");
        let result = s.parse(&runner).await.unwrap();
        assert_eq!(result, "/foo/bar");
    }

    #[tokio::test]
    async fn test_parse_mixed_literal_and_placeholder() {
        let mut runner = make_test_runner().await;
        {
            let mut ctx = runner.context.lock().await;
            ctx.set_item(
                "greeting",
                Box::new(BasicContextItem::new(serde_json::json!("Hello"))),
            );
        }
        let s = WorkflowString::new("Hi {{ context.greeting }}!");
        let result = s.parse(&runner).await.unwrap();
        assert_eq!(result, "Hi Hello!");
    }

    #[tokio::test]
    async fn test_parse_no_placeholders_returns_original() {
        let runner = make_test_runner().await;
        let s = WorkflowString::new("just plain text");
        let result = s.parse(&runner).await.unwrap();
        assert_eq!(result, "just plain text");
    }

    #[tokio::test]
    async fn test_parse_empty_inner_group_ignored() {
        let runner = make_test_runner().await;
        // "{{ }}" has empty value inside — should be left as-is (not erroring)
        let s = WorkflowString::new("prefix {{ }} suffix");
        let result = s.parse(&runner).await.unwrap();
        assert_eq!(result, "prefix  suffix");
    }

    #[tokio::test]
    async fn test_parse_bool_context_value() {
        let mut runner = make_test_runner().await;
        {
            let mut ctx = runner.context.lock().await;
            ctx.set_item(
                "flags",
                Box::new(BasicContextItem::new(serde_json::json!({"enabled": true}))),
            );
        }
        let s = WorkflowString::new("{{ context.flags.enabled }}");
        let result = s.parse(&runner).await.unwrap();
        assert_eq!(result, "true");
    }

    #[tokio::test]
    async fn test_parse_array_context_value_serialized() {
        let mut runner = make_test_runner().await;
        {
            let mut ctx = runner.context.lock().await;
            ctx.set_item(
                "items",
                Box::new(BasicContextItem::new(serde_json::json!([1, 2, 3]))),
            );
        }
        let s = WorkflowString::new("{{ context.items }}");
        let result = s.parse(&runner).await.unwrap();
        assert_eq!(result, "[1,2,3]");
    }

    #[tokio::test]
    async fn test_parse_null_context_value_returns_empty() {
        let mut runner = make_test_runner().await;
        {
            let mut ctx = runner.context.lock().await;
            ctx.set_item(
                "nothing",
                Box::new(BasicContextItem::new(serde_json::json!(null))),
            );
        }
        let s = WorkflowString::new("{{ context.nothing }}");
        let result = s.parse(&runner).await.unwrap();
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn test_parse_object_context_value_serialized() {
        let mut runner = make_test_runner().await;
        {
            let mut ctx = runner.context.lock().await;
            ctx.set_item(
                "obj",
                Box::new(BasicContextItem::new(serde_json::json!({"a": 1}))),
            );
        }
        let s = WorkflowString::new("{{ context.obj }}");
        let result = s.parse(&runner).await.unwrap();
        assert_eq!(result, "{\"a\":1}");
    }

    #[tokio::test]
    async fn test_parse_missing_context_name_returns_empty() {
        let runner = make_test_runner().await;
        let s = WorkflowString::new("{{ context.nonexistent }}");
        let result = s.parse(&runner).await.unwrap();
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn test_parse_url_missing_property_fails() {
        let runner = make_test_runner_with_url("https://example.com").await;
        let s = WorkflowString::new("{{ url.nonexistent }}");
        let result = s.parse(&runner).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid URL property")
        );
    }

    #[tokio::test]
    async fn test_parse_url_without_dot_returns_full_url() {
        let runner = make_test_runner_with_url("https://example.com/path").await;
        let s = WorkflowString::new("{{ url }}");
        let result = s.parse(&runner).await.unwrap();
        assert_eq!(result, "https://example.com/path");
    }

    fn default_config() -> Config {
        Config::default()
    }

    async fn make_test_runner() -> RunningWorkflow {
        RunningWorkflow::new(
            crate::workflow_data::WorkflowData {
                name: "Test".to_string(),
                description: None,
                procedures: vec![],
                steps: vec![],
            },
            Box::new(default_config()),
        )
    }

    async fn make_test_runner_with_url(url: &str) -> RunningWorkflow {
        let runner = make_test_runner().await;
        runner.browser.lock().await.navigate(url).await.ok();
        runner
    }
}
