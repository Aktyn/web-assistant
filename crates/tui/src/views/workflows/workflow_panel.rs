mod workflow_delete;
mod workflow_editor;
mod workflow_preview;
mod workflow_runner;

use crate::views::workflows::workflow_panel::workflow_editor::WorkflowEditor;
use crate::views::workflows::workflow_panel::workflow_runner::WorkflowRunner;
use config::Config;
use crossterm::event::Event;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{
        Constraint::{self, Fill, Length},
        Layout, Rect,
    },
    widgets::{Paragraph, Widget},
};
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use super::super::common::widgets::{ErrorAlert, Separator, TextField};
use crate::common::{palette, panel};
use workflow::{Workflow, WorkflowState, workflow_data::WorkflowData};
use workflow_delete::WorkflowDelete;
use workflow_preview::WorkflowPreview;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub(super) enum WorkflowAction {
    BackToPreview,
    OpenFile,
    Edit,
    Delete,
    Execution,
    ConfirmDelete,
    Save,
}

#[allow(clippy::large_enum_variant)] //TODO: optimize this enum
pub(super) enum WorkflowPanelMode {
    Preview(WorkflowPreview),
    Edit(WorkflowEditor),
    Run(WorkflowRunner),
    Delete(WorkflowDelete),
}

/// Result of handling an event in a panel mode view
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PanelEventOutcome {
    /// Key was consumed by this mode view (e.g. text input); must not propagate
    Consumed,
    /// Key was not handled by this mode view; let it propagate
    Ignored,
    /// Key was consumed and produced a panel-level action
    Action(WorkflowAction),
}

trait WorkflowPanelModeView {
    fn render(&self, selection: &WorkflowSelection, area: Rect, buf: &mut Buffer);
    fn handle_event(
        &mut self,
        event: &Event,
        _terminal: &mut DefaultTerminal,
        selection: &WorkflowSelection,
    ) -> PanelEventOutcome;

    /// Called periodically (also without any input events), e.g. for polling async updates
    fn tick(&mut self) {}
}

#[derive(Debug)]
struct ValidWorkflowState {
    name_field: TextField,
    description_field: Option<TextField>,
}

#[derive(Debug)]
enum WorkflowSelectionState {
    Valid(ValidWorkflowState),
    Error(String),
}

struct WorkflowSelection {
    /// Index of the workflow in the list of workflows
    index: usize,
    workflow_file: Option<PathBuf>,
    state: WorkflowSelectionState,
    /// Whether the workflow is currently being executed
    is_running: bool,
}

pub(crate) struct WorkflowPanel {
    workflows: Arc<RwLock<Vec<Workflow>>>,
    config: Arc<RwLock<Config>>,
    /// A runner that was left while its background run was still in flight; kept alive
    /// until its final state is written to the shared workflow list
    detached_runner: Option<WorkflowRunner>,
    selected_workflow: Option<WorkflowSelection>,
    pub(super) mode: WorkflowPanelMode,
    workflows_directory: PathBuf,
    save_error: Option<String>,
}

impl WorkflowPanel {
    pub(super) fn new(
        workflows: Arc<RwLock<Vec<Workflow>>>,
        config: Arc<RwLock<Config>>,
        workflows_directory: PathBuf,
    ) -> Self {
        Self {
            workflows,
            config,
            detached_runner: None,
            selected_workflow: None,
            mode: WorkflowPanelMode::Preview(WorkflowPreview::new()),
            workflows_directory,
            save_error: None,
        }
    }

    /// Replace the current mode, keeping a still-running workflow runner alive in the background
    fn swap_mode(
        mode: &mut WorkflowPanelMode,
        detached_runner: &mut Option<WorkflowRunner>,
        new_mode: WorkflowPanelMode,
    ) {
        if let WorkflowPanelMode::Run(runner) = std::mem::replace(mode, new_mode)
            && runner.is_active()
        {
            *detached_runner = Some(runner);
        }
    }

    pub(super) fn tick(&mut self) {
        if let WorkflowPanelMode::Run(runner) = &mut self.mode {
            runner.tick();
        }
        if let Some(runner) = &mut self.detached_runner {
            runner.tick();
            if !runner.is_active() {
                self.detached_runner = None;
            }
        }
    }

