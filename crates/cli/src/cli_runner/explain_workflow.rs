use std::{
    io::{Write as _, stdout},
    path::PathBuf,
    sync::LazyLock,
};

use anyhow::Result;
use strum::IntoEnumIterator;

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

    print_lines(2);
    stdout()
        .write_all(b"Some string properties accept dynamic values:\n")
        .ok();
    print_struct_docs!(WorkflowString)?;

    if include_example {
        print_lines(2);
        print_example();
    } else {
        print_lines(1);
        stdout()
            .write_all(b"\nUse --include-example argument to print an example workflow file\n")
            .ok();
    }

    Ok(())
}

pub(crate) static EXAMPLE_WORKFLOW_DATA: LazyLock<WorkflowData> = LazyLock::new(|| WorkflowData {
    name: String::from("Complete example"),
    description: Some(String::from(
        "A comprehensive example workflow demonstrating all step types and their parameters",
    )),
    procedures: vec![WorkflowProcedure {
        name: String::from("main-procedure"),
        steps: main_procedure_steps(),
    }],
    steps: top_level_steps(),
    network_connection_timeout_sec: 600,
});

fn main_procedure_steps() -> Vec<workflow_step::WorkflowStepType> {
    let mut steps = Vec::new();

    steps.extend(
        workflow_step::page_action::PageActionDiscriminants::iter().map(|action| {
            workflow_step::WorkflowStepType::PageAction(workflow_step::PageActionStep {
                page_index: 0,
                action: example_page_action(action),
            })
        }),
    );
    steps.push(example_agent_step());
    steps.extend(
        workflow_step::condition::ConditionDiscriminants::iter().map(|condition| {
            workflow_step::WorkflowStepType::Condition(example_condition_step(condition))
        }),
    );
    steps.extend(example_foreach_steps());
    steps.push(example_update_context_step());
    steps.extend(
        workflow_step::system_action::SystemActionDiscriminants::iter().map(|action| {
            workflow_step::WorkflowStepType::SystemAction(workflow_step::SystemActionStep {
                action: example_system_action(action),
            })
        }),
    );
    steps.push(workflow_step::WorkflowStepType::Wait(
        workflow_step::WaitStep { duration: 2000 },
    ));
    steps.push(workflow_step::WorkflowStepType::Abort(
        workflow_step::AbortStep {
            message: WorkflowString::from("Critical error, aborting"),
        },
    ));

    steps
}

fn example_agent_step() -> workflow_step::WorkflowStepType {
    workflow_step::WorkflowStepType::Agent(workflow_step::AgentStep {
        task: WorkflowString::from("Extract all product names from the page"),
        max_steps: 10,
        history_output: Some(PathBuf::from("./agent-run.json")),
        prompt_mode: agent::PromptMode::Standard,
    })
}

fn example_page_action(
    action: workflow_step::page_action::PageActionDiscriminants,
) -> workflow_step::PageAction {
    match action {
        workflow_step::page_action::PageActionDiscriminants::Navigate => {
            workflow_step::PageAction::Navigate(workflow_step::page_action::NavigateAction {
                url: WorkflowString::from("https://example.com"),
            })
        }
        workflow_step::page_action::PageActionDiscriminants::Input => {
            workflow_step::PageAction::Input(workflow_step::page_action::InputAction {
                selector: WorkflowString::from("#username"),
                value: WorkflowString::from("myusername"),
            })
        }
        workflow_step::page_action::PageActionDiscriminants::Click => {
            workflow_step::PageAction::Click(workflow_step::page_action::ClickAction {
                selector: WorkflowString::from("#submit-btn"),
                wait_for_navigation: true,
            })
        }
        workflow_step::page_action::PageActionDiscriminants::Evaluate => {
            workflow_step::PageAction::Evaluate(workflow_step::page_action::EvaluateAction {
                script: String::from("return document.title;"),
                context_name: Some(String::from("pageTitle")),
            })
        }
    }
}

