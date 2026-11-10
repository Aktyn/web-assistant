use crossterm::event::Event;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::Rect,
    style::Stylize as _,
    widgets::{Paragraph, Widget},
};

use super::palette;

pub(crate) trait Panel {
    fn draw(&mut self, area: Rect, buf: &mut Buffer, focused: bool);
    fn handle_event(&mut self, terminal: &mut DefaultTerminal, event: &Event)
    -> anyhow::Result<()>;
}

pub(crate) struct Header {
    content: String,
    focused: bool,
}

impl Widget for Header {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let content = if self.focused {
            format!("[ {} ]", self.content)
        } else {
            self.content
        };

        Paragraph::new(content)
            .centered()
            .bold()
            .bg(palette::BACKGROUND)
            .fg(if self.focused {
                palette::ACCENT
            } else {
                palette::FOREGROUND
            })
            .render(area, buf);
    }
}

impl Header {
    pub(crate) fn new(content: impl Into<String>, focused: bool) -> Self {
        Self {
            content: content.into(),
            focused,
        }
    }
}
