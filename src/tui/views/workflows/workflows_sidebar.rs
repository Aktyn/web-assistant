use crossterm::event::{Event, KeyCode};
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{
        Constraint::{Fill, Length},
        Layout, Rect,
    },
    style::{Modifier, Stylize},
    widgets::{List, ListItem, ListState, StatefulWidget, Widget},
};
use std::sync::{Arc, RwLock};

use super::super::common::{button::Button, macros, palette, panel};
use crate::core::{self, Core};

pub(crate) struct WorkflowsSidebar {
    core: Arc<RwLock<Core>>,
    list_state: ListState,
}

impl WorkflowsSidebar {
    pub(crate) const WIDTH: u16 = 30;

    pub(crate) fn new(core: Arc<RwLock<Core>>) -> Self {
        Self {
            core,
            list_state: ListState::default().with_selected(Some(0)),
        }
    }
}

impl panel::Panel for WorkflowsSidebar {
    fn draw(&mut self, area: Rect, buf: &mut Buffer, focused: bool) {
        buf.set_style(area, palette::BACKGROUND_STYLE);

        let items = self
            .core
            .read()
            .map(|core| {
                core.workflows
                    .iter()
                    .map(|w| match &w.state {
                        core::workflow::WorkflowState::Data(data) => {
                            ListItem::new(data.name.clone()).bold()
                        }
                        core::workflow::WorkflowState::Error(err) => ListItem::new(format!(
                            "Invalid workflow\nFile: \"{}\"\nError: \"{}\"",
                            w.file_path.display(),
                            err
                        ))
                        .style(palette::ERROR_FOREGROUND),
                    })
                    .collect::<Vec<ListItem>>()
            })
            .unwrap_or(vec![]);

        let [header_area, list_area, buttons_area] =
            area.layout(&Layout::vertical([Length(2), Fill(1), Length(3)]));

        panel::Header::new(format!("Workflows: {}", items.len()), focused).render(header_area, buf);

        let list = List::new(items)
            .style(palette::STYLE)
            .highlight_style(Modifier::REVERSED)
            .highlight_symbol("> ");

        StatefulWidget::render(list, list_area, buf, &mut self.list_state);

        //TODO: workflow creation wizard

        Button::new("+ Create new workflow")
            .set_selected(self.list_state.selected().is_none())
            .render(buttons_area, buf);
    }

    fn handle_event(
        &mut self,
        event: &Event,
        _terminal: &mut DefaultTerminal,
    ) -> anyhow::Result<()> {
        if let Some(key) = event.as_key_event() {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    macros::move_selection!(self.list_state, Previous)
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let workflows_count = macros::workflows_count!(self.core);
                    macros::move_selection!(self.list_state, Next, workflows_count);
                    if self.list_state.selected() == Some(workflows_count) {
                        self.list_state.select(None);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
