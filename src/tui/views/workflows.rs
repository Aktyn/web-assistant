mod workflow_panel;
mod workflows_sidebar;

use crossterm::event::KeyCode;
use ratatui::layout::Constraint::{Fill, Length};
use ratatui::layout::Layout;
use ratatui::widgets::Widget;
use std::sync::{Arc, RwLock};

use super::View;
use super::common::{macros, panel, widgets};
use crate::workflow::Workflow;
use workflow_panel::{WorkflowPanel, WorkflowPanelMode};
use workflows_sidebar::WorkflowsSidebar;

#[derive(PartialEq, Debug)]
enum WorkflowsPanelType {
    Sidebar,
    Panel,
}

pub(crate) struct WorkflowView {
    workflows: Arc<RwLock<Vec<Workflow>>>,
    pub(super) panel: WorkflowPanel,
    pub(super) sidebar: WorkflowsSidebar,
    show_sidebar: bool,
    focused_panel: WorkflowsPanelType,
}

impl WorkflowView {
    pub(crate) fn new(workflows: Arc<RwLock<Vec<Workflow>>>) -> Self {
        let workflows_sidebar_arc = workflows.clone();
        let workflows_panel_arc = workflows.clone();

        Self {
            workflows,
            sidebar: WorkflowsSidebar::new(workflows_sidebar_arc),
            panel: WorkflowPanel::new(workflows_panel_arc),
            show_sidebar: true,
            focused_panel: WorkflowsPanelType::Sidebar,
        }
    }

    fn move_to_next_panel(&mut self) {
        match self.focused_panel {
            WorkflowsPanelType::Sidebar => self.focused_panel = WorkflowsPanelType::Panel,
            WorkflowsPanelType::Panel => {
                if macros::workflows_count!(self.workflows) > 0 {
                    self.focus_sidebar();
                }
            }
        }
    }
    fn move_to_previous_panel(&mut self) {
        //NOTE: this works as long as there are only 2 panels
        self.move_to_next_panel();
    }

    pub(crate) fn create_workflow(&mut self) {
        {
            let Ok(mut workflows_write_guard) = self.workflows.write() else {
                return;
            };
            workflows_write_guard.insert(0, Workflow::new_draft());
        }

        self.sidebar.workflows_list_state.select_first();
        self.panel.open_workflow(0);
        self.focused_panel = WorkflowsPanelType::Panel;
    }

    fn focus_sidebar(&mut self) {
        self.focused_panel = WorkflowsPanelType::Sidebar;
        if let WorkflowPanelMode::Preview(preview) = &mut self.panel.mode {
            preview.list_state.select(None);
        }
    }
}

impl View for WorkflowView {
    fn draw(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        self.panel
            .update(self.sidebar.workflows_list_state.selected());

        if self.show_sidebar {
            let [sidebar_area, separator_area, panel_area] = area.layout(&Layout::horizontal([
                Length(WorkflowsSidebar::WIDTH),
                Length(1),
                Fill(1),
            ]));

            panel::Panel::draw(
                &mut self.sidebar,
                sidebar_area,
                buf,
                self.focused_panel == WorkflowsPanelType::Sidebar,
            );

            widgets::Separator::new(widgets::Separator::VERTICAL).render(separator_area, buf);

            panel::Panel::draw(
                &mut self.panel,
                panel_area,
                buf,
                self.focused_panel == WorkflowsPanelType::Panel,
            );
        } else {
            panel::Panel::draw(
                &mut self.panel,
                area,
                buf,
                self.focused_panel == WorkflowsPanelType::Panel,
            );
        }
    }

