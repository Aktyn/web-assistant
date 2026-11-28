use std::{
    path::PathBuf,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{
        Constraint::{Fill, Length},
        Layout, Rect,
    },
    style::{Color, Style, Stylize as _},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};
use tokio::sync::mpsc;

use super::super::super::common::palette;
use super::super::super::common::widgets::{Button, ButtonData, Separator};
use super::{PanelEventOutcome, WorkflowAction, WorkflowPanelModeView, WorkflowSelection};
use workflow::{FailedWorkflow, StepProgress, SuccessfullWorkflow, Workflow, WorkflowState};

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
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct WorkflowRunner {
    workflows: Arc<RwLock<Vec<Workflow>>>,
    config: Arc<RwLock<config::Config>>,
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
    is_stopping: bool,
    error: Option<String>,

    // Timing
    start_time: Option<Instant>,
    elapsed_time: Duration,

    // Buttons
    buttons: [ButtonData<WorkflowRunnerAction>; 2],
    buttons_list_state: ListState,
    focus_buttons: bool,

    // Channel for receiving updates from the background task
    rx: Option<mpsc::UnboundedReceiver<RunnerUpdate>>,
    runner_handle: Option<std::thread::JoinHandle<()>>,
    stop_requested: Option<Arc<AtomicBool>>,
}

/// Updates sent from the background runner task
#[derive(Debug, Clone)]
enum RunnerUpdate {
    Step(StepProgress),
    Finished {
        stopped: bool,
        success: bool,
        error: Option<String>,
    },
    FailedToStart(String),
}

impl WorkflowRunner {
    pub(crate) fn new(
        workflows: Arc<RwLock<Vec<Workflow>>>,
        config: Arc<RwLock<config::Config>>,
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
            .map(|step| RunningStep {
                description: format_step_description(step),
                status: StepStatus::Pending,
                start_time: None,
                end_time: None,
            })
            .collect();

        let steps_list_state = ListState::default().with_selected(Some(0));

        Self {
            workflows,
            config,
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
            is_stopping: false,
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
            stop_requested: None,
        }
    }

    /// Start running the workflow
    fn start_run(&mut self) {
        if self.is_running || self.is_completed || self.is_stopped {
            return;
        }

        let Some(data) = self.workflow_data.clone() else {
            self.error = Some("No workflow data to run".to_string());
            return;
        };
        let Some(file_path) = self.workflow_file.clone() else {
            self.error = Some("Cannot run a draft workflow. Save it first.".to_string());
            return;
        };
        let (running_box, exec_box) = {
            let Ok(guard) = self.config.read() else {
                self.error = Some("Failed to read config".to_string());
                return;
            };
            (Box::new(guard.clone()), Box::new(guard.clone()))
        };

        // Mark the shared entry as running
        if let Ok(mut wfs) = self.workflows.write()
            && let Some(wf) = wfs.get_mut(self.workflow_index)
        {
            *wf = Workflow::new_running(data.clone(), running_box, file_path.clone());
        }

        let thread_data = data;
        let thread_file = file_path;

        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::unbounded_channel();
        self.rx = Some(rx);
        self.stop_requested = Some(stop.clone());

        self.is_running = true;
        self.is_stopping = false;
        self.error = None;
        self.start_time = Some(Instant::now());
        self.current_step = None;
        for step in &mut self.steps {
            step.status = StepStatus::Pending;
            step.start_time = None;
            step.end_time = None;
        }

        // The workflow future is not `Send` (and neither is the shared workflow list),
        // so it runs on a dedicated thread with its own single-threaded runtime. Only
        // `Send` data crosses the thread boundary; the shared list is updated from
        // `process_updates` on the UI thread.
        let handle = std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(e) => {
                    let _ = tx.send(RunnerUpdate::FailedToStart(format!(
                        "Failed to create runtime: {e}"
                    )));
                    return;
                }
            };

