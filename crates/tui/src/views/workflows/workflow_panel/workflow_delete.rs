use crossterm::event::KeyCode;
use ratatui::widgets::Widget as _;

use super::super::widgets::ErrorAlert;
use super::WorkflowPanelModeView;

#[derive(Default)]
pub(crate) struct WorkflowDelete {
    //
}

impl WorkflowDelete {
    pub fn new() -> Self {
        Self {}
    }
}

impl WorkflowPanelModeView for WorkflowDelete {
    fn render(
        &self,
        _selection: &super::WorkflowSelection,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        ErrorAlert::new("Are you sure you want to permanently delete this workflow file?\n\nEnter -> confirm\nEscape -> cancel")
            .render(area, buf);
    }

    fn handle_event(
        &mut self,
        event: &crossterm::event::Event,
        _terminal: &mut ratatui::DefaultTerminal,
        _selection: &super::WorkflowSelection,
    ) -> Option<super::WorkflowAction> {
        if let Some(key) = event.as_key_event() {
            return match key.code {
                KeyCode::Enter => Some(super::WorkflowAction::ConfirmDelete),
                KeyCode::Esc => Some(super::WorkflowAction::BackToPreview),
                _ => None,
            };
        }

        None
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
        let action = unsafe { delete.handle_event(&event, &mut *no_read_term(), &selection) };
        assert_eq!(action, Some(super::super::WorkflowAction::ConfirmDelete));
    }

    #[test]
    fn test_workflow_delete_handle_escape_returns_back_to_preview() {
        let mut delete = WorkflowDelete::new();
        let selection = selection_with_error();
        let event = Event::Key(KeyCode::Esc.into());
        let action = unsafe { delete.handle_event(&event, &mut *no_read_term(), &selection) };
        assert_eq!(action, Some(super::super::WorkflowAction::BackToPreview));
    }

    #[test]
    fn test_workflow_delete_handle_unknown_key_returns_none() {
        let mut delete = WorkflowDelete::new();
        let selection = selection_with_error();
        let event = Event::Key(KeyCode::Char('x').into());
        let action = unsafe { delete.handle_event(&event, &mut *no_read_term(), &selection) };
        assert!(action.is_none());
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
        let action = unsafe { delete.handle_event(&event, &mut *no_read_term(), &selection) };
        assert!(action.is_none());
    }
}
