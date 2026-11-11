use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Stylize as _;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget};

use super::palette;

#[derive(Debug, Clone)]
pub(crate) struct ErrorAlert<'a> {
    label: Line<'a>,
}

impl<'a> ErrorAlert<'a> {
    pub fn new<T: Into<Line<'a>>>(label: T) -> Self {
        ErrorAlert {
            label: label.into(),
        }
    }
}

impl Widget for ErrorAlert<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(format!("Error!\n{}", self.label))
            .bg(palette::BACKGROUND)
            .fg(palette::ERROR_FOREGROUND)
            .bold()
            .centered()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(palette::ERROR_FOREGROUND),
            )
            .render(
                area.centered(Constraint::Length(60), Constraint::Max(10)),
                buf,
            );
    }
}
