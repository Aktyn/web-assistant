use crossterm::event::{Event, KeyCode};
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{
        Constraint::{self, Fill, Length},
        Layout, Rect,
    },
    widgets::{Paragraph, Widget},
};
use std::sync::{Arc, RwLock};

use super::super::common::{palette, panel};
use crate::core::Core;

pub(crate) struct WorkflowPanel {
    core: Arc<RwLock<Core>>,
}

impl WorkflowPanel {
    pub(crate) fn new(core: Arc<RwLock<Core>>) -> Self {
        Self { core }
    }
}

impl panel::Panel for WorkflowPanel {
    fn draw(&mut self, area: Rect, buf: &mut Buffer, focused: bool) {
        buf.set_style(area, palette::BACKGROUND_STYLE);

        let [header_area, content_area] = area.layout(&Layout::vertical([Length(2), Fill(1)]));

        panel::Header::new("Panel", focused).render(header_area, buf);

        let directory_path = self
            .core
            .read()
            .expect("core lock poisoned")
            .workflows_directory
            .display()
            .to_string();

        Paragraph::new(format!(
            "Workflows directory: \"{}\"\nSelect another directory or run the workflow creator",
            directory_path
        ))
        .centered()
        .render(content_area.centered_vertically(Constraint::Length(2)), buf);
    }

    fn handle_event(
        &mut self,
        event: &Event,
        _terminal: &mut DefaultTerminal,
    ) -> anyhow::Result<()> {
        if let Some(key) = event.as_key_event() {
            match key.code {
                KeyCode::Up => log::info!("content up event"),
                KeyCode::Down => log::info!("content down event"),
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::common::panel::Panel;

    #[test]
    fn test_workflow_panel_new() {
        let core = Arc::new(RwLock::new(Core::default()));
        let _panel = WorkflowPanel::new(core.clone());
    }

    #[test]
    fn test_workflow_panel_draw_no_panic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let core = Arc::new(RwLock::new(Core {
            workflows_directory: temp_dir.path().to_path_buf(),
            workflows: vec![],
        }));
        let mut panel = WorkflowPanel::new(core);
        let area = Rect::new(0, 0, 50, 20);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflow_panel_draw_focused() {
        let temp_dir = tempfile::tempdir().unwrap();
        let core = Arc::new(RwLock::new(Core {
            workflows_directory: temp_dir.path().to_path_buf(),
            workflows: vec![],
        }));
        let mut panel = WorkflowPanel::new(core);
        let area = Rect::new(0, 0, 50, 20);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, true);
    }

    /// handle_event never reads _terminal for the events we exercise. We
    /// store an uninit'ed slot behind a OnceLock and return a raw address
    /// that satisfies the &mut DefaultTerminal type without being deref'd.
    fn _no_read_term() -> *mut DefaultTerminal {
        use std::mem::MaybeUninit;
        static SLOT: std::sync::OnceLock<MaybeUninit<DefaultTerminal>> = std::sync::OnceLock::new();
        let slot = SLOT.get_or_init(MaybeUninit::uninit);
        slot.as_ptr() as *mut DefaultTerminal
    }

    #[test]
    fn test_workflow_panel_handle_up_event_ok() {
        let core = Arc::new(RwLock::new(Core::default()));
        let mut panel = WorkflowPanel::new(core);
        let event = Event::Key(KeyCode::Up.into());
        let result = panel.handle_event(&event, unsafe { &mut *_no_read_term() });
        assert!(result.is_ok());
    }

    #[test]
    fn test_workflow_panel_handle_mouse_event_ok() {
        let core = Arc::new(RwLock::new(Core::default()));
        let mut panel = WorkflowPanel::new(core);
        let event = Event::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        let result = panel.handle_event(&event, unsafe { &mut *_no_read_term() });
        assert!(result.is_ok());
    }

    #[test]
    fn test_workflow_panel_draw_with_default_path_no_panic() {
        let core = Arc::new(RwLock::new(Core::default()));
        let mut panel = WorkflowPanel::new(core);
        let area = Rect::new(0, 0, 50, 20);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }
}
