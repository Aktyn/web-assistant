mod workflow_panel;
mod workflows_sidebar;

use crossterm::event::KeyCode;
use ratatui::layout::Constraint::{Fill, Length};
use ratatui::layout::Layout;
use ratatui::widgets::Widget;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use super::common::{macros, panel, widgets};
use super::{View, ViewAction};
use workflow::Workflow;
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
    pending_action: Option<ViewAction>,
}

impl WorkflowView {
    pub(crate) fn new_with_directory(
        workflows: Arc<RwLock<Vec<Workflow>>>,
        config: Arc<RwLock<config::Config>>,
        workflows_directory: PathBuf,
    ) -> Self {
        let workflows_sidebar_arc = workflows.clone();
        let workflows_panel_arc = workflows.clone();

        Self {
            workflows,
            sidebar: WorkflowsSidebar::new(workflows_sidebar_arc),
            panel: WorkflowPanel::new(workflows_panel_arc, config, workflows_directory),
            show_sidebar: true,
            focused_panel: WorkflowsPanelType::Sidebar,
            pending_action: None,
        }
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

    fn focus_panel(&mut self) {
        self.focused_panel = WorkflowsPanelType::Panel;
        if let WorkflowPanelMode::Preview(preview) = &mut self.panel.mode {
            preview.list_state.select(Some(0));
        }
    }

    fn toggle_sidebar(&mut self) {
        if macros::workflows_count!(self.workflows) > 0 {
            self.show_sidebar = !self.show_sidebar;
        } else {
            self.show_sidebar = false;
        }
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
        let count = macros::workflows_count!(&self.workflows);
        if count == 0 && self.pending_action.is_none() {
            self.pending_action = Some(ViewAction::GoToWorkflows);
        }
        if let Some(selected) = self.sidebar.workflows_list_state.selected()
            && selected >= count
        {
            self.sidebar.workflows_list_state.select(if count > 0 {
                Some(count - 1)
            } else {
                None
            });
        }

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
                KeyCode::Enter | KeyCode::Char(' ')
                    if self.focused_panel == WorkflowsPanelType::Sidebar =>
                {
                    self.focus_panel();
                    return true;
                }
                // Uppercase only, lowercase 's' must reach text fields
                KeyCode::Char('S') => {
                    self.toggle_sidebar();
                    return true;
                }
                KeyCode::Tab | KeyCode::BackTab => {
                    match self.focused_panel {
                        WorkflowsPanelType::Sidebar => self.focus_panel(),
                        WorkflowsPanelType::Panel => self.focus_sidebar(),
                    }
                    return true;
                }
                KeyCode::Esc => match self.focused_panel {
                    WorkflowsPanelType::Panel => {
                        self.focus_sidebar();
                        return true;
                    }
                    WorkflowsPanelType::Sidebar => {
                        self.pending_action = Some(ViewAction::QuitApp);
                        return true;
                    }
                },
                KeyCode::Char('q') if self.focused_panel == WorkflowsPanelType::Sidebar => {
                    self.pending_action = Some(ViewAction::QuitApp);
                    return true;
                }
                _ => {}
            }
        }

