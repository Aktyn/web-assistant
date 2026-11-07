use regex::Regex;
use std::collections::{HashMap, HashSet};

use super::views::{Action, AgentHistoryList, DetectedVariable, InteractedElementMetadata};

pub fn detect_variables_in_history(
    history: &AgentHistoryList,
) -> HashMap<String, DetectedVariable> {
    let mut detected: HashMap<String, DetectedVariable> = HashMap::new();
    let mut detected_values: HashSet<String> = HashSet::new();

    for history_item in &history.history {
        let model_output = match &history_item.model_output {
            Some(out) => out,
            None => continue,
        };

        for (action_idx, action) in model_output.action.iter().enumerate() {
            let element = history_item
                .state
                .as_ref()
                .and_then(|s| s.interacted_element.as_ref())
                .and_then(|elements| elements.get(action_idx));

            detect_in_action(action, element, &mut detected, &mut detected_values);
        }
    }

    detected
}

fn detect_in_action(
    action: &Action,
    element: Option<&InteractedElementMetadata>,
    detected: &mut HashMap<String, DetectedVariable>,
    detected_values: &mut HashSet<String>,
) {
    let mut values_to_check = Vec::new();

    match action {
        Action::Input { text, .. } => {
            values_to_check.push(text.clone());
        }
        Action::Extract { query } => {
            values_to_check.push(query.clone());
        }
        Action::Custom(val) => {
            collect_strings_from_json(val, &mut values_to_check);
        }
        _ => {}
    }

    for value in values_to_check {
        let trimmed = value.trim();
        if trimmed.is_empty() || detected_values.contains(trimmed) {
            continue;
        }

        if let Some((var_name, var_format)) = detect_variable_type(trimmed, element) {
            let unique_name = ensure_unique_name(&var_name, detected);
            detected.insert(
                unique_name.clone(),
                DetectedVariable {
                    name: unique_name,
                    original_value: trimmed.to_string(),
                    r#type: "string".to_string(),
                    format: var_format,
                },
            );
            detected_values.insert(trimmed.to_string());
        }
    }
}

fn collect_strings_from_json(val: &serde_json::Value, out: &mut Vec<String>) {
    match val {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                if k == "text" || k == "query" {
                    if let Some(s) = v.as_str() {
                        out.push(s.to_string());
                    }
                } else {
                    collect_strings_from_json(v, out);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_strings_from_json(v, out);
            }
        }
        _ => {}
    }
}

pub fn detect_variable_type(
    value: &str,
    element: Option<&InteractedElementMetadata>,
) -> Option<(String, Option<String>)> {
    if let Some(elem) = element {
        if !elem.attributes.is_empty() {
            if let Some(res) = detect_from_attributes(&elem.attributes) {
                return Some(res);
            }
        }
    }

    detect_from_value_pattern(value)
}

