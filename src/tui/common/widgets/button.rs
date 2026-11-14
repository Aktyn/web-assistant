use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Stylize as _};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget};

use super::super::super::palette;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Normal,
    Selected,
}

#[derive(Debug, Clone)]
pub(crate) struct Button<'a> {
    label: Line<'a>,
    state: State,
}

impl<'a> Button<'a> {
    pub fn new<T: Into<Line<'a>>>(label: T) -> Self {
        Self {
            label: label.into(),
            state: State::Normal,
        }
    }

    pub const fn set_selected(mut self, selected: bool) -> Self {
        self.state = if selected {
            State::Selected
        } else {
            State::Normal
        };
        self
    }
}

impl Widget for Button<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let color = self.color();
        Paragraph::new(self.label)
            .bg(palette().background)
            .fg(color)
            .bold()
            .centered()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(color),
            )
            .render(area, buf);
    }
}

impl Button<'_> {
    pub(crate) const HEIGHT: u16 = 3;

    fn color(&self) -> Color {
        match self.state {
            State::Normal => palette().foreground,
            State::Selected => palette().accent,
        }
    }
}

pub(crate) struct ButtonData<T> {
    pub label: String,
    pub action: T,
}
impl<T> ButtonData<T> {
    pub(crate) fn new(label: &str, action: T) -> Self {
        Self {
            label: label.to_string(),
            action,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_height_constant() {
        assert_eq!(Button::<>::HEIGHT, 3);
    }

    #[test]
    fn test_button_new_normal_state() {
        let btn = Button::new("Click me");
        // State should be Normal after construction
        assert!(!matches!(btn.state, State::Selected));
    }

    #[test]
    fn test_button_set_selected_true() {
        let btn = Button::new("Click me").set_selected(true);
        assert!(matches!(btn.state, State::Selected));
    }

    #[test]
    fn test_button_set_selected_false() {
        let btn = Button::new("Click me")
            .set_selected(true)
            .set_selected(false);
        assert!(matches!(btn.state, State::Normal));
    }

    #[test]
    fn test_button_color_normal() {
        let btn = Button::new("Normal").set_selected(false);
        assert_eq!(btn.color(), palette().FOREGROUND);
    }

    #[test]
    fn test_button_color_selected() {
        let btn = Button::new("Selected").set_selected(true);
        assert_eq!(btn.color(), palette().ACCENT);
    }

    #[test]
    fn test_button_data_construction() {
        let data: ButtonData<i32> = ButtonData::new("label", 42);
        assert_eq!(data.label, "label");
        assert_eq!(data.action, 42);
    }

    #[test]
    fn test_button_data_string_action() {
        let data: ButtonData<String> = ButtonData::new("go", "action_value".to_string());
        assert_eq!(data.label, "go");
        assert_eq!(data.action, "action_value");
    }

    /// Verify render does not panic with a minimal buffer (unit test for widget impl).
    #[test]
    fn test_button_render_no_panic() {
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        Button::new("Test")
            .set_selected(false)
            .render(area, &mut buf);
        // If we reach here without panicking, the widget rendered successfully.
    }

    #[test]
    fn test_button_render_selected() {
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        Button::new("Selected")
            .set_selected(true)
            .render(area, &mut buf);
    }
}
