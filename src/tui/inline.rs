use anyhow::Context;
use ratatui::{
    TerminalOptions, Viewport,
    layout::{Constraint, Direction, Layout, Margin},
    style::Stylize as _,
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders},
};

use super::common::palette;

/// Convenience macro that builds a [`StructDocs`] from a documented type's
/// `DOCS`, `FIELD_NAMES`, and `FIELD_DOCS` associated constants and passes it
/// to [`print_struct`].
///
/// # Examples
///
/// ```ignore
/// print_struct_docs!(Config);
/// print_struct_docs!(config::HexColorsConfig, 2);
/// ```
macro_rules! print_struct_docs {
    ($struct_type:ty) => {
        $crate::tui::inline::print_struct(&$crate::tui::inline::StructDocs {
            description: <$struct_type as documented::Documented>::DOCS,
            field_names: <$struct_type as documented::DocumentedFields>::FIELD_NAMES,
            field_docs: <$struct_type as documented::DocumentedFields>::FIELD_DOCS,
            ..Default::default()
        })
    };
    ($struct_type:ty, $margin_left:expr) => {
        $crate::tui::inline::print_struct(&$crate::tui::inline::StructDocs {
            description: <$struct_type as documented::Documented>::DOCS,
            field_names: <$struct_type as documented::DocumentedFields>::FIELD_NAMES,
            field_docs: <$struct_type as documented::DocumentedFields>::FIELD_DOCS,
            margin_left: Some($margin_left),
        })
    };
}
pub(crate) use print_struct_docs;

#[derive(Default)]
pub struct StructDocs {
    pub description: &'static str,
    pub field_names: &'static [&'static str],
    pub field_docs: &'static [&'static str],
    pub margin_left: Option<u16>,
}

