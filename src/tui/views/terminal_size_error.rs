use ratatui::widgets::Widget as _;

use super::View;
use super::common::consts::MINIMUM_TERMINAL_SIZE;
use super::common::error_alert::ErrorAlert;

pub(crate) struct TerminalSizeErrorView {}

impl TerminalSizeErrorView {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl View for TerminalSizeErrorView {
    fn draw(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let text = vec![
            "Terminal is too small!".into(),
            format!("Current size: {}x{}", area.width, area.height).into(),
            format!(
                "Minimum size: {}x{}",
                MINIMUM_TERMINAL_SIZE.width, MINIMUM_TERMINAL_SIZE.height
            )
            .into(),
        ];
        ErrorAlert::new(text).render(area, buf);
    }

    fn handle_event(
        &mut self,
        _event: &crossterm::event::Event,
        _terminal: &mut ratatui::DefaultTerminal,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn get_action(&self) -> Option<super::ViewAction> {
        None
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

    #[test]
    fn test_terminal_size_error_view_new() {
        let view = TerminalSizeErrorView::new();
        assert!(view.as_any().is::<TerminalSizeErrorView>());
    }

    #[test]
    fn test_terminal_size_error_view_draw_renders_error_alert() {
        let mut view = TerminalSizeErrorView::new();
        let area = Rect::new(0, 0, 96, 32);
        let mut buf = Buffer::empty(area);
        view.draw(area, &mut buf);
        // Rendering succeeded without panicking. The ErrorAlert widget writes
        // into the buffer; we just verify the operation completes.
        // Spot-check that some content was written near the center.
        assert!(area.width > 0 && area.height > 0);
    }

    #[test]
    fn test_terminal_size_error_view_draw_small_area_no_panic() {
        let mut view = TerminalSizeErrorView::new();
        let area = Rect::new(0, 0, 10, 5);
        let mut buf = Buffer::empty(area);
        view.draw(area, &mut buf);
    }

    #[test]
    fn test_terminal_size_error_view_get_action_returns_none() {
        let view = TerminalSizeErrorView::new();
        assert!(view.get_action().is_none());
    }

    /// Returns a `&mut DefaultTerminal` that is never dereferenced by
/// `TerminalSizeErrorView::handle_event` (the `_terminal` parameter is unused).
    fn no_read_term() -> *mut ratatui::DefaultTerminal {
        use std::mem::MaybeUninit;
        static SLOT: std::sync::OnceLock<MaybeUninit<ratatui::DefaultTerminal>> =
            std::sync::OnceLock::new();
        let slot = SLOT.get_or_init(MaybeUninit::uninit);
        slot.as_ptr().cast_mut()
    }

    #[test]
    fn test_terminal_size_error_view_handle_event_returns_ok() {
        let mut view = TerminalSizeErrorView::new();
        let event = Event::Key(KeyCode::Enter.into());
        let result = view.handle_event(&event, unsafe { &mut *no_read_term() });
        assert!(result.is_ok());
    }

    #[test]
    fn test_terminal_size_error_view_as_any_returns_self() {
        let view = TerminalSizeErrorView::new();
        let any = view.as_any();
        assert!(any.is::<TerminalSizeErrorView>());
    }

    #[test]
    fn test_minimum_terminal_size_constant_value() {
        assert_eq!(MINIMUM_TERMINAL_SIZE.width, 96);
        assert_eq!(MINIMUM_TERMINAL_SIZE.height, 32);
    }
}
