use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Stylize as _,
    text::{Line, Text},
    widgets::{ListState, Paragraph, Widget},
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::sync::oneshot;

use crate::common::widgets::{Button, ButtonData, ErrorAlert};
use crate::common::{palette, utils};
use crate::views::{View, ViewAction};
use config::Config;

type ThisButtonData = ButtonData<ViewAction>;

/// Outcome of the asynchronous "initialize default config" task
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InitConfigOutcome {
    /// The default config file was created at the given path
    Created(PathBuf),
    /// The task failed with the given error message
    Failed(String),
}

pub(crate) struct ConfigView {
    config: Arc<RwLock<Config>>,
    /// Output path for the "initialize default config" task.
    /// If `None`, the file is created in the current working directory.
    init_path: Option<PathBuf>,
    /// Used for navigating over option buttons
    list_state: ListState,
    buttons: [ThisButtonData; 3],
    /// Whether the config details panel is visible
    show_details: bool,
    /// Channel receiving the outcome of the running init task
    init_task: Option<oneshot::Receiver<InitConfigOutcome>>,
    /// Outcome of the last completed init task
    init_outcome: Option<InitConfigOutcome>,
    pending_action: Option<ViewAction>,
}

impl ConfigView {
    pub(crate) fn new(config: Arc<RwLock<Config>>) -> Self {
        Self::with_init_path(config, None)
    }

    /// Creates a view whose init task writes the default config to `init_path`
    /// instead of the current working directory.
    pub(crate) fn with_init_path(config: Arc<RwLock<Config>>, init_path: Option<PathBuf>) -> Self {
        Self {
            config,
            init_path,
            list_state: ListState::default().with_selected(Some(0)),
            buttons: [
                ThisButtonData::new("Show config details", ViewAction::GoToWorkflows),
                ThisButtonData::new("Initialize default config", ViewAction::GoToWorkflows),
                ThisButtonData::new("Return to workflows", ViewAction::GoToWorkflows),
            ],
            show_details: true,
            init_task: None,
            init_outcome: None,
            pending_action: None,
        }
    }

    /// Label for the toggle button depends on the current visibility state
    fn button_label(&self, index: usize) -> &str {
        if index == 0 {
            if self.show_details {
                "Hide config details"
            } else {
                "Show config details"
            }
        } else {
            &self.buttons[index].label
        }
    }

    fn start_init_task(&mut self) {
        if self.init_task.is_some() {
            return;
        }
        let init_path = self.init_path.clone();
        self.init_outcome = None;
        let (tx, rx) = oneshot::channel();
        self.init_task = Some(rx);
        tokio::spawn(async move {
            let outcome = run_init_task(init_path.as_deref()).await;
            let _ = tx.send(outcome);
        });
    }

    /// Polls the running init task, storing its outcome if it has finished.
    /// This is safe to call repeatedly; it never blocks.
    fn poll_init_task(&mut self) {
        let Some(receiver) = self.init_task.as_mut() else {
            return;
        };
        match receiver.try_recv() {
            Ok(outcome) => {
                self.init_task = None;
                self.init_outcome = Some(outcome);
            }
            Err(oneshot::error::TryRecvError::Empty) => {}
            Err(oneshot::error::TryRecvError::Closed) => {
                log::error!("Config init task was dropped without sending an outcome");
                self.init_task = None;
                self.init_outcome = Some(InitConfigOutcome::Failed(
                    "Config init task was dropped without sending an outcome".to_string(),
                ));
            }
        }
    }

