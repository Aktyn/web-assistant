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
                    macros::move_selection!(self.list_state, Previous);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::common::panel::Panel;
    use crossterm::event::{Event, KeyCode};
    use std::mem::MaybeUninit;

    /// `handle_event` never reads `_terminal` for the events we exercise. We
    /// store an uninit'ed slot behind a `OnceLock` and return a raw address
    /// that satisfies the &mut `DefaultTerminal` type without being deref'd.
    fn no_read_term() -> *mut DefaultTerminal {
        static SLOT: std::sync::OnceLock<MaybeUninit<DefaultTerminal>> = std::sync::OnceLock::new();
        let slot = SLOT.get_or_init(MaybeUninit::uninit);
        slot.as_ptr().cast_mut()
    }

    #[test]
    fn test_workflows_sidebar_width_constant() {
        assert_eq!(WorkflowsSidebar::WIDTH, 30);
    }

    #[test]
    fn test_workflows_sidebar_new_initial_state() {
        let core = Arc::new(RwLock::new(Core::default()));
        let sidebar = WorkflowsSidebar::new(core);
        assert_eq!(sidebar.list_state.selected(), Some(0));
    }

    #[test]
    fn test_workflows_sidebar_draw_no_panic_empty() {
        let core = Arc::new(RwLock::new(Core::default()));
        let mut sidebar = WorkflowsSidebar::new(core);
        let area = Rect::new(0, 0, 30, 20);
        let mut buf = Buffer::empty(area);
        sidebar.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflows_sidebar_draw_no_panic_focused() {
        let core = Arc::new(RwLock::new(Core::default()));
        let mut sidebar = WorkflowsSidebar::new(core);
        let area = Rect::new(0, 0, 30, 20);
        let mut buf = Buffer::empty(area);
        sidebar.draw(area, &mut buf, true);
    }

    #[test]
    fn test_workflows_sidebar_handle_up_event() {
        let core = Arc::new(RwLock::new(Core::default()));
        let mut sidebar = WorkflowsSidebar::new(core);
        sidebar.list_state.select(Some(1));
        let key_event = Event::Key(KeyCode::Up.into());
        sidebar
            .handle_event(&key_event, unsafe { &mut *no_read_term() })
            .unwrap();
        assert_eq!(sidebar.list_state.selected(), Some(0));
    }

    #[test]
    fn test_workflows_sidebar_handle_down_event_wraps_to_none() {
        let core = Arc::new(RwLock::new(Core::default()));
        let mut sidebar = WorkflowsSidebar::new(core);
        sidebar.list_state.select(Some(0));
        let key_event = Event::Key(KeyCode::Down.into());
        sidebar
            .handle_event(&key_event, unsafe { &mut *no_read_term() })
            .unwrap();
        assert_eq!(sidebar.list_state.selected(), None);
    }

    #[test]
    fn test_workflows_sidebar_handle_j_key() {
        let core = Arc::new(RwLock::new(Core::default()));
        let mut sidebar = WorkflowsSidebar::new(core);
        sidebar.list_state.select(Some(0));
        let key_event = Event::Key(KeyCode::Char('j').into());
        sidebar
            .handle_event(&key_event, unsafe { &mut *no_read_term() })
            .unwrap();
        assert_eq!(sidebar.list_state.selected(), None);
    }

    #[test]
    fn test_workflows_sidebar_handle_k_key_moves_up() {
        let core = Arc::new(RwLock::new(Core::default()));
        let mut sidebar = WorkflowsSidebar::new(core);
        sidebar.list_state.select(Some(0));
        let key_event = Event::Key(KeyCode::Char('k').into());
        sidebar
            .handle_event(&key_event, unsafe { &mut *no_read_term() })
            .unwrap();
        assert_eq!(sidebar.list_state.selected(), Some(0));
    }

    #[test]
    fn test_workflows_sidebar_non_key_event_ignored() {
        let core = Arc::new(RwLock::new(Core::default()));
        let mut sidebar = WorkflowsSidebar::new(core);
        let mouse_event = Event::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        let result = sidebar.handle_event(&mouse_event, unsafe { &mut *no_read_term() });
        assert!(result.is_ok());
    }
}
