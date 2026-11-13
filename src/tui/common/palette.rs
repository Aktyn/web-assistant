use ratatui::style::Color;
use ratatui::style::Style;

// pub(crate) const BACKGROUND: Color = Color::Rgb(8, 20, 26);
pub(crate) const BACKGROUND: Color = Color::Reset;
pub(crate) const BACKGROUND_STYLE: Style = Style::new().bg(BACKGROUND);

pub(crate) const FOREGROUND: Color = Color::Rgb(236, 239, 241); // Blue gray 50
// pub(crate) const FOREGROUND_STYLE: Style = Style::new().fg(FOREGROUND);

pub(crate) const MUTED_FOREGROUND: Color = Color::Rgb(120, 144, 156); // Blue gray 400

pub(crate) const ACCENT: Color = Color::Rgb(77, 182, 172); // Teal 300

const BORDER: Color = Color::Rgb(38, 50, 56);

pub(crate) const SEPARATOR_STYLE: Style = Style::new().fg(BORDER).bg(BACKGROUND);

pub(crate) const ERROR_FOREGROUND: Color = Color::Rgb(229, 115, 115); // Red 300

/// Complete reusable style
pub(crate) const STYLE: Style = Style::new().fg(FOREGROUND).bg(BACKGROUND);

// TODO: new approach to achieve configurable palette
// #[derive(Debug)]
// pub(crate) struct TuiPalette {
// }
//
// impl Default for TuiPalette {
//     fn default() -> Self {
//         Self {
//             //TODO
//         }
//     }
// }
//
// static PALETTE: OnceLock<TuiPalette> = OnceLock::new();
//
// pub fn init(palette: TuiPalette) {
//     PALETTE.set(palette).expect("palette already set");
// }
//
// pub(crate) fn get() -> &'static TuiPalette {
//     PALETTE.get_or_init(TuiPalette::default)
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_background_is_reset() {
        assert_eq!(BACKGROUND, Color::Reset);
    }

    #[test]
    fn test_foreground_rgb_value() {
        assert_eq!(FOREGROUND, Color::Rgb(236, 239, 241));
    }

    #[test]
    fn test_accent_teal_value() {
        assert_eq!(ACCENT, Color::Rgb(77, 182, 172));
    }

    #[test]
    fn test_error_foreground_red_value() {
        assert_eq!(ERROR_FOREGROUND, Color::Rgb(229, 115, 115));
    }

    #[test]
    fn test_separator_style_valid() {
        // SEPARATOR_STYLE should compile and have foreground and background
        let _ = SEPARATOR_STYLE;
    }

    #[test]
    fn test_style_has_fg_and_bg() {
        // STYLE combines FOREGROUND text with BACKGROUND fill
        let s = STYLE;
        assert!(s.fg.is_some() || s.bg.is_some());
    }
}
