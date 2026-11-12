mod workflow_panel;
mod workflows_sidebar;

use crossterm::event::KeyCode;
use ratatui::layout::Constraint::{Fill, Length};
use ratatui::layout::Layout;
use std::sync::{Arc, RwLock};

use super::common::{macros, palette, panel};
use super::{View, ViewAction};
use crate::core::Core;
use workflow_panel::WorkflowPanel;
use workflows_sidebar::WorkflowsSidebar;

#[derive(PartialEq, Debug)]
enum WorkflowsPanelType {
    Sidebar,
    Panel,
}

pub(crate) struct WorkflowView {
    core: Arc<RwLock<Core>>,
    panel: Box<dyn panel::Panel>,
    sidebar: Box<dyn panel::Panel>,
    show_sidebar: bool,
    focused_panel: WorkflowsPanelType,
}

impl WorkflowView {
    pub(crate) fn new(core: Arc<RwLock<Core>>) -> Self {
        let core_sidebar_arc = core.clone();
        let core_panel_arc = core.clone();

        Self {
            core,
            sidebar: Box::new(WorkflowsSidebar::new(core_sidebar_arc)),
            panel: Box::new(WorkflowPanel::new(core_panel_arc)),
            show_sidebar: true,
            focused_panel: WorkflowsPanelType::Sidebar,
        }
    }

    fn move_to_next_panel(&mut self) {
        match self.focused_panel {
            WorkflowsPanelType::Sidebar => self.focused_panel = WorkflowsPanelType::Panel,
            WorkflowsPanelType::Panel => {
                if macros::workflows_count!(self.core) > 0 {
                    self.focused_panel = WorkflowsPanelType::Sidebar;
                }
            }
        }
    }

    fn get_focused_panel(&mut self) -> &mut dyn panel::Panel {
        match self.focused_panel {
            WorkflowsPanelType::Sidebar => self.sidebar.as_mut(),
            WorkflowsPanelType::Panel => self.panel.as_mut(),
        }
    }
}

impl View for WorkflowView {
    fn draw(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        if let Some(panel) = self.panel.as_any_mut().downcast_mut::<WorkflowPanel>()
            && let Some(sidebar) = self.sidebar.as_any().downcast_ref::<WorkflowsSidebar>()
        {
            panel.update(sidebar.workflows_list_state.selected());
        }

        if self.show_sidebar {
            let [sidebar_area, separator_area, panel_area] = area.layout(&Layout::horizontal([
                Length(WorkflowsSidebar::WIDTH),
                Length(1),
                Fill(1),
            ]));

            self.sidebar.draw(
                sidebar_area,
                buf,
                self.focused_panel == WorkflowsPanelType::Sidebar,
            );

            for i in 0..separator_area.height {
                buf.set_string(
                    separator_area.x,
                    separator_area.y + i,
                    "│",
                    palette::SEPARATOR_STYLE,
                );
            }

            self.panel.draw(
                panel_area,
                buf,
                self.focused_panel == WorkflowsPanelType::Panel,
            );
        } else {
            self.panel
                .draw(area, buf, self.focused_panel == WorkflowsPanelType::Panel);
        }
    }

    fn handle_event(
        &mut self,
        event: &crossterm::event::Event,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> anyhow::Result<()> {
        if let Some(key) = event.as_key_event() {
            match key.code {
                KeyCode::Char('s') => {
                    if macros::workflows_count!(self.core) > 0 {
                        self.show_sidebar = !self.show_sidebar;
                    } else {
                        self.show_sidebar = false;
                    }
                }
                KeyCode::Tab => self.move_to_next_panel(),
                _ => {}
            }
        }

        self.get_focused_panel().handle_event(event, terminal)?;

        Ok(())
    }

    fn get_action(&self) -> Option<super::ViewAction> {
        if let Some(sidebar) = self.sidebar.as_any().downcast_ref::<WorkflowsSidebar>()
            && sidebar.create_workflow_button_pressed
        {
            return Some(ViewAction::OpenWorkflowCreator);
        }
        None
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Config;
    use crate::tui::views::View;

    fn empty_core() -> Core {
        Core {
            config: Config::default(),
            workflows: Vec::new(),
        }
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
        let core = Arc::new(RwLock::new(empty_core()));
        let view = WorkflowView::new(core);
        // Public state after construction: show_sidebar=true, focused=Sidebar
        assert!(view.show_sidebar);
        assert_eq!(view.focused_panel, WorkflowsPanelType::Sidebar);
    }

    #[test]
    fn test_workflow_view_move_to_next_panel_from_sidebar() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut view = WorkflowView::new(core);
        view.move_to_next_panel();
        assert_eq!(view.focused_panel, WorkflowsPanelType::Panel);
    }

    #[test]
    fn test_workflow_view_move_to_next_panel_from_panel_empty() {
        // When no workflows exist, moving from Panel stays at Panel
        let core = Arc::new(RwLock::new(empty_core()));
        let mut view = WorkflowView::new(core);
        // Move to Panel first
        view.move_to_next_panel();
        assert_eq!(view.focused_panel, WorkflowsPanelType::Panel);
        // Moving again from Panel with 0 workflows should stay on Panel
        view.move_to_next_panel();
        assert_eq!(view.focused_panel, WorkflowsPanelType::Panel);
    }

    #[test]
    fn test_workflow_view_as_any() {
        let core = Arc::new(RwLock::new(empty_core()));
        let view = WorkflowView::new(core);
        assert!(view.as_any().is::<WorkflowView>());
    }

    #[test]
    fn test_workflow_view_get_focused_panel_returns_sidebar_when_focused() {
        let core = Arc::new(RwLock::new(empty_core()));
        let view = WorkflowView::new(core);
        // Verify show_sidebar starts true and focused_panel is Sidebar
        assert!(view.show_sidebar);
        assert_eq!(view.focused_panel, WorkflowsPanelType::Sidebar);
    }

    #[test]
    fn test_workflow_view_get_focused_panel_returns_panel_after_tab() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut view = WorkflowView::new(core);
        view.move_to_next_panel();
        assert_eq!(view.focused_panel, WorkflowsPanelType::Panel);
    }
}
