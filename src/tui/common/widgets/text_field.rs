use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize as _;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget};
use web_assistant::utils;

use super::super::super::palette;

#[derive(Debug, Clone)]
pub(crate) struct TextField {
    label: String,
    value: String,
}

impl TextField {
    pub fn new(label: String, value: String) -> Self {
        Self { label, value }
    }

    pub(crate) fn draw(&self, area: Rect, buf: &mut Buffer) {
        let value = self.value.clone();

        let constrained = Rect::new(
            area.x,
            area.y,
            area.width
                .min(u16::try_from(value.len() + 2).unwrap_or(u16::MAX)),
            area.height.min(3),
        );

        Paragraph::new(utils::truncate_text(
            &value,
            constrained.width.saturating_sub(2).into(),
            None,
        ))
        .style(palette::STYLE)
        .bold()
        .block(
            Block::default()
                .title(
                    Line::from(self.label.clone())
                        .bg(palette::BACKGROUND)
                        .fg(palette::MUTED_FOREGROUND),
                )
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(palette::SEPARATOR_STYLE),
        )
        .render(constrained, buf);
    }
}

// impl Widget for TextField {
//     fn render(self, area: Rect, buf: &mut Buffer) {
//         // let color = self.color();
//         // Paragraph::new(self.label)
//         //     .bg(palette::BACKGROUND)
//         //     .fg(color)
//         //     .bold()
//         //     .centered()
//         //     .block(
//         //         Block::default()
//         //             .borders(Borders::ALL)
//         //             .border_type(BorderType::Rounded)
//         //             .border_style(palette::color())
//         //     )
//         //     .render(area, buf);
//     }
// }

// impl TextField {
// pub(crate) const HEIGHT: u16 = 3;

// const fn color(&self) -> Color {
//     match self.state {
//         State::Normal => palette::FOREGROUND,
//         State::Selected => palette::ACCENT,
//     }
// }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_field_new() {
        let field = TextField::new("Name".to_string(), "Alice".to_string());
        assert_eq!(field.label, "Name");
        assert_eq!(field.value, "Alice");
    }

    #[test]
    fn test_text_field_draw_no_panic() {
        let field = TextField::new("Name".to_string(), "Alice".to_string());
        let area = Rect::new(0, 0, 50, 3);
        let mut buf = Buffer::empty(area);
        field.draw(area, &mut buf);
    }

    #[test]
    fn test_text_field_draw_with_long_value_truncates() {
        let value = "x".repeat(100);
        let field = TextField::new("Label".to_string(), value);
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        field.draw(area, &mut buf);
    }

    #[test]
    fn test_text_field_draw_with_empty_value() {
        let field = TextField::new("Label".to_string(), String::new());
        let area = Rect::new(0, 0, 50, 3);
        let mut buf = Buffer::empty(area);
        field.draw(area, &mut buf);
    }

    #[test]
    fn test_text_field_draw_constrains_height() {
        // Area height > 3 should be clamped to 3.
        let field = TextField::new("Label".to_string(), "value".to_string());
        let area = Rect::new(0, 0, 50, 10);
        let mut buf = Buffer::empty(area);
        field.draw(area, &mut buf);
    }

    #[test]
    fn test_text_field_draw_constrains_width() {
        // Area width < value length should be clamped.
        let field = TextField::new("Label".to_string(), "a very long value".to_string());
        let area = Rect::new(0, 0, 5, 3);
        let mut buf = Buffer::empty(area);
        field.draw(area, &mut buf);
    }
}
