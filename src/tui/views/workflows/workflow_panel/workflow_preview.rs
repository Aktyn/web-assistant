use crossterm::event::Event;
use ratatui::DefaultTerminal;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint::{Fill, Length, Ratio};
use ratatui::layout::{Layout, Offset, Rect};
use ratatui::text::ToLine as _;
use ratatui::widgets::{ListState, Widget as _};

use super::super::super::common::utils::{NavigationEvent, handle_navigation_event};
use super::super::widgets::{Button, ButtonData, ErrorAlert, Separator};
use super::{WorkflowAction, WorkflowPanelModeView};

const OPTIONS_HEIGHT: u16 = 3;

type ThisButtonData = ButtonData<WorkflowAction>;

pub(crate) struct WorkflowPreview {
    /// Used for navigating over option buttons
    pub(crate) list_state: ListState,
    buttons: [ThisButtonData; 4],
}

impl WorkflowPreview {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            buttons: [
                ThisButtonData::new("Open file", WorkflowAction::OpenFile),
                ThisButtonData::new("Edit", WorkflowAction::Edit),
                ThisButtonData::new("Delete", WorkflowAction::Delete),
                ThisButtonData::new("Execution", WorkflowAction::Execution),
            ],
        }
    }

    fn render_selected_workflow_options(
        &self,
        selection: &super::WorkflowSelection,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let button_areas: [Rect; 4] = area.layout(&Layout::horizontal([Ratio(1, 4); 4]));

        assert!(
            button_areas.len() == self.buttons.len(),
            "button_areas.len() != self.buttons.len()"
        );

        // WorkflowSelectionState::Error(error) => {
        let is_invalid = matches!(selection.state, super::WorkflowSelectionState::Error(_));

        self.buttons
            .iter()
            .enumerate()
            .for_each(|(index, button_data)| {
                Button::new(button_data.label.to_line())
                    .set_selected(self.list_state.selected() == Some(index))
                    .set_disabled(
                        (button_data.action == WorkflowAction::Execution
                            || button_data.action == WorkflowAction::Edit)
                            && is_invalid,
                    )
                    .render(button_areas[index], buf);
            });
    }

    fn get_button_action(
        &self,
        selected: usize,
        selection: &super::WorkflowSelection,
    ) -> Option<WorkflowAction> {
        assert!(
            selected < self.buttons.len(),
            "selected >= self.buttons.len()"
        );

        let button_data = &self.buttons[selected];

        let is_invalid = matches!(selection.state, super::WorkflowSelectionState::Error(_));

        match button_data.action {
            WorkflowAction::OpenFile => Some(WorkflowAction::OpenFile),
            WorkflowAction::Delete => Some(super::WorkflowAction::Delete),
            WorkflowAction::Edit => {
                if is_invalid {
                    None
                } else {
                    Some(WorkflowAction::Edit)
                }
            }
            WorkflowAction::Execution => {
                if is_invalid {
                    None
                } else {
                    Some(WorkflowAction::Execution)
                }
            }
            _ => None,
        }
    }
}

impl WorkflowPanelModeView for WorkflowPreview {
    fn render(&self, selection: &super::WorkflowSelection, area: Rect, buf: &mut Buffer) {
        let [options_area, state_separator_area, content_area] = area.layout(&Layout::vertical([
            Length(OPTIONS_HEIGHT),
            Length(1),
            Fill(1),
        ]));

        self.render_selected_workflow_options(selection, options_area, buf);

        Separator::new(Separator::HORIZONTAL).render(state_separator_area, buf);

        render_workflow_state(&selection.state, content_area, buf);
    }

    fn handle_event(
        &mut self,
        event: &Event,
        _terminal: &mut DefaultTerminal,
        selection: &super::WorkflowSelection,
    ) -> Option<WorkflowAction> {
        if let Some(key) = event.as_key_event() {
            let navigation =
                handle_navigation_event(key, &mut self.list_state, self.buttons.len() - 1);
            if let Some(event) = navigation
                && let NavigationEvent::Select(selected) = event
            {
                return self.get_button_action(selected, selection);
            }
        }

        None
    }
}

fn render_workflow_state(state: &super::WorkflowSelectionState, area: Rect, buf: &mut Buffer) {
    match state {
        super::WorkflowSelectionState::Valid(valid_workflow_selection) => {
            render_valid_workflow(valid_workflow_selection, area, buf);
        }
        super::WorkflowSelectionState::Error(error) => {
            let text = vec![
                "The file is invalid".into(),
                error.clone().into(),
                "Please edit the file to fix it".into(),
            ];
            ErrorAlert::new(text).render(area, buf);
        }
    }
}

