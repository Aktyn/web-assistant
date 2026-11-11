pub(super) mod workflows;
pub(super) mod empty;

use crossterm::event::Event;
use ratatui::{DefaultTerminal, buffer::Buffer, layout::Rect};

use super::common;

pub(super) trait View {
    fn draw(&mut self, area: Rect, buf: &mut Buffer);
    fn handle_event(&mut self, terminal: &mut DefaultTerminal, event: &Event)
    -> anyhow::Result<()>;
}