    fn handle_event(
        &mut self,
        event: &crossterm::event::Event,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> bool {
        let prevent_propagation = match self.focused_panel {
            WorkflowsPanelType::Sidebar => {
                panel::Panel::handle_event(&mut self.sidebar, event, terminal)
            }
            WorkflowsPanelType::Panel => {
                panel::Panel::handle_event(&mut self.panel, event, terminal)
            }
        };

        if self.sidebar.create_workflow_button_pressed {
            self.sidebar.create_workflow_button_pressed = false;
            self.create_workflow();
            return true;
        }

        if prevent_propagation {
            return true;
        }

        if let Some(key) = event.as_key_event() {
            match key.code {
                KeyCode::Char('s') => {
                    if macros::workflows_count!(self.workflows) > 0 {
                        self.show_sidebar = !self.show_sidebar;
                    } else {
                        self.show_sidebar = false;
                    }
                }
                KeyCode::Tab => {
                    self.move_to_next_panel();
                    return true;
                }
                KeyCode::BackTab => {
                    self.move_to_previous_panel();
                    return true;
                }
                KeyCode::Enter | KeyCode::Char(' ')
                    if (self.focused_panel == WorkflowsPanelType::Sidebar) =>
                {
                    self.focused_panel = WorkflowsPanelType::Panel;
                    if let WorkflowPanelMode::Preview(preview) = &mut self.panel.mode {
                        preview.list_state.select(Some(0));
                    }
                    return true;
                }
                KeyCode::Backspace if (self.focused_panel == WorkflowsPanelType::Panel) => {
                    self.focus_sidebar();
                    return true;
                }
                _ => {}
            }
        }

        prevent_propagation
    }

    fn get_action(&self) -> Option<super::ViewAction> {
        None
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::views::View;
    use crate::workflow::{Workflow, WorkflowDataJson};
    use crossterm::event::{Event, KeyCode};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use std::mem::MaybeUninit;
    use std::path::PathBuf;

    fn empty_workflows() -> Arc<RwLock<Vec<Workflow>>> {
        Arc::new(RwLock::new(Vec::new()))
    }

    fn make_workflow(name: &str) -> Workflow {
        let json = WorkflowDataJson::from_raw(
            PathBuf::from("/tmp/test.json"),
            format!(r#"{{"name":"{name}"}}"#),
        );
        Workflow::new(json)
    }

    #[test]
    fn test_panel_type_debug() {
        assert_eq!(format!("{:?}", WorkflowsPanelType::Sidebar), "Sidebar");
        assert_eq!(format!("{:?}", WorkflowsPanelType::Panel), "Panel");
        assert_ne!(WorkflowsPanelType::Sidebar, WorkflowsPanelType::Panel);
    }

    #[test]
    fn test_panel_type_partial_eq() {
        assert_eq!(WorkflowsPanelType::Sidebar, WorkflowsPanelType::Sidebar);
        assert_eq!(WorkflowsPanelType::Panel, WorkflowsPanelType::Panel);
    }

    #[test]
    fn test_workflow_view_new_default_state() {
        let workflows = empty_workflows();
        let view = WorkflowView::new(workflows);
        // Public state after construction: show_sidebar=true, focused=Sidebar
        assert!(view.show_sidebar);
        assert_eq!(view.focused_panel, WorkflowsPanelType::Sidebar);
    }

    #[test]
    fn test_workflow_view_move_to_next_panel_from_sidebar() {
        let workflows = empty_workflows();
        let mut view = WorkflowView::new(workflows);
        view.move_to_next_panel();
        assert_eq!(view.focused_panel, WorkflowsPanelType::Panel);
    }

    #[test]
    fn test_workflow_view_move_to_next_panel_from_panel_empty() {
        // When no workflows exist, moving from Panel stays at Panel
        let workflows = empty_workflows();
        let mut view = WorkflowView::new(workflows);
        // Move to Panel first
        view.move_to_next_panel();
        assert_eq!(view.focused_panel, WorkflowsPanelType::Panel);
        // Moving again from Panel with 0 workflows should stay on Panel
        view.move_to_next_panel();
        assert_eq!(view.focused_panel, WorkflowsPanelType::Panel);
    }

    #[test]
    fn test_workflow_view_as_any() {
        let workflows = empty_workflows();
        let view = WorkflowView::new(workflows);
        assert!(view.as_any().is::<WorkflowView>());
    }

    #[test]
    fn test_workflow_view_get_focused_panel_returns_sidebar_when_focused() {
        let workflows = empty_workflows();
        let view = WorkflowView::new(workflows);
        // Verify show_sidebar starts true and focused_panel is Sidebar
        assert!(view.show_sidebar);
        assert_eq!(view.focused_panel, WorkflowsPanelType::Sidebar);
    }

    #[test]
    fn test_workflow_view_get_focused_panel_returns_panel_after_tab() {
        let workflows = empty_workflows();
        let mut view = WorkflowView::new(workflows);
        view.move_to_next_panel();
        assert_eq!(view.focused_panel, WorkflowsPanelType::Panel);
    }

    /// `handle_event` never reads `_terminal` for the events we exercise. We
    /// store an uninit'ed slot behind a `OnceLock` and return a raw address
    /// that satisfies the &mut `DefaultTerminal` type without being deref'd.
    fn no_read_term() -> *mut ratatui::DefaultTerminal {
        static SLOT: std::sync::OnceLock<MaybeUninit<ratatui::DefaultTerminal>> =
            std::sync::OnceLock::new();
        let slot = SLOT.get_or_init(MaybeUninit::uninit);
        slot.as_ptr().cast_mut()
    }

    #[test]
    fn test_workflow_view_draw_no_panic() {
        let workflows = empty_workflows();
        let mut view = WorkflowView::new(workflows);
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        view.draw(area, &mut buf);
    }

    #[test]
    fn test_workflow_view_handle_event_tab_moves_focus() {
        let workflows = empty_workflows();
        let mut view = WorkflowView::new(workflows);
        let event = Event::Key(KeyCode::Tab.into());
        // Use a dummy terminal reference — handle_event never reads it for Tab key
        let _ = view.handle_event(&event, unsafe { &mut *no_read_term() });
        assert_eq!(view.focused_panel, WorkflowsPanelType::Panel);
    }

    #[test]
    fn test_workflow_view_get_action_no_button_pressed_returns_none() {
        let workflows = empty_workflows();
        let view = WorkflowView::new(workflows);
        // The sidebar has create_workflow_button_pressed = false by default
        assert!(view.get_action().is_none());
    }

    #[test]
    fn test_workflow_view_move_back_from_panel_with_workflows() {
        let workflows = Arc::new(RwLock::new(vec![make_workflow("test")]));
        let mut view = WorkflowView::new(workflows);
        // Move to panel
        view.move_to_next_panel();
        assert_eq!(view.focused_panel, WorkflowsPanelType::Panel);
        // Moving again from Panel should return to Sidebar (workflows_count > 0)
        view.move_to_next_panel();
        assert_eq!(view.focused_panel, WorkflowsPanelType::Sidebar);
    }

    /// Drive the *real* create-workflow path: select the "+ Create new
    /// workflow" button (list selection `None`) and press Enter, then draw.
    /// The draft must appear in the rendered buffer from this single event —
    /// no second key press should be required.
    #[test]
    fn test_create_workflow_via_enter_renders_draft() {
        let workflows = empty_workflows();
        let mut view = WorkflowView::new(workflows);
        // With no workflows, the list selection is `Some(0)` by default, which
        // is *not* the create button. Select `None` to land on the create
        // button while the sidebar is still focused.
        view.sidebar.workflows_list_state.select(None);

        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        view.draw(area, &mut buf);
        let before: String = buf.content().iter().map(|c| c.symbol()).collect::<String>();
        assert!(
            !before.contains("Draft Workflow"),
            "draft should not exist before pressing Enter"
        );

        // Pressing Enter on the create button must create the draft inline —
        // `handle_event` is the only call, mirroring the real event loop.
        let enter = Event::Key(KeyCode::Enter.into());
        view.handle_event(&enter, unsafe { &mut *no_read_term() });

        let mut buf2 = Buffer::empty(area);
        view.draw(area, &mut buf2);
        let after: String = buf2
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(
            after.contains("Draft Workflow"),
            "draft workflow should appear after pressing Enter on the create button, got: {after:?}"
        );
    }

    /// Pressing Enter on a real workflow item (not the create button) must
    /// move focus to the panel, not create a draft.
    #[test]
    fn test_enter_on_workflow_item_moves_focus_to_panel() {
        let workflows = Arc::new(RwLock::new(vec![make_workflow("test")]));
        let mut view = WorkflowView::new(workflows);
        assert_eq!(view.focused_panel, WorkflowsPanelType::Sidebar);
        let enter = Event::Key(KeyCode::Enter.into());
        view.handle_event(&enter, unsafe { &mut *no_read_term() });
        assert_eq!(view.focused_panel, WorkflowsPanelType::Panel);
    }
}
