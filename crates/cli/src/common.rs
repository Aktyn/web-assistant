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
        tui::inline::print_struct(&tui::inline::StructDocs {
            description: <$struct_type as documented::Documented>::DOCS,
            field_names: <$struct_type as documented::DocumentedFields>::FIELD_NAMES,
            field_docs: <$struct_type as documented::DocumentedFields>::FIELD_DOCS,
            ..Default::default()
        })
    };
    ($struct_type:ty, $margin_left:expr) => {
        tui::inline::print_struct(&tui::inline::StructDocs {
            description: <$struct_type as documented::Documented>::DOCS,
            field_names: <$struct_type as documented::DocumentedFields>::FIELD_NAMES,
            field_docs: <$struct_type as documented::DocumentedFields>::FIELD_DOCS,
            margin_left: Some($margin_left),
        })
    };
}
pub(crate) use print_struct_docs;