fn example_condition_step(
    condition: workflow_step::condition::ConditionDiscriminants,
) -> workflow_step::ConditionStep {
    match condition {
        workflow_step::condition::ConditionDiscriminants::MatchRegex => {
            workflow_step::ConditionStep {
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
            }
        }
        workflow_step::condition::ConditionDiscriminants::Truthy => workflow_step::ConditionStep {
            condition: workflow_step::condition::Condition::Truthy(
                workflow_step::condition::TruthyCondition {
                    value: WorkflowString::from("{{ context.loggedIn }}"),
                },
            ),
            then: vec![],
            else_: vec![workflow_step::WorkflowStepType::Wait(
                workflow_step::WaitStep { duration: 5000 },
            )],
        },
        workflow_step::condition::ConditionDiscriminants::Equal => workflow_step::ConditionStep {
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
        },
        workflow_step::condition::ConditionDiscriminants::HasElement => {
            workflow_step::ConditionStep {
                condition: workflow_step::condition::Condition::HasElement(
                    workflow_step::condition::HasElementCondition {
                        selector: WorkflowString::from("#login-form"),
                    },
                ),
                then: vec![],
                else_: vec![],
            }
        }
    }
}

fn example_foreach_steps() -> Vec<workflow_step::WorkflowStepType> {
    workflow_step::iteration::IterationItemsDiscriminants::iter()
        .map(|items| {
            workflow_step::WorkflowStepType::ForEach(workflow_step::iteration::ForEachStep {
                context_name: String::from("items"),
                items: example_iteration_items(items),
                preserve_context: false,
                order: workflow_step::iteration::IterationOrder::Random,
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
            })
        })
        .collect()
}

fn example_iteration_items(
    items: workflow_step::iteration::IterationItemsDiscriminants,
) -> workflow_step::iteration::IterationItems {
    match items {
        workflow_step::iteration::IterationItemsDiscriminants::File => {
            workflow_step::iteration::IterationItems::File(workflow_step::iteration::FileIterator {
                source_file: PathBuf::from("./data/items.txt"),
            })
        }
    }
}

fn example_update_context_step() -> workflow_step::WorkflowStepType {
    workflow_step::WorkflowStepType::UpdateContext(workflow_step::UpdateContextStep {
        key: String::from("lastUpdated"),
        value: Some(WorkflowString::from("{{ currentTime }}")),
    })
}

fn example_system_action(
    action: workflow_step::system_action::SystemActionDiscriminants,
) -> workflow_step::SystemAction {
    match action {
        workflow_step::system_action::SystemActionDiscriminants::OpenUrl => {
            workflow_step::SystemAction::OpenUrl(workflow_step::system_action::OpenUrlAction {
                url: WorkflowString::from("https://example.com/thank-you"),
            })
        }
        workflow_step::system_action::SystemActionDiscriminants::Notify => {
            workflow_step::SystemAction::Notify(workflow_step::system_action::NotifyAction {
                message: WorkflowString::from("Workflow completed successfully"),
            })
        }
    }
}

fn top_level_steps() -> Vec<workflow_step::WorkflowStepType> {
    // Only PageAction and RunProcedure appear at the top level; every other
    // step type is already demonstrated inside the main procedure, but it
    // must still be matched explicitly so a new variant breaks compilation.
    workflow_step::WorkflowStepTypeDiscriminants::iter()
        .filter_map(|step_type| match step_type {
            workflow_step::WorkflowStepTypeDiscriminants::PageAction => Some(
                workflow_step::WorkflowStepType::PageAction(workflow_step::PageActionStep {
                    page_index: 0,
                    action: workflow_step::PageAction::Navigate(
                        workflow_step::page_action::NavigateAction {
                            url: WorkflowString::from("https://github.com"),
                        },
                    ),
                }),
            ),
            workflow_step::WorkflowStepTypeDiscriminants::RunProcedure => Some(
                workflow_step::WorkflowStepType::RunProcedure(workflow_step::RunProcedureStep {
                    procedure: String::from("main-procedure"),
                }),
            ),
            workflow_step::WorkflowStepTypeDiscriminants::Agent
            | workflow_step::WorkflowStepTypeDiscriminants::SystemAction
            | workflow_step::WorkflowStepTypeDiscriminants::Condition
            | workflow_step::WorkflowStepTypeDiscriminants::ForEach
            | workflow_step::WorkflowStepTypeDiscriminants::UpdateContext
            | workflow_step::WorkflowStepTypeDiscriminants::Wait
            | workflow_step::WorkflowStepTypeDiscriminants::Abort => None,
        })
        .collect()
}

fn print_example() {
    let mut logger = utils::logger::CliLogger::default();
    logger.json(&*EXAMPLE_WORKFLOW_DATA);
}
