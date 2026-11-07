use std::collections::HashMap;
use web_assistant::agent::views::{
    Action, AgentHistory, AgentHistoryList, AgentOutput, AgentStepState, InteractedElementMetadata,
};
use web_assistant::agent::{detect_variables_in_history, substitute_variables_in_history};

#[test]
fn test_all_input_types_detection() {
    let types = [
        ("email", "test@example.com", "email", Some("email")),
        ("tel", "+1234567890", "phone", Some("phone")),
        ("date", "2025-01-01", "date", Some("date")),
        ("number", "42", "number", Some("number")),
        ("url", "https://example.com", "url", Some("url")),
    ];

    for (input_type, val, expected_name, expected_fmt) in types {
        let mut attrs = HashMap::new();
        attrs.insert("type".into(), input_type.into());

        let history = AgentHistoryList {
            history: vec![AgentHistory {
                model_output: Some(AgentOutput {
                    action: vec![Action::Input {
                        index: 1,
                        text: val.into(),
                    }],
                    ..Default::default()
                }),
                state: Some(AgentStepState {
                    interacted_element: Some(vec![InteractedElementMetadata {
                        index: 1,
                        tag: "input".into(),
                        attributes: attrs,
                    }]),
                    ..Default::default()
                }),
                ..Default::default()
            }],
        };

        let vars = detect_variables_in_history(&history);
        assert!(
            vars.contains_key(expected_name),
            "Expected variable {} for type {}",
            expected_name,
            input_type
        );
        let var = vars.get(expected_name).unwrap();
        assert_eq!(var.original_value, val);
        assert_eq!(var.format.as_deref(), expected_fmt);
    }
}

#[test]
fn test_semantic_attributes_detection() {
    let cases = [
        ("shipping_street_addr", "123 Main St", "shipping_address"),
        ("billing_address_line1", "456 Oak Ave", "billing_address"),
        ("customer_first_name", "Alice", "first_name"),
        ("customer_last_name", "Smith", "last_name"),
        ("postal_code", "90210", "zip_code"),
        ("company_name", "Acme Corp", "company"),
        ("order_comment", "Leave at porch", "comment"),
    ];

    for (id_attr, val, expected_name) in cases {
        let mut attrs = HashMap::new();
        attrs.insert("id".into(), id_attr.into());

        let history = AgentHistoryList {
            history: vec![AgentHistory {
                model_output: Some(AgentOutput {
                    action: vec![Action::Input {
                        index: 1,
                        text: val.into(),
                    }],
                    ..Default::default()
                }),
                state: Some(AgentStepState {
                    interacted_element: Some(vec![InteractedElementMetadata {
                        index: 1,
                        tag: "input".into(),
                        attributes: attrs,
                    }]),
                    ..Default::default()
                }),
                ..Default::default()
            }],
        };

        let vars = detect_variables_in_history(&history);
        assert!(
            vars.contains_key(expected_name),
            "Expected {} for attr {}",
            expected_name,
            id_attr
        );
    }
}

#[test]
fn test_duplicate_variable_naming() {
    let mut attrs1 = HashMap::new();
    attrs1.insert("name".into(), "first_name".into());

    let mut attrs2 = HashMap::new();
    attrs2.insert("name".into(), "first_name".into());

    let history = AgentHistoryList {
        history: vec![
            AgentHistory {
                model_output: Some(AgentOutput {
                    action: vec![Action::Input {
                        index: 1,
                        text: "Bob".into(),
                    }],
                    ..Default::default()
                }),
                state: Some(AgentStepState {
                    interacted_element: Some(vec![InteractedElementMetadata {
                        index: 1,
                        tag: "input".into(),
                        attributes: attrs1,
                    }]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            AgentHistory {
                model_output: Some(AgentOutput {
                    action: vec![Action::Input {
                        index: 2,
                        text: "Charlie".into(),
                    }],
                    ..Default::default()
                }),
                state: Some(AgentStepState {
                    interacted_element: Some(vec![InteractedElementMetadata {
                        index: 2,
                        tag: "input".into(),
                        attributes: attrs2,
                    }]),
                    ..Default::default()
                }),
                ..Default::default()
            },
        ],
    };

    let vars = detect_variables_in_history(&history);
    assert_eq!(vars.len(), 2);
    assert!(vars.contains_key("first_name"));
    assert!(vars.contains_key("first_name_2"));
    assert_eq!(vars.get("first_name").unwrap().original_value, "Bob");
    assert_eq!(vars.get("first_name_2").unwrap().original_value, "Charlie");
}

#[test]
fn test_substitute_variables_partial_and_nested() {
    let history = AgentHistoryList {
        history: vec![AgentHistory {
            model_output: Some(AgentOutput {
                action: vec![
                    Action::Input {
                        index: 1,
                        text: "user@example.com".into(),
                    },
                    Action::Input {
                        index: 2,
                        text: "1990-05-20".into(),
                    },
                    Action::Custom(serde_json::json!({
                        "query": "user@example.com",
                        "nested": { "text": "user@example.com" }
                    })),
                ],
                ..Default::default()
            }),
            ..Default::default()
        }],
    };

    let mut subs = HashMap::new();
    subs.insert("email".into(), "newuser@domain.org".into());

    let modified = substitute_variables_in_history(&history, &subs);
    let actions = &modified.history[0].model_output.as_ref().unwrap().action;

    match &actions[0] {
        Action::Input { text, .. } => assert_eq!(text, "newuser@domain.org"),
        _ => panic!("Expected Input"),
    }
    // Date was not substituted
    match &actions[1] {
        Action::Input { text, .. } => assert_eq!(text, "1990-05-20"),
        _ => panic!("Expected Input"),
    }
    // Nested json values were substituted
    match &actions[2] {
        Action::Custom(val) => {
            assert_eq!(val["query"], "newuser@domain.org");
            assert_eq!(val["nested"]["text"], "newuser@domain.org");
        }
        _ => panic!("Expected Custom"),
    }
}