pub fn detect_from_attributes(
    attributes: &HashMap<String, String>,
) -> Option<(String, Option<String>)> {
    let input_type = attributes
        .get("type")
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    match input_type.as_str() {
        "email" => return Some(("email".into(), Some("email".into()))),
        "tel" => return Some(("phone".into(), Some("phone".into()))),
        "date" => return Some(("date".into(), Some("date".into()))),
        "number" => return Some(("number".into(), Some("number".into()))),
        "url" => return Some(("url".into(), Some("url".into()))),
        _ => {}
    }

    let semantic_attrs: Vec<&str> = ["id", "name", "placeholder", "aria-label"]
        .iter()
        .filter_map(|key| attributes.get(*key).map(|s| s.as_str()))
        .collect();

    let combined_text = semantic_attrs.join(" ").to_lowercase();

    if combined_text.contains("address")
        || combined_text.contains("street")
        || combined_text.contains("addr")
    {
        if combined_text.contains("billing") {
            return Some(("billing_address".into(), None));
        } else if combined_text.contains("shipping") {
            return Some(("shipping_address".into(), None));
        } else {
            return Some(("address".into(), None));
        }
    }

    if combined_text.contains("comment")
        || combined_text.contains("note")
        || combined_text.contains("message")
        || combined_text.contains("description")
    {
        return Some(("comment".into(), None));
    }

    if combined_text.contains("email") || combined_text.contains("e-mail") {
        return Some(("email".into(), Some("email".into())));
    }

    if combined_text.contains("phone")
        || combined_text.contains("tel")
        || combined_text.contains("mobile")
        || combined_text.contains("cell")
    {
        return Some(("phone".into(), Some("phone".into())));
    }

    if combined_text.contains("company") || combined_text.contains("organization") {
        return Some(("company".into(), None));
    }

    if combined_text.contains("first") && combined_text.contains("name") {
        return Some(("first_name".into(), None));
    } else if combined_text.contains("last") && combined_text.contains("name") {
        return Some(("last_name".into(), None));
    } else if combined_text.contains("full") && combined_text.contains("name") {
        return Some(("full_name".into(), None));
    } else if combined_text.contains("name") {
        return Some(("name".into(), None));
    }

    if combined_text.contains("date")
        || combined_text.contains("dob")
        || combined_text.contains("birth")
    {
        return Some(("date".into(), Some("date".into())));
    }

    if combined_text.contains("city") {
        return Some(("city".into(), None));
    }

    if combined_text.contains("state") || combined_text.contains("province") {
        return Some(("state".into(), None));
    }

    if combined_text.contains("country") {
        return Some(("country".into(), None));
    }

    if combined_text.contains("zip")
        || combined_text.contains("postal")
        || combined_text.contains("postcode")
    {
        return Some(("zip_code".into(), Some("postal_code".into())));
    }

    None
}

pub fn detect_from_value_pattern(value: &str) -> Option<(String, Option<String>)> {
    if value.contains('@') && value.contains('.') {
        let email_regex = Regex::new(r"^[\w\.-]+@[\w\.-]+\.\w+$").unwrap();
        if email_regex.is_match(value) {
            return Some(("email".into(), Some("email".into())));
        }
    }

    let phone_chars_regex = Regex::new(r"^[\d\s\-\(\)\+]+$").unwrap();
    if phone_chars_regex.is_match(value) {
        let digits_only: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits_only.len() >= 10 {
            return Some(("phone".into(), Some("phone".into())));
        }
    }

    let date_regex = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    if date_regex.is_match(value) {
        return Some(("date".into(), Some("date".into())));
    }

    if !value.is_empty()
        && value.chars().next().map_or(false, |c| c.is_uppercase())
        && value
            .chars()
            .all(|c| c.is_alphabetic() || c == ' ' || c == '-')
        && value.len() >= 2
        && value.len() <= 30
    {
        let words: Vec<&str> = value.split_whitespace().collect();
        return match words.len() {
            1 => Some(("first_name".into(), None)),
            2 => Some(("full_name".into(), None)),
            _ => Some(("name".into(), None)),
        };
    }

    if value.chars().all(|c| c.is_ascii_digit()) && !value.is_empty() && value.len() <= 9 {
        return Some(("number".into(), Some("number".into())));
    }

    None
}

fn ensure_unique_name(base_name: &str, existing: &HashMap<String, DetectedVariable>) -> String {
    if !existing.contains_key(base_name) {
        return base_name.to_string();
    }

    let mut counter = 2;
    loop {
        let candidate = format!("{base_name}_{counter}");
        if !existing.contains_key(&candidate) {
            return candidate;
        }
        counter += 1;
    }
}

pub fn substitute_variables_in_history(
    history: &AgentHistoryList,
    variables: &HashMap<String, String>,
) -> AgentHistoryList {
    let detected = detect_variables_in_history(history);

    let mut value_replacements: HashMap<String, String> = HashMap::new();
    for (var_name, new_val) in variables {
        if let Some(detected_var) = detected.get(var_name) {
            value_replacements.insert(detected_var.original_value.clone(), new_val.clone());
        }
    }

    if value_replacements.is_empty() {
        return history.clone();
    }

    let mut modified = history.clone();
    for item in &mut modified.history {
        if let Some(ref mut output) = item.model_output {
            for action in &mut output.action {
                substitute_in_action(action, &value_replacements);
            }
        }
    }

    modified
}

