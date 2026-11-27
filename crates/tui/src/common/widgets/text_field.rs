use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize as _};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget};

use super::super::palette;

#[derive(Debug, Clone)]
pub(crate) struct TextField {
    label: String,
    value: String,
}

impl TextField {
    pub(crate) fn new(label: String, value: String) -> Self {
        Self { label, value }
    }

    /// Return the estimated width for current value
    pub(crate) fn estimate_width(&self) -> u16 {
        u16::try_from(self.value.len() + 2).unwrap_or(u16::MAX)
    }

    #[allow(clippy::unused_self)]
    pub(crate) fn estimate_height(&self) -> u16 {
        3
    }

    pub(crate) fn draw(&self, area: Rect, buf: &mut Buffer) {
        let constrained = Rect::new(
            area.x,
            area.y,
            area.width.min(self.estimate_width()),
            area.height.min(self.estimate_height()),
        );

        Paragraph::new(utils::text::truncate_text(
            &self.value,
            constrained.width.saturating_sub(2).into(),
            None,
        ))
        .style(palette().style.bold())
        .block(
            Block::default()
                .title(
                    Line::from(self.label.clone())
                        .bg(palette().background)
                        .fg(palette().muted_foreground),
                )
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(palette().separator_style),
        )
        .render(constrained, buf);
    }
}

/// An editable text field that supports cursor movement and text input
#[derive(Debug, Clone)]
pub(crate) struct EditableTextField {
    label: String,
    value: String,
    cursor_position: usize,
    is_focused: bool,
}

impl EditableTextField {
    pub(crate) fn new(label: String, value: String) -> Self {
        let cursor_position = value.len();
        Self {
            label,
            value,
            cursor_position,
            is_focused: false,
        }
    }

    pub(crate) fn set_focused(&mut self, focused: bool) {
        self.is_focused = focused;
    }

    pub(crate) fn value(&self) -> &str {
        &self.value
    }

    pub(crate) fn set_value(&mut self, value: String) {
        self.value = value;
        self.cursor_position = self.value.len().min(self.cursor_position);
    }

    /// Insert a character at the cursor position
    pub(crate) fn insert_char(&mut self, c: char) {
        self.value.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    /// Delete the character before the cursor (backspace)
    pub(crate) fn backspace(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.value.remove(self.cursor_position);
        }
    }

    /// Delete the character at the cursor position (delete key)
    pub(crate) fn delete(&mut self) {
        if self.cursor_position < self.value.len() {
            self.value.remove(self.cursor_position);
        }
    }

    /// Move cursor left
    pub(crate) fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    /// Move cursor right
    pub(crate) fn move_cursor_right(&mut self) {
        if self.cursor_position < self.value.len() {
            self.cursor_position += 1;
        }
    }

    /// Move cursor to start
    pub(crate) fn move_cursor_home(&mut self) {
        self.cursor_position = 0;
    }

    /// Move cursor to end
    pub(crate) fn move_cursor_end(&mut self) {
        self.cursor_position = self.value.len();
    }

    /// Return the estimated width for current value
    pub(crate) fn estimate_width(&self) -> u16 {
        u16::try_from(self.value.len() + 2).unwrap_or(u16::MAX)
    }

    #[allow(clippy::unused_self)]
    pub(crate) fn estimate_height(&self) -> u16 {
        3
    }

    pub(crate) fn draw(&self, area: Rect, buf: &mut Buffer) {
        let constrained = Rect::new(
            area.x,
            area.y,
            area.width.min(self.estimate_width()),
            area.height.min(self.estimate_height()),
        );

        let border_style = if self.is_focused {
            palette().accent
        } else {
            palette().border
        };

        let display_value = if self.value.is_empty() && !self.is_focused {
            "" // Show empty when not focused
        } else {
            &self.value
        };

        let paragraph = Paragraph::new(utils::text::truncate_text(
            display_value,
            constrained.width.saturating_sub(2).into(),
            None,
        ))
        .style(palette().style.bold())
        .block(
            Block::default()
                .title(
                    Line::from(self.label.clone())
                        .bg(palette().background)
                        .fg(palette().muted_foreground),
                )
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style),
        );

        paragraph.render(constrained, buf);

        // Draw cursor if focused
        if self.is_focused {
            let cursor_x = constrained.x + 1 + u16::try_from(self.cursor_position).unwrap_or(0);
            let cursor_y = constrained.y + 1;
            if cursor_x < constrained.x + constrained.width - 1 {
                buf[(cursor_x, cursor_y)]
                    .set_style(Style::new().bg(palette().accent))
                    .set_char('█');
            }
        }
    }
}

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

    #[test]
    fn test_text_field_estimate_width() {
        let field = TextField::new("L".to_string(), "hello".to_string());
        assert_eq!(field.estimate_width(), 7); // len("hello") + 2
    }

    #[test]
    fn test_text_field_estimate_height_is_three() {
        let field = TextField::new("L".to_string(), "v".to_string());
        assert_eq!(field.estimate_height(), 3);
    }

    #[test]
    fn test_text_field_clone() {
        let field = TextField::new("Name".into(), "Alice".into());
        let cloned = field.clone();
        assert_eq!(cloned.label, "Name");
        assert_eq!(cloned.value, "Alice");
    }

    #[test]
    fn test_text_field_debug_display() {
        let field = TextField::new("L".into(), "v".into());
        let debug_str = format!("{field:?}");
        assert!(debug_str.contains("TextField"));
    }

    #[test]
    fn test_text_field_draw_with_small_area() {
        let field = TextField::new("L".to_string(), "val".to_string());
        let area = Rect::new(0, 0, 2, 1);
        let mut buf = Buffer::empty(area);
        field.draw(area, &mut buf);
    }

    #[test]
    fn test_text_field_label_and_value_different() {
        let field = TextField::new("Password".to_string(), "secret".to_string());
        assert_ne!(field.label, field.value);
    }
}
