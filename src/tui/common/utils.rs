use super::macros;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::ListState;

#[derive(Debug, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, crossterm::event::KeyModifiers::NONE)
    }

    #[test]
    fn test_up_returns_previous() {
        let mut list_state = ListState::default().with_selected(Some(2));
        let event = handle_navigation_event(key(KeyCode::Up), &mut list_state, 3);
        assert!(matches!(event, Some(NavigationEvent::Previous(1))));
    }

    #[test]
    fn test_left_returns_previous() {
        let mut list_state = ListState::default().with_selected(Some(3));
        let event = handle_navigation_event(key(KeyCode::Left), &mut list_state, 3);
        assert!(matches!(event, Some(NavigationEvent::Previous(2))));
    }

    #[test]
    fn test_h_returns_previous() {
        let mut list_state = ListState::default().with_selected(Some(3));
        let event = handle_navigation_event(key(KeyCode::Char('h')), &mut list_state, 3);
        assert!(matches!(event, Some(NavigationEvent::Previous(2))));
    }

    #[test]
    fn test_backtab_returns_previous() {
        let mut list_state = ListState::default().with_selected(Some(3));
        let event = handle_navigation_event(key(KeyCode::BackTab), &mut list_state, 3);
        assert!(matches!(event, Some(NavigationEvent::Previous(2))));
    }

    #[test]
    fn test_down_returns_next() {
        let mut list_state = ListState::default().with_selected(Some(1));
        let event = handle_navigation_event(key(KeyCode::Down), &mut list_state, 3);
        assert!(matches!(event, Some(NavigationEvent::Next(2))));
    }

    #[test]
    fn test_right_returns_next() {
        let mut list_state = ListState::default().with_selected(Some(1));
        let event = handle_navigation_event(key(KeyCode::Right), &mut list_state, 3);
        assert!(matches!(event, Some(NavigationEvent::Next(2))));
    }

    #[test]
    fn test_l_returns_next() {
        let mut list_state = ListState::default().with_selected(Some(1));
        let event = handle_navigation_event(key(KeyCode::Char('l')), &mut list_state, 3);
        assert!(matches!(event, Some(NavigationEvent::Next(2))));
    }

    #[test]
    fn test_tab_returns_next() {
        let mut list_state = ListState::default().with_selected(Some(1));
        let event = handle_navigation_event(key(KeyCode::Tab), &mut list_state, 3);
        assert!(matches!(event, Some(NavigationEvent::Next(2))));
    }

    #[test]
    fn test_next_clamps_to_max_selection() {
        let mut list_state = ListState::default().with_selected(Some(3));
        let event = handle_navigation_event(key(KeyCode::Down), &mut list_state, 3);
        // Already at max; Next still emitted but selection stays at 3.
        assert!(matches!(event, Some(NavigationEvent::Next(3))));
        assert_eq!(list_state.selected(), Some(3));
    }

    #[test]
    fn test_enter_returns_select() {
        let mut list_state = ListState::default().with_selected(Some(2));
        let event = handle_navigation_event(key(KeyCode::Enter), &mut list_state, 3);
        assert!(matches!(event, Some(NavigationEvent::Select(2))));
    }

    #[test]
    fn test_space_returns_select() {
        let mut list_state = ListState::default().with_selected(Some(0));
        let event = handle_navigation_event(key(KeyCode::Char(' ')), &mut list_state, 3);
        assert!(matches!(event, Some(NavigationEvent::Select(0))));
    }

    #[test]
    fn test_enter_without_selection_returns_none() {
        let mut list_state = ListState::default();
        let event = handle_navigation_event(key(KeyCode::Enter), &mut list_state, 3);
        assert!(event.is_none());
    }

    #[test]
    fn test_unknown_key_returns_none() {
        let mut list_state = ListState::default().with_selected(Some(1));
        let event = handle_navigation_event(key(KeyCode::Char('z')), &mut list_state, 3);
        assert!(event.is_none());
    }

    #[test]
    fn test_esc_returns_none() {
        let mut list_state = ListState::default().with_selected(Some(1));
        let event = handle_navigation_event(key(KeyCode::Esc), &mut list_state, 3);
        assert!(event.is_none());
    }

    #[test]
    fn test_navigation_event_variants_are_distinct() {
        // Smoke check that the three variants are distinct.
        let prev = NavigationEvent::Previous(0);
        let next = NavigationEvent::Next(0);
        let select = NavigationEvent::Select(0);
        assert!(!matches!(prev, NavigationEvent::Next(_)));
        assert!(!matches!(prev, NavigationEvent::Select(_)));
        assert!(!matches!(next, NavigationEvent::Select(_)));
        // Use all three to avoid dead-code warnings.
        assert_eq!(prev, NavigationEvent::Previous(0));
        assert_eq!(next, NavigationEvent::Next(0));
        assert_eq!(select, NavigationEvent::Select(0));
    }
}
