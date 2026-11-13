use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize as _;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget};
use web_assistant::utils;

use super::super::palette;

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
//         //             .border_style(color),
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
