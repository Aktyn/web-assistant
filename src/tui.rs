mod content;
mod macros;
mod palette;
mod panel;
mod sidebar;

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
use crate::tui::content::Content;
use crate::tui::panel::Panel;
use crate::tui::sidebar::Sidebar;
use crate::workflows_count;

pub fn render_tui(core: Core) -> Result<()> {
    let app = TuiApp::new(core);
    ratatui::run(|terminal| app.run(terminal))
}

#[derive(PartialEq, Debug)]
enum PanelType {
    Sidebar,
    Content,
}

struct TuiApp {
    core: Arc<RwLock<Core>>,
    is_running: bool,
    show_logs: bool,
    focused_panel: PanelType,
    content: Box<dyn Panel>,
    sidebar: Box<dyn Panel>,
    show_sidebar: bool,
}

impl TuiApp {
    fn new(core: Core) -> Self {
        let core_arc = Arc::new(RwLock::new(core));
        let core_sidebar_arc = Arc::clone(&core_arc);
        let core_content_arc = Arc::clone(&core_arc);

        Self {
            core: core_arc,
            is_running: false,
            show_logs: cfg!(debug_assertions),
            focused_panel: PanelType::Content,
            sidebar: Box::new(Sidebar::new(core_sidebar_arc)),
            content: Box::new(Content::new(core_content_arc)),
            show_sidebar: true,
        }
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.is_running = true;

        if workflows_count!(self.core) > 0 {
            self.show_sidebar = true;
            self.focused_panel = PanelType::Sidebar;
        } else {
            self.show_sidebar = false;
            self.focused_panel = PanelType::Content;
        }

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
                KeyCode::Tab => self.move_to_next_panel(),
                KeyCode::Char('s') => {
                    if workflows_count!(self.core) > 0 {
                        self.show_sidebar = !self.show_sidebar;
                    } else {
                        self.show_sidebar = false;
                    }
                }
                _ => {}
            },
            Event::Resize(_width, _height) => {
                terminal.autoresize()?;
            }
            _ => {}
        }

        self.get_focused_panel().handle_event(terminal, &event)?;

        Ok(())
    }

    fn move_to_next_panel(&mut self) {
        match self.focused_panel {
            PanelType::Sidebar => self.focused_panel = PanelType::Content,
            PanelType::Content => {
                if workflows_count!(self.core) > 0 {
                    self.focused_panel = PanelType::Sidebar
                }
            }
        }
    }

    fn get_focused_panel(&mut self) -> &mut dyn panel::Panel {
        match self.focused_panel {
            PanelType::Sidebar => self.sidebar.as_mut(),
            PanelType::Content => self.content.as_mut(),
        }
    }

    const fn exit(&mut self) {
        self.is_running = false;
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let buf = frame.buffer_mut();

        let main_area = if self.show_logs {
            let [main_area, log_area] = area.layout(&Layout::vertical([Fill(1), Length(12)]));
            self.render_logs(log_area, buf);

            main_area
        } else {
            area
        };

        if self.show_sidebar {
            let [sidebar_area, separator_area, content_area] =
                main_area.layout(&Layout::horizontal([
                    Length(Sidebar::WIDTH),
                    Length(1),
                    Fill(1),
                ]));

            self.sidebar
                .draw(sidebar_area, buf, self.focused_panel == PanelType::Sidebar);

            for i in 0..separator_area.height {
                buf.set_string(
                    separator_area.x,
                    separator_area.y + i,
                    "│",
                    palette::SEPARATOR_STYLE,
                );
            }

            self.content
                .draw(content_area, buf, self.focused_panel == PanelType::Content);
        } else {
            self.content
                .draw(main_area, buf, self.focused_panel == PanelType::Content);
        }
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
