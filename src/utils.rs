#[must_use]
pub fn truncate_text(text: &str, limit: usize, suffix: Option<&str>) -> String {
    truncate(text, limit, suffix, false)
}

#[must_use]
pub fn truncate_text_left(text: &str, limit: usize, suffix: Option<&str>) -> String {
    truncate(text, limit, suffix, true)
}

fn truncate(text: &str, limit: usize, suffix: Option<&str>, from_left: bool) -> String {
    let char_count = text.chars().count();
    if char_count <= limit {
        return text.to_string();
    }

    let suffix_ = suffix.unwrap_or("...");
    if from_left {
        let keep = limit - suffix_.len();
        format!(
            "{}{}",
            suffix_,
            text.chars().skip(char_count - keep).collect::<String>(),
        )
    } else {
        let suffix_ = suffix.unwrap_or(r"...");
        format!(
            "{}{}",
            text.chars().take(limit - suffix_.len()).collect::<String>(),
            suffix_
        )
    }
}
