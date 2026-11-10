use crossterm::event::{Event, KeyCode};
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{
        Constraint::{Fill, Length},
        Layout, Rect,
    },
    style::{Modifier, Stylize},
    widgets::{
        Block, BorderType, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget,
    },
};
use std::sync::{Arc, RwLock};

use super::palette;
use super::panel::Panel;
use crate::{
    core::{Core, workflow::WorkflowState},
    tui::panel,
    workflows_count,
};

pub(crate) struct Sidebar {
    core: Arc<RwLock<Core>>,
    list_state: ListState,
}

impl Sidebar {
    pub(crate) const WIDTH: u16 = 30;

    pub(crate) fn new(core: Arc<RwLock<Core>>) -> Self {
        Self {
            core,
            list_state: ListState::default().with_selected(Some(0)),
        }
    }
}

impl Panel for Sidebar {
    fn draw(&mut self, area: Rect, buf: &mut Buffer, focused: bool) {
        buf.set_style(area, palette::BACKGROUND_STYLE);

        let items = self
            .core
            .read()
            .map(|core| {
                core.workflows
                    .iter()
                    .map(|w| match &w.state {
                        WorkflowState::Data(data) => ListItem::new(data.name.clone()).bold(),
                        WorkflowState::Error(err) => ListItem::new(format!(
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

        let button_color = if self.list_state.selected().is_some() {
            palette::FOREGROUND
        } else {
            palette::ACCENT
        };

        Paragraph::new("+ New workflow")
            .bg(palette::BACKGROUND)
            .fg(button_color)
            .bold()
            .centered()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(button_color),
            )
            .render(buttons_area, buf);
    }

    fn handle_event(
        &mut self,
        _terminal: &mut DefaultTerminal,
        event: &Event,
    ) -> anyhow::Result<()> {
        if let Some(key) = event.as_key_event() {
            match key.code {
                KeyCode::Up => self.list_state.select_previous(),
                KeyCode::Down => match self.list_state.selected() {
                    Some(selected) => {
                        if selected < workflows_count!(self.core) - 1 {
                            self.list_state.select_next()
                        } else {
                            self.list_state.select(None);
                        }
                    }
                    None => self.list_state.select_first(),
                },
                _ => {}
            }
        }
        Ok(())
    }
}
