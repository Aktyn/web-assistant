use web_assistant::agent::judge::{construct_judge_messages, parse_judgement_response};

#[test]
fn test_construct_judge_messages_with_and_without_ground_truth() {
    let steps = vec!["Step 1: Navigated to site".to_string()];

    let msgs_without_gt = construct_judge_messages("Search something", "Done", &steps, None);
    assert_eq!(msgs_without_gt.len(), 2);
    assert!(!msgs_without_gt[1].content.contains("<ground_truth>"));

    let msgs_with_gt = construct_judge_messages(
        "Search something",
        "Done",
        &steps,
        Some("Must extract 5 items"),
    );
    assert_eq!(msgs_with_gt.len(), 2);
    assert!(msgs_with_gt[1].content.contains("<ground_truth>"));
    assert!(msgs_with_gt[1].content.contains("Must extract 5 items"));
    assert!(msgs_with_gt[0].content.contains("GROUND TRUTH VALIDATION"));
}

#[test]
fn test_parse_judgement_various_formats() {
    let raw_clean = r#"{
        "reasoning": "Completed all points",
        "verdict": true,
        "failure_reason": "",
        "impossible_task": false,
        "reached_captcha": false
    }"#;
    let res = parse_judgement_response(raw_clean).unwrap();
    assert!(res.verdict);
    assert!(!res.impossible_task);

    let raw_markdown = r#"```json
    {
        "reasoning": "Hit captcha on target page",
        "verdict": false,
        "failure_reason": "Blocked by Cloudflare challenge",
        "impossible_task": false,
        "reached_captcha": true
    }
    ```"#;
    let res_md = parse_judgement_response(raw_markdown).unwrap();
    assert!(!res_md.verdict);
    assert!(res_md.reached_captcha);
    assert_eq!(res_md.failure_reason, "Blocked by Cloudflare challenge");
}
