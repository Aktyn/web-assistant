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

use super::super::common::{error_alert::ErrorAlert, palette, panel};
use crate::{
    core::{Core, workflow},
    utils,
};

struct WorkflowSelection {
    /// Index of the workflow in the list of workflows
    index: usize,
    // TODO: keep UI related objects here
}

pub(crate) struct WorkflowPanel {
    core: Arc<RwLock<Core>>,
    selected_workflow: Option<WorkflowSelection>,
}

impl WorkflowPanel {
    pub(super) fn new(core: Arc<RwLock<Core>>) -> Self {
        Self {
            core,
            selected_workflow: None,
        }
    }

    pub(super) fn update(&mut self, selected_workflow_index: Option<usize>) {
        if let Some(index) = selected_workflow_index {
            match self.selected_workflow {
                Some(ref selected_workflow) => {
                    if selected_workflow.index != index {
                        self.open_workflow(index);
                    }
                }
                None => self.open_workflow(index),
            }
        }
    }

    fn open_workflow(&mut self, index: usize) {
        log::info!("opening workflow: {index:?}");
        self.selected_workflow = Some(WorkflowSelection { index });
    }

    fn render_workflow_selection(
        &self,
        selection: &WorkflowSelection,
        area: Rect,
        buf: &mut Buffer,
    ) {
        match self.core.read() {
            Ok(unlocked_core) => {
                if let Some(workflow) = unlocked_core.workflows.get(selection.index) {
                    let [path_area, content_area] =
                        area.layout(&Layout::vertical([Length(1), Fill(1)]));

                    let path = &workflow.file_path;
                    let absolute_path =
                        std::path::absolute(workflow.file_path.as_path()).unwrap_or(path.clone());
                    let absolute_path_str = absolute_path.to_str().unwrap_or("");
                    let label = "Workflow file: ";

                    Paragraph::new(format!(
                        "{}{}",
                        label,
                        utils::truncate_text_left(
                            absolute_path_str,
                            (content_area.width as usize).saturating_sub(label.len()),
                            None
                        )
                    ))
                    .style(palette::STYLE)
                    .render(path_area, buf);
                    self.render_workflow_state(&workflow.state, content_area, buf);
                } else {
                    ErrorAlert::new(format!(
                        "Cannot get workflow with index: {}",
                        selection.index
                    ))
                    .render(area, buf);
                }
            }
            Err(error) => {
                let text = vec!["Cannot read core".into(), error.to_string().into()];
                ErrorAlert::new(text).render(area, buf);
            }
        }
    }

    #[allow(clippy::unused_self)]
    fn render_workflow_state(&self, state: &workflow::WorkflowState, area: Rect, buf: &mut Buffer) {
        match state {
            workflow::WorkflowState::Data(data) => {
                //TODO: editable field with label "Name"
                Paragraph::new(data.name.clone())
                    .style(palette::STYLE)
                    .render(area, buf);
            }
            workflow::WorkflowState::Error(error) => {
                let text = vec![
                    "The file is invalid".into(),
                    error.to_string().into(),
                    "Please edit the file to fix it".into(),
                ];
                ErrorAlert::new(text).render(area, buf);
            }
        }
    }
}

impl panel::Panel for WorkflowPanel {
    fn draw(&mut self, area: Rect, buf: &mut Buffer, focused: bool) {
        buf.set_style(area, palette::BACKGROUND_STYLE);

        let [header_area, content_area] = area.layout(&Layout::vertical([Length(2), Fill(1)]));

        panel::Header::new("Panel", focused).render(header_area, buf);

        if let Some(ref selection) = self.selected_workflow {
            self.render_workflow_selection(selection, content_area, buf);
        } else {
            render_no_selection_info(content_area, buf);
        }
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

fn render_no_selection_info(area: Rect, buf: &mut Buffer) {
    let text = "No workflow selected";
    Paragraph::new(text)
        .style(palette::MUTED_FOREGROUND)
        .render(
            area.centered(
                Constraint::Length(text.len().try_into().unwrap_or(u16::MAX)),
                Constraint::Length(1),
            ),
            buf,
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::common::panel::Panel;

    fn empty_core() -> Core {
        Core {
            config: crate::core::config::Config::default(),
            workflows: Vec::new(),
        }
    }

    #[test]
    fn test_workflow_panel_new() {
        let core = Arc::new(RwLock::new(empty_core()));
        let _panel = WorkflowPanel::new(core.clone());
    }

    #[test]
    fn test_workflow_panel_draw_no_panic() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut panel = WorkflowPanel::new(core);
        let area = Rect::new(0, 0, 50, 20);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }

    #[test]
    fn test_workflow_panel_draw_focused() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut panel = WorkflowPanel::new(core);
        let area = Rect::new(0, 0, 50, 20);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, true);
    }

    /// `handle_event` never reads `_terminal` for the events we exercise. We
    /// store an uninit'ed slot behind a `OnceLock` and return a raw address
    /// that satisfies the &mut `DefaultTerminal` type without being deref'd.
    fn no_read_term() -> *mut DefaultTerminal {
        use std::mem::MaybeUninit;
        static SLOT: std::sync::OnceLock<MaybeUninit<DefaultTerminal>> = std::sync::OnceLock::new();
        let slot = SLOT.get_or_init(MaybeUninit::uninit);
        slot.as_ptr().cast_mut()
    }

    #[test]
    fn test_workflow_panel_handle_up_event_ok() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut panel = WorkflowPanel::new(core);
        let event = Event::Key(KeyCode::Up.into());
        let result = panel.handle_event(&event, unsafe { &mut *no_read_term() });
        assert!(result.is_ok());
    }

    #[test]
    fn test_workflow_panel_handle_mouse_event_ok() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut panel = WorkflowPanel::new(core);
        let event = Event::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        let result = panel.handle_event(&event, unsafe { &mut *no_read_term() });
        assert!(result.is_ok());
    }

    #[test]
    fn test_workflow_panel_draw_with_default_path_no_panic() {
        let core = Arc::new(RwLock::new(empty_core()));
        let mut panel = WorkflowPanel::new(core);
        let area = Rect::new(0, 0, 50, 20);
        let mut buf = Buffer::empty(area);
        panel.draw(area, &mut buf, false);
    }
}