    fn render(&self, config: &Config, area: Rect, buf: &mut Buffer) {
        let [_, info_area, details_area, status_area, buttons_area, _] =
            area.layout(&Layout::vertical([
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Length(8),
                Constraint::Length(1),
                Constraint::Length(Button::HEIGHT),
                Constraint::Fill(1),
            ]));

        // Header
        Paragraph::new("Config Management")
            .style(palette().style.bold())
            .centered()
            .render(info_area, buf);

        // Config details
        if self.show_details {
            let details = Text::from(vec![
                Line::from(format!(
                    "Workflows directory: {}",
                    config.workflows_directory.display()
                )),
                Line::from(format!(
                    "TUI colors: {}",
                    if config.tui_colors.is_some() {
                        "custom"
                    } else {
                        "default"
                    }
                )),
                Line::from(format!(
                    "Browser headless mode: {}",
                    config.browser.headless
                )),
                Line::from(format!(
                    "Agent model: {}",
                    if config.agent.model.is_empty() {
                        "default"
                    } else {
                        &config.agent.model
                    }
                )),
            ]);
            Paragraph::new(details)
                .style(palette().style)
                .render(details_area, buf);
        }

        // Outcome of the init task
        if let Some(outcome) = &self.init_outcome {
            let is_error = matches!(outcome, InitConfigOutcome::Failed(_));
            let text = match outcome {
                InitConfigOutcome::Created(path) => format!(
                    "The default config was successfully saved to: {}",
                    path.display()
                ),
                InitConfigOutcome::Failed(message) => {
                    format!("Failed to initialize config: {message}")
                }
            };
            Paragraph::new(text)
                .style(if is_error {
                    palette().error_foreground
                } else {
                    palette().accent
                })
                .bold()
                .centered()
                .render(status_area, buf);
        }

        // Buttons
        let button_areas: [Rect; 3] = buttons_area
            .centered_horizontally(Constraint::Max(84))
            .layout(&Layout::horizontal([
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
            ]));

        assert!(
            button_areas.len() == self.buttons.len(),
            "button_areas.len() != self.buttons.len()"
        );

        self.buttons.iter().enumerate().for_each(|(index, _)| {
            let button = Button::new(self.button_label(index));
            let button = if index == 1 && self.init_task.is_some() {
                button.set_disabled(true)
            } else {
                button
            };
            button
                .set_selected(self.list_state.selected() == Some(index))
                .render(button_areas[index], buf);
        });
    }

    pub(crate) fn run_button_action(&mut self, selected: usize) {
        assert!(
            selected < self.buttons.len(),
            "selected >= self.buttons.len()"
        );

        match selected {
            0 => self.show_details = !self.show_details,
            1 => self.start_init_task(),
            2 => self.pending_action = Some(ViewAction::GoToWorkflows),
            _ => unreachable!(),
        }
    }
}

/// Runs the async default config creation task, mapping the result to `InitConfigOutcome`
async fn run_init_task(init_path: Option<&Path>) -> InitConfigOutcome {
    match config::create_default_config(init_path).await {
        Ok(path) => InitConfigOutcome::Created(path.to_path_buf()),
        Err(error) => InitConfigOutcome::Failed(error.to_string()),
    }
}

