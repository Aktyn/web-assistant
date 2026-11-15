use ratatui::{
    buffer::Buffer,
    layout::{
        Constraint::{self, Fill, Length, Ratio},
        Layout, Rect,
    },
    style::Stylize,
    text::{Line, Span, Text, ToLine, ToSpan},
    widgets::{ListState, Paragraph, Widget as _},
};
use std::sync::{Arc, RwLock};

use super::common::widgets::{Button, ButtonData, ErrorAlert};
use super::common::{palette, utils};
use super::{View, ViewAction};
use crate::config::Config;

type ThisButtonData = ButtonData<ViewAction>;

pub(crate) struct NoWorkflowsView {
    config: Arc<RwLock<Config>>,
    /// Used for navigating over option buttons
    list_state: ListState,
    buttons: [ThisButtonData; 2],
    pending_action: Option<ViewAction>,
}

impl NoWorkflowsView {
    pub(crate) fn new(config: Arc<RwLock<Config>>) -> Self {
        Self {
            config,
            list_state: ListState::default(),
            buttons: [
                ThisButtonData::new("Create new workflow", ViewAction::CreateWorkflow),
                ThisButtonData::new("Quit app", ViewAction::QuitApp),
            ],
            pending_action: None,
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer, directory_path: &str) {
        let [_, info_area, buttons_area, _] = area.layout(&Layout::vertical([
            Fill(1),
            Length(2),
            Length(Button::HEIGHT),
            Fill(1),
        ]));

        let text = Text::from(vec![
            Line::from(vec![
                "Update ".into(),
                "workflowsDirectory"
                    .to_span()
                    .style(palette().accent)
                    .bold(),
                " in your config file or create a new workflow".into(),
            ]),
            Line::from(vec![
                "No workflow files were found in the given directory: \"".into(),
                Span::from(directory_path).style(palette().accent),
                "\"".into(),
            ]),
        ])
        .centered();
        Paragraph::new(text).centered().render(info_area, buf);

        let button_areas: [Rect; 2] = buttons_area
            .centered_horizontally(Constraint::Max(64))
            .layout(&Layout::horizontal([Ratio(1, 2); 2]));

        assert!(
            button_areas.len() == self.buttons.len(),
            "button_areas.len() != self.buttons.len()"
        );

        self.buttons.iter().enumerate().for_each(|(index, button)| {
            Button::new(button.label.to_line())
                .set_selected(self.list_state.selected() == Some(index))
                .render(button_areas[index], buf);
        });
    }

    pub(crate) fn run_button_action(&mut self, selected: usize) {
        assert!(
            selected < self.buttons.len(),
            "selected >= self.buttons.len()"
        );

        self.pending_action = Some(self.buttons[selected].action);
    }
}

impl View for NoWorkflowsView {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let workflows_directory = self
            .config
            .read()
            .map(|config| config.workflowsDirectory.display().to_string());

        match workflows_directory {
            Ok(directory_path) => {
                self.render(area, buf, &directory_path);
            }
            Err(error) => {
                ErrorAlert::new(error.to_line()).render(area, buf);
            }
        }
    }

    fn handle_event(
        &mut self,
        event: &crossterm::event::Event,
        _terminal: &mut ratatui::DefaultTerminal,
    ) -> bool {
        if let Some(key) = event.as_key_event() {
            let navigation =
                utils::handle_navigation_event(key, &mut self.list_state, self.buttons.len() - 1);
            if let Some(event) = navigation
                && let utils::NavigationEvent::Select(selected) = event
            {
                self.run_button_action(selected);
            }
        }

        false
    }

    fn get_action(&self) -> Option<super::ViewAction> {
        self.pending_action
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::views::View;

fn empty_config() -> Config {
        Config::default()
    }

    #[test]
    fn test_no_workflows_view_new_pending_action() {
        let config = Arc::new(RwLock::new(empty_config()));
        let view = NoWorkflowsView::new(config);
        assert_eq!(view.pending_action, None);
        assert!(view.list_state.selected().is_none());
    }

    #[test]
    fn test_run_button_action_sets_correct_choice() {
        let config = Arc::new(RwLock::new(empty_config()));
        let mut view = NoWorkflowsView::new(config);
        view.run_button_action(0);
        assert_eq!(view.pending_action, Some(ViewAction::CreateWorkflow));
        view.run_button_action(1);
        assert_eq!(view.pending_action, Some(ViewAction::QuitApp));
    }

    #[test]
    #[should_panic(expected = "selected >= self.buttons.len()")]
    fn test_run_button_action_out_of_bounds_panics() {
        let config = Arc::new(RwLock::new(empty_config()));
        let mut view = NoWorkflowsView::new(config);
        view.run_button_action(5); // invalid index
    }

    #[test]
    fn test_button_data_for_no_workflows() {
        // Verify ButtonData construction works with ViewAction
        let data = ButtonData::new("label", ViewAction::QuitApp);
        assert_eq!(data.label, "label");
        assert_eq!(data.action, ViewAction::QuitApp);
    }

    /// Render test with valid area - checks no panics during draw
    #[test]
    fn test_no_workflows_view_draw_with_valid_config() {
        let config = Arc::new(RwLock::new(Config::default()));
        let mut view = NoWorkflowsView::new(config);
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        view.draw(area, &mut buf);
        // If we get here without panicking, rendering succeeded.
    }

    #[test]
    fn test_no_workflows_view_as_any() {
        let config = Arc::new(RwLock::new(Config::default()));
        let view = NoWorkflowsView::new(config);
        assert!(view.as_any().is::<NoWorkflowsView>());
    }
}
