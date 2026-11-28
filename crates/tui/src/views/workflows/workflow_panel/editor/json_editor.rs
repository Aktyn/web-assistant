use crate::common::{palette, widgets::EditableTextField};
use ratatui::{
    layout::Layout,
    style::Stylize as _,
    text::{Line, Text},
    widgets::{Paragraph, Block, BorderType, Borders},
};
use ratatui::widgets::Paragraph;
use workflow::workflow_data::step::WorkflowStepType;

#[derive(Debug, Clone)]
pub(crate) struct JsonEditor {
    text_area: TextArea,
    error: Option<String>,
}

impl JsonEditor {
    pub(crate) fn new(initial_json: String) -> Self {
        Self {
            text_area: TextArea::new(initial_json),
            error: None,
        }
    }

    pub(crate) fn get_steps(&self) -> Vec<WorkflowStepType> {
        if let Ok(steps) = serde_json::from_str::<Vec<WorkflowStepType>>(&self.text_area.content) {
            steps
        } else if let Ok(step) = serde_json::from_str::<WorkflowStepType>(&self.text_area.content) {
            vec![step]
        } else {
            vec![]
        }
    }

    pub(crate) fn delete_step(&mut self, index: usize) {
        if let Ok(mut steps) = serde_json::from_str::<Vec<WorkflowStepType>>(&self.text_area.content) {
            if index < steps.len() {
                steps.remove(index);
                self.text_area.content = serde_json::to_string_pretty(&steps).unwrap_or_default();
            }
        }
    }

    pub(crate) fn handle_key_event(&mut self, key: ratatui::event::KeyEvent) -> Option<JsonEditorAction> {
        match key.code {
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.validate() {
                    Some(JsonEditorAction::Save(self.get_steps()))
                } else {
                    None
                }
            }
            KeyCode::Esc => Some(JsonEditorAction::Cancel),
            _ => {
                self.text_area.handle_key(key);
                self.validate();
                None
            }
        }
    }

    fn validate(&mut self) -> bool {
        if serde_json::from_str::<Vec<WorkflowStepType>>(&self.text_area.content).is_ok() {
            self.error = None;
            true
        } else if let Err(e) = serde_json::from_str::<WorkflowStepType>(&self.text_area.content) {
            self.error = Some(e.to_string());
            false
        } else {
            self.error = None;
            true
        }
    }

    fn render(&self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let [title_area, editor_area, error_area] = area.layout(&Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ]));

        Paragraph::new("Step Editor (JSON) - Ctrl+S to save, Esc to cancel")
            .style(palette().muted_foreground)
            .render(title_area, buf);

        self.text_area.render(editor_area, buf);

        if let Some(error) = &self.error {
            Paragraph::new(format!("Error: {}", error))
                .style(palette().error_foreground)
                .render(error_area, buf);
        }
    }
}

/// Simple text area for JSON editing
#[derive(Debug, Clone)]
struct TextArea {
    content: String,
    cursor_position: usize,
}

impl TextArea {
    fn new(content: String) -> Self {
        let cursor_position = content.len();
        Self { content, cursor_position }
    }

    fn handle_key(&mut self, key: ratatui::event::KeyEvent) {
        match key.code {
            KeyCode::Char(c) => {
                self.content.insert(self.cursor_position, c);
                self.cursor_position += 1;
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.content.remove(self.cursor_position);
                }
            }
            KeyCode::Delete => {
                if self.cursor_position < self.content.len() {
                    self.content.remove(self.cursor_position);
                }
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_position < self.content.len() {
                    self.cursor_position += 1;
                }
            }
            KeyCode::Home => {
                let before_cursor = &self.content[..self.cursor_position];
                if let Some(pos) = before_cursor.rfind('\n') {
                    self.cursor_position = pos + 1;
                } else {
                    self.cursor_position = 0;
                }
            }
            KeyCode::End => {
                let after_cursor = &self.content[self.cursor_position..];
                if let Some(pos) = after_cursor.find('\n') {
                    self.cursor_position += pos;
                } else {
                    self.cursor_position = self.content.len();
                }
            }
            KeyCode::Enter => {
                self.content.insert(self.cursor_position, '\n');
                self.cursor_position += 1;
            }
            KeyCode::Tab => {
                self.content.insert_str(self.cursor_position, "  ");
                self.cursor_position += 2;
            }
            _ => {}
        }
    }

    fn render(&self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let lines: Vec<Line> = self.content.lines().map(|l| Line::from(l)).collect();
        let paragraph = Paragraph::new(lines)
            .style(palette().style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(palette().accent),
            );
        paragraph.render(area, buf);
    }
}

#[derive(Debug, Clone)]
pub(crate) enum JsonEditorAction {
    Save(Vec<WorkflowStepType>),
    Cancel,
}