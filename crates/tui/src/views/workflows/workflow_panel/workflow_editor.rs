use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{
        Constraint::{Fill, Length, Ratio},
        Layout, Rect,
    },
    style::Style,
    text::Line,
    widgets::{
        Block, BorderType, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget,
    },
};

use super::super::super::common::palette;
use super::super::super::common::widgets::{Button, ButtonData, EditableTextField, Separator};
use super::{WorkflowAction, WorkflowPanelModeView, WorkflowSelection};
use workflow::workflow_data::WorkflowData;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorFocus {
    Name,
    Description,
    StepsList,
    Buttons,
}

impl EditorFocus {
    fn next(self) -> Self {
        match self {
            EditorFocus::Name => EditorFocus::Description,
            EditorFocus::Description => EditorFocus::StepsList,
            EditorFocus::StepsList => EditorFocus::Buttons,
            EditorFocus::Buttons => EditorFocus::Name,
        }
    }

    fn prev(self) -> Self {
        match self {
            EditorFocus::Name => EditorFocus::Buttons,
            EditorFocus::Description => EditorFocus::Name,
            EditorFocus::StepsList => EditorFocus::Description,
            EditorFocus::Buttons => EditorFocus::StepsList,
        }
    }
}

/// Action that the workflow editor can return
#[derive(Debug, Clone)]
pub(crate) enum WorkflowEditorAction {
    Save(WorkflowData),
    Cancel,
}

impl PartialEq for WorkflowEditorAction {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (WorkflowEditorAction::Cancel, WorkflowEditorAction::Cancel)
                | (WorkflowEditorAction::Save(_), WorkflowEditorAction::Save(_))
        )
    }
}
impl Eq for WorkflowEditorAction {}

/// The main workflow editor
pub(crate) struct WorkflowEditor {
    // workflows: Arc<RwLock<Vec<Workflow>>>,
    // workflow_index: usize,
    // workflow_file: Option<PathBuf>,
    original_data: Option<WorkflowData>,

    // Editor fields
    name_field: EditableTextField,
    description_field: EditableTextField,
    steps_list_state: ListState,
    json_editor: Option<JsonEditor>,

    // Buttons
    buttons: [ButtonData<WorkflowEditorAction>; 2],
    buttons_list_state: ListState,

    // Current focus
    focus: EditorFocus,
}

impl WorkflowEditor {
    pub(crate) fn new(
        // workflows: Arc<RwLock<Vec<Workflow>>>,
        // workflow_index: usize,
        workflow_data: Option<WorkflowData>,
        // workflow_file: Option<PathBuf>,
    ) -> Self {
        let (name, description, steps) = if let Some(ref data) = workflow_data {
            (
                data.name.clone(),
                data.description.clone().unwrap_or_default(),
                data.steps.clone(),
            )
        } else {
            ("Draft workflow".to_string(), String::new(), vec![])
        };

        let mut name_field = EditableTextField::new("Name".to_string(), name);
        name_field.set_focused(true);

        let mut description_field = EditableTextField::new("Description".to_string(), description);
        description_field.set_focused(false);

        let steps_list_state =
            ListState::default().with_selected(if steps.is_empty() { None } else { Some(0) });

        Self {
            // workflows,
            // workflow_index,
            // workflow_file,
            original_data: workflow_data,
            name_field,
            description_field,
            steps_list_state,
            json_editor: None,
            buttons: [
                ButtonData::new("Save", WorkflowEditorAction::Save(WorkflowData::default())),
                ButtonData::new("Cancel", WorkflowEditorAction::Cancel),
            ],
            buttons_list_state: ListState::default().with_selected(Some(0)),
            focus: EditorFocus::Name,
        }
    }

    fn build_workflow_data(&self) -> WorkflowData {
        let steps = if let Some(editor) = &self.json_editor {
            editor.get_steps()
        } else {
            self.original_data
                .as_ref()
                .map_or_default(|d| d.steps.clone())
        };

        WorkflowData {
            name: self.name_field.value().to_string(),
            description: if self.description_field.value().is_empty() {
                None
            } else {
                Some(self.description_field.value().to_string())
            },
            procedures: self
                .original_data
                .as_ref()
                .map_or_default(|d| d.procedures.clone()),
            steps,
            network_connection_timeout_sec: self
                .original_data
                .as_ref()
                .map_or(600, |d| d.network_connection_timeout_sec),
        }
    }

