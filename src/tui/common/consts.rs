use ratatui::layout::Size;

/// Defines minimum area height required to open logs panel
pub(crate) const LOGS_PANEL_HEIGHT_REQUIREMENT: u16 = 40;

pub(crate) const MINIMUM_TERMINAL_SIZE: Size = Size {
    width: 96,
    height: 32,
};
