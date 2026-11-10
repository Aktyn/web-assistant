use std::sync::{Arc, RwLock};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    widgets::{Paragraph, Widget},
};

use super::palette;
use crate::core::Core;

pub(crate) struct Content {
    core: Arc<RwLock<Core>>,
}

impl Widget for &Content {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, palette::BACKGROUND_STYLE);

        let directory_path = self
            .core
            .read()
            .expect("core lock poisoned")
            .workflows_directory
            .display()
            .to_string();

        Paragraph::new(format!("Workflows directory: {}", directory_path))
            .centered()
            .render(area.centered_vertically(Constraint::Length(1)), buf);
    }
}

impl Content {
    pub(crate) fn new(core: Arc<RwLock<Core>>) -> Self {
        Self { core }
    }
}