fn substitute_in_action(action: &mut Action, replacements: &HashMap<String, String>) {
    match action {
        Action::Input { text, .. } => {
            if let Some(new_val) = replacements.get(text) {
                *text = new_val.clone();
            }
        }
        Action::Extract { query } => {
            if let Some(new_val) = replacements.get(query) {
                *query = new_val.clone();
            }
        }
        Action::Custom(val) => {
            substitute_in_json(val, replacements);
        }
        _ => {}
    }
}

fn substitute_in_json(val: &mut serde_json::Value, replacements: &HashMap<String, String>) {
    match val {
        serde_json::Value::String(s) => {
            if let Some(new_val) = replacements.get(s) {
                *s = new_val.clone();
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                substitute_in_json(item, replacements);
            }
        }
        serde_json::Value::Object(map) => {
            for (_k, v) in map {
                substitute_in_json(v, replacements);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::views::{AgentHistory, AgentOutput, AgentStepState};

    #[test]
    fn test_variable_detection_by_element_attribute() {
        let mut attrs = HashMap::new();
        attrs.insert("type".into(), "email".into());

        let element = InteractedElementMetadata {
            index: 1,
            tag: "input".into(),
            attributes: attrs,
        };

        let action = Action::Input {
            index: 1,
            text: "user@example.com".into(),
        };

        let history = AgentHistoryList {
            history: vec![AgentHistory {
                model_output: Some(AgentOutput {
                    thought: None,
                    memory: None,
                    next_goal: None,
                    action: vec![action],
                }),
                result: vec![],
                state: Some(AgentStepState {
                    url: "https://example.com".into(),
                    title: "Test".into(),
                    interacted_element: Some(vec![element]),
                }),
            }],
        };

        let vars = detect_variables_in_history(&history);
        assert_eq!(vars.len(), 1);
        let email_var = vars.get("email").unwrap();
        assert_eq!(email_var.original_value, "user@example.com");
        assert_eq!(email_var.format.as_deref(), Some("email"));
    }

    #[test]
    fn test_variable_detection_by_pattern() {
        let action = Action::Input {
            index: 1,
            text: "john.doe@test.com".into(),
        };

        let history = AgentHistoryList {
            history: vec![AgentHistory {
                model_output: Some(AgentOutput {
                    action: vec![action],
                    ..Default::default()
                }),
                ..Default::default()
            }],
        };

        let vars = detect_variables_in_history(&history);
        assert_eq!(vars.len(), 1);
        assert!(vars.contains_key("email"));
    }

    #[test]
    fn test_variable_substitution() {
        let action1 = Action::Input {
            index: 1,
            text: "old@example.com".into(),
        };
        let action2 = Action::Input {
            index: 2,
            text: "John Doe".into(),
        };

        let mut email_attrs = HashMap::new();
        email_attrs.insert("type".into(), "email".into());

        let mut name_attrs = HashMap::new();
        name_attrs.insert("name".into(), "full_name".into());

        let history = AgentHistoryList {
            history: vec![
                AgentHistory {
                    model_output: Some(AgentOutput {
                        action: vec![action1],
                        ..Default::default()
                    }),
                    state: Some(AgentStepState {
                        interacted_element: Some(vec![InteractedElementMetadata {
                            index: 1,
                            tag: "input".into(),
                            attributes: email_attrs,
                        }]),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                AgentHistory {
                    model_output: Some(AgentOutput {
                        action: vec![action2],
                        ..Default::default()
                    }),
                    state: Some(AgentStepState {
                        interacted_element: Some(vec![InteractedElementMetadata {
                            index: 2,
                            tag: "input".into(),
                            attributes: name_attrs,
                        }]),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ],
        };

        let mut subs = HashMap::new();
        subs.insert("email".into(), "new@example.com".into());
        subs.insert("full_name".into(), "Jane Smith".into());

        let new_hist = substitute_variables_in_history(&history, &subs);
        let first_act = &new_hist.history[0].model_output.as_ref().unwrap().action[0];
        let second_act = &new_hist.history[1].model_output.as_ref().unwrap().action[0];

        match first_act {
            Action::Input { text, .. } => assert_eq!(text, "new@example.com"),
            _ => panic!("Expected Input action"),
        }

        match second_act {
            Action::Input { text, .. } => assert_eq!(text, "Jane Smith"),
            _ => panic!("Expected Input action"),
        }
    }
}
