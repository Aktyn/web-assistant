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
use crate::{
    utils,
    workflow::{Workflow, WorkflowState},
};

pub(crate) struct WorkflowsSidebar {
    workflows: Arc<RwLock<Vec<Workflow>>>,
    pub(super) workflows_list_state: ListState,
    pub(super) create_workflow_button_pressed: bool,
}

impl WorkflowsSidebar {
    pub(super) const WIDTH: u16 = 30;

    pub(super) fn new(workflows: Arc<RwLock<Vec<Workflow>>>) -> Self {
        Self {
            workflows,
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
        buf.set_style(area, palette().background_style);

        let items = self
            .workflows
            .read()
            .map(|workflows| {
                workflows
                    .iter()
                    .map(|w| match &w.state {
                        WorkflowState::Running(running_workflow, ..) => {
                            let label = utils::truncate_text(
                                &running_workflow.data.name,
                                WorkflowsSidebar::WIDTH.into(),
                                None,
                            );
                            ListItem::new(label).fg(palette().accent).bold()
                        }
                        WorkflowState::SuccessfulRun(succesfull_workflow) => {
                            let label = utils::truncate_text(
                                &succesfull_workflow.data.name,
                                WorkflowsSidebar::WIDTH.into(),
                                None,
                            );
                            ListItem::new(label).fg(palette().success_foreground).bold()
                        }
                        WorkflowState::FailedRun(failed_workflow) => {
                            let label = utils::truncate_text(
                                &failed_workflow.data.name,
                                WorkflowsSidebar::WIDTH.into(),
                                None,
                            );
                            ListItem::new(label).fg(palette().error_foreground).bold()
                        }
                        WorkflowState::Valid(data) => {
                            let label = utils::truncate_text(
                                &data.name,
                                WorkflowsSidebar::WIDTH.into(),
                                None,
                            );
                            ListItem::new(label).bold()
                        }
                        WorkflowState::Invalid(_) => {
                            let file_path = w
                                .file_path
                                .as_ref()
                                .expect("Workflow that failed to load must have a file path");
                            let file_name_str =
                                file_path.file_name().and_then(OsStr::to_str).unwrap_or("");
                            let text = vec![
                                utils::truncate_text(
                                    "Invalid workflow file found",
                                    WorkflowsSidebar::WIDTH.into(),
                                    None,
                                )
                                .into(),
                                utils::truncate_text(
                                    file_name_str,
                                    WorkflowsSidebar::WIDTH.into(),
                                    None,
                                )
                                .into(),
                            ];
                            ListItem::new(text).style(palette().error_foreground)
                        }
                    })
                    .collect::<Vec<ListItem>>()
            })
            .unwrap_or(vec![]);

        let [header_area, list_area, buttons_area] =
            area.layout(&Layout::vertical([Length(2), Fill(1), Length(3)]));

        panel::Header::new(format!("Workflows: {}", items.len()), focused).render(header_area, buf);

        let list = List::new(items)
            .style(palette().style)
            .highlight_style(Modifier::REVERSED);

        StatefulWidget::render(list, list_area, buf, &mut self.workflows_list_state);

        Button::new("+ Create new workflow")
            .set_selected(self.is_create_workflow_button_selected())
            .render(buttons_area, buf);
    }

    fn handle_event(&mut self, event: &Event, _terminal: &mut DefaultTerminal) -> bool {
        if let Some(key) = event.as_key_event() {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    macros::move_selection!(self.workflows_list_state, Previous);
                    return true;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let workflows_count = macros::workflows_count!(self.workflows);
                    macros::move_selection!(self.workflows_list_state, Next, workflows_count);
                    if self.workflows_list_state.selected() == Some(workflows_count) {
                        self.workflows_list_state.select(None);
                    }
                    return true;
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.create_workflow_button_pressed = self.is_create_workflow_button_selected();
                }
                _ => {}
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::common::panel::Panel;
    use crossterm::event::{Event, KeyCode};
    use std::mem::MaybeUninit;

    fn empty_workflows() -> Arc<RwLock<Vec<Workflow>>> {
        Arc::new(RwLock::new(Vec::new()))
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
        let workflows = empty_workflows();
        let sidebar = WorkflowsSidebar::new(workflows);
        assert_eq!(sidebar.workflows_list_state.selected(), Some(0));
    }

    #[test]
    fn test_workflows_sidebar_draw_no_panic_empty() {
        let workflows = empty_workflows();
        let mut sidebar = WorkflowsSidebar::new(workflows);
        let area = Rect::new(0, 0, 30, 20);
        let mut buf = Buffer::empty(area);
        sidebar.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflows_sidebar_draw_no_panic_focused() {
        let workflows = empty_workflows();
        let mut sidebar = WorkflowsSidebar::new(workflows);
        let area = Rect::new(0, 0, 30, 20);
        let mut buf = Buffer::empty(area);
        sidebar.draw(area, &mut buf, true);
    }

    #[test]
    fn test_workflows_sidebar_handle_up_event() {
        let workflows = empty_workflows();
        let mut sidebar = WorkflowsSidebar::new(workflows);
        sidebar.workflows_list_state.select(Some(1));
        let key_event = Event::Key(KeyCode::Up.into());
        let handled = sidebar.handle_event(&key_event, unsafe { &mut *no_read_term() });
        assert!(handled);
        assert_eq!(sidebar.workflows_list_state.selected(), Some(0));
    }

    #[test]
    fn test_workflows_sidebar_handle_down_event_wraps_to_none() {
        let workflows = empty_workflows();
        let mut sidebar = WorkflowsSidebar::new(workflows);
        sidebar.workflows_list_state.select(Some(0));
        let key_event = Event::Key(KeyCode::Down.into());
        let handled = sidebar.handle_event(&key_event, unsafe { &mut *no_read_term() });
        assert!(handled);
        assert_eq!(sidebar.workflows_list_state.selected(), None);
    }

    #[test]
    fn test_workflows_sidebar_handle_j_key() {
        let workflows = empty_workflows();
        let mut sidebar = WorkflowsSidebar::new(workflows);
        sidebar.workflows_list_state.select(Some(0));
        let key_event = Event::Key(KeyCode::Char('j').into());
        let handled = sidebar.handle_event(&key_event, unsafe { &mut *no_read_term() });
        assert!(handled);
        assert_eq!(sidebar.workflows_list_state.selected(), None);
    }

    #[test]
    fn test_workflows_sidebar_handle_k_key_moves_up() {
        let workflows = empty_workflows();
        let mut sidebar = WorkflowsSidebar::new(workflows);
        sidebar.workflows_list_state.select(Some(0));
        let key_event = Event::Key(KeyCode::Char('k').into());
        let handled = sidebar.handle_event(&key_event, unsafe { &mut *no_read_term() });
        assert!(handled);
        assert_eq!(sidebar.workflows_list_state.selected(), Some(0));
    }

    #[test]
    fn test_workflows_sidebar_non_key_event_ignored() {
        let workflows = empty_workflows();
        let mut sidebar = WorkflowsSidebar::new(workflows);
        let mouse_event = Event::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        let result = sidebar.handle_event(&mouse_event, unsafe { &mut *no_read_term() });
        assert!(!result);
    }
}
