use crossterm::event::{Event, KeyCode};
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{
        Constraint::{self, Fill, Length},
        Layout, Rect,
    },
    widgets::{Paragraph, Widget},
};
use std::sync::{Arc, RwLock};

use super::super::common::{palette, panel};
use crate::core::Core;

pub(crate) struct WorkflowPanel {
    core: Arc<RwLock<Core>>,
}

impl WorkflowPanel {
    pub(crate) fn new(core: Arc<RwLock<Core>>) -> Self {
        Self { core }
    }
}

impl panel::Panel for WorkflowPanel {
    fn draw(&mut self, area: Rect, buf: &mut Buffer, focused: bool) {
        buf.set_style(area, palette::BACKGROUND_STYLE);

        let [header_area, content_area] = area.layout(&Layout::vertical([Length(2), Fill(1)]));

        panel::Header::new("Panel", focused).render(header_area, buf);

        let directory_path = self
            .core
            .read()
            .expect("core lock poisoned")
            .workflows_directory
            .display()
            .to_string();

        Paragraph::new(format!(
            "Workflows directory: \"{}\"\nSelect another directory or run the workflow creator",
            directory_path
        ))
        .centered()
        .render(content_area.centered_vertically(Constraint::Length(2)), buf);
    }

    fn handle_event(
        &mut self,
        event: &Event,
        _terminal: &mut DefaultTerminal,
    ) -> anyhow::Result<()> {
        if let Some(key) = event.as_key_event() {
            match key.code {
                KeyCode::Up => log::info!("content up event"),
                KeyCode::Down => log::info!("content down event"),
                _ => {}
            }
        }
        Ok(())
    }
}