    fn render_fields(&self, area: Rect, buf: &mut Buffer) {
        let [name_area, desc_area] = area.layout(&Layout::vertical([Length(3), Length(3)]));
        self.name_field.draw(name_area, buf);
        self.description_field.draw(desc_area, buf);
    }

    fn render_steps_list(&self, area: Rect, buf: &mut Buffer) {
        let steps = self.get_current_steps();
        let items: Vec<ListItem> = steps
            .iter()
            .enumerate()
            .map(|(i, step)| {
                let label = match step {
                    workflow::workflow_data::step::WorkflowStepType::PageAction(s) => {
                        format!("{}. [page-action] {}", i + 1, format_page_action(&s.action))
                    }
                    workflow::workflow_data::step::WorkflowStepType::Agent(s) => {
                        format!("{}. [agent] {}", i + 1, s.task)
                    }
                    workflow::workflow_data::step::WorkflowStepType::SystemAction(s) => {
                        format!(
                            "{}. [system-action] {}",
                            i + 1,
                            format_system_action(&s.action)
                        )
                    }
                    workflow::workflow_data::step::WorkflowStepType::Condition(s) => {
                        format!("{}. [condition] {}", i + 1, format_condition(&s.condition))
                    }
                    workflow::workflow_data::step::WorkflowStepType::ForEach(s) => {
                        format!("{}. [for-each] {}", i + 1, s.context_name)
                    }
                    workflow::workflow_data::step::WorkflowStepType::RunProcedure(s) => {
                        format!("{}. [run-procedure] {}", i + 1, s.procedure)
                    }
                    workflow::workflow_data::step::WorkflowStepType::UpdateContext(s) => {
                        format!("{}. [update-context] {}", i + 1, s.key)
                    }
                    workflow::workflow_data::step::WorkflowStepType::Wait(s) => {
                        format!("{}. [wait] {}ms", i + 1, s.duration)
                    }
                    workflow::workflow_data::step::WorkflowStepType::Abort(s) => {
                        format!("{}. [abort] {}", i + 1, s.message)
                    }
                };
                ListItem::new(label).style(palette().style)
            })
            .collect();

        let list = List::new(items)
            .style(palette().style)
            .highlight_style(Style::new().bg(palette().accent).fg(palette().background));

        StatefulWidget::render(list, area, buf, &mut self.steps_list_state.clone());
    }

    fn render_buttons(&self, area: Rect, buf: &mut Buffer) {
        let [save_area, cancel_area] = area.layout(&Layout::horizontal([Ratio(1, 2), Ratio(1, 2)]));

        let save_selected =
            self.focus == EditorFocus::Buttons && self.buttons_list_state.selected() == Some(0);
        let cancel_selected =
            self.focus == EditorFocus::Buttons && self.buttons_list_state.selected() == Some(1);

        Button::new("Save")
            .set_selected(save_selected)
            .render(save_area, buf);
        Button::new("Cancel")
            .set_selected(cancel_selected)
            .render(cancel_area, buf);
    }

