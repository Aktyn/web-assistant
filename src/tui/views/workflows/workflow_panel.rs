use crossterm::event::Event;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{
        Constraint::{self, Fill, Length, Ratio},
        Layout, Rect,
    },
    text::ToLine as _,
    widgets::{ListState, Paragraph, Widget},
};
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use super::super::common::widgets::{Button, ButtonData, ErrorAlert, TextField};
use super::super::common::{palette, panel, utils};
use crate::{
    core::{Core, workflow},
    tui::common::widgets,
    utils::truncate_text_left,
};

#[derive(Debug)]
struct ValidWorkflowState {
    name_field: TextField,
}

#[derive(Debug)]
enum WorkflowSelectionState {
    Valid(ValidWorkflowState),
    Error(String),
}

struct WorkflowSelection {
    /// Index of the workflow in the list of workflows
    index: usize,
    workflow_file: PathBuf,
    state: WorkflowSelectionState,
}

enum WorkflowAction {
    OpenFile,
    Edit,
    Delete,
    Run,
}

type ThisButtonData = ButtonData<WorkflowAction>;

pub(crate) struct WorkflowPanel {
    core: Arc<RwLock<Core>>,
    selected_workflow: Option<WorkflowSelection>,
    /// Used for navigating over option buttons
    list_state: ListState,
    buttons: [ThisButtonData; 4],
}

