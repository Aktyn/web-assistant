use std::io;

use anyhow::Context;
use ratatui::{
    TerminalOptions, Viewport,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Margin},
    style::Stylize as _,
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

use super::common::palette;

pub(crate) struct StructDocs {
    pub description: &'static str,
    pub field_names: &'static [&'static str],
    pub field_docs: &'static [&'static str],
}

/// Pretty prints given struct documentation utilizing inline ratatui viewport
pub(crate) fn print_struct(docs: &StructDocs) -> anyhow::Result<()> {
    if docs.field_names.len() != docs.field_docs.len() {
        anyhow::bail!("Field names and docs must have the same length");
    }

    let total_docs_lines: usize = docs.field_docs.iter().map(|doc| doc.lines().count()).sum();

    let mut terminal = ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline(u16::try_from(total_docs_lines + 2).unwrap_or(u16::MAX)),
    });
    terminal.clear().ok();

    terminal
        .draw(|frame| {
            let area = frame.area();

            let block = Block::bordered()
                .fg(palette().muted_foreground)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!(" {} ", docs.description));
            frame.render_widget(block, frame.area());

            let content_area = area.inner(Margin::from(1));

            let constraints: Vec<Constraint> = (0..docs.field_docs.len())
                .map(|index| {
                    Constraint::Length(
                        u16::try_from(docs.field_docs[index].lines().count()).unwrap_or(u16::MAX),
                    )
                })
                .collect();
            let docs_areas = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(content_area);

            for index in 0..docs_areas.len() {
                let name = docs.field_names[index];

                let [name_area, doc_area] = docs_areas[index].layout(&Layout::horizontal([
                    Constraint::Length(
                        u16::try_from(name.len())
                            .unwrap_or(u16::MAX)
                            .saturating_add(1),
                    ),
                    Constraint::Fill(1),
                ]));

                let name_span = Span::from(name).fg(palette().foreground).bold();
                frame.render_widget(name_span, name_area);

                let docs = Text::from(
                    docs.field_docs[index]
                        .split('\n')
                        .map(|line| Line::from(format!("// {line}")))
                        .collect::<Vec<Line>>(),
                );
                frame.render_widget(docs, doc_area);
            }
        })
        .context("Failed to draw struct documentation")?;

    Ok(())
}

pub(crate) fn print_success(message: &str) -> anyhow::Result<()> {
    let backend = CrosstermBackend::new(io::stdout());
    let size = backend.size()?;
    let terminal_width = size.width;

    let paragraph = Paragraph::new(message)
        .style(palette().success_foreground)
        .bold()
        .wrap(Wrap { trim: true });
    let line_count = paragraph.line_count(terminal_width);

    let mut terminal = ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline(u16::try_from(line_count).unwrap_or(1)),
    });
    terminal.clear().ok();

    terminal
        .draw(|frame| {
            let area = frame.area();
            frame.render_widget(paragraph, area);
        })
        .context("Failed to draw success message")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_struct_docs_mismatched_lengths_returns_error() {
        let docs = StructDocs {
            description: "Test",
            field_names: &["field1"],
            field_docs: &["doc1", "doc2"],
        };
        let result = print_struct(&docs);
        assert!(result.is_err());
    }

    #[test]
    fn test_struct_docs_valid_lengths_constructs_ok() {
        let docs = StructDocs {
            description: "Test struct",
            field_names: &["name", "value"],
            field_docs: &["Name doc", "Value doc"],
        };
        // Should not panic or error on construction.
        assert_eq!(docs.field_names.len(), 2);
        assert_eq!(docs.field_docs.len(), 2);
    }
}
