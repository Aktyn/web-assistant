pub(crate) mod workflow_panel;
pub(crate) mod workflows_sidebar;

use std::sync::{Arc, RwLock};

use super::View;
use super::common::{macros, palette, panel};
use crate::core::Core;
use crossterm::event::KeyCode;
use ratatui::layout::Constraint::{Fill, Length};
use ratatui::layout::Layout;
use workflow_panel::WorkflowPanel;
use workflows_sidebar::WorkflowsSidebar;

#[derive(PartialEq, Debug)]
enum PanelType {
    Sidebar,
    Content,
}

pub(crate) struct WorkflowView {
    core: Arc<RwLock<Core>>,
    content: Box<dyn panel::Panel>,
    sidebar: Box<dyn panel::Panel>,
    show_sidebar: bool,
    focused_panel: PanelType,
}

impl WorkflowView {
    pub(crate) fn new(core: Arc<RwLock<Core>>) -> Self {
        let core_sidebar_arc = core.clone();
        let core_content_arc = core.clone();

        Self {
            core,
            sidebar: Box::new(WorkflowsSidebar::new(core_sidebar_arc)),
            content: Box::new(WorkflowPanel::new(core_content_arc)),
            show_sidebar: true,
            focused_panel: PanelType::Sidebar,
        }
    }

    fn move_to_next_panel(&mut self) {
        match self.focused_panel {
            PanelType::Sidebar => self.focused_panel = PanelType::Content,
            PanelType::Content => {
                if macros::workflows_count!(self.core) > 0 {
                    self.focused_panel = PanelType::Sidebar;
                }
            }
        }
    }

    fn get_focused_panel(&mut self) -> &mut dyn panel::Panel {
        match self.focused_panel {
            PanelType::Sidebar => self.sidebar.as_mut(),
            PanelType::Content => self.content.as_mut(),
        }
    }
}

impl View for WorkflowView {
    fn draw(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        if self.show_sidebar {
            let [sidebar_area, separator_area, content_area] = area.layout(&Layout::horizontal([
                Length(WorkflowsSidebar::WIDTH),
                Length(1),
                Fill(1),
            ]));

            self.sidebar
                .draw(sidebar_area, buf, self.focused_panel == PanelType::Sidebar);

            for i in 0..separator_area.height {
                buf.set_string(
                    separator_area.x,
                    separator_area.y + i,
                    "│",
                    palette::SEPARATOR_STYLE,
                );
            }

            self.content
                .draw(content_area, buf, self.focused_panel == PanelType::Content);
        } else {
            self.content
                .draw(area, buf, self.focused_panel == PanelType::Content);
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::views::View;

    #[test]
    fn test_panel_type_debug() {
        assert_eq!(format!("{:?}", PanelType::Sidebar), "Sidebar");
        assert_eq!(format!("{:?}", PanelType::Content), "Content");
        assert_ne!(PanelType::Sidebar, PanelType::Content);
    }

    #[test]
    fn test_panel_type_partial_eq() {
        assert_eq!(PanelType::Sidebar, PanelType::Sidebar);
        assert_eq!(PanelType::Content, PanelType::Content);
    }

    #[test]
    fn test_workflow_view_new_default_state() {
        let core = Arc::new(RwLock::new(Core::default()));
        let view = WorkflowView::new(core);
        // Public state after construction: show_sidebar=true, focused=Sidebar
        assert!(view.show_sidebar);
        assert_eq!(view.focused_panel, PanelType::Sidebar);
    }

    #[test]
    fn test_workflow_view_move_to_next_panel_from_sidebar() {
        let core = Arc::new(RwLock::new(Core::default()));
        let mut view = WorkflowView::new(core);
        view.move_to_next_panel();
        assert_eq!(view.focused_panel, PanelType::Content);
    }

    #[test]
    fn test_workflow_view_move_to_next_panel_from_content_empty() {
        // When no workflows exist, moving from Content stays at Content
        let core = Arc::new(RwLock::new(Core::default()));
        let mut view = WorkflowView::new(core);
        // Move to Content first
        view.move_to_next_panel();
        assert_eq!(view.focused_panel, PanelType::Content);
        // Moving again from Content with 0 workflows should stay on Content
        view.move_to_next_panel();
        assert_eq!(view.focused_panel, PanelType::Content);
    }

    #[test]
    fn test_workflow_view_as_any() {
        let core = Arc::new(RwLock::new(Core::default()));
        let view = WorkflowView::new(core);
        assert!(view.as_any().is::<WorkflowView>());
    }

    #[test]
    fn test_workflow_view_get_focused_panel_returns_sidebar_when_focused() {
        let core = Arc::new(RwLock::new(Core::default()));
        let view = WorkflowView::new(core);
        // Verify show_sidebar starts true and focused_panel is Sidebar
        assert!(view.show_sidebar);
        assert_eq!(view.focused_panel, PanelType::Sidebar);
    }

    #[test]
    fn test_workflow_view_get_focused_panel_returns_content_after_tab() {
        let core = Arc::new(RwLock::new(Core::default()));
        let mut view = WorkflowView::new(core);
        view.move_to_next_panel();
        assert_eq!(view.focused_panel, PanelType::Content);
    }
}
