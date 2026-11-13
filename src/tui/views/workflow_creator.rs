use crossterm::event::KeyCode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Paragraph, Widget as _},
};

use super::common::palette;
use super::{View, ViewAction};
// use crate::core::Core;

pub(crate) struct WorkflowCreatorView {
    // core: Arc<RwLock<Core>>,
    pending_action: Option<ViewAction>,
}

impl WorkflowCreatorView {
    pub(crate) fn new() -> Self {
        Self {
            // core: Arc::new(RwLock::new(Core::default())),
            pending_action: None,
        }
    }
}

impl View for WorkflowCreatorView {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Press backspace to exit creator")
            .style(palette::STYLE)
            .centered()
            .render(area, buf);

        //TODO: develop this view
    }

    fn handle_event(
        &mut self,
        event: &crossterm::event::Event,
        _terminal: &mut ratatui::DefaultTerminal,
    ) -> anyhow::Result<()> {
        if let Some(key) = event.as_key_event() {
            // match key.code {
            //     KeyCode::Backspace => {
            //         self.pending_action = Some(ViewAction::ExitWorkflowCreator);
            //     }
            //     _ => {}
            // }
            if key.code == KeyCode::Backspace {
                self.pending_action = Some(ViewAction::ExitWorkflowCreator);
            }
        }

        Ok(())
    }

    fn get_action(&self) -> Option<ViewAction> {
        self.pending_action
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{Event, KeyCode};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    /// Returns a `&mut DefaultTerminal` that is never dereferenced by
    /// `WorkflowCreatorView::handle_event` (the `_terminal` parameter is unused).
    fn no_read_term() -> *mut ratatui::DefaultTerminal {
        use std::mem::MaybeUninit;
        static SLOT: std::sync::OnceLock<MaybeUninit<ratatui::DefaultTerminal>> =
            std::sync::OnceLock::new();
        let slot = SLOT.get_or_init(MaybeUninit::uninit);
        slot.as_ptr().cast_mut()
    }

    #[test]
    fn test_workflow_creator_view_new_has_no_pending_action() {
        let view = WorkflowCreatorView::new();
        assert!(view.get_action().is_none());
    }

    #[test]
    fn test_workflow_creator_view_as_any_returns_self() {
        let view = WorkflowCreatorView::new();
        assert!(view.as_any().is::<WorkflowCreatorView>());
    }

    #[test]
    fn test_workflow_creator_view_draw_no_panic() {
        let mut view = WorkflowCreatorView::new();
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        view.draw(area, &mut buf);
        // Rendering succeeded without panicking.
    }

    #[test]
    fn test_workflow_creator_view_draw_small_area_no_panic() {
        let mut view = WorkflowCreatorView::new();
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        view.draw(area, &mut buf);
    }

    #[test]
    fn test_workflow_creator_view_backspace_sets_exit_action() {
        let mut view = WorkflowCreatorView::new();
        let event = Event::Key(KeyCode::Backspace.into());
        view.handle_event(&event, unsafe { &mut *no_read_term() })
            .unwrap();
        assert_eq!(view.get_action(), Some(ViewAction::ExitWorkflowCreator));
    }

    #[test]
    fn test_workflow_creator_view_non_backspace_key_ignored() {
        let mut view = WorkflowCreatorView::new();
        let event = Event::Key(KeyCode::Char('a').into());
        view.handle_event(&event, unsafe { &mut *no_read_term() })
            .unwrap();
        assert!(view.get_action().is_none());
    }

    #[test]
    fn test_workflow_creator_view_non_key_event_ignored() {
        let mut view = WorkflowCreatorView::new();
        let event = Event::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        view.handle_event(&event, unsafe { &mut *no_read_term() })
            .unwrap();
        assert!(view.get_action().is_none());
    }
}
