pub(super) mod empty;
pub(super) mod no_workflows;
pub(super) mod terminal_size_error;
pub(super) mod workflows;

use crossterm::event::Event;
use ratatui::{DefaultTerminal, buffer::Buffer, layout::Rect};
use std::any::Any;

use super::common;

pub(super) trait View {
    fn draw(&mut self, area: Rect, buf: &mut Buffer);
    fn handle_event(&mut self, event: &Event, terminal: &mut DefaultTerminal)
    -> anyhow::Result<()>;

    fn as_any(&self) -> &dyn Any;
}
