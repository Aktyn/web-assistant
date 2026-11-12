#[must_use]
pub fn truncate_text(text: &str, limit: usize, suffix: Option<&str>) -> String {
    if text.chars().count() <= limit {
        text.to_string()
    } else {
        let suffix_ = suffix.unwrap_or(r"...");
        format!(
            "{}{}",
            text.chars().take(limit - suffix_.len()).collect::<String>(),
            suffix_
        )
    }
}

#[must_use]
pub fn truncate_text_left(text: &str, limit: usize, suffix: Option<&str>) -> String {
    let char_count = text.chars().count();
    if char_count <= limit {
        return text.to_string();
    }
    let suffix_ = suffix.unwrap_or("...");
    let keep = limit - suffix_.len();
    format!(
        "{}{}",
        suffix_,
        text.chars().skip(char_count - keep).collect::<String>(),
    )
}