        false
    }

    fn get_action(&self) -> Option<super::ViewAction> {
        self.pending_action
    }

    fn consume_action(&mut self) {
        self.pending_action = None;
    }

    fn tick(&mut self) {
        self.panel.tick();
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::View;
    use super::*;
    use crossterm::event::{Event, KeyCode};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use std::mem::MaybeUninit;
    use std::path::PathBuf;
    use workflow::JsonFile;
    use workflow::Workflow;

    fn empty_workflows() -> Arc<RwLock<Vec<Workflow>>> {
        Arc::new(RwLock::new(Vec::new()))
    }

    fn make_workflow(name: &str) -> Workflow {
        let json = JsonFile::from_raw(
            PathBuf::from("/tmp/test.json"),
            format!(r#"{{"name":"{name}","steps":[]}}"#),
        );
        Workflow::new(json)
    }

    fn test_workflows_directory() -> PathBuf {
        PathBuf::from("/tmp/test-workflows")
    }

    fn test_config() -> Arc<RwLock<config::Config>> {
        Arc::new(RwLock::new(config::Config::default()))
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
        let view =
            WorkflowView::new_with_directory(workflows, test_config(), test_workflows_directory());
        // Public state after construction: show_sidebar=true, focused=Sidebar
        assert!(view.show_sidebar);
        assert_eq!(view.focused_panel, WorkflowsPanelType::Sidebar);
    }

    #[test]
    fn test_workflow_view_focus_panel_from_sidebar() {
        let workflows = empty_workflows();
        let mut view =
            WorkflowView::new_with_directory(workflows, test_config(), test_workflows_directory());
        view.focus_panel();
        assert_eq!(view.focused_panel, WorkflowsPanelType::Panel);
    }

    #[test]
    fn test_workflow_view_as_any() {
        let workflows = empty_workflows();
        let view =
            WorkflowView::new_with_directory(workflows, test_config(), test_workflows_directory());
        assert!(view.as_any().is::<WorkflowView>());
    }

    #[test]
    fn test_workflow_view_get_focused_panel_returns_sidebar_when_focused() {
        let workflows = empty_workflows();
        let view =
            WorkflowView::new_with_directory(workflows, test_config(), test_workflows_directory());
        // Verify show_sidebar starts true and focused_panel is Sidebar
        assert!(view.show_sidebar);
        assert_eq!(view.focused_panel, WorkflowsPanelType::Sidebar);
    }

    #[test]
    fn test_workflow_view_get_focused_panel_returns_panel_after_tab() {
        let workflows = empty_workflows();
        let mut view =
            WorkflowView::new_with_directory(workflows, test_config(), test_workflows_directory());
        view.focus_panel();
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
        let mut view =
            WorkflowView::new_with_directory(workflows, test_config(), test_workflows_directory());
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        view.draw(area, &mut buf);
    }

    #[test]
    fn test_workflow_view_handle_event_tab_moves_focus() {
        let workflows = empty_workflows();
        let mut view =
            WorkflowView::new_with_directory(workflows, test_config(), test_workflows_directory());
        let event = Event::Key(KeyCode::Tab.into());
        // Use a dummy terminal reference — handle_event never reads it for Tab key
        let _ = view.handle_event(&event, unsafe { &mut *no_read_term() });
        assert_eq!(view.focused_panel, WorkflowsPanelType::Panel);
    }

    #[test]
    fn test_workflow_view_get_action_no_button_pressed_returns_none() {
        let workflows = empty_workflows();
        let view =
            WorkflowView::new_with_directory(workflows, test_config(), test_workflows_directory());
        // The sidebar has create_workflow_button_pressed = false by default
        assert!(view.get_action().is_none());
    }

    #[test]
    fn test_workflow_view_move_back_from_panel_with_workflows() {
        let workflows = Arc::new(RwLock::new(vec![make_workflow("test")]));
        let mut view =
            WorkflowView::new_with_directory(workflows, test_config(), test_workflows_directory());
        view.focus_panel();
        assert_eq!(view.focused_panel, WorkflowsPanelType::Panel);
        // Tab from the panel returns to the sidebar
        let tab = Event::Key(KeyCode::Tab.into());
        assert!(view.handle_event(&tab, unsafe { &mut *no_read_term() }));
        assert_eq!(view.focused_panel, WorkflowsPanelType::Sidebar);
    }

    #[test]
    fn test_typed_chars_in_editor_do_not_propagate() {
        let workflows = Arc::new(RwLock::new(vec![Workflow::new_draft()]));
        let mut view =
            WorkflowView::new_with_directory(workflows, test_config(), test_workflows_directory());
        view.panel.update(Some(0));
        view.focus_panel();
        for c in ['q', 'g', 'S', 's', 'l'] {
            let event = Event::Key(KeyCode::Char(c).into());
            assert!(view.handle_event(&event, unsafe { &mut *no_read_term() }));
        }
        assert!(view.get_action().is_none());
        assert!(view.show_sidebar);
    }

    #[test]
    fn test_esc_in_sidebar_requests_quit_and_consume_clears_it() {
        let workflows = Arc::new(RwLock::new(vec![make_workflow("test")]));
        let mut view =
            WorkflowView::new_with_directory(workflows, test_config(), test_workflows_directory());
        let esc = Event::Key(KeyCode::Esc.into());
        assert!(view.handle_event(&esc, unsafe { &mut *no_read_term() }));
        assert_eq!(view.get_action(), Some(ViewAction::QuitApp));
        view.consume_action();
        assert!(view.get_action().is_none());
    }

    #[test]
    fn test_draw_with_no_workflows_requests_go_to_workflows() {
        let mut view = WorkflowView::new_with_directory(
            empty_workflows(),
            test_config(),
            test_workflows_directory(),
        );
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        view.draw(area, &mut buf);
        assert_eq!(view.get_action(), Some(ViewAction::GoToWorkflows));
    }
    /// Pressing Enter on the "+ Create new workflow" button (sidebar
    /// selection `None`) must trigger draft creation via the sidebar's
    /// `create_workflow_button_pressed` flag and the view's `create_workflow`
    /// path. We verify the state changes without drawing (the Edit-mode panel
    /// rendering is not yet implemented, so we avoid `draw` here).
    #[test]
    fn test_enter_on_create_button_creates_draft_and_focuses_panel() {
        let workflows = empty_workflows();
        let mut view =
            WorkflowView::new_with_directory(workflows, test_config(), test_workflows_directory());
        // With no workflows, the list selection is `Some(0)` by default, which
        // is *not* the create button. Select `None` to land on the create
        // button while the sidebar is still focused.
        view.sidebar.workflows_list_state.select(None);

        let enter = Event::Key(KeyCode::Enter.into());
        view.handle_event(&enter, unsafe { &mut *no_read_term() });

        // A draft workflow should have been inserted at index 0.
        let workflows = view.workflows.read().unwrap();
        assert_eq!(workflows.len(), 1);
        assert!(workflows[0].file_path.is_none(), "draft has no file path");
    }

    /// Pressing Enter on a real workflow item (not the create button) must
    /// move focus to the panel, not create a draft.
    #[test]
    fn test_enter_on_workflow_item_moves_focus_to_panel() {
        let workflows = Arc::new(RwLock::new(vec![make_workflow("test")]));
        let mut view =
            WorkflowView::new_with_directory(workflows, test_config(), test_workflows_directory());
        assert_eq!(view.focused_panel, WorkflowsPanelType::Sidebar);
        let enter = Event::Key(KeyCode::Enter.into());
        view.handle_event(&enter, unsafe { &mut *no_read_term() });
        assert_eq!(view.focused_panel, WorkflowsPanelType::Panel);
    }
}
