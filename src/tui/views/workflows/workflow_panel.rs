mod workflow_delete;
mod workflow_preview;

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

use super::super::common::widgets::{Separator, TextField};
use super::super::common::{palette, panel};
use crate::utils;
use crate::workflow::{Workflow, WorkflowState};
use workflow_delete::WorkflowDelete;
use workflow_preview::WorkflowPreview;

#[derive(PartialEq)]
enum WorkflowAction {
    BackToPreview,
    OpenFile,
    Edit,
    Delete,
    Execution,
    ConfirmDelete,
}

pub(super) enum WorkflowPanelMode {
    Preview(WorkflowPreview),
    Edit,
    Run,
    Delete(WorkflowDelete),
}

trait WorkflowPanelModeView {
    fn render(&self, selection: &WorkflowSelection, area: Rect, buf: &mut Buffer);
    fn handle_event(
        &mut self,
        event: &Event,
        _terminal: &mut DefaultTerminal,
        selection: &WorkflowSelection,
    ) -> Option<WorkflowAction>;
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
}

pub(crate) struct WorkflowPanel {
    workflows: Arc<RwLock<Vec<Workflow>>>,
    selected_workflow: Option<WorkflowSelection>,
    pub(super) mode: WorkflowPanelMode,
}

impl WorkflowPanel {
    pub(super) fn new(workflows: Arc<RwLock<Vec<Workflow>>>) -> Self {
        Self {
            workflows,
            selected_workflow: None,
            mode: WorkflowPanelMode::Preview(WorkflowPreview::new()),
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

        match self.workflows.read() {
            Ok(unlocked_workflows) => {
                if let Some(workflow) = unlocked_workflows.get(index) {
                    self.selected_workflow = Some(WorkflowSelection {
                        index,
                        workflow_file: workflow.file_path.clone(),
                        state: match &workflow.state {
                            WorkflowState::Data(data) => {
                                WorkflowSelectionState::Valid(ValidWorkflowState {
                                    name_field: TextField::new(
                                        "Name".to_string(),
                                        data.name.clone(),
                                    ),
                                    description_field: data.description.clone().map(
                                        |description| {
                                            TextField::new("Description".to_string(), description)
                                        },
                                    ),
                                })
                            }
                            WorkflowState::Error(error) => {
                                let error_message = error.to_string();
                                log::warn!("Invalid workflow: {error_message}");
                                WorkflowSelectionState::Error(error_message)
                            }
                        },
                    });

                    let is_draft = self
                        .selected_workflow
                        .as_ref()
                        .is_some_and(|w| w.workflow_file.is_none());

                    if is_draft {
                        self.mode = WorkflowPanelMode::Edit;
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
        let [path_area, options_separator_area, mode_area] =
            area.layout(&Layout::vertical([Length(1), Length(1), Fill(1)]));

        if let Some(workflow_file) = &selection.workflow_file {
            let absolute_path =
                std::path::absolute(workflow_file.as_path()).unwrap_or(workflow_file.clone());
            let absolute_path_str = absolute_path.to_str().unwrap_or("");
            let label = "Workflow file: ";

            Paragraph::new(format!(
                "{}{}",
                label,
                utils::truncate_text_left(
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
            WorkflowPanelMode::Edit => todo!(),
            WorkflowPanelMode::Run => todo!(),
            WorkflowPanelMode::Delete(workflow_delete) => {
                workflow_delete.render(selection, mode_area, buf);
            }
        }
    }

    fn run_workflow_action(&mut self, action: &WorkflowAction) {
        let Some(selected_workflow) = &self.selected_workflow else {
            return;
        };

        match action {
            WorkflowAction::OpenFile => {
                if let Some(file_path) = selected_workflow.workflow_file.as_ref()
                    && let Err(err) = utils::open_file(file_path)
                {
                    log::error!("Failed to open file: {err}");
                }
            }
            WorkflowAction::Edit => {
                self.mode = WorkflowPanelMode::Edit;
            }
            WorkflowAction::Delete => self.mode = WorkflowPanelMode::Delete(WorkflowDelete::new()),
            WorkflowAction::Execution => {
                self.mode = WorkflowPanelMode::Run;
            }
            WorkflowAction::ConfirmDelete => {
                todo!();
                // self.mode = WorkflowPanelMode::Preview(WorkflowPreview::new());
            }
            WorkflowAction::BackToPreview => {
                self.mode = WorkflowPanelMode::Preview(WorkflowPreview::new());
            }
        }
    }
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

        let action: Option<WorkflowAction> = match &mut self.mode {
            WorkflowPanelMode::Preview(preview) => preview.handle_event(event, terminal, selection),
            // WorkflowPanelMode::Edit => todo!(),
            // WorkflowPanelMode::Run => todo!(),
            WorkflowPanelMode::Delete(workflow_delete) => {
                workflow_delete.handle_event(event, terminal, selection)
            }
            _ => None,
        };
        if let Some(action) = action {
            self.run_workflow_action(&action);
            return true;
        }

        false
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
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
    use super::*;
    use crate::tui::common::panel::Panel;
    use crate::workflow::{Workflow, WorkflowDataJson};
    use crossterm::event::KeyCode;
    use std::path::PathBuf;

    fn empty_workflows() -> Arc<RwLock<Vec<Workflow>>> {
        Arc::new(RwLock::new(Vec::new()))
    }

    #[test]
    fn test_workflow_panel_new() {
        let workflows = empty_workflows();
        let _panel = WorkflowPanel::new(workflows.clone());
    }

    #[test]
    fn test_workflow_panel_draw_no_panic() {
        let workflows = empty_workflows();
        let mut panel = WorkflowPanel::new(workflows);
        let area = Rect::new(0, 0, 50, 20);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflow_panel_draw_focused() {
        let workflows = empty_workflows();
        let mut panel = WorkflowPanel::new(workflows);
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
        let mut panel = WorkflowPanel::new(workflows);
        let event = Event::Key(KeyCode::Up.into());
        let result = panel.handle_event(&event, unsafe { &mut *no_read_term() });
        assert!(result.is_ok());
    }

    #[test]
    fn test_workflow_panel_handle_mouse_event_ok() {
        let workflows = empty_workflows();
        let mut panel = WorkflowPanel::new(workflows);
        let event = Event::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        let result = panel.handle_event(&event, unsafe { &mut *no_read_term() });
        assert!(result.is_ok());
    }

    #[test]
    fn test_workflow_panel_draw_with_default_path_no_panic() {
        let workflows = empty_workflows();
        let mut panel = WorkflowPanel::new(workflows);
        let area = Rect::new(0, 0, 50, 20);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    fn workflows_with_one(name: &str) -> Arc<RwLock<Vec<Workflow>>> {
        let json = WorkflowDataJson::from_raw(
            PathBuf::from("/tmp/workflow.json"),
            format!(r#"{{"name":"{name}"}}"#),
        );
        let workflow = Workflow::new(json);
        Arc::new(RwLock::new(vec![workflow]))
    }

    fn workflows_with_two() -> Arc<RwLock<Vec<Workflow>>> {
        let mk = |name: &str, path: &str| {
            let json =
                WorkflowDataJson::from_raw(PathBuf::from(path), format!(r#"{{"name":"{name}"}}"#));
            Workflow::new(json)
        };
        Arc::new(RwLock::new(vec![
            mk("First", "/tmp/first.json"),
            mk("Second", "/tmp/second.json"),
        ]))
    }

    fn workflows_with_invalid() -> Arc<RwLock<Vec<Workflow>>> {
        let json =
            WorkflowDataJson::from_raw(PathBuf::from("/tmp/bad.json"), "{invalid json".to_string());
        let workflow = Workflow::new(json);
        Arc::new(RwLock::new(vec![workflow]))
    }

    #[test]
    fn test_workflow_panel_update_opens_new_selection() {
        let workflows = workflows_with_one("My Workflow");
        let mut panel = WorkflowPanel::new(workflows);
        assert!(panel.selected_workflow.is_none());
        panel.update(Some(0));
        assert_eq!(panel.selected_workflow.as_ref().unwrap().index, 0);
    }

    #[test]
    fn test_workflow_panel_update_skips_same_index() {
        let workflows = workflows_with_one("My Workflow");
        let mut panel = WorkflowPanel::new(workflows);
        panel.update(Some(0));
        // Update again with the same index — should not re-open (no-op).
        panel.update(Some(0));
        assert_eq!(panel.selected_workflow.as_ref().unwrap().index, 0);
    }

    #[test]
    fn test_workflow_panel_update_changes_index() {
        let workflows = workflows_with_two();
        let mut panel = WorkflowPanel::new(workflows);
        panel.update(Some(0));
        assert_eq!(panel.selected_workflow.as_ref().unwrap().index, 0);
        // Different index triggers re-open.
        panel.update(Some(1));
        assert_eq!(panel.selected_workflow.as_ref().unwrap().index, 1);
    }

    #[test]
    fn test_workflow_panel_update_none_is_noop() {
        let workflows = workflows_with_one("My Workflow");
        let mut panel = WorkflowPanel::new(workflows);
        panel.update(Some(0));
        assert!(panel.selected_workflow.is_some());
        // None means "keep current selection".
        panel.update(None);
        assert_eq!(panel.selected_workflow.as_ref().unwrap().index, 0);
    }

    #[test]
    fn test_workflow_panel_draw_with_valid_workflow_no_panic() {
        let workflows = workflows_with_one("My Workflow");
        let mut panel = WorkflowPanel::new(workflows);
        panel.update(Some(0));
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflow_panel_draw_with_invalid_workflow_renders_error() {
        let workflows = workflows_with_invalid();
        let mut panel = WorkflowPanel::new(workflows);
        panel.update(Some(0));
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflow_panel_draw_with_out_of_bounds_index_renders_error() {
        let workflows = workflows_with_one("X");
        let mut panel = WorkflowPanel::new(workflows);
        // Index 5 does not exist in a single-workflow list.
        panel.update(Some(5));
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflow_panel_draw_with_small_area_no_panic() {
        let workflows = workflows_with_one("Hi");
        let mut panel = WorkflowPanel::new(workflows);
        panel.update(Some(0));
        // Give enough vertical space for header (2) + content separators.
        let area = Rect::new(0, 0, 10, 8);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflow_panel_as_any_mut_returns_self() {
        let workflows = empty_workflows();
        let mut panel = WorkflowPanel::new(workflows);
        let any = panel.as_any_mut();
        assert!(any.is::<WorkflowPanel>());
    }

    #[test]
    fn test_workflow_panel_handle_down_event_ok() {
        let workflows = empty_workflows();
        let mut panel = WorkflowPanel::new(workflows);
        let event = Event::Key(KeyCode::Down.into());
        let result = panel.handle_event(&event, unsafe { &mut *no_read_term() });
        assert!(result.is_ok());
    }

    #[test]
    fn test_workflow_panel_handle_unknown_key_ignored() {
        let workflows = empty_workflows();
        let mut panel = WorkflowPanel::new(workflows);
        let event = Event::Key(KeyCode::Char('z').into());
        let result = panel.handle_event(&event, unsafe { &mut *no_read_term() });
        assert!(result.is_ok());
    }
}
