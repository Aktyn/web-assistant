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
