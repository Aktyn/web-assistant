use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Stylize as _;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget};

use super::palette;

#[derive(Debug, Clone)]
pub(crate) struct ErrorAlert<'a> {
    text: Text<'a>,
}

impl<'a> ErrorAlert<'a> {
    pub fn new<T: Into<Text<'a>>>(text: T) -> Self {
        ErrorAlert { text: text.into() }
    }
}

impl Widget for ErrorAlert<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let centered_area = area.centered(Constraint::Length(60), Constraint::Max(10));

        Paragraph::new(self.text)
            .bg(palette::BACKGROUND)
            .fg(palette::ERROR_FOREGROUND)
            .bold()
            .centered()
            .block(
                Block::default()
                    .title(Line::from("Error").centered())
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(palette::ERROR_FOREGROUND),
            )
            .render(centered_area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    #[test]
    fn test_error_alert_creation() {
        let label = "Test error message";
        let alert = ErrorAlert::new(label);
        assert_eq!(alert.text, Text::from("Test error message"));
    }

    #[test]
    fn test_error_alert_creation_from_text() {
        let text = Text::from(vec![
            ratatui::text::Line::from("line 1"),
            ratatui::text::Line::from("line 2"),
        ]);
        let alert = ErrorAlert::new(text.clone());
        assert_eq!(alert.text, text);
    }

    #[test]
    fn test_error_alert_render_no_panic() {
        let alert = ErrorAlert::new("Something went wrong");
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        alert.render(area, &mut buf);
        // Rendering succeeded without panicking.
    }

    #[test]
    fn test_error_alert_render_small_area_no_panic() {
        let alert = ErrorAlert::new("Error");
        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        alert.render(area, &mut buf);
    }

    #[test]
    fn test_error_alert_render_large_area_no_panic() {
        let alert = ErrorAlert::new("A very long error message that explains what happened in detail");
        let area = Rect::new(0, 0, 200, 60);
        let mut buf = Buffer::empty(area);
        alert.render(area, &mut buf);
    }

    #[test]
    fn test_error_alert_render_empty_text() {
        let alert = ErrorAlert::new("");
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        alert.render(area, &mut buf);
    }

    #[test]
    fn test_error_alert_multiline_text() {
        let text = Text::from(vec![
            ratatui::text::Line::from("First line"),
            ratatui::text::Line::from("Second line"),
            ratatui::text::Line::from("Third line"),
        ]);
        let alert = ErrorAlert::new(text);
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        alert.render(area, &mut buf);
    }
}
