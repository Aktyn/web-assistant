use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Stylize as _};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget};

use super::palette;

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
        Button {
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
            .bg(palette::BACKGROUND)
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

    const fn color(&self) -> Color {
        match self.state {
            State::Normal => palette::FOREGROUND,
            State::Selected => palette::ACCENT,
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