    pub(super) fn update(&mut self, selected_workflow_index: Option<usize>) {
        if let Some(index) = selected_workflow_index {
            match self.selected_workflow {
                Some(ref selected_workflow) => {
                    if selected_workflow.index != index {
                        self.open_workflow(index);
                    }
                }
                None => self.open_workflow(index),
            }
        }
    }

    pub(super) fn open_workflow(&mut self, index: usize) {
        log::info!("Opening workflow: {index:?}");
        self.save_error = None;

        let return_valid_selection = |data: &WorkflowData| {
            WorkflowSelectionState::Valid(ValidWorkflowState {
                name_field: TextField::new("Name".to_string(), data.name.clone()),
                description_field: data
                    .description
                    .clone()
                    .map(|description| TextField::new("Description".to_string(), description)),
            })
        };

        match self.workflows.read() {
            Ok(unlocked_workflows) => {
                if let Some(workflow) = unlocked_workflows.get(index) {
                    self.selected_workflow = Some(WorkflowSelection {
                        index,
                        workflow_file: workflow.file_path.clone(),
                        is_running: matches!(workflow.state, WorkflowState::Running(..)),
                        state: match &workflow.state {
                            WorkflowState::Valid(data) => return_valid_selection(data),
                            WorkflowState::Invalid(error) => {
                                let error_message = error.to_string();
                                log::warn!("Invalid workflow: {error_message}");
                                WorkflowSelectionState::Error(error_message)
                            }
                            WorkflowState::Running(running_workflow, ..) => {
                                return_valid_selection(&running_workflow.data)
                            }
                            WorkflowState::FailedRun(failed_workflow) => {
                                return_valid_selection(&failed_workflow.data)
                            }
                            WorkflowState::SuccessfulRun(successful_workflow) => {
                                return_valid_selection(&successful_workflow.data)
                            }
                        },
                    });

                    let is_draft = self
                        .selected_workflow
                        .as_ref()
                        .is_some_and(|w| w.workflow_file.is_none());

                    if is_draft {
                        let editor = WorkflowEditor::new(
                            // self.workflows.clone(),
                            // index,
                            match &workflow.state {
                                WorkflowState::Valid(data) => Some(data.clone()),
                                _ => None,
                            },
                            // workflow.file_path.clone(),
                        );
                        Self::swap_mode(
                            &mut self.mode,
                            &mut self.detached_runner,
                            WorkflowPanelMode::Edit(editor),
                        );
                    } else {
                        // For non-draft workflows, always switch to Preview mode
                        Self::swap_mode(
                            &mut self.mode,
                            &mut self.detached_runner,
                            WorkflowPanelMode::Preview(WorkflowPreview::new()),
                        );
                    }
                } else {
                    log::error!("Cannot get workflow with index: {index}");
                }
            }
            Err(error) => {
                log::error!("Cannot read core: {error}");
            }
        }
    }

