mod common;
mod views;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint::{Fill, Length};
use ratatui::layout::{Layout, Rect};
use ratatui::widgets::{Block, Borders, Widget};
use ratatui::{DefaultTerminal, Frame};
use std::sync::{Arc, RwLock};
use tui_logger::TuiLoggerWidget;

use crate::core::Core;
use common::{consts, palette};
use views::empty::EmptyView;
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

        self.view = Box::new(WorkflowView::new(self.core.clone()));

        self.clear(terminal)?;

        while self.is_running {
            terminal
                .draw(|frame| self.draw(frame))
                .context("failed to draw frame")?;
            self.handle_events(terminal)?;
        }
        Ok(())
    }

    #[cfg(debug_assertions)]
    fn clear(&self, terminal: &mut DefaultTerminal) -> Result<()> {
        terminal
            .draw(|frame| {
                let buffer = frame.buffer_mut();
                buffer.set_style(*buffer.area(), palette::FOREGROUND);
            })
            .context("failed to clear frame")?;
        Ok(())
    }

    fn handle_events(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let event = event::read()?;

        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.exit(),
                KeyCode::Char('l') => {
                    self.show_logs = !self.show_logs;
                }
                _ => {}
            },
            Event::Resize(_width, _height) => {
                terminal.autoresize()?;
            }
            _ => {}
        }

        self.view.handle_event(terminal, &event)?;

        Ok(())
    }

    const fn exit(&mut self) {
        self.is_running = false;
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let buf = frame.buffer_mut();

        let main_area = if self.show_logs && area.height > consts::LOGS_PANEL_HEIGHT_REQUIREMENT {
            let [main_area, log_area] = area.layout(&Layout::vertical([Fill(1), Length(12)]));
            self.render_logs(log_area, buf);

            main_area
        } else {
            area
        };

        self.view.draw(main_area, buf);
    }

    fn render_logs(&self, area: Rect, buf: &mut Buffer) {
        let title = "Logs (press L to toggle)";
        let log_widget = TuiLoggerWidget::default()
            .output_timestamp(None)
            // .output_level(None)
            .output_file(false)
            .output_line(false)
            .output_target(false)
            .block(
                Block::default()
                    .title(format!(
                        "{} {} ",
                        "—".repeat(((area.width as usize) - title.len() - 2) / 2),
                        title
                    ))
                    .borders(Borders::TOP)
                    .border_style(palette::SEPARATOR_STYLE),
            )
            .style(palette::STYLE);
        log_widget.render(area, buf);
    }
}
