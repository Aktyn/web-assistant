use ratatui::style::Color;
use ratatui::style::Style;

// pub(crate) const BACKGROUND: Color = Color::Rgb(8, 20, 26);
pub(crate) const BACKGROUND: Color = Color::Reset;
pub(crate) const BACKGROUND_STYLE: Style = Style::new().bg(BACKGROUND);

pub(crate) const FOREGROUND: Color = Color::Rgb(236, 239, 241);
// pub(crate) const FOREGROUND_STYLE: Style = Style::new().fg(FOREGROUND);

pub(crate) const ACCENT: Color = Color::Rgb(77, 182, 172); // Teal 300

const BORDER: Color = Color::Rgb(38, 50, 56);

pub(crate) const SEPARATOR_STYLE: Style = Style::new().fg(BORDER).bg(BACKGROUND);

pub(crate) const ERROR_FOREGROUND: Color = Color::Rgb(229, 115, 115); // Red 300

/// Complete reusable style
pub(crate) const STYLE: Style = Style::new().fg(FOREGROUND).bg(BACKGROUND);
