pub(crate) mod common;
pub(crate) mod inline;
mod views;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint::{Fill, Length};
use ratatui::layout::{Layout, Rect, Size};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Widget};
use ratatui::{DefaultTerminal, Frame};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tui_logger::TuiLoggerWidget;

use crate::config::Config;
use crate::workflow::{self, Workflow};
use common::{consts, init_palette, macros, palette};
use views::empty::EmptyView;
use views::no_workflows::NoWorkflowsView;
use views::terminal_size_error::TerminalSizeErrorView;
use views::workflow_creator::WorkflowCreatorView;
use views::workflows::WorkflowView;

/// Initializes the TUI and runs it
/// Note that asynchronization occurs only during the app initialization
pub async fn render_tui(config: Config) -> Result<()> {
    let app = TuiApp::new(config).await;
    ratatui::run(|terminal| app.run(terminal))
}

struct TuiApp {
    config: Arc<RwLock<Config>>,
    workflows: Arc<RwLock<Vec<Workflow>>>,
    is_running: bool,
    show_logs: bool,
    view: Box<dyn views::View>,
}

impl TuiApp {
    async fn new(config: Config) -> Self {
        if let Some(colors) = &config.tuiColors {
            init_palette(colors);
        }

        let workflows = workflow::load_from_directory(&config.workflowsDirectory).await;

        for workflow in &workflows {
            log::info!("Workflow: {workflow:?}");
        }

        let workflows_arc = Arc::new(RwLock::new(workflows));
        let config_arc = Arc::new(RwLock::new(config));

        Self {
            config: config_arc,
            workflows: workflows_arc,
            is_running: false,
            show_logs: cfg!(debug_assertions),
            view: Box::new(EmptyView),
        }
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.is_running = true;

        #[cfg(debug_assertions)]
        clear_terminal(terminal)?;

        while self.is_running {
            self.update(terminal);
            terminal
                .draw(|frame| self.draw(frame))
                .context("failed to draw frame")?;
            self.handle_events(terminal)?;
            self.post_update();
        }
        Ok(())
    }

    fn handle_events(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        if smart_event_poll(terminal)? {
            terminal.autoresize()?;
            return Ok(());
        }

        let event = event::read()?;

        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.exit(),
                KeyCode::Char('L') => {
                    self.show_logs = !self.show_logs;
                }
                _ => {}
            },
            Event::Resize(_width, _height) => {
                log::info!("Resizing terminal");
                terminal.autoresize()?;
            }
            _ => {}
        }

        self.view.handle_event(&event, terminal)?;

        Ok(())
    }

    const fn exit(&mut self) {
        self.is_running = false;
    }

    /// Should run before pooling events
    fn update(&mut self, terminal: &mut DefaultTerminal) {
        let size = terminal.size().unwrap_or(Size::ZERO);
        if size.width < consts::MINIMUM_TERMINAL_SIZE.width
            || size.height < consts::MINIMUM_TERMINAL_SIZE.height
        {
            if !self.view.as_any().is::<TerminalSizeErrorView>() {
                self.view = Box::new(TerminalSizeErrorView::new());
            }
            return;
        }

        if self.view.as_any().is::<EmptyView>() || self.view.as_any().is::<TerminalSizeErrorView>()
        {
            self.go_to_starting_view();
        }
    }

    /// Should run after handling events
    /// Reacts to state changes in current view
    fn post_update(&mut self) {
        match self.view.get_action() {
            Some(views::ViewAction::QuitApp) => self.exit(),
            Some(views::ViewAction::OpenWorkflowCreator) => {
                self.view = Box::new(WorkflowCreatorView::new());
            }
            Some(views::ViewAction::ExitWorkflowCreator) => {
                self.go_to_starting_view();
            }
            None => (),
        }
    }

    fn go_to_starting_view(&mut self) {
        match macros::workflows_count!(&self.workflows) {
            0 => self.view = Box::new(NoWorkflowsView::new(self.config.clone())),
            _ => self.view = Box::new(WorkflowView::new(self.workflows.clone())),
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let buf = frame.buffer_mut();

        let main_area = if self.show_logs && area.height > consts::LOGS_PANEL_HEIGHT_REQUIREMENT {
            let [main_area, log_area] = area.layout(&Layout::vertical([Fill(1), Length(12)]));
            render_logs(log_area, buf);

            main_area
        } else {
            area
        };

        self.view.draw(main_area, buf);
    }
}

#[cfg(debug_assertions)]
fn clear_terminal(terminal: &mut DefaultTerminal) -> Result<()> {
    terminal
        .draw(|frame| {
            let buffer = frame.buffer_mut();
            buffer.set_style(*buffer.area(), palette().foreground);
        })
        .context("failed to clear frame")?;
    Ok(())
}

fn render_logs(area: Rect, buf: &mut Buffer) {
    let log_widget = TuiLoggerWidget::default()
        .output_timestamp(None)
        // .output_level(None)
        .output_file(false)
        .output_line(false)
        .output_target(false)
        .block(
            Block::default()
                .title(Line::from(" Logs (press L to toggle) ").centered())
                .borders(Borders::TOP)
                .border_style(palette().separator_style),
        )
        .style(palette().style);
    log_widget.render(area, buf);
}

