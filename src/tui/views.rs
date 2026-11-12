pub(super) mod empty;
pub(super) mod no_workflows;
pub(super) mod terminal_size_error;
pub(super) mod workflows;

use crossterm::event::Event;
use ratatui::{DefaultTerminal, buffer::Buffer, layout::Rect};
use std::any::Any;

use super::common;

// /// A way for view to register global action that will be run by TuiApp
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub(crate) enum ViewAction {
//     QuitApp,
//     OpenWorkflowCreator,
// }

pub(super) trait View {
    fn draw(&mut self, area: Rect, buf: &mut Buffer);
    fn handle_event(&mut self, event: &Event, terminal: &mut DefaultTerminal)
    -> anyhow::Result<()>;
    // fn get_action() -> Option<ViewAction>;

    fn as_any(&self) -> &dyn Any;
}
