use ratatui::{
    buffer::Buffer,
    layout::{
        Constraint::{Fill, Length},
        Layout, Rect,
    },
    style::{Modifier, Stylize as _},
    widgets::{List, Paragraph, Widget},
};
use std::sync::{Arc, RwLock};

use super::palette;
use crate::core::Core;

pub(crate) struct Sidebar {
    core: Arc<RwLock<Core>>,
    pub(crate) show: bool,
}

impl Widget for &Sidebar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, palette::BACKGROUND_STYLE);

        let workflows_len = self
            .core
            .read()
            .map(|core| core.workflows.len())
            .unwrap_or(0);

        let [header_area, list_area] = area.layout(&Layout::vertical([Length(2), Fill(1)]));

        Paragraph::new(format!("[ Workflows: {} ]", workflows_len))
            .centered()
            .bold()
            .bg(palette::BACKGROUND)
            .fg(palette::FOREGROUND)
            .render(header_area, buf);

        let items = ["Item 1", "Item 2", "Item 3", "Item 4"];
        let list = List::new(items)
            .style(palette::FOREGROUND_STYLE)
            .highlight_style(Modifier::REVERSED)
            .highlight_symbol("> ");

        // list.s
        list.render(list_area, buf);
    }
}

impl Sidebar {
    pub(crate) const WIDTH: u16 = 30;

    pub(crate) fn new(core: Arc<RwLock<Core>>) -> Self {
        Self {
            core,
            show: true,
        }
    }

    pub(crate) fn select_item(&mut self, arg: i16) {
        log::info!("Selecting item: {}", arg);
        // self.item = arg;
    }

    // const fn new(constraint: Constraint) -> Self {
    // fn new() -> Self {
    //     Self {
    //         // constraint,
    //     }
    // }
}