    fn get_current_steps(&self) -> Vec<workflow::workflow_data::step::WorkflowStepType> {
        if let Some(editor) = &self.json_editor {
            editor.get_steps()
        } else {
            self.original_data
                .as_ref()
                .map_or_default(|d| d.steps.clone())
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Option<WorkflowEditorAction> {
        if let Some(editor) = &mut self.json_editor {
            if let Some(action) = editor.handle_key_event(key) {
                match action {
                    JsonEditorAction::Save(_steps) => {
                        self.json_editor = None;
                    }
                    JsonEditorAction::Cancel => {
                        self.json_editor = None;
                    }
                }
            }
            return None;
        }

        match (key.code, key.modifiers) {
            (KeyCode::Tab, KeyModifiers::NONE) => {
                self.focus = self.focus.next();
                self.update_focus();
                None
            }
            (KeyCode::BackTab, KeyModifiers::SHIFT) => {
                self.focus = self.focus.prev();
                self.update_focus();
                None
            }
            (KeyCode::Up, _) if self.focus == EditorFocus::StepsList => {
                let steps = self.get_current_steps();
                if !steps.is_empty() {
                    self.move_selection(steps.len(), -1);
                }
                None
            }
            (KeyCode::Down, _) if self.focus == EditorFocus::StepsList => {
                let steps = self.get_current_steps();
                if !steps.is_empty() {
                    self.move_selection(steps.len(), 1);
                }
                None
            }
            (KeyCode::Enter, _) if self.focus == EditorFocus::StepsList => {
                if let Some(selected) = self.steps_list_state.selected() {
                    self.edit_step(selected);
                } else {
                    self.add_new_step();
                }
                None
            }
            (KeyCode::Delete, _) if self.focus == EditorFocus::StepsList => {
                if let Some(selected) = self.steps_list_state.selected() {
                    self.delete_step(selected);
                }
                None
            }
            (KeyCode::Left, _) if self.focus == EditorFocus::Buttons => {
                self.buttons_list_state.select(Some(0));
                None
            }
            (KeyCode::Right, _) if self.focus == EditorFocus::Buttons => {
                self.buttons_list_state.select(Some(1));
                None
            }
            (KeyCode::Enter, _) if self.focus == EditorFocus::Buttons => {
                if let Some(selected) = self.buttons_list_state.selected()
                    && let Some(button_data) = self.buttons.get(selected)
                {
                    if button_data.action == WorkflowEditorAction::Cancel {
                        return Some(WorkflowEditorAction::Cancel);
                    }
                    let data = self.build_workflow_data();
                    return Some(WorkflowEditorAction::Save(data));
                }
                None
            }
            (KeyCode::Esc, _) => Some(WorkflowEditorAction::Cancel),
            _ => match self.focus {
                EditorFocus::Name => {
                    self.handle_text_input(true, key);
                    None
                }
                EditorFocus::Description => {
                    self.handle_text_input(false, key);
                    None
                }
                _ => None,
            },
        }
    }

    fn update_focus(&mut self) {
        self.name_field.set_focused(self.focus == EditorFocus::Name);
        self.description_field
            .set_focused(self.focus == EditorFocus::Description);
    }

    fn handle_text_input(&mut self, is_name: bool, key: KeyEvent) {
        let field = if is_name {
            &mut self.name_field
        } else {
            &mut self.description_field
        };
        match key.code {
            KeyCode::Char(c) => field.insert_char(c),
            KeyCode::Backspace => field.backspace(),
            KeyCode::Delete => field.delete(),
            KeyCode::Left => field.move_cursor_left(),
            KeyCode::Right => field.move_cursor_right(),
            KeyCode::Home => field.move_cursor_home(),
            KeyCode::End => field.move_cursor_end(),
            _ => {}
        }
    }

    fn move_selection(&mut self, count: usize, delta: isize) {
        let current = self.steps_list_state.selected().unwrap_or(0);
        let new = (current.cast_signed() + delta)
            .clamp(0, count.cast_signed() - 1)
            .cast_unsigned();
        self.steps_list_state.select(Some(new));
    }

    fn edit_step(&mut self, index: usize) {
        let steps = self.get_current_steps();
        if let Some(step) = steps.get(index) {
            let step_json = serde_json::to_string_pretty(step).unwrap_or_default();
            self.json_editor = Some(JsonEditor::new(step_json));
        }
    }

    fn add_new_step(&mut self) {
        let template = r#"{
  "type": "page-action",
  "pageIndex": 0,
  "action": {
    "type": "navigate",
    "url": "https://example.com"
  }
}"#;
        self.json_editor = Some(JsonEditor::new(template.to_string()));
    }

    fn delete_step(&mut self, index: usize) {
        if let Some(editor) = &mut self.json_editor {
            editor.delete_step(index);
        }
    }
}

impl WorkflowPanelModeView for WorkflowEditor {
    fn render(&self, _selection: &WorkflowSelection, area: Rect, buf: &mut Buffer) {
        let [
            fields_area,
            separator1,
            steps_area,
            separator2,
            buttons_area,
        ] = area.layout(&Layout::vertical([
            Length(7),
            Length(1),
            Fill(1),
            Length(1),
            Length(3),
        ]));

        self.render_fields(fields_area, buf);
        Separator::new(Separator::HORIZONTAL).render(separator1, buf);

        if let Some(editor) = &self.json_editor {
            let [steps_list_area, step_editor_area] =
                steps_area.layout(&Layout::horizontal([Ratio(1, 2), Ratio(1, 2)]));
            self.render_steps_list(steps_list_area, buf);
            Separator::new(Separator::VERTICAL).render(
                Rect::new(
                    steps_list_area.x + steps_list_area.width,
                    steps_list_area.y,
                    1,
                    steps_list_area.height,
                ),
                buf,
            );
            editor.render(step_editor_area, buf);
        } else {
            self.render_steps_list(steps_area, buf);
        }

        Separator::new(Separator::HORIZONTAL).render(separator2, buf);
        self.render_buttons(buttons_area, buf);
    }

