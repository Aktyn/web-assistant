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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_separator_new_horizontal() {
        let sep = Separator::new(Orientation::Horizontal);
        assert!(matches!(sep.orientation, Orientation::Horizontal));
    }

    #[test]
    fn test_separator_new_vertical() {
        let sep = Separator::new(Orientation::Vertical);
        assert!(matches!(sep.orientation, Orientation::Vertical));
    }

    #[test]
    fn test_separator_constants() {
        assert!(matches!(Separator::HORIZONTAL, Orientation::Horizontal));
        assert!(matches!(Separator::VERTICAL, Orientation::Vertical));
    }

    #[test]
    fn test_separator_render_horizontal_no_panic() {
        let sep = Separator::new(Orientation::Horizontal);
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        sep.render(area, &mut buf);
    }

    #[test]
    fn test_separator_render_vertical_no_panic() {
        let sep = Separator::new(Orientation::Vertical);
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        sep.render(area, &mut buf);
    }

    #[test]
    fn test_separator_render_horizontal_writes_center_row() {
        // A 3-row area centered vertically places the separator at y=1.
        let sep = Separator::new(Orientation::Horizontal);
        let area = Rect::new(0, 0, 5, 3);
        let mut buf = Buffer::empty(area);
        sep.render(area, &mut buf);
        // The cell at (0, 1) should have the horizontal separator glyph.
        assert_eq!(buf[(0, 1)].symbol(), "─");
    }

    #[test]
    fn test_separator_render_vertical_writes_center_column() {
        // A 5-col area centered horizontally places the separator at x=2.
        let sep = Separator::new(Orientation::Vertical);
        let area = Rect::new(0, 0, 5, 3);
        let mut buf = Buffer::empty(area);
        sep.render(area, &mut buf);
        assert_eq!(buf[(2, 0)].symbol(), "│");
    }

    #[test]
    fn test_separator_render_horizontal_zero_width_no_panic() {
        let sep = Separator::new(Orientation::Horizontal);
        let area = Rect::new(0, 0, 0, 3);
        let mut buf = Buffer::empty(area);
        sep.render(area, &mut buf);
    }

    #[test]
    fn test_separator_render_vertical_zero_height_no_panic() {
        let sep = Separator::new(Orientation::Vertical);
        let area = Rect::new(0, 0, 5, 0);
        let mut buf = Buffer::empty(area);
        sep.render(area, &mut buf);
    }

    #[test]
    fn test_separator_is_clone() {
        let sep = Separator::new(Orientation::Horizontal);
        let _clone = sep.clone();
    }

    #[test]
    fn test_separator_debug_format() {
        let sep = Separator::new(Orientation::Vertical);
        assert!(format!("{sep:?}").contains("Vertical"));
    }
}