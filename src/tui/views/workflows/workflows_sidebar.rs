use crossterm::event::{Event, KeyCode};
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{
        Constraint::{Fill, Length},
        Layout, Rect,
    },
    style::{Modifier, Stylize as _},
    widgets::{List, ListItem, ListState, StatefulWidget, Widget},
};
use std::{
    ffi::OsStr,
    sync::{Arc, RwLock},
};

use super::super::common::{macros, palette, panel, widgets::Button};
use crate::core::{self, Core};
use crate::utils;

pub(crate) struct WorkflowsSidebar {
    core: Arc<RwLock<Core>>,
    pub(super) workflows_list_state: ListState,
    pub(super) create_workflow_button_pressed: bool,
}

impl WorkflowsSidebar {
    pub(super) const WIDTH: u16 = 30;

    pub(super) fn new(core: Arc<RwLock<Core>>) -> Self {
        Self {
            core,
            workflows_list_state: ListState::default().with_selected(Some(0)),
            create_workflow_button_pressed: false,
        }
    }

    fn is_create_workflow_button_selected(&mut self) -> bool {
        self.workflows_list_state.selected().is_none()
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
                            let label = utils::truncate_text(
                                &data.name,
                                WorkflowsSidebar::WIDTH as usize,
                                None,
                            );
                            ListItem::new(label).bold()
                        }
                        core::workflow::WorkflowState::Error(_) => {
                            let text = vec![
                                utils::truncate_text(
                                    "Invalid workflow file found",
                                    WorkflowsSidebar::WIDTH as usize,
                                    None,
                                )
                                .into(),
                                utils::truncate_text(
                                    &format!(
                                        "{}",
                                        w.file_path
                                            .file_name()
                                            .unwrap_or(OsStr::new(&w.file_path))
                                            .display()
                                    ),
                                    WorkflowsSidebar::WIDTH as usize,
                                    None,
                                )
                                .into(),
                            ];
                            ListItem::new(text).style(palette::ERROR_FOREGROUND)
                        }
                    })
                    .collect::<Vec<ListItem>>()
            })
            .unwrap_or(vec![]);

        let [header_area, list_area, buttons_area] =
            area.layout(&Layout::vertical([Length(2), Fill(1), Length(3)]));

        panel::Header::new(format!("Workflows: {}", items.len()), focused).render(header_area, buf);

        let list = List::new(items)
            .style(palette::STYLE)
            .highlight_style(Modifier::REVERSED);
        // .highlight_symbol("");

        StatefulWidget::render(list, list_area, buf, &mut self.workflows_list_state);

        Button::new("+ Create new workflow")
            .set_selected(self.is_create_workflow_button_selected())
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
                    macros::move_selection!(self.workflows_list_state, Previous);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let workflows_count = macros::workflows_count!(self.core);
                    macros::move_selection!(self.workflows_list_state, Next, workflows_count);
                    if self.workflows_list_state.selected() == Some(workflows_count) {
                        self.workflows_list_state.select(None);
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.create_workflow_button_pressed = self.is_create_workflow_button_selected();
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Config;
    use crate::tui::common::panel::Panel;
    use crossterm::event::{Event, KeyCode};
    use std::mem::MaybeUninit;

    fn empty_core() -> Core {
        Core {
            config: Config::default(),
            workflows: Vec::new(),
        }
    }

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
        let core = Arc::new(RwLock::new(empty_core()));
        let sidebar = WorkflowsSidebar::new(core);
        assert_eq!(sidebar.workflows_list_state.selected(), Some(0));
    }

    #[test]
    fn test_workflows_sidebar_draw_no_panic_empty() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut sidebar = WorkflowsSidebar::new(core);
        let area = Rect::new(0, 0, 30, 20);
        let mut buf = Buffer::empty(area);
        sidebar.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflows_sidebar_draw_no_panic_focused() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut sidebar = WorkflowsSidebar::new(core);
        let area = Rect::new(0, 0, 30, 20);
        let mut buf = Buffer::empty(area);
        sidebar.draw(area, &mut buf, true);
    }

    #[test]
    fn test_workflows_sidebar_handle_up_event() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut sidebar = WorkflowsSidebar::new(core);
        sidebar.workflows_list_state.select(Some(1));
        let key_event = Event::Key(KeyCode::Up.into());
        sidebar
            .handle_event(&key_event, unsafe { &mut *no_read_term() })
            .unwrap();
        assert_eq!(sidebar.workflows_list_state.selected(), Some(0));
    }

    #[test]
    fn test_workflows_sidebar_handle_down_event_wraps_to_none() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut sidebar = WorkflowsSidebar::new(core);
        sidebar.workflows_list_state.select(Some(0));
        let key_event = Event::Key(KeyCode::Down.into());
        sidebar
            .handle_event(&key_event, unsafe { &mut *no_read_term() })
            .unwrap();
        assert_eq!(sidebar.workflows_list_state.selected(), None);
    }

    #[test]
    fn test_workflows_sidebar_handle_j_key() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut sidebar = WorkflowsSidebar::new(core);
        sidebar.workflows_list_state.select(Some(0));
        let key_event = Event::Key(KeyCode::Char('j').into());
        sidebar
            .handle_event(&key_event, unsafe { &mut *no_read_term() })
            .unwrap();
        assert_eq!(sidebar.workflows_list_state.selected(), None);
    }

    #[test]
    fn test_workflows_sidebar_handle_k_key_moves_up() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut sidebar = WorkflowsSidebar::new(core);
        sidebar.workflows_list_state.select(Some(0));
        let key_event = Event::Key(KeyCode::Char('k').into());
        sidebar
            .handle_event(&key_event, unsafe { &mut *no_read_term() })
            .unwrap();
        assert_eq!(sidebar.workflows_list_state.selected(), Some(0));
    }

    #[test]
    fn test_workflows_sidebar_non_key_event_ignored() {
        let core = Arc::new(RwLock::new(empty_core()));
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