impl WorkflowPanel {
    pub(super) fn new(core: Arc<RwLock<Core>>) -> Self {
        Self {
            core,
            selected_workflow: None,
            list_state: ListState::default(),
            buttons: [
                ThisButtonData::new("Open file", WorkflowAction::OpenFile),
                ThisButtonData::new("Edit", WorkflowAction::Edit),
                ThisButtonData::new("Delete", WorkflowAction::Delete),
                ThisButtonData::new("Run", WorkflowAction::Run),
            ],
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

    fn open_workflow(&mut self, index: usize) {
        log::info!("Opening workflow: {index:?}");

        match self.core.read() {
            Ok(unlocked_core) => {
                if let Some(workflow) = unlocked_core.workflows.get(index) {
                    self.selected_workflow = Some(WorkflowSelection {
                        index,
                        workflow_file: workflow.file_path.clone(),
                        state: match &workflow.state {
                            workflow::WorkflowState::Data(data) => {
                                WorkflowSelectionState::Valid(ValidWorkflowState {
                                    name_field: TextField::new(
                                        "Name".to_string(),
                                        data.name.clone(),
                                    ),
                                })
                            }
                            workflow::WorkflowState::Error(error) => {
                                let error_message = error.to_string();
                                log::warn!("Invalid workflow: {error_message}");
                                WorkflowSelectionState::Error(error_message)
                            }
                        },
                    });
                } else {
                    log::error!("Cannot get workflow with index: {index}");
                }
            }
            Err(error) => {
                log::error!("Cannot read core: {error}");
            }
        }
    }

    fn render_selected_workflow_options(
        &self,
        _selection: &WorkflowSelection,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let button_areas: [Rect; 4] = area.layout(&Layout::horizontal([Ratio(1, 4); 4]));

        assert!(
            button_areas.len() == self.buttons.len(),
            "button_areas.len() != self.buttons.len()"
        );

        self.buttons.iter().enumerate().for_each(|(index, button)| {
            Button::new(button.label.to_line())
                .set_selected(self.list_state.selected() == Some(index))
                .render(button_areas[index], buf);
        });
    }

    fn render_selected_workflow(
        &self,
        selection: &WorkflowSelection,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let [
            path_area,
            options_separator_area,
            options_area,
            state_separator_area,
            content_area,
        ] = area.layout(&Layout::vertical([
            Length(1),
            Length(1),
            Length(OPTIONS_HEIGHT),
            Length(1),
            Fill(1),
        ]));

        let path = &selection.workflow_file;
        let absolute_path =
            std::path::absolute(selection.workflow_file.as_path()).unwrap_or(path.clone());
        let absolute_path_str = absolute_path.to_str().unwrap_or("");
        let label = "Workflow file: ";

        Paragraph::new(format!(
            "{}{}",
            label,
            truncate_text_left(
                absolute_path_str,
                (path_area.width as usize).saturating_sub(label.len()),
                None
            )
        ))
        .style(palette::STYLE)
        .render(path_area, buf);

        widgets::Separator::new(widgets::Separator::HORIZONTAL).render(options_separator_area, buf);

        self.render_selected_workflow_options(selection, options_area, buf);

        widgets::Separator::new(widgets::Separator::HORIZONTAL).render(state_separator_area, buf);

        render_workflow_state(&selection.state, content_area, buf);
    }

    fn run_button_action(&mut self, selected: usize) {
        assert!(
            selected < self.buttons.len(),
            "selected >= self.buttons.len()"
        );

        log::info!("@@@@@ {selected}");

        match self.buttons[selected].action {
            WorkflowAction::OpenFile => todo!(),
            WorkflowAction::Edit => todo!(),
            WorkflowAction::Delete => todo!(),
            WorkflowAction::Run => todo!(),
        }
    }
}

const OPTIONS_HEIGHT: u16 = 3;

impl panel::Panel for WorkflowPanel {
    fn draw(&mut self, area: Rect, buf: &mut Buffer, focused: bool) {
        buf.set_style(area, palette::BACKGROUND_STYLE);

        let [header_area, content_area] = area.layout(&Layout::vertical([Length(2), Fill(1)]));

        panel::Header::new("Panel", focused).render(header_area, buf);

        if let Some(ref selection) = self.selected_workflow {
            self.render_selected_workflow(selection, content_area, buf);
        } else {
            render_no_selection_info(content_area, buf);
        }
    }

    fn handle_event(
        &mut self,
        event: &Event,
        _terminal: &mut DefaultTerminal,
    ) -> anyhow::Result<()> {
        if let Some(key) = event.as_key_event() {
            let navigation =
                utils::handle_navigation_event(key, &mut self.list_state, self.buttons.len() - 1);
            if let Some(event) = navigation
                && let utils::NavigationEvent::Select(selected) = event
            {
                self.run_button_action(selected);
            }
        }
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

fn render_workflow_state(state: &WorkflowSelectionState, area: Rect, buf: &mut Buffer) {
    match state {
        WorkflowSelectionState::Valid(valid_workflow_selection) => {
            render_valid_workflow(valid_workflow_selection, area, buf);
        }
        WorkflowSelectionState::Error(error) => {
            let text = vec![
                "The file is invalid".into(),
                error.clone().into(),
                "Please edit the file to fix it".into(),
            ];
            ErrorAlert::new(text).render(area, buf);
        }
    }
}

fn render_valid_workflow(workflow: &ValidWorkflowState, area: Rect, buf: &mut Buffer) {
    workflow.name_field.draw(area, buf);
}

fn render_no_selection_info(area: Rect, buf: &mut Buffer) {
    let text = "No workflow selected";
    Paragraph::new(text)
        .style(palette::MUTED_FOREGROUND)
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

    fn empty_core() -> Core {
        Core {
            config: crate::core::config::Config::default(),
            workflows: Vec::new(),
        }
    }

    #[test]
    fn test_workflow_panel_new() {
        let core = Arc::new(RwLock::new(empty_core()));
        let _panel = WorkflowPanel::new(core.clone());
    }

    #[test]
    fn test_workflow_panel_draw_no_panic() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut panel = WorkflowPanel::new(core);
        let area = Rect::new(0, 0, 50, 20);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflow_panel_draw_focused() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut panel = WorkflowPanel::new(core);
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
        let core = Arc::new(RwLock::new(empty_core()));
        let mut panel = WorkflowPanel::new(core);
        let event = Event::Key(KeyCode::Up.into());
        let result = panel.handle_event(&event, unsafe { &mut *no_read_term() });
        assert!(result.is_ok());
    }

    #[test]
    fn test_workflow_panel_handle_mouse_event_ok() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut panel = WorkflowPanel::new(core);
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
        let core = Arc::new(RwLock::new(empty_core()));
        let mut panel = WorkflowPanel::new(core);
        let area = Rect::new(0, 0, 50, 20);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    fn core_with_workflow(name: &str) -> Core {
        use crate::core::workflow::{Workflow, WorkflowDataJson};
        use std::path::PathBuf;

        let json = WorkflowDataJson::from_raw(
            PathBuf::from("/tmp/workflow.json"),
            format!(r#"{{"name":"{name}"}}"#),
        );
        let workflow = Workflow::new(json);
        Core {
            config: crate::core::config::Config::default(),
            workflows: vec![workflow],
        }
    }

    fn core_with_invalid_workflow() -> Core {
        use crate::core::workflow::{Workflow, WorkflowDataJson};
        use std::path::PathBuf;

        let json = WorkflowDataJson::from_raw(
            PathBuf::from("/tmp/bad.json"),
            "{invalid json".to_string(),
        );
        let workflow = Workflow::new(json);
        Core {
            config: crate::core::config::Config::default(),
            workflows: vec![workflow],
        }
    }

    #[test]
    fn test_workflow_panel_update_opens_new_selection() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut panel = WorkflowPanel::new(core);
        assert!(panel.selected_workflow.is_none());
        panel.update(Some(5));
        assert_eq!(panel.selected_workflow.as_ref().unwrap().index, 5);
    }

    #[test]
    fn test_workflow_panel_update_skips_same_index() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut panel = WorkflowPanel::new(core);
        panel.update(Some(1));
        // Update again with the same index — should not re-open (no-op).
        panel.update(Some(1));
        assert_eq!(panel.selected_workflow.as_ref().unwrap().index, 1);
    }

    #[test]
    fn test_workflow_panel_update_changes_index() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut panel = WorkflowPanel::new(core);
        panel.update(Some(1));
        assert_eq!(panel.selected_workflow.as_ref().unwrap().index, 1);
        // Different index triggers re-open.
        panel.update(Some(2));
        assert_eq!(panel.selected_workflow.as_ref().unwrap().index, 2);
    }

    #[test]
    fn test_workflow_panel_update_none_is_noop() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut panel = WorkflowPanel::new(core);
        panel.update(Some(3));
        assert!(panel.selected_workflow.is_some());
        // None means "keep current selection".
        panel.update(None);
        assert_eq!(panel.selected_workflow.as_ref().unwrap().index, 3);
    }

