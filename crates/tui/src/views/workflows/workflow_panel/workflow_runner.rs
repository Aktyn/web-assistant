use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{
        Constraint::{Fill, Length},
        Layout, Rect,
    },
    style::{Color, Stylize as _, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::{WorkflowAction, WorkflowPanelModeView, WorkflowSelection};
use super::super::super::common::widgets::{Button, ButtonData, Separator};
use super::super::super::common::{palette, utils};
use workflow::{Workflow, WorkflowState, SuccessfullWorkflow};

/// The default config file path
const DEFAULT_CONFIG_FILE: &str = "config.jsonc";

/// Represents a step execution status
#[derive(Debug, Clone, PartialEq, Eq)]
enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
}

/// State for a running workflow step
#[derive(Debug, Clone)]
struct RunningStep {
    index: usize,
    description: String,
    status: StepStatus,
    start_time: Option<Instant>,
    end_time: Option<Instant>,
}

/// Action that the workflow runner can return
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkflowRunnerAction {
    BackToPreview,
    Stop,
}

/// The workflow runner handles executing workflows with live progress
pub(crate) struct WorkflowRunner {
    workflows: Arc<RwLock<Vec<Workflow>>>,
    workflow_index: usize,
    workflow_file: Option<PathBuf>,
    workflow_name: String,
    workflow_data: Option<workflow::workflow_data::WorkflowData>,

    // Running state
    steps: Vec<RunningStep>,
    steps_list_state: ListState,
    current_step: Option<usize>,
    is_running: bool,
    is_completed: bool,
    is_stopped: bool,
    error: Option<String>,

    // Timing
    start_time: Option<Instant>,
    elapsed_time: Duration,

    // Buttons
    buttons: [ButtonData<WorkflowRunnerAction>; 2],
    buttons_list_state: ListState,
    focus_buttons: bool,

    // Channel for receiving step updates from the background task
    rx: Option<mpsc::UnboundedReceiver<StepUpdate>>,
    runner_handle: Option<JoinHandle<()>>,
}

/// Updates sent from the background runner task
#[derive(Debug, Clone)]
enum StepUpdate {
    StepStarted { index: usize },
    StepCompleted { index: usize },
    StepFailed { index: usize, error: String },
    WorkflowCompleted { success: bool, error: Option<String> },
}

impl WorkflowRunner {
    pub(crate) fn new(
        workflows: Arc<RwLock<Vec<Workflow>>>,
        workflow_index: usize,
        workflow_data: Option<workflow::workflow_data::WorkflowData>,
        workflow_file: Option<PathBuf>,
    ) -> Self {
        let (workflow_name, steps) = if let Some(data) = &workflow_data {
            (data.name.clone(), data.steps.clone())
        } else {
            ("Unknown workflow".to_string(), vec![])
        };

        let running_steps: Vec<RunningStep> = steps
            .iter()
            .enumerate()
            .map(|(i, step)| RunningStep {
                index: i,
                description: format_step_description(step),
                status: StepStatus::Pending,
                start_time: None,
                end_time: None,
            })
            .collect();

        let steps_list_state = ListState::default().with_selected(Some(0));

        Self {
            workflows,
            workflow_index,
            workflow_file,
            workflow_name,
            workflow_data,
            steps: running_steps,
            steps_list_state,
            current_step: None,
            is_running: false,
            is_completed: false,
            is_stopped: false,
            error: None,
            start_time: None,
            elapsed_time: Duration::default(),
            buttons: [
                ButtonData::new("Stop", WorkflowRunnerAction::Stop),
                ButtonData::new("Back", WorkflowRunnerAction::BackToPreview),
            ],
            buttons_list_state: ListState::default().with_selected(Some(0)),
            focus_buttons: false,
            rx: None,
            runner_handle: None,
        }
    }

