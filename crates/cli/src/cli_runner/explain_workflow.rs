use std::{
    io::{Write as _, stdout},
    path::PathBuf,
    sync::LazyLock,
};

use anyhow::Result;

use crate::common::print_struct_docs;
use utils::logger::print_lines;
use workflow::workflow_data::{
    WorkflowData, WorkflowProcedure, step as workflow_step, workflow_string::WorkflowString,
};

pub(crate) fn explain_workflow(include_example: bool) -> Result<()> {
    print_struct_docs!(WorkflowData)?;
    print_lines(1);
    print_struct_docs!(WorkflowProcedure, 2)?;
    print_lines(1);

    print_struct_docs!(workflow_step::PageActionStep, 2)?;
    print_lines(1);
    print_struct_docs!(workflow_step::page_action::NavigateAction, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::page_action::InputAction, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::page_action::ClickAction, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::page_action::EvaluateAction, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::AgentStep, 2)?;
    print_lines(1);
    print_struct_docs!(workflow_step::ConditionStep, 2)?;
    print_lines(1);
    print_struct_docs!(workflow_step::condition::MatchRegexCondition, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::condition::TruthyCondition, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::condition::EqualCondition, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::condition::HasElementCondition, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::ForEachStep, 2)?;
    print_lines(1);
    print_struct_docs!(workflow_step::iteration::FileIterator, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::RunProcedureStep, 2)?;
    print_lines(1);
    print_struct_docs!(workflow_step::UpdateContextStep, 2)?;
    print_lines(1);
    print_struct_docs!(workflow_step::SystemActionStep, 2)?;
    print_lines(1);
    print_struct_docs!(workflow_step::system_action::NotifyAction, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::system_action::OpenUrlAction, 4)?;
    print_lines(1);
    print_struct_docs!(workflow_step::WaitStep, 2)?;
    print_lines(1);
    print_struct_docs!(workflow_step::AbortStep, 2)?;

    if include_example {
        print_lines(2);

        print_example()?;
    } else {
        print_lines(1);
        stdout()
            .write_all(b"\nUse --include-example argument to print an example workflow file\n")
            .ok();
    }

    Ok(())
}

