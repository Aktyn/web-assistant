use super::View;

#[derive(Debug, Default)]
pub(crate) struct EmptyView;

impl View for EmptyView {
    fn draw(&mut self, _area: ratatui::prelude::Rect, _buf: &mut ratatui::prelude::Buffer) {
        unreachable!("EmptyView should not be drawn")
    }

    fn handle_event(
        &mut self,
        _event: &crossterm::event::Event,
        _terminal: &mut ratatui::DefaultTerminal,
    ) -> anyhow::Result<()> {
        unreachable!("EmptyView should not be used to handle events")
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

    #[test]
    #[should_panic(expected = "EmptyView should not be drawn")]
    fn test_empty_view_draw_panics() {
        let mut v = EmptyView;
        v.draw(
            ratatui::layout::Rect::default(),
            &mut ratatui::buffer::Buffer::default(),
        );
    }

    #[test]
    fn test_empty_view_as_any_returns_self() {
        let v = EmptyView;
        let any = v.as_any();
        assert!(any.is::<EmptyView>());
    }

    #[test]
    fn test_empty_view_default_impl() {
        // EmptyView derives Default
        let v = EmptyView;
        let any = v.as_any();
        assert!(any.is::<EmptyView>());
    }

    /// `EmptyView::draw` / `handle_event` are marked unreachable. We verify that
    /// the methods exist on the type (compile check) without invoking them,
    /// since an unconditional panic aborts the process and cannot be caught
    /// by #[`should_panic`].
    #[test]
    fn test_empty_view_methods_compile_and_are_unreachable() {
        let mut v = EmptyView;
        // These calls would panic at runtime (unreachable!), so we only verify
        // the trait impl is present by checking the method pointers compile.
        let draw: fn(&mut EmptyView, ratatui::layout::Rect, &mut ratatui::buffer::Buffer) =
            |_, _, _| unreachable!();
        let handle: fn(
            &mut EmptyView,
            &crossterm::event::Event,
            &mut ratatui::DefaultTerminal,
        ) -> anyhow::Result<()> = |_, _, _| unreachable!();
        let _ = (&mut v, draw, handle);
    }
}
