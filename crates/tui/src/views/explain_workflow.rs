use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Stylize as _,
    text::{Line, Text},
    widgets::{Paragraph, Block, BorderType, Borders},
};

use crate::common::palette;
use crate::views::{View, ViewAction};
use crate::views::workflows::workflow_data::{StepStatus, StepUpdate, WorkflowData, WorkflowProcedure, workflow_step};

/** View that explains the workflow structure by printing step struct docs */
#[derive(Debug, Clone)]
pub(crate) struct ExplainWorkflowView {
    include_example: bool,
    pending_action: Option<ViewAction>,
}

impl ExplainWorkflowView {
    pub(crate) fn new(include_example: bool) -> Self {
        Self {
            include_example,
            pending_action: None,
        }
    }

    fn render(&self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let [_, info_area, details_area, status_area, buttons_area, _] =
            area.layout(&Layout::vertical([
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Length(45),
                Constraint::Length(1),
                Constraint::Length(Button::HEIGHT),
                Constraint::Fill(1),
            ]));

        // Header
        Paragraph::new("Explain Workflow")
            .style(palette().style.bold())
            .centered()
            .render(info_area, buf);

        // Workflow structure
        let procedures = if self.include_example {
            // Include the example workflow data
            vec![WorkflowProcedure {
                name: String::from("main-procedure"),
                steps: workflow_step::WorkflowStepType::PageAction(workflow_step::PageActionStep {
                    page_index: 0,
                    action: workflow_step::page_action::NavigateAction {
                        url: "https://example.com".into(),
                    },
                }),
            }]
        } else {
            vec![]
        };

        let details = Text::from(vec![
            Line::from("Workflow Structure"),
            Line::from(""),
            if self.include_example {
                Line::from("Example workflow demonstrating all step types"),
            } else {
                Line::from("Use --include-example to show full workflow documentation")
            },
            Line::from(""),
        ]);

        Paragraph::new(details)
            .style(palette().style)
            .block(
                Block::bordered()
                    .fg(palette().muted_foreground)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(format!(" {} ", if self.include_example {
                        "Full Workflow"
                    } else { "Workflow Summary" }).bold()),
            )
            .render(details_area, buf);

        // Info about step types
        let step_info = Text::from(vec![
            Line::from("Step Types Documented:"),
            Line::from("  - PageAction: Navigate, Input, Click, Evaluate"),
            Line::from("  - Agent: AI agent task execution"),
            Line::from("  - Condition: MatchRegex, Truthy, Equal, HasElement"),
            Line::from("  - ForEach: File iterator, order random/sequential"),
            Line::from("  - RunProcedure: Run a named procedure"),
            Line::from("  - UpdateContext: Update context key-value pairs"),
            Line::from("  - SystemAction: Notify, OpenUrl, Wait, Abort"),
            Line::from(""),
            Line::from("Use 'Include example' button to show full example data."),
        ]);

        Paragraph::new(step_info)
            .style(palette().style)
            .render(details_area, buf);

        // Buttons
        let button_areas: [ratatui::layout::Rect; 2] = buttons_area
            .centered_horizontally(Constraint::Max(50))
            .layout(&Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        // "Close" button
        let close_btn = crate::common::widgets::Button::new("Close")
            .render(button_areas[0], buf);
    }
}

impl View for ExplainWorkflowView {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, palette().style);
        self.render(area, buf);
    }

    fn handle_event(
        &mut self,
        event: &crossterm::event::Event,
        terminal: &mut DefaultTerminal,
    ) -> bool {
        if let Some(key) = event.as_key_event() {
            // Enter or Esc closes
            if key.code == crossterm::event::KeyCode::Enter
                || key.code == crossterm::event::KeyCode::Esc
            {
                self.pending_action = Some(ViewAction::GoToWorkflows);
                return true;
            }
        }
        false
    }

    fn get_action(&self) -> Option<ViewAction> {
        self.pending_action
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}