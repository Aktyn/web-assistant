use ratatui::widgets::Widget as _;

use super::View;
use super::common::consts::MINIMUM_TERMINAL_SIZE;
use super::common::error_alert::ErrorAlert;

pub(crate) struct TerminalSizeErrorView {}

impl TerminalSizeErrorView {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl View for TerminalSizeErrorView {
    fn draw(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let text = vec![
            "Terminal is too small!".into(),
            format!("Current size: {}x{}", area.width, area.height).into(),
            format!(
                "Minimum size: {}x{}",
                MINIMUM_TERMINAL_SIZE.width, MINIMUM_TERMINAL_SIZE.height
            )
            .into(),
        ];
        ErrorAlert::new(text).render(area, buf);
    }

    fn handle_event(
        &mut self,
        _event: &crossterm::event::Event,
        _terminal: &mut ratatui::DefaultTerminal,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn get_action(&self) -> Option<super::ViewAction> {
        None
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