/// Apart from waiting for the next event, it also checks if the terminal size has changed
fn smart_event_poll(terminal: &mut DefaultTerminal) -> Result<bool> {
    let start_size = terminal.size().unwrap_or(Size::ZERO);
    loop {
        if event::poll(Duration::from_secs(1))? {
            return Ok(false);
        }
        if start_size != terminal.size().unwrap_or(Size::ZERO) {
            return Ok(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::workflow::{Workflow, WorkflowDataJson};
    use crossterm::event::{Event, KeyCode};
    use std::path::PathBuf;
    use views::View;

    fn empty_config() -> Config {
        Config::default()
    }

    /// Returns a `&mut DefaultTerminal` that is never dereferenced by the
    /// views we exercise (they ignore the `_terminal` parameter).
    fn no_read_term() -> *mut DefaultTerminal {
        use std::mem::MaybeUninit;
        static SLOT: std::sync::OnceLock<MaybeUninit<DefaultTerminal>> = std::sync::OnceLock::new();
        let slot = SLOT.get_or_init(MaybeUninit::uninit);
        slot.as_ptr().cast_mut()
    }

    /// Helper to inspect the concrete type behind the `view` Box<dyn View>.
    fn view_is<VT: std::any::Any>(app: &TuiApp) -> bool {
        app.view.as_any().is::<VT>()
    }

    /// Build a `NoWorkflowsView` whose `pending_action` is set by calling
    /// `run_button_action` on the given button index.
    fn no_workflows_view_with_action(
        config: &Arc<RwLock<Config>>,
        button_index: usize,
    ) -> NoWorkflowsView {
        let mut view = NoWorkflowsView::new(config.clone());
        view.run_button_action(button_index);
        view
    }

    #[tokio::test]
    async fn test_tui_app_new_with_empty_config() {
        let config = empty_config();
        let app = TuiApp::new(config).await;
        assert!(!app.is_running);
        let _ = app.config;
    }

    #[tokio::test]
    async fn test_tui_app_exit_sets_running_false() {
        let config = empty_config();
        let mut app = TuiApp::new(config).await;
        assert!(!app.is_running);
        app.exit();
        assert!(!app.is_running);
    }

    #[tokio::test]
    async fn test_tui_app_exit_while_not_running_stays_false() {
        let config = empty_config();
        let mut app = TuiApp::new(config).await;
        app.exit();
        app.exit();
        assert!(!app.is_running);
    }

    #[test]
    fn test_render_logs_no_panic() {
        let area = Rect::new(0, 0, 80, 12);
        let mut buf = Buffer::empty(area);
        render_logs(area, &mut buf);
    }

    #[test]
    fn test_constants_have_expected_values() {
        assert_eq!(consts::LOGS_PANEL_HEIGHT_REQUIREMENT, 40);
    }

    #[test]
    fn test_palette_colors_are_valid() {
        let palette = palette();
        let _ = palette.background;
        let _ = palette.foreground;
        let _ = palette.accent;
        let _ = palette.error_foreground;
    }

    // ---- go_to_starting_view / post_update coverage ----

    #[tokio::test]
    async fn test_go_to_starting_view_with_empty_config_selects_no_workflows() {
        let config = empty_config();
        let mut app = TuiApp::new(config).await;
        assert!(view_is::<EmptyView>(&app));
        app.go_to_starting_view();
        assert!(view_is::<NoWorkflowsView>(&app));
    }

    #[tokio::test]
    async fn test_go_to_starting_view_with_workflows_selects_workflow_view() {
        let config = empty_config();
        let mut app = TuiApp::new(config).await;
        // Inject a workflow into the app's workflows list.
        let json = WorkflowDataJson::from_raw(
            PathBuf::from("/tmp/test-workflow.json"),
            r#"{"name":"Test Workflow"}"#.to_string(),
        );
        let workflow = Workflow::new(json);
        app.workflows = Arc::new(RwLock::new(vec![workflow]));
        app.go_to_starting_view();
        assert!(view_is::<WorkflowView>(&app));
    }

    #[tokio::test]
    async fn test_post_update_quit_action_exits_app() {
        let config = empty_config();
        let mut app = TuiApp::new(config).await;
        app.is_running = true;
        let no_wf = no_workflows_view_with_action(&app.config, 1); // index 1 → QuitApp
        app.view = Box::new(no_wf);
        app.post_update();
        assert!(!app.is_running);
    }

    #[tokio::test]
    async fn test_post_update_open_workflow_creator_action_swaps_view() {
        let config = empty_config();
        let mut app = TuiApp::new(config).await;
        let no_wf = no_workflows_view_with_action(&app.config, 0); // index 0 → OpenWorkflowCreator
        app.view = Box::new(no_wf);
        app.post_update();
        assert!(view_is::<WorkflowCreatorView>(&app));
    }

    #[tokio::test]
    async fn test_post_update_exit_workflow_creator_returns_to_starting_view() {
        let config = empty_config();
        let mut app = TuiApp::new(config).await;
        // Build a WorkflowCreatorView and press backspace to set ExitWorkflowCreator.
        let mut creator = WorkflowCreatorView::new();
        let event = Event::Key(KeyCode::Backspace.into());
        let _ = creator.handle_event(&event, unsafe { &mut *no_read_term() });
        app.view = Box::new(creator);
        app.post_update();
        // With an empty config, the starting view is NoWorkflowsView.
        assert!(view_is::<NoWorkflowsView>(&app));
    }

    #[tokio::test]
    async fn test_post_update_none_action_is_noop() {
        let config = empty_config();
        let mut app = TuiApp::new(config).await;
        app.is_running = true;
        app.view = Box::new(EmptyView);
        app.post_update();
        assert!(app.is_running);
    }
}
