use ratatui::{
    TerminalOptions, Viewport,
    style::Stylize as _,
    text::{Line, Text, ToSpan},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use super::common::palette;

pub(crate) struct StructDocs {
    pub description: &'static str,
    pub field_names: &'static [&'static str],
    pub field_docs: &'static [&'static str],
}

/// Pretty prints given struct documentation utilizing inline ratatui viewport
pub(crate) fn print_struct(docs: &StructDocs) {
    assert!(
        docs.field_names.len() == docs.field_docs.len(),
        "Field names and docs must have the same length"
    );

    let mut terminal = ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline(
            u16::try_from(docs.field_names.len() + 2)
                .expect("Failed to convert field names length to u16"),
        ),
    });
    terminal.clear().ok();

    terminal
        .draw(|frame| {
            let block = Block::bordered()
                .fg(palette().muted_foreground)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(docs.description);

            let mut lines: Vec<Line> = vec![];
            for index in 0..docs.field_names.len() {
                let line = Line::from(vec![
                    docs.field_names[index].to_span().bold(),
                    format!(" // {}", docs.field_docs[index].replace('\n', " "))
                        .fg(palette().muted_foreground),
                ])
                .style(palette().style);
                lines.push(line);
            }

            let content = Text::from(lines).fg(palette().foreground);
            let paragraph = Paragraph::new(content).block(block);
            frame.render_widget(paragraph, frame.area());
        })
        .expect("Failed to draw description");
}
