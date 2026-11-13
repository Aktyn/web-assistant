use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::Widget;

use super::super::palette;

#[derive(Debug, Clone)]
pub(crate) enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone)]
pub(crate) struct Separator {
    orientation: Orientation,
}

impl Separator {
    pub(crate) fn new(orientation: Orientation) -> Self {
        Separator { orientation }
    }
}

impl Separator {
    pub const HORIZONTAL: Orientation = Orientation::Horizontal;
    pub const VERTICAL: Orientation = Orientation::Vertical;
}

impl Widget for Separator {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.orientation {
            Orientation::Vertical => {
                let centered_area = area.centered_horizontally(Constraint::Length(1));
                for i in 0..area.height {
                    buf.set_string(
                        centered_area.x,
                        centered_area.y + i,
                        "│",
                        palette::SEPARATOR_STYLE,
                    );
                }
            }
            Orientation::Horizontal => {
                let centered_area = area.centered_vertically(Constraint::Length(1));
                for i in 0..area.width {
                    buf.set_string(
                        centered_area.x + i,
                        centered_area.y,
                        "─",
                        palette::SEPARATOR_STYLE,
                    );
                }
            }
        }
    }
}
