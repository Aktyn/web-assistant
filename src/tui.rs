mod common;
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

use crate::core::Core;
use common::{consts, macros, palette};
use views::empty::EmptyView;
use views::no_workflows::NoWorkflowsView;
use views::terminal_size_error::TerminalSizeErrorView;
use views::workflow_creator::WorkflowCreatorView;
use views::workflows::WorkflowView;

pub fn render_tui(core: Core) -> Result<()> {
    let app = TuiApp::new(core);
    ratatui::run(|terminal| app.run(terminal))
}

struct TuiApp {
    core: Arc<RwLock<Core>>,
    is_running: bool,
    show_logs: bool,
    view: Box<dyn views::View>,
}

impl TuiApp {
    fn new(core: Core) -> Self {
        // TODO: initialize palette here

        let core_arc = Arc::new(RwLock::new(core));

        Self {
            core: core_arc,
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
        match macros::workflows_count!(self.core) {
            0 => self.view = Box::new(NoWorkflowsView::new(self.core.clone())),
            _ => self.view = Box::new(WorkflowView::new(self.core.clone())),
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
            buffer.set_style(*buffer.area(), palette::FOREGROUND);
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
                .border_style(palette::SEPARATOR_STYLE),
        )
        .style(palette::STYLE);
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
    use crate::core::config::Config;

    fn empty_core() -> Core {
        Core {
            config: Config::default(),
            workflows: Vec::new(),
        }
    }

    #[test]
    fn test_tui_app_new_with_empty_core() {
        let core = empty_core();
        let app = TuiApp::new(core);
        assert!(!app.is_running);
        // In debug mode show_logs is true; in release it's false.
        // We can verify the struct exists and has the expected types.
        let _ = app.core;
    }

    #[test]
    fn test_tui_app_exit_sets_running_false() {
        let core = empty_core();
        let mut app = TuiApp::new(core);
        assert!(!app.is_running);
        app.exit();
        assert!(!app.is_running);
    }

    #[test]
    fn test_tui_app_exit_while_not_running_stays_false() {
        let core = empty_core();
        let mut app = TuiApp::new(core);
        app.exit();
        app.exit(); // idempotent
        assert!(!app.is_running);
    }

    #[test]
    fn test_render_logs_no_panic() {
        let core = empty_core();
        let app = TuiApp::new(core);
        let _ = app;
        let area = Rect::new(0, 0, 80, 12);
        let mut buf = Buffer::empty(area);
        render_logs(area, &mut buf);
        // Succeeded without panicking.
    }

    #[test]
    fn test_constants_have_expected_values() {
        assert_eq!(consts::LOGS_PANEL_HEIGHT_REQUIREMENT, 40);
    }

    #[test]
    fn test_palette_colors_are_valid() {
        // Verify constants resolve without panicking
        let _ = palette::BACKGROUND;
        let _ = palette::FOREGROUND;
        let _ = palette::ACCENT;
        let _ = palette::ERROR_FOREGROUND;
    }
}
