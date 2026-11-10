mod content;
mod palette;
mod sidebar;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode};
use ratatui::DefaultTerminal;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint::{Fill, Length};
use ratatui::layout::{Layout, Rect};
use ratatui::widgets::{Block, Borders, Widget};
use std::sync::{Arc, RwLock};
use tui_logger::TuiLoggerWidget;

use crate::core::Core;
use crate::tui::sidebar::Sidebar;

pub fn render_tui(core: Core) -> Result<()> {
    let app = App::new(core);
    ratatui::run(|terminal| app.run(terminal))
}

struct App {
    core: Arc<RwLock<Core>>,
    is_running: bool,
    show_logs: bool,
    sidebar: sidebar::Sidebar,
    content: content::Content,
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_layout(area, buf);
    }
}

impl App {
    fn new(core: Core) -> Self {
        let core_arc = Arc::new(RwLock::new(core));
        let core_sidebar_arc = Arc::clone(&core_arc);
        let core_content_arc = Arc::clone(&core_arc);

        Self {
            core: core_arc,
            is_running: false,
            show_logs: cfg!(debug_assertions),
            sidebar: sidebar::Sidebar::new(core_sidebar_arc),
            content: content::Content::new(core_content_arc),
        }
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.is_running = true;
        self.sidebar.show = self.core.read().map(|core| !core.workflows.is_empty()).unwrap_or(false);

        self.clear(terminal)?;

        while self.is_running {
            self.draw(terminal)?;
            self.handle_events(terminal)?;
        }
        Ok(())
    }

    fn draw(&self, terminal: &mut DefaultTerminal) -> Result<()> {
        terminal
            .draw(|frame| frame.render_widget(self, frame.area()))
            .context("failed to draw frame")?;
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
        match event::read()? {
            Event::Key(key) => {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => self.exit(),
                    KeyCode::Char('1') => self.sidebar.select_item(1),
                    KeyCode::Char('l') => {
                        self.show_logs = !self.show_logs;
                    }
                    KeyCode::Char('s') => {
                        self.sidebar.show = !self.sidebar.show;
                    }
                    // KeyCode::Char('k') | KeyCode::Up => self.increment_value(),
                    // KeyCode::Char('j') | KeyCode::Down => self.decrement_value(),
                    // KeyCode::Char('h') | KeyCode::Left => self.prev_block(),
                    // KeyCode::Char('l') | KeyCode::Right => self.next_block(),
                    // self.sidebar.handle_events(key.code)?;
                    _ => {}
                }
            }
            Event::Resize(_width, _height) => {
                terminal.autoresize()?;
            }
            _ => {}
        }

        Ok(())
    }

    const fn exit(&mut self) {
        self.is_running = false;
    }

    fn render_layout(&self, area: Rect, buf: &mut Buffer) {
        let main_area = if self.show_logs {
            let [main_area, log_area] = area.layout(&Layout::vertical([Fill(1), Length(12)]));
            self.render_logs(log_area, buf);

            main_area
        } else {
            area
        };

        if self.sidebar.show {
            let [sidebar_area, separator_area, content_area] =
                main_area.layout(&Layout::horizontal([Length(Sidebar::WIDTH), Length(1), Fill(1)]));

            self.sidebar.render(sidebar_area, buf);

            for i in 0..separator_area.height {
                buf.set_string(
                    separator_area.x,
                    separator_area.y + i,
                    "│",
                    palette::SEPARATOR_STYLE,
                );
            }

            self.content.render(content_area, buf);
        } else {
            self.content.render(main_area, buf);
        }
    }

    fn render_logs(&self, area: Rect, buf: &mut Buffer) {
        let log_widget = TuiLoggerWidget::default()
            .output_timestamp(None)
            // .output_level(None)
            .output_file(false)
            .output_line(false)
            .output_target(false)
            .block(
                Block::default()
                    .title("Logs")
                    .borders(Borders::ALL)
                    .border_style(palette::SEPARATOR_STYLE),
            )
            .style(palette::STYLE);
        log_widget.render(area, buf);
    }
}