static EXAMPLE_JSON: LazyLock<Result<String, serde_json::Error>> = LazyLock::new(|| {
    serde_json::to_string_pretty(&WorkflowData {
        name: String::from("Complete example"),
        description: Some(String::from(
            "A comprehensive example workflow demonstrating all step types and their parameters",
        )),
        procedures: vec![WorkflowProcedure {
            name: String::from("main-procedure"),
            steps: vec![
                workflow_step::WorkflowStepType::PageAction(workflow_step::PageActionStep {
                    page_index: 0,
                    action: workflow_step::PageAction::Navigate(
                        workflow_step::page_action::NavigateAction {
                            url: WorkflowString::from("https://example.com"),
                        },
                    ),
                }),
                workflow_step::WorkflowStepType::PageAction(workflow_step::PageActionStep {
                    page_index: 0,
                    action: workflow_step::PageAction::Input(
                        workflow_step::page_action::InputAction {
                            selector: WorkflowString::from("#username"),
                            value: WorkflowString::from("myusername"),
                        },
                    ),
                }),
                workflow_step::WorkflowStepType::PageAction(workflow_step::PageActionStep {
                    page_index: 0,
                    action: workflow_step::PageAction::Click(
                        workflow_step::page_action::ClickAction {
                            selector: WorkflowString::from("#submit-btn"),
                            wait_for_navigation: true,
                        },
                    ),
                }),
                workflow_step::WorkflowStepType::PageAction(workflow_step::PageActionStep {
                    page_index: 0,
                    action: workflow_step::PageAction::Evaluate(
                        workflow_step::page_action::EvaluateAction {
                            script: String::from("return document.title;"),
                            context_name: Some(String::from("pageTitle")),
                        },
                    ),
                }),
                workflow_step::WorkflowStepType::Agent(workflow_step::AgentStep {
                    task: WorkflowString::from("Extract all product names from the page"),
                    max_steps: 10,
                    history_output: Some(PathBuf::from("./agent-run.json")),
                    prompt_mode: agent::PromptMode::Standard,
                }),
                workflow_step::WorkflowStepType::Condition(workflow_step::ConditionStep {
                    condition: workflow_step::condition::Condition::MatchRegex(
                        workflow_step::condition::MatchRegexCondition {
                            value: WorkflowString::from("{{ url.hostname }}"),
                            pattern: String::from("example\\.com"),
                        },
                    ),
                    then: vec![workflow_step::WorkflowStepType::PageAction(
                        workflow_step::PageActionStep {
                            page_index: 0,
                            action: workflow_step::PageAction::Navigate(
                                workflow_step::page_action::NavigateAction {
                                    url: WorkflowString::from("https://new-example.com"),
                                },
                            ),
                        },
                    )],
                    else_: vec![workflow_step::WorkflowStepType::Abort(
                        workflow_step::AbortStep {
                            message: WorkflowString::from("Unexpected hostname"),
                        },
                    )],
                }),
                workflow_step::WorkflowStepType::Condition(workflow_step::ConditionStep {
                    condition: workflow_step::condition::Condition::Truthy(
                        workflow_step::condition::TruthyCondition {
                            value: WorkflowString::from("{{ context.loggedIn }}"),
                        },
                    ),
                    then: vec![],
                    else_: vec![workflow_step::WorkflowStepType::Wait(
                        workflow_step::WaitStep { duration: 5000 },
                    )],
                }),
                workflow_step::WorkflowStepType::Condition(workflow_step::ConditionStep {
                    condition: workflow_step::condition::Condition::Equal(
                        workflow_step::condition::EqualCondition {
                            left: WorkflowString::from("{{ context.counter }}"),
                            right: WorkflowString::from("5"),
                        },
                    ),
                    then: vec![workflow_step::WorkflowStepType::SystemAction(
                        workflow_step::SystemActionStep {
                            action: workflow_step::SystemAction::Notify(
                                workflow_step::system_action::NotifyAction {
                                    message: WorkflowString::from("Counter reached 5!"),
                                },
                            ),
                        },
                    )],
                    else_: vec![],
                }),
                workflow_step::WorkflowStepType::Condition(workflow_step::ConditionStep {
                    condition: workflow_step::condition::Condition::HasElement(
                        workflow_step::condition::HasElementCondition {
                            selector: WorkflowString::from("#login-form"),
                        },
                    ),
                    then: vec![],
                    else_: vec![],
                }),
                workflow_step::WorkflowStepType::ForEach(workflow_step::iteration::ForEachStep {
                    context_name: String::from("items"),
                    items: workflow_step::iteration::IterationItems::File(
                        workflow_step::iteration::FileIterator {
                            source_file: PathBuf::from("./data/items.txt"),
                        },
                    ),
                    preserve_context: false,
                    steps: vec![workflow_step::WorkflowStepType::PageAction(
                        workflow_step::PageActionStep {
                            page_index: 0,
                            action: workflow_step::PageAction::Navigate(
                                workflow_step::page_action::NavigateAction {
                                    url: WorkflowString::from("https://example.com"),
                                },
                            ),
                        },
                    )],
                    on_error: None,
                }),
                workflow_step::WorkflowStepType::UpdateContext(workflow_step::UpdateContextStep {
                    key: String::from("lastUpdated"),
                    value: Some(WorkflowString::from("{{ currentTime }}")),
                }),
                workflow_step::WorkflowStepType::SystemAction(workflow_step::SystemActionStep {
                    action: workflow_step::SystemAction::OpenUrl(
                        workflow_step::system_action::OpenUrlAction {
                            url: WorkflowString::from("https://example.com/thank-you"),
                        },
                    ),
                }),
                workflow_step::WorkflowStepType::SystemAction(workflow_step::SystemActionStep {
                    action: workflow_step::SystemAction::Notify(
                        workflow_step::system_action::NotifyAction {
                            message: WorkflowString::from("Workflow completed successfully"),
                        },
                    ),
                }),
                workflow_step::WorkflowStepType::Wait(workflow_step::WaitStep { duration: 2000 }),
                workflow_step::WorkflowStepType::Abort(workflow_step::AbortStep {
                    message: WorkflowString::from("Critical error, aborting"),
                }),
            ],
        }],
        steps: vec![
            workflow_step::WorkflowStepType::PageAction(workflow_step::PageActionStep {
                page_index: 0,
                action: workflow_step::PageAction::Navigate(
                    workflow_step::page_action::NavigateAction {
                        url: WorkflowString::from("https://github.com"),
                    },
                ),
            }),
            workflow_step::WorkflowStepType::RunProcedure(workflow_step::RunProcedureStep {
                procedure: String::from("main-procedure"),
            }),
        ],
    })
    .map(|json| json.replace('\n', "\r\n"))
});

fn print_example() -> Result<()> {
    if let Ok(json) = EXAMPLE_JSON.as_ref() {
        let output = format!("Example workflow json:\r\n{json}");
        stdout().write_all(output.as_bytes()).ok();
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to serialize example workflow"))
    }
}