            let mut exec = Workflow {
                state: WorkflowState::Valid(thread_data),
                file_path: Some(thread_file),
            };
            let mut on_step = |update: StepProgress| -> std::ops::ControlFlow<()> {
                let _ = tx.send(RunnerUpdate::Step(update));
                if stop.load(Ordering::Relaxed) {
                    std::ops::ControlFlow::Break(())
                } else {
                    std::ops::ControlFlow::Continue(())
                }
            };
            let result = runtime.block_on(exec.run_with_progress(exec_box, &mut on_step));
            let stopped = stop.load(Ordering::Relaxed);
            let (success, error) = match &exec.state {
                WorkflowState::SuccessfulRun(_) => (true, None),
                WorkflowState::FailedRun(failed) => (false, Some(failed.error.to_string())),
                _ => (false, None),
            };
            let _ = match result {
                Ok(()) => tx.send(RunnerUpdate::Finished {
                    stopped,
                    success,
                    error,
                }),
                Err(e) => tx.send(RunnerUpdate::FailedToStart(e.to_string())),
            };
        });

        self.runner_handle = Some(handle);
    }

    /// Ask the running workflow to stop at the next step boundary
    fn request_stop(&mut self) {
        if self.is_running {
            if let Some(stop) = &self.stop_requested {
                stop.store(true, Ordering::Relaxed);
            }
            self.is_stopping = true;
        }
    }

    /// Request a stop when leaving the run view. The channel is kept so the final
    /// state can still be applied to the shared list (see `is_active`).
    fn stop_run(&mut self) {
        self.request_stop();
    }

    /// Whether a background run is still in flight (its final state has not been applied yet)
    pub(super) fn is_active(&self) -> bool {
        self.rx.is_some()
    }

    /// Write the final state of the workflow into the shared list
    fn set_shared_state(&self, state: WorkflowState) {
        if let Ok(mut wfs) = self.workflows.write()
            && let Some(wf) = wfs.get_mut(self.workflow_index)
        {
            *wf = Workflow {
                state,
                file_path: self.workflow_file.clone(),
            };
        }
    }

    /// Stop the run if it is in progress, otherwise start it (unless already finished)
    fn toggle_run(&mut self) {
        if self.is_running {
            self.request_stop();
        } else if !self.is_completed && !self.is_stopped {
            self.start_run();
        }
    }

    fn clear_run_handles(&mut self) {
        self.runner_handle = None;
        self.rx = None;
        self.stop_requested = None;
        self.is_running = false;
        self.is_stopping = false;
    }

    /// Process any pending updates from the background task
    fn process_updates(&mut self) {
        let mut updates = Vec::new();
        if let Some(rx) = &mut self.rx {
            while let Ok(update) = rx.try_recv() {
                updates.push(update);
            }
        }

        for update in updates {
            match update {
                RunnerUpdate::Step(StepProgress::StepStarted { index }) => {
                    if index < self.steps.len() {
                        self.steps[index].status = StepStatus::Running;
                        self.steps[index].start_time = Some(Instant::now());
                        self.current_step = Some(index);
                    }
                }
                RunnerUpdate::Step(StepProgress::StepCompleted { index }) => {
                    if index < self.steps.len() {
                        self.steps[index].status = StepStatus::Completed;
                        self.steps[index].end_time = Some(Instant::now());
                    }
                }
                RunnerUpdate::Finished {
                    stopped,
                    success,
                    error,
                } => {
                    self.clear_run_handles();
                    if let Some(data) = self.workflow_data.clone() {
                        self.set_shared_state(if stopped {
                            WorkflowState::Valid(data)
                        } else if success {
                            WorkflowState::SuccessfulRun(SuccessfullWorkflow { data })
                        } else {
                            WorkflowState::FailedRun(FailedWorkflow {
                                data,
                                error: anyhow::anyhow!(
                                    error.clone().unwrap_or_else(|| "Unknown error".to_string())
                                ),
                            })
                        });
                    }
                    if stopped {
                        self.is_stopped = true;
                    } else {
                        self.is_completed = true;
                        if success {
                            self.error = None;
                        } else {
                            if let (Some(message), Some(index)) = (&error, self.current_step)
                                && let Some(step) = self.steps.get_mut(index)
                                && step.status == StepStatus::Running
                            {
                                step.status = StepStatus::Failed(message.clone());
                                step.end_time = Some(Instant::now());
                            }
                            self.error = error;
                        }
                    }
                }
                RunnerUpdate::FailedToStart(message) => {
                    self.clear_run_handles();
                    if let Some(data) = self.workflow_data.clone() {
                        self.set_shared_state(WorkflowState::Valid(data));
                    }
                    self.error = Some(message);
                }
            }
        }

        // Update elapsed time
        if self.is_running
            && let Some(start) = self.start_time
        {
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
                    if let Some(selected) = self.buttons_list_state.selected()
                        && let Some(button_data) = self.buttons.get(selected)
                    {
                        return Some(button_data.action.clone());
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

    fn is_start_stop_disabled(&self) -> bool {
        (self.is_completed && !self.is_stopped) || self.workflow_file.is_none()
    }
}

impl WorkflowPanelModeView for WorkflowRunner {
    #[allow(clippy::too_many_lines)]
    fn render(&self, _selection: &WorkflowSelection, area: Rect, buf: &mut Buffer) {
        // Render workflow info header
        let [header_area, separator, list_area, buttons_area] = area.layout(&Layout::vertical([
            Length(3),
            Length(1),
            Fill(1),
            Length(if self.workflow_file.is_none() { 4 } else { 3 }),
        ]));

        // Header with workflow name and status
        let status_text = if self.is_stopping {
            " ■ STOPPING".bold().fg(palette().accent)
        } else if self.is_running {
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
            format!(
                " ({:02}:{:02})",
                self.elapsed_time.as_secs() / 60,
                self.elapsed_time.as_secs() % 60
            )
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
            Line::from(
                Span::raw(if let Some(file) = &self.workflow_file {
                    format!("File: {}", file.display())
                } else {
                    "Draft workflow".to_string()
                })
                .style(palette().muted_foreground),
            ),
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
                    StepStatus::Failed(_e) => ("✗", palette().error_foreground),
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
                    Span::styled(format!("{status_icon} "), status_style),
                    Span::styled(
                        format!("{}. {}{}", i + 1, step.description, duration),
                        style,
                    ),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items).style(palette().style).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(if self.focus_buttons {
                    palette().border
                } else {
                    palette().accent
                })
                .title(if self.is_running {
                    " Steps (running) "
                } else {
                    " Steps "
                }),
        );

        StatefulWidget::render(list, list_area, buf, &mut self.steps_list_state.clone());

        // Error message if any
        if let Some(error) = &self.error {
            let [_list_area2, error_area] =
                list_area.layout(&Layout::vertical([Fill(1), Length(3)]));
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
        let buttons_area = if self.workflow_file.is_none() {
            let [hint_area, rest] = buttons_area.layout(&Layout::vertical([Length(1), Fill(1)]));
            Paragraph::new("Save the workflow to run it")
                .style(palette().muted_foreground)
                .render(hint_area, buf);
            rest
        } else {
            buttons_area
        };
        let [stop_area, back_area] = buttons_area.layout(&Layout::horizontal([
            Length((buttons_area.width / 2).max(1)),
            Length((buttons_area.width / 2).max(1)),
        ]));

        let stop_selected = self.focus_buttons && self.buttons_list_state.selected() == Some(0);
        let back_selected = self.focus_buttons && self.buttons_list_state.selected() == Some(1);

        let stop_label = if self.is_stopping {
            "Stopping…"
        } else if self.is_running {
            "Stop"
        } else {
            "Start"
        };
        Button::new(stop_label)
            .set_selected(stop_selected)
            .set_disabled(self.is_start_stop_disabled())
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
    ) -> PanelEventOutcome {
        // Process any updates from the background task
        self.process_updates();

        let Some(key) = event.as_key_event() else {
            return PanelEventOutcome::Ignored;
        };

        // Handle Stop/Start button when focused on steps list
        if !self.focus_buttons && key.code == KeyCode::Enter {
            self.toggle_run();
            return PanelEventOutcome::Consumed;
        }

        if let Some(action) = self.handle_key_event(key) {
            return match action {
                WorkflowRunnerAction::Stop => {
                    self.toggle_run();
                    PanelEventOutcome::Consumed
                }
                WorkflowRunnerAction::BackToPreview => {
                    if self.is_running {
                        self.stop_run();
                    }
                    PanelEventOutcome::Action(WorkflowAction::BackToPreview)
                }
            };
        }

        match key.code {
            KeyCode::Tab | KeyCode::BackTab => PanelEventOutcome::Consumed,
            KeyCode::Left | KeyCode::Right if self.focus_buttons => PanelEventOutcome::Consumed,
            _ => PanelEventOutcome::Ignored,
        }
    }

    fn tick(&mut self) {
        self.process_updates();
    }
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
        workflow::workflow_data::step::PageAction::Evaluate(a) => {
            format!("evaluate -> {}", a.script)
        }
    }
}

fn format_system_action(action: &workflow::workflow_data::step::SystemAction) -> String {
    match action {
        workflow::workflow_data::step::SystemAction::OpenUrl(a) => format!("open-url -> {}", a.url),
        workflow::workflow_data::step::SystemAction::Notify(a) => {
            format!("notify -> {}", a.message)
        }
    }
}

fn format_condition(condition: &workflow::workflow_data::step::Condition) -> String {
    match condition {
        workflow::workflow_data::step::Condition::MatchRegex(a) => {
            format!("match-regex -> {}", a.pattern)
        }
        workflow::workflow_data::step::Condition::Truthy(a) => format!("truthy -> {}", a.value),
        workflow::workflow_data::step::Condition::Equal(a) => {
            format!("equal -> {} == {}", a.left, a.right)
        }
        workflow::workflow_data::step::Condition::HasElement(a) => {
            format!("has-element -> {}", a.selector)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Arc<RwLock<config::Config>> {
        Arc::new(RwLock::new(config::Config::default()))
    }

    fn data_from(json: &str) -> workflow::workflow_data::WorkflowData {
        serde_json::from_str(json).unwrap()
    }

    fn simple_runner(file: Option<PathBuf>) -> WorkflowRunner {
        let data = data_from(r#"{"name":"W","steps":[{"type":"wait","duration":10}]}"#);
        let workflows = Arc::new(RwLock::new(Vec::new()));
        WorkflowRunner::new(workflows, test_config(), 0, Some(data), file)
    }

    #[tokio::test]
    async fn test_start_run_executes_real_workflow() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("run-me.json");
        let json = r#"{"name":"RunMe","steps":[{"type":"wait","duration":100}],"networkConnectionTimeout":0}"#;
        std::fs::write(&path, json).unwrap();
        let workflows = Arc::new(RwLock::new(vec![Workflow::new(
            workflow::JsonFile::from_raw(path.clone(), json.to_string()),
        )]));
        let data = match &workflows.read().unwrap()[0].state {
            WorkflowState::Valid(data) => data.clone(),
            _ => panic!("workflow should be valid"),
        };
        let mut config = config::Config::default();
        config.browser.headless = true;
        let mut runner = WorkflowRunner::new(
            workflows.clone(),
            Arc::new(RwLock::new(config)),
            0,
            Some(data),
            Some(path),
        );

        runner.start_run();
        assert!(runner.is_running);
        assert!(matches!(
            workflows.read().unwrap()[0].state,
            WorkflowState::Running(..)
        ));

        for _ in 0..100 {
            runner.process_updates();
            if runner.is_completed {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        assert!(runner.is_completed);
        assert!(runner.error.is_none());
        assert!(!runner.is_running);
        assert_eq!(runner.steps[0].status, StepStatus::Completed);
        assert!(matches!(
            workflows.read().unwrap()[0].state,
            WorkflowState::SuccessfulRun(_)
        ));
    }

    #[test]
    fn test_finished_stopped_updates_state() {
        let mut runner = simple_runner(Some(PathBuf::from("/tmp/x.json")));
        let (tx, rx) = mpsc::unbounded_channel();
        runner.rx = Some(rx);
        runner.is_running = true;
        runner.is_stopping = true;
        tx.send(RunnerUpdate::Finished {
            stopped: true,
            success: false,
            error: None,
        })
        .unwrap();
        runner.process_updates();
        assert!(runner.is_stopped);
        assert!(!runner.is_running);
        assert!(!runner.is_stopping);
        assert!(runner.error.is_none());
    }

    #[test]
    fn test_failed_to_start_sets_error_and_allows_retry() {
        let mut runner = simple_runner(Some(PathBuf::from("/tmp/x.json")));
        let (tx, rx) = mpsc::unbounded_channel();
        runner.rx = Some(rx);
        runner.is_running = true;
        tx.send(RunnerUpdate::FailedToStart("boom".into())).unwrap();
        runner.process_updates();
        assert_eq!(runner.error.as_deref(), Some("boom"));
        assert!(!runner.is_running);
        assert!(!runner.is_completed && !runner.is_stopped);
        assert!(!runner.is_start_stop_disabled());
    }

    #[test]
    fn test_draft_cannot_start() {
        let mut runner = simple_runner(None);
        runner.start_run();
        assert!(!runner.is_running);
        assert!(
            runner
                .error
                .as_deref()
                .is_some_and(|e| e.contains("Save it first"))
        );
        assert!(runner.is_start_stop_disabled());
        let area = Rect::new(0, 0, 60, 20);
        let mut buf = Buffer::empty(area);
        runner.render(
            &super::super::WorkflowSelection {
                index: 0,
                workflow_file: None,
                is_running: false,
                state: super::super::WorkflowSelectionState::Error(String::new()),
            },
            area,
            &mut buf,
        );
    }
}
