pub(super) mod config_view;
pub(super) mod empty;
pub(super) mod no_workflows;
pub(super) mod terminal_size_error;
pub(super) mod workflows;

use crossterm::event::Event;
use ratatui::{DefaultTerminal, buffer::Buffer, layout::Rect};
use std::any::Any;

use super::common;

/// A way for view to register global action that will be run by `TuiApp`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ViewAction {
    QuitApp,
    CreateWorkflow,
    OpenConfig,
    GoToWorkflows,
}

pub(super) trait View {
    fn draw(&mut self, area: Rect, buf: &mut Buffer);
    /// Should return true if event was handled internally and should not be propagated further
    fn handle_event(&mut self, event: &Event, terminal: &mut DefaultTerminal) -> bool;
    fn get_action(&self) -> Option<ViewAction>;
    /// Called after the action returned by `get_action` has been processed
    fn consume_action(&mut self) {}
    /// Called on every iteration of the main loop, even when there were no events
    fn tick(&mut self) {}

    fn as_any(&self) -> &dyn Any;
}