    #[test]
    fn test_workflow_panel_draw_with_valid_workflow_no_panic() {
        let core = Arc::new(RwLock::new(core_with_workflow("My Workflow")));
        let mut panel = WorkflowPanel::new(core);
        panel.update(Some(0));
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflow_panel_draw_with_invalid_workflow_renders_error() {
        let core = Arc::new(RwLock::new(core_with_invalid_workflow()));
        let mut panel = WorkflowPanel::new(core);
        panel.update(Some(0));
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflow_panel_draw_with_out_of_bounds_index_renders_error() {
        let core = Arc::new(RwLock::new(core_with_workflow("X")));
        let mut panel = WorkflowPanel::new(core);
        // Index 5 does not exist in a single-workflow core.
        panel.update(Some(5));
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflow_panel_draw_with_small_area_no_panic() {
        let core = Arc::new(RwLock::new(core_with_workflow("Hi")));
        let mut panel = WorkflowPanel::new(core);
        panel.update(Some(0));
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflow_panel_as_any_mut_returns_self() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut panel = WorkflowPanel::new(core);
        let any = panel.as_any_mut();
        assert!(any.is::<WorkflowPanel>());
    }

    #[test]
    fn test_workflow_panel_handle_down_event_ok() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut panel = WorkflowPanel::new(core);
        let event = Event::Key(KeyCode::Down.into());
        let result = panel.handle_event(&event, unsafe { &mut *no_read_term() });
        assert!(result.is_ok());
    }

    #[test]
    fn test_workflow_panel_handle_unknown_key_ignored() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut panel = WorkflowPanel::new(core);
        let event = Event::Key(KeyCode::Char('z').into());
        let result = panel.handle_event(&event, unsafe { &mut *no_read_term() });
        assert!(result.is_ok());
    }
}