fn render_valid_workflow(workflow: &super::ValidWorkflowState, area: Rect, buf: &mut Buffer) {
    let mut offset_y: u16 = 0;
    let mut offset_x: u16 = 0;

    for field in [
        Some(&workflow.name_field),
        workflow.description_field.as_ref(),
    ]
    .iter()
    .flatten()
    {
        let required_width = field.estimate_width();
        if offset_x + required_width >= area.width {
            offset_x = 0;
            offset_y += field.estimate_height();
        }

        field.draw(
            area.offset(Offset {
                x: offset_x.into(),
                y: offset_y.into(),
            }),
            buf,
        );
        offset_x += required_width;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;

    /// Build a valid selection using an already-constructed `ValidWorkflowState`.
    fn valid_selection() -> super::super::WorkflowSelection {
        super::super::WorkflowSelection {
            index: 0,
            workflow_file: None,
            state: super::super::WorkflowSelectionState::Valid(super::super::ValidWorkflowState {
                name_field: super::super::super::super::common::widgets::TextField::new(
                    "Name".to_string(),
                    "My Workflow".to_string(),
                ),
                description_field: None,
            }),
        }
    }

    /// Build an error selection that disables Edit and Execution buttons.
    fn error_selection() -> super::super::WorkflowSelection {
        super::super::WorkflowSelection {
            index: 0,
            workflow_file: None,
            state: super::super::WorkflowSelectionState::Error("file is bad".to_string()),
        }
    }

    #[test]
    fn test_workflow_preview_new_has_four_buttons() {
        let preview = WorkflowPreview::new();
        // The internal button array has length 4 — we verify via rendering.
        let selection = valid_selection();
        let area = Rect::new(0, 0, 80, 5);
        let mut buf = Buffer::empty(area);
        preview.render(&selection, area, &mut buf);
        // If rendering didn't panic with the assertion, the counts match.
    }

    #[test]
    fn test_open_file_always_enabled() {
        let preview = WorkflowPreview::new();
        let selection = error_selection();
        let action = preview.get_button_action(0, &selection);
        assert_eq!(action, Some(WorkflowAction::OpenFile));
    }

    #[test]
    fn test_delete_always_enabled() {
        let preview = WorkflowPreview::new();
        let selection = error_selection();
        let action = preview.get_button_action(2, &selection);
        assert_eq!(action, Some(WorkflowAction::Delete));
    }

    #[test]
    fn test_edit_disabled_when_invalid() {
        let preview = WorkflowPreview::new();
        let selection = error_selection();
        let action = preview.get_button_action(1, &selection);
        assert!(action.is_none());
    }

    #[test]
    fn test_edit_enabled_when_valid() {
        let preview = WorkflowPreview::new();
        let selection = valid_selection();
        let action = preview.get_button_action(1, &selection);
        assert_eq!(action, Some(WorkflowAction::Edit));
    }

    #[test]
    fn test_execution_disabled_when_invalid() {
        let preview = WorkflowPreview::new();
        let selection = error_selection();
        let action = preview.get_button_action(3, &selection);
        assert!(action.is_none());
    }

    #[test]
    fn test_execution_enabled_when_valid() {
        let preview = WorkflowPreview::new();
        let selection = valid_selection();
        let action = preview.get_button_action(3, &selection);
        assert_eq!(action, Some(WorkflowAction::Execution));
    }

    #[test]
    fn test_handle_navigation_enter_selects_action() {
        let mut preview = WorkflowPreview::new();
        let selection = valid_selection();
        // ListState defaults to `None`; select index 0 so Enter resolves to OpenFile.
        preview.list_state.select(Some(0));
        let event = Event::Key(KeyCode::Enter.into());
        let action = unsafe { preview.handle_event(&event, &mut *no_read_term(), &selection) };
        assert_eq!(action, Some(WorkflowAction::OpenFile));
    }

    #[test]
    fn test_render_with_error_selection_no_panic() {
        let preview = WorkflowPreview::new();
        let selection = error_selection();
        let area = Rect::new(0, 0, 80, 10);
        let mut buf = Buffer::empty(area);
        preview.render(&selection, area, &mut buf);
    }

    /// Returns a `&mut DefaultTerminal` that is never dereferenced by the
    /// views we exercise (they ignore the `_terminal` parameter).
    fn no_read_term() -> *mut ratatui::DefaultTerminal {
        use std::mem::MaybeUninit;
        static SLOT: std::sync::OnceLock<MaybeUninit<ratatui::DefaultTerminal>> =
            std::sync::OnceLock::new();
        let slot = SLOT.get_or_init(MaybeUninit::uninit);
        slot.as_ptr().cast_mut()
    }
}
