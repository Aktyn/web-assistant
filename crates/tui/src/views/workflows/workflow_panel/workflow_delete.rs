use crossterm::event::KeyCode;
use ratatui::layout::Constraint::{Fill, Length, Ratio};
use ratatui::layout::Layout;
use ratatui::text::ToLine as _;
use ratatui::widgets::{ListState, Widget as _};

use super::super::widgets::{Button, ButtonData, ErrorAlert};
use super::{PanelEventOutcome, WorkflowAction, WorkflowPanelModeView};

type ThisButtonData = ButtonData<WorkflowAction>;

pub(crate) struct WorkflowDelete {
    buttons: [ThisButtonData; 2],
    buttons_list_state: ListState,
}

impl Default for WorkflowDelete {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowDelete {
    pub fn new() -> Self {
        Self {
            buttons: [
                ThisButtonData::new("Delete", WorkflowAction::ConfirmDelete),
                ThisButtonData::new("Cancel", WorkflowAction::BackToPreview),
            ],
            buttons_list_state: ListState::default().with_selected(Some(0)),
        }
    }
}

impl WorkflowPanelModeView for WorkflowDelete {
    fn render(
        &self,
        _selection: &super::WorkflowSelection,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        let [text_area, buttons_area] = area.layout(&Layout::vertical([Fill(1), Length(3)]));

        ErrorAlert::new("Are you sure you want to permanently delete this workflow file?")
            .render(text_area, buf);

        let button_areas: [ratatui::prelude::Rect; 2] =
            buttons_area.layout(&Layout::horizontal([Ratio(1, 2); 2]));
        for (index, button_data) in self.buttons.iter().enumerate() {
            Button::new(button_data.label.to_line())
                .set_selected(self.buttons_list_state.selected() == Some(index))
                .render(button_areas[index], buf);
        }
    }

    fn handle_event(
        &mut self,
        event: &crossterm::event::Event,
        _terminal: &mut ratatui::DefaultTerminal,
        _selection: &super::WorkflowSelection,
    ) -> PanelEventOutcome {
        let Some(key) = event.as_key_event() else {
            return PanelEventOutcome::Ignored;
        };

        match key.code {
            KeyCode::Left => {
                self.buttons_list_state.select(Some(0));
                PanelEventOutcome::Consumed
            }
            KeyCode::Right => {
                self.buttons_list_state.select(Some(1));
                PanelEventOutcome::Consumed
            }
            KeyCode::Enter => match self.buttons_list_state.selected() {
                Some(0) => PanelEventOutcome::Action(WorkflowAction::ConfirmDelete),
                _ => PanelEventOutcome::Action(WorkflowAction::BackToPreview),
            },
            KeyCode::Esc => PanelEventOutcome::Action(WorkflowAction::BackToPreview),
            _ => PanelEventOutcome::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::Event;
    use ratatui::prelude::{Buffer, Rect};
    use std::mem::MaybeUninit;

    /// Helper that builds a minimal `WorkflowSelection` whose state is
    /// `Error`, which is the simplest valid selection for exercising the
    /// delete-mode rendering and event handling without touching the file
    /// system.
    fn selection_with_error() -> super::super::WorkflowSelection {
        super::super::WorkflowSelection {
            index: 0,
            workflow_file: None,
            is_running: false,
            state: super::super::WorkflowSelectionState::Error("boom".to_string()),
        }
    }

    /// Returns a `&mut DefaultTerminal` that is never dereferenced by the
    /// views we exercise (they ignore the `_terminal` parameter).
    fn no_read_term() -> *mut ratatui::DefaultTerminal {
        static SLOT: std::sync::OnceLock<MaybeUninit<ratatui::DefaultTerminal>> =
            std::sync::OnceLock::new();
        let slot = SLOT.get_or_init(MaybeUninit::uninit);
        slot.as_ptr().cast_mut()
    }

    #[test]
    fn test_workflow_delete_new_is_constructable() {
        let _delete = WorkflowDelete::new();
    }

    #[test]
    fn test_workflow_delete_render_no_panic() {
        let delete = WorkflowDelete::new();
        let selection = selection_with_error();
        let area = Rect::new(0, 0, 60, 8);
        let mut buf = Buffer::empty(area);
        delete.render(&selection, area, &mut buf);
    }

    #[test]
    fn test_workflow_delete_handle_enter_returns_confirm_delete() {
        let mut delete = WorkflowDelete::new();
        let selection = selection_with_error();
        let event = Event::Key(KeyCode::Enter.into());
        let outcome = unsafe { delete.handle_event(&event, &mut *no_read_term(), &selection) };
        assert_eq!(
            outcome,
            PanelEventOutcome::Action(WorkflowAction::ConfirmDelete)
        );
    }

    #[test]
    fn test_workflow_delete_handle_escape_returns_back_to_preview() {
        let mut delete = WorkflowDelete::new();
        let selection = selection_with_error();
        let event = Event::Key(KeyCode::Esc.into());
        let outcome = unsafe { delete.handle_event(&event, &mut *no_read_term(), &selection) };
        assert_eq!(
            outcome,
            PanelEventOutcome::Action(WorkflowAction::BackToPreview)
        );
    }

    #[test]
    fn test_workflow_delete_handle_unknown_key_returns_none() {
        let mut delete = WorkflowDelete::new();
        let selection = selection_with_error();
        let event = Event::Key(KeyCode::Char('x').into());
        let outcome = unsafe { delete.handle_event(&event, &mut *no_read_term(), &selection) };
        assert_eq!(outcome, PanelEventOutcome::Ignored);
    }

    #[test]
    fn test_workflow_delete_right_then_enter_cancels() {
        let mut delete = WorkflowDelete::new();
        let selection = selection_with_error();
        let right = Event::Key(KeyCode::Right.into());
        let outcome = unsafe { delete.handle_event(&right, &mut *no_read_term(), &selection) };
        assert_eq!(outcome, PanelEventOutcome::Consumed);
        let enter = Event::Key(KeyCode::Enter.into());
        let outcome = unsafe { delete.handle_event(&enter, &mut *no_read_term(), &selection) };
        assert_eq!(
            outcome,
            PanelEventOutcome::Action(WorkflowAction::BackToPreview)
        );
    }

    #[test]
    fn test_workflow_delete_handle_mouse_event_returns_none() {
        let mut delete = WorkflowDelete::new();
        let selection = selection_with_error();
        let event = Event::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        let outcome = unsafe { delete.handle_event(&event, &mut *no_read_term(), &selection) };
        assert_eq!(outcome, PanelEventOutcome::Ignored);
    }
}
