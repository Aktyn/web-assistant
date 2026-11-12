use crossterm::event::KeyCode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Paragraph, Widget as _},
};

use super::common::palette;
use super::{View, ViewAction};
// use crate::core::Core;

pub(crate) struct WorkflowCreatorView {
    // core: Arc<RwLock<Core>>,
    pending_action: Option<ViewAction>,
}

impl WorkflowCreatorView {
    pub(crate) fn new() -> Self {
        Self {
            // core: Arc::new(RwLock::new(Core::default())),
            pending_action: None,
        }
    }
}

impl View for WorkflowCreatorView {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Press backspace to exit creator")
            .style(palette::STYLE)
            .centered()
            .render(area, buf);

        //TODO: develop this view
    }

    fn handle_event(
        &mut self,
        event: &crossterm::event::Event,
        _terminal: &mut ratatui::DefaultTerminal,
    ) -> anyhow::Result<()> {
        if let Some(key) = event.as_key_event() {
            // match key.code {
            //     KeyCode::Backspace => {
            //         self.pending_action = Some(ViewAction::ExitWorkflowCreator);
            //     }
            //     _ => {}
            // }
            if key.code == KeyCode::Backspace {
                self.pending_action = Some(ViewAction::ExitWorkflowCreator);
            }
        }

        Ok(())
    }

    fn get_action(&self) -> Option<ViewAction> {
        self.pending_action
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
