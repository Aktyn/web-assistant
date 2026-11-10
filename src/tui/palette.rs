use ratatui::style::Color;
use ratatui::style::Style;

// pub const BACKGROUND: Color = Color::Rgb(8, 20, 26);
pub const BACKGROUND: Color = Color::Reset;
pub const BACKGROUND_STYLE: Style = Style::new().bg(BACKGROUND);

pub const FOREGROUND: Color = Color::Rgb(236, 239, 241);
pub const FOREGROUND_STYLE: Style = Style::new().fg(FOREGROUND);

const BORDER: Color = Color::Rgb(38, 50, 56);

pub const SEPARATOR_STYLE: Style = Style::new().fg(BORDER).bg(BACKGROUND);

/// Complete reusable style
pub const STYLE: Style = Style::new().fg(FOREGROUND).bg(BACKGROUND);
