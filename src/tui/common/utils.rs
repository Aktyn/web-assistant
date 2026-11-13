use super::macros;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::ListState;

pub(crate) enum NavigationEvent {
    #[allow(dead_code)]
    Previous(usize),
    #[allow(dead_code)]
    Next(usize),
    Select(usize),
}

pub(crate) fn handle_navigation_event(
    key: KeyEvent,
    list_state: &mut ListState,
    max_selection: usize,
) -> Option<NavigationEvent> {
    match key.code {
        KeyCode::Left | KeyCode::Up | KeyCode::Char('h') | KeyCode::BackTab => {
            let selected = macros::move_selection!(list_state, Previous);
            return Some(NavigationEvent::Previous(selected));
        }
        KeyCode::Right | KeyCode::Down | KeyCode::Char('l') | KeyCode::Tab => {
            let selected = macros::move_selection!(list_state, Next, max_selection);
            return Some(NavigationEvent::Next(selected));
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            if let Some(selected) = list_state.selected() {
                return Some(NavigationEvent::Select(selected));
            }
        }
        _ => {}
    }

    None
}