impl View for ConfigView {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, palette().style);
        self.poll_init_task();

        match self.config.read() {
            Ok(config) => self.render(&config, area, buf),
            Err(error) => {
                let message = error.to_string();
                ErrorAlert::new(message.as_str()).render(area, buf);
            }
        }
    }

    fn handle_event(
        &mut self,
        event: &crossterm::event::Event,
        _terminal: &mut DefaultTerminal,
    ) -> bool {
        if let Some(key) = event.as_key_event() {
            let navigation =
                utils::handle_navigation_event(key, &mut self.list_state, self.buttons.len() - 1);
            if let Some(event) = navigation
                && let utils::NavigationEvent::Select(selected) = event
            {
                self.run_button_action(selected);
            }

            // 'q' or ESC returns to the workflows view instead of quitting the app
            if matches!(
                key.code,
                crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Esc
            ) {
                self.pending_action = Some(ViewAction::GoToWorkflows);
                return true;
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
    use crossterm::event::Event;
    use std::path::PathBuf;

    fn empty_config() -> Config {
        Config::default()
    }

    fn config_arc() -> Arc<RwLock<Config>> {
        Arc::new(RwLock::new(empty_config()))
    }

    /// Helper to build a `&mut DefaultTerminal` that is never dereferenced.
    fn no_read_term() -> *mut DefaultTerminal {
        use std::mem::MaybeUninit;
        static SLOT: std::sync::OnceLock<MaybeUninit<DefaultTerminal>> = std::sync::OnceLock::new();
        let slot = SLOT.get_or_init(MaybeUninit::uninit);
        slot.as_ptr().cast_mut()
    }

    /// Polls the running init task until it finishes, with a bounded number of attempts
    async fn await_init_task(view: &mut ConfigView, attempts: usize) {
        for _ in 0..attempts {
            view.poll_init_task();
            if view.init_task.is_none() {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        panic!("init task did not finish within {attempts} attempts");
    }

    #[test]
    fn test_config_view_new_defaults() {
        let view = ConfigView::new(config_arc());
        assert!(view.show_details);
        assert!(view.list_state.selected() == Some(0));
        assert!(view.init_task.is_none());
        assert!(view.init_outcome.is_none());
        assert_eq!(view.pending_action, None);
        assert_eq!(view.init_path, None);
    }

    #[test]
    fn test_button_labels_toggle() {
        let mut view = ConfigView::new(config_arc());
        assert_eq!(view.button_label(0), "Hide config details");
        view.run_button_action(0);
        assert!(!view.show_details);
        assert_eq!(view.button_label(0), "Show config details");
        assert_eq!(view.button_label(1), "Initialize default config");
        assert_eq!(view.button_label(2), "Return to workflows");
    }

    #[test]
    fn test_run_button_action_return_sets_go_to_workflows() {
        let mut view = ConfigView::new(config_arc());
        view.run_button_action(2);
        assert_eq!(view.get_action(), Some(ViewAction::GoToWorkflows));
    }

    #[test]
    #[should_panic(expected = "selected >= self.buttons.len()")]
    fn test_run_button_action_out_of_bounds_panics() {
        let mut view = ConfigView::new(config_arc());
        view.run_button_action(3);
    }

    #[test]
    fn test_draw_no_panic() {
        let mut view = ConfigView::new(config_arc());
        let area = Rect::new(0, 0, 96, 32);
        let mut buf = Buffer::empty(area);
        view.draw(area, &mut buf);
    }

    #[test]
    fn test_handle_event_q_returns_to_workflows_and_blocks_propagation() {
        let mut view = ConfigView::new(config_arc());
        let event = Event::Key(crossterm::event::KeyCode::Char('q').into());
        let handled = view.handle_event(&event, unsafe { &mut *no_read_term() });
        assert!(handled);
        assert_eq!(view.get_action(), Some(ViewAction::GoToWorkflows));
    }

    #[test]
    fn test_handle_event_esc_returns_to_workflows_and_blocks_propagation() {
        let mut view = ConfigView::new(config_arc());
        let event = Event::Key(crossterm::event::KeyCode::Esc.into());
        let handled = view.handle_event(&event, unsafe { &mut *no_read_term() });
        assert!(handled);
        assert_eq!(view.get_action(), Some(ViewAction::GoToWorkflows));
    }

    #[tokio::test]
    async fn test_enter_on_init_button_spawns_task() {
        let tmp = tempfile::tempdir().unwrap();
        let mut view =
            ConfigView::with_init_path(config_arc(), Some(tmp.path().join("config.jsonc")));
        view.list_state.select(Some(1));
        let event = Event::Key(crossterm::event::KeyCode::Enter.into());
        let handled = view.handle_event(&event, unsafe { &mut *no_read_term() });
        assert!(!handled);
        assert!(view.init_task.is_some());
    }

    #[tokio::test]
    async fn test_init_task_creates_config_file() {
        let tmp = tempfile::tempdir().unwrap();
        let output_path = tmp.path().join("config.jsonc");
        let mut view = ConfigView::with_init_path(config_arc(), Some(output_path.clone()));

        view.run_button_action(1);
        await_init_task(&mut view, 200).await;

        assert_eq!(
            view.init_outcome,
            Some(InitConfigOutcome::Created(output_path.clone()))
        );
        assert!(output_path.is_file());
    }

    #[tokio::test]
    async fn test_init_task_fails_when_file_already_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let output_path = tmp.path().join("existing.jsonc");
        std::fs::write(&output_path, "{}").unwrap();
        let mut view = ConfigView::with_init_path(config_arc(), Some(output_path));

        view.run_button_action(1);
        await_init_task(&mut view, 200).await;

        match &view.init_outcome {
            Some(InitConfigOutcome::Failed(message)) => {
                assert!(
                    message.contains("already exists"),
                    "unexpected error message: {message}"
                );
            }
            other => panic!("expected Failed outcome, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_start_init_task_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let mut view =
            ConfigView::with_init_path(config_arc(), Some(tmp.path().join("config.jsonc")));

        view.run_button_action(1);
        assert!(view.init_task.is_some());
        view.run_button_action(1); // no-op while running
        await_init_task(&mut view, 200).await;
    }

    #[test]
    fn test_init_outcome_created_path() {
        let outcome = InitConfigOutcome::Created(PathBuf::from("./config.jsonc"));
        assert_eq!(
            outcome,
            InitConfigOutcome::Created(PathBuf::from("./config.jsonc"))
        );
    }
}
