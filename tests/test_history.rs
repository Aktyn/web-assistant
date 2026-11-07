use web_assistant::agent::views::{
    Action, ActionResult, AgentHistory, AgentHistoryList, AgentOutput, AgentStepState,
};

#[test]
fn test_history_file_roundtrip() {
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let path = temp_file.path();

    let history = AgentHistoryList {
        history: vec![
            AgentHistory {
                model_output: Some(AgentOutput {
                    thought: Some("Navigating to repo".into()),
                    memory: None,
                    next_goal: Some("Go to github".into()),
                    action: vec![Action::Navigate {
                        url: "https://github.com/browser-use/browser-use".into(),
                    }],
                }),
                result: vec![ActionResult::success("Navigated")],
                state: Some(AgentStepState {
                    url: "https://github.com/browser-use/browser-use".into(),
                    title: "GitHub - browser-use".into(),
                    interacted_element: None,
                }),
            },
            AgentHistory {
                model_output: Some(AgentOutput {
                    thought: Some("Stars found".into()),
                    memory: None,
                    next_goal: None,
                    action: vec![Action::Done {
                        text: "55.4k stars".into(),
                        success: true,
                    }],
                }),
                result: vec![ActionResult::done("55.4k stars", true)],
                state: Some(AgentStepState {
                    url: "https://github.com/browser-use/browser-use".into(),
                    title: "GitHub - browser-use".into(),
                    interacted_element: None,
                }),
            },
        ],
    };

    assert!(history.is_done());
    assert_eq!(history.is_successful(), Some(true));
    assert_eq!(history.final_result().as_deref(), Some("55.4k stars"));

    history.save_to_file(path).unwrap();

    let loaded = AgentHistoryList::load_from_file(path).unwrap();
    assert_eq!(loaded.len(), 2);
    assert!(loaded.is_done());
    assert_eq!(loaded.is_successful(), Some(true));
    assert_eq!(loaded.final_result().as_deref(), Some("55.4k stars"));
}