    fn handle_event(
        &mut self,
        event: &Event,
        _terminal: &mut ratatui::DefaultTerminal,
        _selection: &WorkflowSelection,
    ) -> Option<WorkflowAction> {
        if let Some(key) = event.as_key_event()
            && let Some(action) = self.handle_key_event(key)
        {
            match action {
                WorkflowEditorAction::Save(_data) => {
                    return Some(WorkflowAction::BackToPreview);
                }
                WorkflowEditorAction::Cancel => {
                    return Some(WorkflowAction::BackToPreview);
                }
            }
        }
        None
    }
}

fn format_page_action(action: &workflow::workflow_data::step::PageAction) -> String {
    match action {
        workflow::workflow_data::step::PageAction::Navigate(a) => format!("navigate -> {}", a.url),
        workflow::workflow_data::step::PageAction::Input(a) => format!("input -> {}", a.selector),
        workflow::workflow_data::step::PageAction::Click(a) => format!("click -> {}", a.selector),
        workflow::workflow_data::step::PageAction::Evaluate(a) => {
            format!("evaluate -> {}", a.script)
        }
    }
}

fn format_system_action(action: &workflow::workflow_data::step::SystemAction) -> String {
    match action {
        workflow::workflow_data::step::SystemAction::OpenUrl(a) => format!("open-url -> {}", a.url),
        workflow::workflow_data::step::SystemAction::Notify(a) => {
            format!("notify -> {}", a.message)
        }
    }
}

fn format_condition(condition: &workflow::workflow_data::step::Condition) -> String {
    match condition {
        workflow::workflow_data::step::Condition::MatchRegex(a) => {
            format!("match-regex -> {}", a.pattern)
        }
        workflow::workflow_data::step::Condition::Truthy(a) => format!("truthy -> {}", a.value),
        workflow::workflow_data::step::Condition::Equal(a) => {
            format!("equal -> {} == {}", a.left, a.right)
        }
        workflow::workflow_data::step::Condition::HasElement(a) => {
            format!("has-element -> {}", a.selector)
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum JsonEditorAction {
    Save(Vec<workflow::workflow_data::step::WorkflowStepType>),
    Cancel,
}

/// JSON-based step editor
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

    pub(crate) fn get_steps(&self) -> Vec<workflow::workflow_data::step::WorkflowStepType> {
        if let Ok(steps) = serde_json::from_str::<
            Vec<workflow::workflow_data::step::WorkflowStepType>,
        >(&self.text_area.content)
        {
            steps
        } else if let Ok(step) = serde_json::from_str::<
            workflow::workflow_data::step::WorkflowStepType,
        >(&self.text_area.content)
        {
            vec![step]
        } else {
            vec![]
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Option<JsonEditorAction> {
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
        if serde_json::from_str::<Vec<workflow::workflow_data::step::WorkflowStepType>>(
            &self.text_area.content,
        )
        .is_ok()
        {
            self.error = None;
            true
        } else if let Err(e) = serde_json::from_str::<workflow::workflow_data::step::WorkflowStepType>(
            &self.text_area.content,
        ) {
            self.error = Some(e.to_string());
            false
        } else {
            self.error = None;
            true
        }
    }

    fn delete_step(&mut self, index: usize) {
        if let Ok(mut steps) = serde_json::from_str::<
            Vec<workflow::workflow_data::step::WorkflowStepType>,
        >(&self.text_area.content)
            && index < steps.len()
        {
            steps.remove(index);
            self.text_area.content = serde_json::to_string_pretty(&steps).unwrap_or_default();
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        let [title_area, editor_area, error_area] =
            area.layout(&Layout::vertical([Length(1), Fill(1), Length(1)]));

        Paragraph::new("Step Editor (JSON) - Ctrl+S to save, Esc to cancel")
            .style(palette().muted_foreground)
            .render(title_area, buf);

        self.text_area.render(editor_area, buf);

        if let Some(error) = &self.error {
            Paragraph::new(format!("Error: {error}"))
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
        Self {
            content,
            cursor_position,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
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

    fn render(&self, area: Rect, buf: &mut Buffer) {
        let lines: Vec<Line> = self.content.lines().map(Line::from).collect();
        let paragraph = Paragraph::new(lines).style(palette().style).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(palette().accent),
        );
        paragraph.render(area, buf);
    }
}
