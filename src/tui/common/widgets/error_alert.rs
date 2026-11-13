use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Stylize as _;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget};

use super::super::palette;

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

    #[test]
    fn test_error_alert_creation() {
        let label = "Test error message";
        let alert = ErrorAlert::new(label);
        assert_eq!(alert.text, Text::from("Test error message"));
    }
}
