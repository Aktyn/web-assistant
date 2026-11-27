use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Stylize as _,
    text::{Line, Text},
    widgets::{Paragraph, Block, BorderType, Borders},
};

use crate::common::palette;
use crate::views::{View, ViewAction};

/// View that explains the config structure
#[derive(Debug, Clone)]
pub(crate) struct ExplainConfigView {
    pending_action: Option<ViewAction>,
}

impl ExplainConfigView {
    pub(crate) fn new() -> Self {
        Self {
            pending_action: None,
        }
    }

    fn render(&self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let [_, info_area, details_area, status_area, buttons_area, _] =
            area.layout(&Layout::vertical([
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Length(25),
                Constraint::Length(1),
                Constraint::Length(Button::HEIGHT),
                Constraint::Fill(1),
            ]));

        // Header
        Paragraph::new("Explain Config")
            .style(palette().style.bold())
            .centered()
            .render(info_area, buf);

        // Config details - same as what ConfigView shows
        let details = Text::from(vec![
            Line::from("Config Structure"),
            Line::from(""),
            Line::from("Workflows directory:"),
            Line::from("  (shown in Config Management view)"),
            Line::from(""),
            Line::from("TUI colors:"),
            Line::from("  - custom: customized color scheme"),
            Line::from("  - default: standard TUI colors"),
            Line::from(""),
            Line::from("Browser headless mode:"),
            Line::from("  - true: runs without browser UI"),
            Line::from("  - false: browser UI is shown"),
            Line::from(""),
            Line::from("Agent model:"),
            Line::from("  - default: uses default model"),
            Line::from("  - named: specifies a particular model"),
        ]);

        Paragraph::new(details)
            .style(palette().style)
            .block(
                Block::bordered()
                    .fg(palette().muted_foreground)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(format!(" {} ", "Config").bold()),
            )
            .render(details_area, buf);

        // Buttons
        let button_areas: [ratatui::layout::Rect; 2] = buttons_area
            .centered_horizontally(Constraint::Max(50))
            .layout(&Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        // "Close" button
        let close_btn = crate::common::widgets::Button::new("Close")
            .render(button_areas[0], buf);
    }
}

impl View for ExplainConfigView {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, palette().style);
        self.render(area, buf);
    }

    fn handle_event(
        &mut self,
        event: &crossterm::event::Event,
        terminal: &mut DefaultTerminal,
    ) -> bool {
        if let Some(key) = event.as_key_event() {
            // Enter or Esc closes
            if key.code == crossterm::event::KeyCode::Enter
                || key.code == crossterm::event::KeyCode::Esc
            {
                self.pending_action = Some(ViewAction::GoToWorkflows);
                return true;
            }
        }
        false
    }

    fn get_action(&self) -> Option<ViewAction> {
        self.pending_action
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}