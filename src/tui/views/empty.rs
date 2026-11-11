use super::View;

#[derive(Debug, Default)]
pub(crate) struct EmptyView;

impl View for EmptyView {
    fn draw(&mut self, _area: ratatui::prelude::Rect, _buf: &mut ratatui::prelude::Buffer) {
        unreachable!("EmptyView should not be drawn")
    }

    fn handle_event(
        &mut self,
        _terminal: &mut ratatui::DefaultTerminal,
        _event: &crossterm::event::Event,
    ) -> anyhow::Result<()> {
        unreachable!("EmptyView should not be used to handle events")
    }
}
