use crossterm::event::KeyCode;
use ratatui::{
    buffer::Buffer,
    layout::{
        Constraint::{self, Fill, Length, Ratio},
        Layout, Rect,
    },
    text::ToLine,
    widgets::{ListState, Paragraph, Widget as _},
};
use std::sync::{Arc, RwLock};

use super::View;
use super::common::button::{Button, ButtonData};
use super::common::error_alert::ErrorAlert;
use super::common::macros;
use crate::core::Core;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoWorkflowsChoice {
    Pending,
    ChooseWorkflowsDirectory,
    CreateNewWorkflow,
    QuitApp,
}

type ThisButtonData = ButtonData<NoWorkflowsChoice>;

pub(crate) struct NoWorkflowsView {
    core: Arc<RwLock<Core>>,
    /// Used for navigating over option buttons
    list_state: ListState,
    buttons: [ThisButtonData; 3],
    pub(crate) pending_action: NoWorkflowsChoice,
}

impl NoWorkflowsView {
    pub(crate) fn new(core: Arc<RwLock<Core>>) -> Self {
        Self {
            core,
            list_state: ListState::default(),
            buttons: [
                ThisButtonData::new(
                    "Choose another directory",
                    NoWorkflowsChoice::ChooseWorkflowsDirectory,
                ),
                ThisButtonData::new("Create new workflow", NoWorkflowsChoice::CreateNewWorkflow),
                ThisButtonData::new("Quit app", NoWorkflowsChoice::QuitApp),
            ],
            pending_action: NoWorkflowsChoice::Pending,
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer, directory_path: String) {
        let [_, info_area, buttons_area, _] = area.layout(&Layout::vertical([
            Fill(1),
            Length(2),
            Length(Button::HEIGHT),
            Fill(1),
        ]));

        let text = format!(
            "No workflows files were found in the given directory: \"{}\"\nSelect another directory or create a new workflow",
            directory_path
        );
        Paragraph::new(text).centered().render(info_area, buf);

        let button_areas: [Rect; 3] = buttons_area
            .centered_horizontally(Constraint::Max(92))
            .layout(&Layout::horizontal([Ratio(1, 3); 3]));

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

    fn run_button_action(&mut self, selected: usize) {
        assert!(
            selected < self.buttons.len(),
            "selected >= self.buttons.len()"
        );

        self.pending_action = self.buttons[selected].action;
    }
}

impl View for NoWorkflowsView {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let workflows_directory = self
            .core
            .read()
            .map(|core| core.workflows_directory.display().to_string());

        match workflows_directory {
            Ok(directory_path) => {
                self.render(area, buf, directory_path);
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
    ) -> anyhow::Result<()> {
        if let Some(key) = event.as_key_event() {
            match key.code {
                KeyCode::Left | KeyCode::Up | KeyCode::Char('h') => {
                    macros::move_selection!(self.list_state, Previous)
                }
                KeyCode::Right | KeyCode::Down | KeyCode::Char('l') => {
                    macros::move_selection!(self.list_state, Next, self.buttons.len() - 1)
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if let Some(selected) = self.list_state.selected() {
                        self.run_button_action(selected)
                    }
                }
                _ => {}
            }
        }

        log::info!("Selected: {:?}", self.list_state.selected());

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