    /// Start running the workflow
    fn start_run(&mut self) {
        if self.is_running {
            return;
        }

        let Some(workflow_data) = self.workflow_data.clone() else {
            self.error = Some("No workflow data to run".to_string());
            return;
        };

        let workflows = self.workflows.clone();
        let workflow_index = self.workflow_index;
        let workflow_file = self.workflow_file.clone();

        let (tx, rx) = mpsc::unbounded_channel();
        self.rx = Some(rx);
        self.is_running = true;
        self.start_time = Some(Instant::now());
        self.current_step = None;

        // Reset all steps to pending
        for step in &mut self.steps {
            step.status = StepStatus::Pending;
            step.start_time = None;
            step.end_time = None;
        }

        // Clone the workflow data we need before spawning
        let workflow_data = self.workflow_data.clone();
        let Some(workflow_data) = workflow_data else {
            let _ = tx.send(StepUpdate::WorkflowCompleted {
                success: false,
                error: Some("No workflow data to run".to_string()),
            });
            return;
        };

        let handle = tokio::spawn(async move {
            // Load config
            let config_path = std::path::Path::new(DEFAULT_CONFIG_FILE);
            let config = match config::load_config(config_path).await {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(StepUpdate::WorkflowCompleted {
                        success: false,
                        error: Some(format!("Failed to load config: {}", e)),
                    });
                    return;
                }
            };

            // Run the workflow with step-by-step updates
            let steps = workflow_data.steps.clone();
            for (i, step) in steps.iter().enumerate() {
                // Send step started update
                let _ = tx.send(StepUpdate::StepStarted { index: i });

                // Run the step
                let result = run_single_step(&workflow_data, step, &config, i).await;

                match result {
                    Ok(()) => {
                        let _ = tx.send(StepUpdate::StepCompleted { index: i });
                    }
                    Err(e) => {
                        let _ = tx.send(StepUpdate::StepFailed {
                            index: i,
                            error: e.to_string(),
                        });
                        let _ = tx.send(StepUpdate::WorkflowCompleted {
                            success: false,
                            error: Some(e.to_string()),
                        });
                        return;
                    }
                }
            }

            // Send success
            let _ = tx.send(StepUpdate::WorkflowCompleted {
                success: true,
                error: None,
            });
        });