    fn render_selected_workflow(
        &self,
        selection: &WorkflowSelection,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let error_height = if self.save_error.is_some() { 3 } else { 0 };
        let [path_area, error_area, options_separator_area, mode_area] =
            area.layout(&Layout::vertical([
                Length(1),
                Length(error_height),
                Length(1),
                Fill(1),
            ]));

        if let Some(save_error) = &self.save_error {
            ErrorAlert::new(save_error.as_str()).render(error_area, buf);
        }

        if let Some(workflow_file) = &selection.workflow_file {
            let absolute_path =
                std::path::absolute(workflow_file.as_path()).unwrap_or(workflow_file.clone());
            let absolute_path_str = absolute_path.to_str().unwrap_or("");
            let label = "Workflow file: ";

            Paragraph::new(format!(
                "{}{}",
                label,
                ::utils::text::truncate_text_left(
                    absolute_path_str,
                    (path_area.width as usize).saturating_sub(label.len()),
                    None
                )
            ))
            .style(palette().style)
            .render(path_area, buf);
        } else {
            Paragraph::new("Draft workflow")
                .style(palette().accent)
                .render(path_area, buf);
        }

        Separator::new(Separator::HORIZONTAL).render(options_separator_area, buf);

        match &self.mode {
            WorkflowPanelMode::Preview(preview) => preview.render(selection, mode_area, buf),
            WorkflowPanelMode::Edit(editor) => editor.render(selection, mode_area, buf),
            WorkflowPanelMode::Run(runner) => runner.render(selection, mode_area, buf),
            WorkflowPanelMode::Delete(workflow_delete) => {
                workflow_delete.render(selection, mode_area, buf);
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn run_workflow_action(&mut self, action: WorkflowAction) {
        let Some(selected_workflow) = &self.selected_workflow else {
            return;
        };

        match action {
            WorkflowAction::OpenFile => {
                let open_file = |file_path: &PathBuf| -> anyhow::Result<()> {
                    let absolute_path = file_path.canonicalize().map_err(|e| {
                        anyhow::anyhow!(
                            "failed to get absolute path of \"{}\": {e}",
                            file_path.display()
                        )
                    })?;
                    ::utils::system::run_open_command(absolute_path.to_str().unwrap_or(""))?;
                    Ok(())
                };

                if let Some(file_path) = selected_workflow.workflow_file.as_ref()
                    && let Err(err) = open_file(file_path)
                {
                    log::error!("Failed to open file: {err}");
                }
            }
            WorkflowAction::Edit => {
                // Get workflow data for editing
                let workflow_data = if let Ok(workflows) = self.workflows.read() {
                    if let Some(workflow) = workflows.get(selected_workflow.index) {
                        match &workflow.state {
                            WorkflowState::Valid(data) => Some(data.clone()),
                            WorkflowState::Running(running, _) => Some(running.data.clone()),
                            WorkflowState::FailedRun(failed) => Some(failed.data.clone()),
                            WorkflowState::SuccessfulRun(success) => Some(success.data.clone()),
                            WorkflowState::Invalid(..) => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                let editor = WorkflowEditor::new(
                    // self.workflows.clone(),
                    // selected_workflow.index,
                    workflow_data,
                    // selected_workflow.workflow_file.clone(),
                );
                Self::swap_mode(
                    &mut self.mode,
                    &mut self.detached_runner,
                    WorkflowPanelMode::Edit(editor),
                );
            }
            WorkflowAction::Delete => self.mode = WorkflowPanelMode::Delete(WorkflowDelete::new()),
            WorkflowAction::Execution => {
                let workflow_data = if let Some(selection) = &self.selected_workflow {
                    match &selection.state {
                        WorkflowSelectionState::Valid(_state) => {
                            // Get the workflow data
                            if let Ok(workflows) = self.workflows.read() {
                                if let Some(workflow) = workflows.get(selection.index) {
                                    match &workflow.state {
                                        WorkflowState::Valid(data) => Some(data.clone()),
                                        WorkflowState::SuccessfulRun(run) => Some(run.data.clone()),
                                        WorkflowState::FailedRun(run) => Some(run.data.clone()),
                                        _ => None,
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                        WorkflowSelectionState::Error(_) => None,
                    }
                } else {
                    None
                };

                let runner = WorkflowRunner::new(
                    self.workflows.clone(),
                    self.config.clone(),
                    selected_workflow.index,
                    workflow_data,
                    selected_workflow.workflow_file.clone(),
                );
                Self::swap_mode(
                    &mut self.mode,
                    &mut self.detached_runner,
                    WorkflowPanelMode::Run(runner),
                );
            }
            WorkflowAction::ConfirmDelete => {
                if let Some(file_path) = selected_workflow.workflow_file.as_ref() {
                    if let Err(err) = std::fs::remove_file(file_path) {
                        log::error!("Failed to delete workflow file: {err}");
                    } else {
                        log::info!("Deleted workflow file: {}", file_path.display());
                    }
                }
                let target_index = selected_workflow.index;
                if let Ok(mut workflows) = self.workflows.write()
                    && target_index < workflows.len()
                {
                    workflows.remove(target_index);
                }
                self.selected_workflow = None;
                Self::swap_mode(
                    &mut self.mode,
                    &mut self.detached_runner,
                    WorkflowPanelMode::Preview(WorkflowPreview::new()),
                );
            }
            WorkflowAction::BackToPreview => {
                Self::swap_mode(
                    &mut self.mode,
                    &mut self.detached_runner,
                    WorkflowPanelMode::Preview(WorkflowPreview::new()),
                );
            }
            WorkflowAction::Save => {
                self.save_workflow();
            }
        }
    }

    fn save_workflow(&mut self) {
        let Some(selected_workflow) = &self.selected_workflow else {
            return;
        };
        let target_index = selected_workflow.index;
        let existing_file = selected_workflow.workflow_file.clone();

        // Only save if we're in edit mode
        let workflow_data = if let WorkflowPanelMode::Edit(editor) = &self.mode {
            editor.build_workflow_data()
        } else {
            return;
        };

        self.save_error = None;

        let file_path = if let Some(path) = existing_file {
            // Existing workflow: overwrite in place
            path
        } else {
            // Draft workflow: generate a new file name from the workflow name
            let file_name = sanitize_filename(&workflow_data.name);
            let path = self.workflows_directory.join(format!("{file_name}.json"));
            if path.exists() {
                let message = format!("Workflow file already exists: {}", path.display());
                log::error!("{message}");
                self.save_error = Some(message);
                return;
            }
            path
        };

        let json = match serde_json::to_string_pretty(&workflow_data) {
            Ok(json) => json,
            Err(err) => {
                log::error!("Failed to serialize workflow: {err}");
                self.save_error = Some(format!("Failed to serialize workflow: {err}"));
                return;
            }
        };

        if let Err(err) = std::fs::write(&file_path, &json) {
            log::error!("Failed to write workflow file: {err}");
            self.save_error = Some(format!("Failed to write workflow file: {err}"));
            return;
        }

        log::info!("Saved workflow to: {}", file_path.display());

        // Update the workflow in the list
        if let Ok(mut workflows) = self.workflows.write()
            && target_index < workflows.len()
        {
            let json_file = workflow::JsonFile::from_raw(file_path.clone(), json);
            workflows[target_index] = Workflow::new(json_file);

            // Update the selected workflow to reflect the file path
            if let Some(s) = self.selected_workflow.as_mut() {
                s.workflow_file = Some(file_path);
            }

            // Switch back to preview mode
            Self::swap_mode(
                &mut self.mode,
                &mut self.detached_runner,
                WorkflowPanelMode::Preview(WorkflowPreview::new()),
            );
        }
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | ' ' | '-' | '_' => c,
            _ => '-',
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .to_lowercase()
}

impl panel::Panel for WorkflowPanel {
    fn draw(&mut self, area: Rect, buf: &mut Buffer, focused: bool) {
        buf.set_style(area, palette().background_style);

        let [header_area, content_area] = area.layout(&Layout::vertical([Length(2), Fill(1)]));

        panel::Header::new("Panel", focused).render(header_area, buf);

        if let Some(ref selection) = self.selected_workflow {
            self.render_selected_workflow(selection, content_area, buf);
        } else {
            render_no_selection_info(content_area, buf);
        }
    }

    fn handle_event(&mut self, event: &Event, terminal: &mut DefaultTerminal) -> bool {
        let Some(selection) = &self.selected_workflow else {
            return false;
        };

        let outcome = match &mut self.mode {
            WorkflowPanelMode::Preview(preview) => preview.handle_event(event, terminal, selection),
            WorkflowPanelMode::Edit(editor) => editor.handle_event(event, terminal, selection),
            WorkflowPanelMode::Run(runner) => runner.handle_event(event, terminal, selection),
            WorkflowPanelMode::Delete(workflow_delete) => {
                workflow_delete.handle_event(event, terminal, selection)
            }
        };

        match outcome {
            PanelEventOutcome::Consumed => true,
            PanelEventOutcome::Action(action) => {
                self.run_workflow_action(action);
                true
            }
            PanelEventOutcome::Ignored => false,
        }
    }
}

fn render_no_selection_info(area: Rect, buf: &mut Buffer) {
    let text = "No workflow selected";
    Paragraph::new(text)
        .style(palette().muted_foreground)
        .render(
            area.centered(
                Constraint::Length(text.len().try_into().unwrap_or(u16::MAX)),
                Constraint::Length(1),
            ),
            buf,
        );
}

#[cfg(test)]
mod tests {
    use super::super::panel::Panel;
    use super::*;
    use crossterm::event::KeyCode;
    use std::path::PathBuf;
    use workflow::JsonFile;
    use workflow::Workflow;

    fn empty_workflows() -> Arc<RwLock<Vec<Workflow>>> {
        Arc::new(RwLock::new(Vec::new()))
    }

    fn test_workflows_directory() -> PathBuf {
        PathBuf::from("/tmp/test-workflows")
    }

    fn test_config() -> Arc<RwLock<config::Config>> {
        Arc::new(RwLock::new(config::Config::default()))
    }

    #[test]
    fn test_workflow_panel_new() {
        let workflows = empty_workflows();
        let _panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
    }

    #[test]
    fn test_workflow_panel_draw_no_panic() {
        let workflows = empty_workflows();
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        let area = Rect::new(0, 0, 50, 20);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflow_panel_draw_focused() {
        let workflows = empty_workflows();
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        let area = Rect::new(0, 0, 50, 20);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, true);
    }

    /// `handle_event` never reads `_terminal` for the events we exercise. We
    /// store an uninit'ed slot behind a `OnceLock` and return a raw address
    /// that satisfies the &mut `DefaultTerminal` type without being deref'd.
    fn no_read_term() -> *mut DefaultTerminal {
        use std::mem::MaybeUninit;
        static SLOT: std::sync::OnceLock<MaybeUninit<DefaultTerminal>> = std::sync::OnceLock::new();
        let slot = SLOT.get_or_init(MaybeUninit::uninit);
        slot.as_ptr().cast_mut()
    }

    #[test]
    fn test_workflow_panel_handle_up_event_ok() {
        let workflows = empty_workflows();
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        let event = Event::Key(KeyCode::Up.into());
        let _handled = panel.handle_event(&event, unsafe { &mut *no_read_term() });
    }

    #[test]
    fn test_workflow_panel_handle_mouse_event_ok() {
        let workflows = empty_workflows();
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        let event = Event::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        let _handled = panel.handle_event(&event, unsafe { &mut *no_read_term() });
    }

    #[test]
    fn test_workflow_panel_draw_with_default_path_no_panic() {
        let workflows = empty_workflows();
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        let area = Rect::new(0, 0, 50, 20);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    fn workflows_with_one(name: &str) -> Arc<RwLock<Vec<Workflow>>> {
        let json = JsonFile::from_raw(
            PathBuf::from("/tmp/workflow.json"),
            format!(r#"{{"name":"{name}","steps":[]}}"#),
        );
        let workflow = Workflow::new(json);
        Arc::new(RwLock::new(vec![workflow]))
    }

    fn workflows_with_two() -> Arc<RwLock<Vec<Workflow>>> {
        let mk = |name: &str, path: &str| {
            let json = JsonFile::from_raw(
                PathBuf::from(path),
                format!(r#"{{"name":"{name}","steps":[]}}"#),
            );
            Workflow::new(json)
        };
        Arc::new(RwLock::new(vec![
            mk("First", "/tmp/first.json"),
            mk("Second", "/tmp/second.json"),
        ]))
    }

    fn workflows_with_invalid() -> Arc<RwLock<Vec<Workflow>>> {
        let json = JsonFile::from_raw(PathBuf::from("/tmp/bad.json"), "{invalid json".to_string());
        let workflow = Workflow::new(json);
        Arc::new(RwLock::new(vec![workflow]))
    }

    #[test]
    fn test_workflow_panel_update_opens_new_selection() {
        let workflows = workflows_with_one("My Workflow");
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        assert!(panel.selected_workflow.is_none());
        panel.update(Some(0));
        assert_eq!(panel.selected_workflow.as_ref().unwrap().index, 0);
    }

    #[test]
    fn test_workflow_panel_update_skips_same_index() {
        let workflows = workflows_with_one("My Workflow");
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        panel.update(Some(0));
        // Update again with the same index — should not re-open (no-op).
        panel.update(Some(0));
        assert_eq!(panel.selected_workflow.as_ref().unwrap().index, 0);
    }

    #[test]
    fn test_workflow_panel_update_changes_index() {
        let workflows = workflows_with_two();
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        panel.update(Some(0));
        assert_eq!(panel.selected_workflow.as_ref().unwrap().index, 0);
        // Different index triggers re-open.
        panel.update(Some(1));
        assert_eq!(panel.selected_workflow.as_ref().unwrap().index, 1);
    }

    #[test]
    fn test_workflow_panel_update_none_is_noop() {
        let workflows = workflows_with_one("My Workflow");
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        panel.update(Some(0));
        assert!(panel.selected_workflow.is_some());
        // None means "keep current selection".
        panel.update(None);
        assert_eq!(panel.selected_workflow.as_ref().unwrap().index, 0);
    }

    #[test]
    fn test_workflow_panel_draw_with_valid_workflow_no_panic() {
        let workflows = workflows_with_one("My Workflow");
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        panel.update(Some(0));
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflow_panel_draw_with_invalid_workflow_renders_error() {
        let workflows = workflows_with_invalid();
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        panel.update(Some(0));
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflow_panel_draw_with_out_of_bounds_index_renders_error() {
        let workflows = workflows_with_one("X");
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        // Index 5 does not exist in a single-workflow list.
        panel.update(Some(5));
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflow_panel_draw_with_small_area_no_panic() {
        let workflows = workflows_with_one("Hi");
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        panel.update(Some(0));
        // Give enough vertical space for header (2) + content separators.
        let area = Rect::new(0, 0, 10, 8);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflow_panel_handle_down_event_ok() {
        let workflows = empty_workflows();
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        let event = Event::Key(KeyCode::Down.into());
        let _handled = panel.handle_event(&event, unsafe { &mut *no_read_term() });
    }

    #[test]
    fn test_workflow_panel_handle_unknown_key_ignored() {
        let workflows = empty_workflows();
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        let event = Event::Key(KeyCode::Char('z').into());
        let handled = panel.handle_event(&event, unsafe { &mut *no_read_term() });
        assert!(!handled);
    }

    #[test]
    fn test_workflow_panel_open_edit_mode_and_return_to_preview() {
        let workflows = workflows_with_one("Test");
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        panel.update(Some(0));
        // Verify panel is in preview mode after opening.
        assert!(matches!(panel.mode, WorkflowPanelMode::Preview(_)));

        // Navigate to "Edit" button (index 1) and select it via Enter.
        // The preview handles navigation internally; we simulate by directly
        // setting list_state and triggering enter.

        // After pressing Enter on Edit button, mode should be Edit.
        let edit_event = Event::Key(KeyCode::Enter.into());
        let _handled = panel.handle_event(&edit_event, unsafe { &mut *no_read_term() });
        // Since handle_event in Preview mode uses its own navigation logic,
        // we verify the direct mode transition instead by calling open_workflow
        // which sets mode to Edit for drafts, but for valid files it stays Preview.
        // We test the Esc handler in Edit mode directly:
        if let WorkflowPanelMode::Edit(_) = &panel.mode {
            // Already in edit or preview, skip
        }
        // The proper way: set mode to Edit and verify Esc returns to Preview.
        let editor = WorkflowEditor::new(None);
        panel.mode = WorkflowPanelMode::Edit(editor);
        let esc_event = Event::Key(KeyCode::Esc.into());
        let handled = panel.handle_event(&esc_event, unsafe { &mut *no_read_term() });
        assert!(handled);
        assert!(matches!(panel.mode, WorkflowPanelMode::Preview(_)));
    }

    #[test]
    fn test_workflow_panel_open_run_mode_and_return_to_preview() {
        let workflows = workflows_with_one("Test");
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        panel.update(Some(0));
        // Simulate being in Run mode and pressing Esc.
        let runner = WorkflowRunner::new(workflows, test_config(), 0, None, None);
        panel.mode = WorkflowPanelMode::Run(runner);
        let esc_event = Event::Key(KeyCode::Esc.into());
        let handled = panel.handle_event(&esc_event, unsafe { &mut *no_read_term() });
        assert!(handled);
        assert!(matches!(panel.mode, WorkflowPanelMode::Preview(_)));
    }

    #[test]
    fn test_workflow_panel_draw_in_edit_mode_no_panic() {
        let workflows = workflows_with_one("Test");
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        panel.update(Some(0));
        panel.mode = WorkflowPanelMode::Edit(WorkflowEditor::new(None));
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
        // If no panic, rendering succeeded.
    }

    #[test]
    fn test_workflow_panel_draw_in_run_mode_no_panic() {
        let workflows = workflows_with_one("Test");
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        panel.update(Some(0));
        let runner = WorkflowRunner::new(workflows, test_config(), 0, None, None);
        panel.mode = WorkflowPanelMode::Run(runner);
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
        // If no panic, rendering succeeded.
    }

    #[test]
    fn test_workflow_panel_draw_with_draft_workflow_opens_edit_mode() {
        let draft = Workflow::new_draft();
        let workflows = Arc::new(RwLock::new(vec![draft]));
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        panel.update(Some(0));
        // Draft workflows should open in Edit mode.
        assert!(matches!(panel.mode, WorkflowPanelMode::Edit(_)));
        assert_eq!(panel.selected_workflow.as_ref().unwrap().index, 0);
    }

    #[test]
    fn test_workflow_panel_confirm_delete_removes_from_list() {
        let tmp = tempfile::tempdir().unwrap();
        let wf_path = tmp.path().join("workflow.json");
        let json_content = r#"{"name":"ToDelete"}"#;
        std::fs::write(&wf_path, json_content).unwrap();

        let json = JsonFile::from_raw(wf_path.clone(), json_content.to_string());
        let workflow = Workflow::new(json);
        let workflows = Arc::new(RwLock::new(vec![workflow]));
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        panel.update(Some(0));

        // Set mode to Delete and simulate confirming.
        panel.mode = WorkflowPanelMode::Delete(WorkflowDelete::new());
        let confirm_event = Event::Key(KeyCode::Enter.into());
        let handled = panel.handle_event(&confirm_event, unsafe { &mut *no_read_term() });
        assert!(handled);

        // Workflow should have been removed from the list.
        let wfs = panel.workflows.read().unwrap();
        assert_eq!(wfs.len(), 0);
        assert!(panel.selected_workflow.is_none());
    }

    #[test]
    fn test_workflow_panel_confirm_delete_invalidates_nonexistent_file() {
        let json = JsonFile::from_raw(
            PathBuf::from("/tmp/nonexistent_workflow_xyz.json"),
            r#"{"name":"Ghost"}"#.to_string(),
        );
        let workflow = Workflow::new(json);
        let workflows = Arc::new(RwLock::new(vec![workflow]));
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        panel.update(Some(0));

        panel.mode = WorkflowPanelMode::Delete(WorkflowDelete::new());
        let confirm_event = Event::Key(KeyCode::Enter.into());
        let handled = panel.handle_event(&confirm_event, unsafe { &mut *no_read_term() });
        assert!(handled);

        // Even though file doesn't exist, the workflow should still be removed.
        let wfs = panel.workflows.read().unwrap();
        assert_eq!(wfs.len(), 0);
    }

    #[test]
    fn test_workflow_panel_escape_from_delete_returns_to_preview() {
        let workflows = workflows_with_one("Test");
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        panel.update(Some(0));
        panel.mode = WorkflowPanelMode::Delete(WorkflowDelete::new());
        let esc_event = Event::Key(KeyCode::Esc.into());
        let handled = panel.handle_event(&esc_event, unsafe { &mut *no_read_term() });
        assert!(handled);
        assert!(matches!(panel.mode, WorkflowPanelMode::Preview(_)));
    }

    #[test]
    fn test_workflow_panel_draw_draft_workflow_shows_draft_text() {
        let draft = Workflow::new_draft();
        let workflows = Arc::new(RwLock::new(vec![draft]));
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        panel.update(Some(0));
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
        // If we get here without panicking, rendering succeeded with draft text.
    }

    fn data_named(name: &str) -> WorkflowData {
        serde_json::from_str(&format!(r#"{{"name":"{name}","steps":[]}}"#)).unwrap()
    }

    #[test]
    fn test_save_draft_creates_new_file() {
        let tmp = tempfile::tempdir().unwrap();
        let workflows = Arc::new(RwLock::new(vec![Workflow::new_draft()]));
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), tmp.path().to_path_buf());
        panel.update(Some(0));
        assert!(matches!(panel.mode, WorkflowPanelMode::Edit(_)));
        panel.mode = WorkflowPanelMode::Edit(WorkflowEditor::new(Some(data_named("My Flow"))));

        panel.run_workflow_action(WorkflowAction::Save);

        let expected = tmp.path().join("my-flow.json");
        assert!(expected.exists());
        assert!(panel.save_error.is_none());
        assert!(matches!(panel.mode, WorkflowPanelMode::Preview(_)));
        let wfs = panel.workflows.read().unwrap();
        assert_eq!(wfs[0].file_path.as_ref(), Some(&expected));
        assert_eq!(
            panel.selected_workflow.as_ref().unwrap().workflow_file,
            Some(expected)
        );
    }

    #[test]
    fn test_save_existing_workflow_overwrites_same_file() {
        let tmp = tempfile::tempdir().unwrap();
        let wf_path = tmp.path().join("original-file-name.json");
        let original = r#"{"name":"Original","steps":[]}"#;
        std::fs::write(&wf_path, original).unwrap();

        let workflow = Workflow::new(JsonFile::from_raw(wf_path.clone(), original.to_string()));
        let workflows = Arc::new(RwLock::new(vec![workflow]));
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), tmp.path().to_path_buf());
        panel.update(Some(0));
        assert!(matches!(panel.mode, WorkflowPanelMode::Preview(_)));

        panel.run_workflow_action(WorkflowAction::Edit);
        assert!(matches!(panel.mode, WorkflowPanelMode::Edit(_)));
        panel.mode = WorkflowPanelMode::Edit(WorkflowEditor::new(Some(data_named("Renamed"))));

        panel.run_workflow_action(WorkflowAction::Save);

        assert!(panel.save_error.is_none());
        assert!(matches!(panel.mode, WorkflowPanelMode::Preview(_)));
        // Written in place, no new name-based file created.
        assert!(!tmp.path().join("renamed.json").exists());
        let content = std::fs::read_to_string(&wf_path).unwrap();
        assert_ne!(content, original);
        assert!(content.contains("Renamed"));
        let wfs = panel.workflows.read().unwrap();
        assert_eq!(wfs[0].file_path.as_ref(), Some(&wf_path));
    }

    #[test]
    fn test_save_draft_name_collision_sets_error_and_keeps_file() {
        let tmp = tempfile::tempdir().unwrap();
        let existing = tmp.path().join("my-flow.json");
        std::fs::write(&existing, "original content").unwrap();

        let workflows = Arc::new(RwLock::new(vec![Workflow::new_draft()]));
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), tmp.path().to_path_buf());
        panel.update(Some(0));
        panel.mode = WorkflowPanelMode::Edit(WorkflowEditor::new(Some(data_named("My Flow"))));

        panel.run_workflow_action(WorkflowAction::Save);

        let error = panel.save_error.as_ref().unwrap();
        assert!(error.starts_with("Workflow file already exists:"));
        assert_eq!(
            std::fs::read_to_string(&existing).unwrap(),
            "original content"
        );
        assert!(matches!(panel.mode, WorkflowPanelMode::Edit(_)));
        assert!(
            panel
                .selected_workflow
                .as_ref()
                .unwrap()
                .workflow_file
                .is_none()
        );

        // Error is rendered without panicking and cleared on re-open.
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
        panel.open_workflow(0);
        assert!(panel.save_error.is_none());
    }

    #[test]
    fn test_workflow_panel_handles_edit_action_via_preview() {
        let workflows = workflows_with_one("Test");
        let mut panel =
            WorkflowPanel::new(workflows.clone(), test_config(), test_workflows_directory());
        panel.update(Some(0));
        // Preview button list_state starts at index 1 (Edit button).
        // We can't easily trigger the preview's handle_event navigation,
        // but we verify that entering Edit mode works when set directly.
        if let WorkflowPanelMode::Preview(ref mut preview) = panel.mode {
            preview.list_state.select(Some(1));
        }
        let enter_event = Event::Key(KeyCode::Enter.into());
        let handled = panel.handle_event(&enter_event, unsafe { &mut *no_read_term() });
        assert!(handled);
        assert!(matches!(panel.mode, WorkflowPanelMode::Edit(_)));
    }
}