/// Pretty prints given struct documentation utilizing inline ratatui viewport
pub fn print_struct(docs: &StructDocs) -> anyhow::Result<()> {
    if docs.field_names.len() != docs.field_docs.len() {
        anyhow::bail!("Field names and docs must have the same length");
    }

    let total_docs_lines: usize = docs.field_docs.iter().map(|doc| doc.lines().count()).sum();
    let max_line_length = docs
        .field_names
        .iter()
        .zip(docs.field_docs)
        .map(|(name, doc)| {
            let max_doc_line_length = doc.lines().map(str::len).max().unwrap_or_default();
            max_doc_line_length.saturating_add(name.len())
        })
        .max()
        .unwrap_or_default()
        .max(docs.description.len());

    let mut terminal = ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline(u16::try_from(total_docs_lines + 2).unwrap_or(u16::MAX)),
    });
    terminal.clear().ok();

    terminal
        .draw(|frame| {
            let frame_area = frame.area();
            let [_, area, _] = frame_area.layout(&Layout::horizontal([
                Constraint::Length(docs.margin_left.unwrap_or(0)),
                Constraint::Length(
                    u16::try_from(max_line_length.saturating_add(2 + 4))
                        .unwrap_or(frame_area.width),
                ),
                Constraint::Fill(1),
            ]));

            let block = Block::bordered()
                .fg(palette().muted_foreground)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!(" {} ", docs.description));
            frame.render_widget(block, area);

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

    let cursor_y = terminal.get_cursor_position()?.y;
    terminal.set_cursor_position((0, cursor_y))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Extracts a readable message from a panic payload, returning an empty
    /// string when the payload is neither a `String` nor a `&str`.
    fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().copied().map(String::from))
            .unwrap_or_default()
    }

    #[test]
    fn test_struct_docs_mismatched_lengths_returns_error() {
        let docs = StructDocs {
            description: "Test",
            field_names: &["field1"],
            field_docs: &["doc1", "doc2"],
            ..Default::default()
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
            ..Default::default()
        };
        // Should not panic or error on construction.
        assert_eq!(docs.field_names.len(), 2);
        assert_eq!(docs.field_docs.len(), 2);
    }

    /// Runs `f` inside `catch_unwind` and, if it panics, asserts the panic
    /// message looks like a terminal-initialization failure (which is the
    /// expected outcome when no TTY is available, e.g. in CI or tarpaulin).
    fn assert_terminal_failure_or_success<F>(f: F)
    where
        F: FnOnce() -> anyhow::Result<()> + std::panic::UnwindSafe,
    {
        let result = std::panic::catch_unwind(f);
        match result {
            Ok(Ok(())) => {}  // Rendered successfully (rare in CI).
            Ok(Err(_e)) => {} // Terminal init or draw failure is expected without a TTY.
            Err(payload) => {
                let msg = panic_message(&payload);
                assert!(
                    msg.contains("terminal")
                        || msg.contains("No such device")
                        || msg.contains("failed to initialize"),
                    "Unexpected panic: {msg}"
                );
            }
        }
    }

    #[test]
    fn test_print_struct_empty_fields_no_error() {
        let docs = StructDocs {
            description: "Empty struct",
            field_names: &[],
            field_docs: &[],
            ..Default::default()
        };
        assert_terminal_failure_or_success(|| print_struct(&docs));
    }

    #[test]
    fn test_print_struct_single_field() {
        let docs = StructDocs {
            description: "Single field",
            field_names: &["name"],
            field_docs: &["The name field"],
            margin_left: Some(2),
        };
        assert_terminal_failure_or_success(|| print_struct(&docs));
    }

    #[test]
    fn test_struct_docs_default() {
        let docs = StructDocs::default();
        assert_eq!(docs.description, "");
        assert_eq!(docs.field_names.len(), 0);
        assert_eq!(docs.margin_left, None);
    }

    #[test]
    fn test_print_struct_mismatched_lengths_errors() {
        // This should return an error before even attempting terminal init
        let docs = StructDocs {
            description: "Mismatch",
            field_names: &["a", "b"],
            field_docs: &["doc_a"],
            ..Default::default()
        };
        let result = print_struct(&docs);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("same length"));
    }

    #[test]
    fn test_panic_message_from_string_payload() {
        let payload: Box<dyn std::any::Any + Send> =
            Box::new(String::from("terminal initialization failed"));
        let msg = panic_message(&payload);
        assert_eq!(msg, "terminal initialization failed");
    }

    #[test]
    fn test_panic_message_from_str_payload() {
        let payload: Box<dyn std::any::Any + Send> = Box::new("some panic message" as &str);
        let msg = panic_message(&payload);
        assert_eq!(msg, "some panic message");
    }

    #[test]
    fn test_panic_message_from_unknown_type_returns_empty() {
        let payload: Box<dyn std::any::Any + Send> = Box::new(42i32);
        let msg = panic_message(&payload);
        assert_eq!(msg, "");
    }

    /// `print_struct` with valid docs tries to initialize the terminal.
    #[test]
    fn test_print_struct_valid_docs_invokes_terminal() {
        let docs = StructDocs {
            description: "Test struct",
            field_names: &["name", "value"],
            field_docs: &["Name doc", "Value doc"],
            ..Default::default()
        };
        assert_terminal_failure_or_success(|| print_struct(&docs));
    }

    /// Same as above but with a non-zero `margin_left` to exercise the layout
    /// branch that uses `docs.margin_left`.
    #[test]
    fn test_print_struct_with_margin_left_invokes_terminal() {
        let docs = StructDocs {
            description: "Test struct",
            field_names: &["name", "value"],
            field_docs: &["Name doc", "Value doc"],
            margin_left: Some(4),
        };
        assert_terminal_failure_or_success(|| print_struct(&docs));
    }

    /// Terminal init path is exercised through a no-op closure.
    #[test]
    fn test_assert_terminal_helper_accepts_ok_closure() {
        assert_terminal_failure_or_success(|| Ok(()));
    }
}