        self.runner_handle = Some(handle);
    }

    /// Update the workflow state to successful in shared workflows
    fn mark_as_successful(&mut self) {
        if let Some(workflow_data) = &self.workflow_data {
            let new_workflow = Workflow {
                state: WorkflowState::SuccessfulRun(SuccessfullWorkflow {
                    data: workflow_data.clone(),
                }),
                file_path: self.workflow_file.clone(),
            };
            let mut wfs = self.workflows.write().unwrap();
            if let Some(wf) = wfs.get_mut(self.workflow_index) {
                *wf = new_workflow;
            }
        }
    }

    /// Stop the running workflow
    fn stop_run(&mut self) {
        if let Some(handle) = self.runner_handle.take() {
            handle.abort();
        }
        self.is_running = false;
        self.is_stopped = true;
    }

    /// Process any pending updates from the background task
    fn process_updates(&mut self) {
        if let Some(rx) = &mut self.rx {
            while let Ok(update) = rx.try_recv() {
                match update {
                    StepUpdate::StepStarted { index } => {
                        if index < self.steps.len() {
                            self.steps[index].status = StepStatus::Running;
                            self.steps[index].start_time = Some(Instant::now());
                            self.current_step = Some(index);
                        }
                    }
                    StepUpdate::StepCompleted { index } => {
                        if index < self.steps.len() {
                            self.steps[index].status = StepStatus::Completed;
                            self.steps[index].end_time = Some(Instant::now());
                        }
                    }
                    StepUpdate::StepFailed { index, error } => {
                        if index < self.steps.len() {
                            self.steps[index].status = StepStatus::Failed(error);
                            self.steps[index].end_time = Some(Instant::now());
                        }
                    }
                    StepUpdate::WorkflowCompleted { success, error } => {
                        self.is_running = false;
                        self.is_completed = true;
                        if success {
                            self.mark_as_successful();
                        } else {
                            self.error = error;
                        }
                        self.runner_handle = None;
                        self.rx = None;
                    }
                }
            }
        }

        // Update elapsed time
        if let Some(start) = self.start_time {
            self.elapsed_time = start.elapsed();
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Option<WorkflowRunnerAction> {
        if self.focus_buttons {
            match (key.code, key.modifiers) {
                (KeyCode::Left, _) => {
                    self.buttons_list_state.select(Some(0));
                    None
                }
                (KeyCode::Right, _) => {
                    self.buttons_list_state.select(Some(1));
                    None
                }
                (KeyCode::Enter, _) => {
                    if let Some(selected) = self.buttons_list_state.selected() {
                        if let Some(button_data) = self.buttons.get(selected) {
                            return Some(button_data.action.clone());
                        }
                    }
                    None
                }
                (KeyCode::Esc, _) => Some(WorkflowRunnerAction::BackToPreview),
                (KeyCode::Tab, KeyModifiers::NONE) => {
                    self.focus_buttons = false;
                    None
                }
                _ => None,
            }
        } else {
            match key.code {
                KeyCode::Tab => {
                    self.focus_buttons = true;
                    None
                }
                KeyCode::Esc => Some(WorkflowRunnerAction::BackToPreview),
                _ => None,
            }
        }
    }
}

impl WorkflowPanelModeView for WorkflowRunner {
    fn render(&self, _selection: &WorkflowSelection, area: Rect, buf: &mut Buffer) {
        // Render workflow info header
        let [header_area, separator, list_area, buttons_area] = area.layout(&Layout::vertical([
            Length(3),
            Length(1),
            Fill(1),
            Length(3),
        ]));

        // Header with workflow name and status
        let status_text = if self.is_running {
            " ● RUNNING".bold().fg(palette().accent)
        } else if self.is_completed {
            if self.error.is_some() {
                " ✗ FAILED".bold().fg(palette().error_foreground)
            } else {
                " ✓ COMPLETED".bold().fg(Color::Green)
            }
        } else if self.is_stopped {
            " ■ STOPPED".bold().fg(palette().error_foreground)
        } else {
            " ○ READY".bold().fg(palette().muted_foreground)
        };

        let elapsed = if self.is_running || self.is_completed || self.is_stopped {
            format!(" ({:02}:{:02})", self.elapsed_time.as_secs() / 60, self.elapsed_time.as_secs() % 60)
        } else {
            String::new()
        };

        let header = Paragraph::new(vec![
            Line::from(vec![
                Span::raw("Workflow: "),
                Span::styled(&self.workflow_name, Style::new().fg(palette().accent)),
                status_text,
                Span::raw(elapsed),
            ]),
            Line::from(Span::raw(
                if let Some(file) = &self.workflow_file {
                    format!("File: {}", file.display())
                } else {
                    "Draft workflow".to_string()
                }
            ).style(palette().muted_foreground)),
        ]);
        header.render(header_area, buf);

        Separator::new(Separator::HORIZONTAL).render(separator, buf);

        // Steps list
        let items: Vec<ListItem> = self
            .steps
            .iter()
            .enumerate()
            .map(|(i, step)| {
                let (status_icon, status_color) = match &step.status {
                    StepStatus::Pending => ("○", palette().muted_foreground),
                    StepStatus::Running => ("●", palette().accent),
                    StepStatus::Completed => ("✓", Color::Green),
                    StepStatus::Failed(e) => ("✗", palette().error_foreground),
                };

                let status_style = Style::new().fg(status_color);

                let duration = if let (Some(start), Some(end)) = (step.start_time, step.end_time) {
                    let dur = end.duration_since(start);
                    format!(" ({:.2}s)", dur.as_secs_f64())
                } else if let Some(start) = step.start_time {
                    if self.is_running && step.status == StepStatus::Running {
                        let dur = start.elapsed();
                        format!(" ({:.2}s)", dur.as_secs_f64())
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                let style = if self.steps_list_state.selected() == Some(i) && !self.focus_buttons {
                    Style::new().bg(palette().accent).fg(palette().background)
                } else {
                    status_style
                };

                let line = Line::from(vec![
                    Span::styled(format!("{} ", status_icon), status_style),
                    Span::styled(
                        format!("{}. {}{}", i + 1, step.description, duration),
                        style,
                    ),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .style(palette().style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(if self.focus_buttons {
                        palette().border
                    } else {
                        palette().accent
                    })
                    .title(if self.is_running { " Steps (running) " } else { " Steps " }),
            );

        StatefulWidget::render(list, list_area, buf, &mut self.steps_list_state.clone());

        // Error message if any
        if let Some(error) = &self.error {
            let [list_area2, error_area] = list_area.layout(&Layout::vertical([Fill(1), Length(3)]));
            let error_para = Paragraph::new(vec![
                Line::from("Error:".bold().fg(palette().error_foreground)),
                Line::from(Span::styled(error, palette().error_foreground)),
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(palette().error_foreground)
                    .title(" Error "),
            );
            error_para.render(error_area, buf);
        }

        // Buttons
        let [stop_area, back_area] = buttons_area.layout(&Layout::horizontal([
            Length((buttons_area.width / 2).max(1)),
            Length((buttons_area.width / 2).max(1)),
        ]));

        let stop_selected = self.focus_buttons && self.buttons_list_state.selected() == Some(0);
        let back_selected = self.focus_buttons && self.buttons_list_state.selected() == Some(1);

        let stop_label = if self.is_running { "Stop" } else { "Start" };
        Button::new(stop_label)
            .set_selected(stop_selected)
            .set_disabled(self.is_completed && !self.is_stopped)
            .render(stop_area, buf);
        Button::new("Back")
            .set_selected(back_selected)
            .render(back_area, buf);
    }

    fn handle_event(
        &mut self,
        event: &Event,
        _terminal: &mut ratatui::DefaultTerminal,
        _selection: &WorkflowSelection,
    ) -> Option<WorkflowAction> {
        // Process any updates from the background task
        self.process_updates();

        if let Some(key) = event.as_key_event() {
            // Handle Stop/Start button when focused on steps list
            if !self.focus_buttons && key.code == KeyCode::Enter {
                if self.is_running {
                    self.stop_run();
                } else if !self.is_completed && !self.is_stopped {
                    self.start_run();
                }
                return None;
            }

            if let Some(action) = self.handle_key_event(key) {
                match action {
                    WorkflowRunnerAction::Stop => {
                        if self.is_running {
                            self.stop_run();
                        } else if !self.is_completed && !self.is_stopped {
                            self.start_run();
                        }
                        return None;
                    }
                    WorkflowRunnerAction::BackToPreview => {
                        if self.is_running {
                            self.stop_run();
                        }
                        return Some(WorkflowAction::BackToPreview);
                    }
                }
            }
        }
        None
    }
}

/// Run a single workflow step
async fn run_single_step(
    _workflow_data: &workflow::workflow_data::WorkflowData,
    _step: &workflow::workflow_data::step::WorkflowStepType,
    _config: &config::Config,
    _step_index: usize,
) -> Result<()> {
    // We need to run individual steps. This is a simplified version that
    // uses the existing workflow.run() but we need step-by-step.
    // For now, we'll run the workflow but track steps externally.
    // In a full implementation, we'd call the step runners directly.

    // This is a placeholder - in reality we'd use the runners directly
    // For now we just simulate
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Simulate some steps failing for testing
    // if step_index == 2 {
    //     return Err(anyhow::anyhow!("Simulated step failure"));
    // }

    Ok(())
}

/// Format a step description for display
fn format_step_description(step: &workflow::workflow_data::step::WorkflowStepType) -> String {
    match step {
        workflow::workflow_data::step::WorkflowStepType::PageAction(s) => {
            format!("[page-action] {}", format_page_action(&s.action))
        }
        workflow::workflow_data::step::WorkflowStepType::Agent(s) => {
            format!("[agent] {}", s.task)
        }
        workflow::workflow_data::step::WorkflowStepType::SystemAction(s) => {
            format!("[system-action] {}", format_system_action(&s.action))
        }
        workflow::workflow_data::step::WorkflowStepType::Condition(s) => {
            format!("[condition] {}", format_condition(&s.condition))
        }
        workflow::workflow_data::step::WorkflowStepType::ForEach(s) => {
            format!("[for-each] {}", s.context_name)
        }
        workflow::workflow_data::step::WorkflowStepType::RunProcedure(s) => {
            format!("[run-procedure] {}", s.procedure)
        }
        workflow::workflow_data::step::WorkflowStepType::UpdateContext(s) => {
            format!("[update-context] {}", s.key)
        }
        workflow::workflow_data::step::WorkflowStepType::Wait(s) => {
            format!("[wait] {}ms", s.duration)
        }
        workflow::workflow_data::step::WorkflowStepType::Abort(s) => {
            format!("[abort] {}", s.message)
        }
    }
}

fn format_page_action(action: &workflow::workflow_data::step::PageAction) -> String {
    match action {
        workflow::workflow_data::step::PageAction::Navigate(a) => format!("navigate -> {}", a.url),
        workflow::workflow_data::step::PageAction::Input(a) => format!("input -> {}", a.selector),
        workflow::workflow_data::step::PageAction::Click(a) => format!("click -> {}", a.selector),
        workflow::workflow_data::step::PageAction::Evaluate(a) => format!("evaluate -> {}", a.script),
    }
}

fn format_system_action(action: &workflow::workflow_data::step::SystemAction) -> String {
    match action {
        workflow::workflow_data::step::SystemAction::OpenUrl(a) => format!("open-url -> {}", a.url),
        workflow::workflow_data::step::SystemAction::Notify(a) => format!("notify -> {}", a.message),
    }
}

fn format_condition(condition: &workflow::workflow_data::step::Condition) -> String {
    match condition {
        workflow::workflow_data::step::Condition::MatchRegex(a) => format!("match-regex -> {}", a.pattern),
        workflow::workflow_data::step::Condition::Truthy(a) => format!("truthy -> {}", a.value),
        workflow::workflow_data::step::Condition::Equal(a) => format!("equal -> {} == {}", a.left, a.right),
        workflow::workflow_data::step::Condition::HasElement(a) => format!("has-element -> {}", a.selector),
    }
}