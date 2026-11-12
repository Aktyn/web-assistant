use crossterm::event::Event;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::Rect,
    style::Stylize as _,
    widgets::{Paragraph, Widget},
};
use std::any::Any;

use super::palette;

pub(crate) trait Panel {
    fn draw(&mut self, area: Rect, buf: &mut Buffer, focused: bool);
    fn handle_event(&mut self, event: &Event, terminal: &mut DefaultTerminal)
    -> anyhow::Result<()>;

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
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

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    use super::*;

    #[test]
    fn test_header_new_focused() {
        let header = Header::new("Title", true);
        assert!(header.focused);
        assert_eq!(header.content, "Title");
    }

    #[test]
    fn test_header_new_unfocused() {
        let header = Header::new("Subtitle", false);
        assert!(!header.focused);
    }

    #[test]
    fn test_header_content_into_string() {
        // &str implements Into<String>
        let header = Header::new("works", true);
        assert_eq!(header.content, "works");
    }

    #[test]
    fn test_header_string_type() {
        let s = String::from("dynamic");
        let header = Header::new(s, true);
        assert_eq!(header.content, "dynamic");
    }

    /// Verify render does not panic (integration-style widget test).
    #[test]
    fn test_header_render_focused_no_panic() {
        let area = Rect::new(0, 0, 30, 2);
        let mut buf = Buffer::empty(area);
        Header::new("Focused", true).render(area, &mut buf);
    }

    #[test]
    fn test_header_render_unfocused_no_panic() {
        let area = Rect::new(0, 0, 30, 2);
        let mut buf = Buffer::empty(area);
        Header::new("Unfocused", false).render(area, &mut buf);
    }
}
