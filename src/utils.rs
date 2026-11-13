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
    // If the suffix itself is longer than the limit, just return the suffix.
    if suffix_.len() >= limit {
        return suffix_.to_string();
    }
    let keep = limit - suffix_.len();
    if from_left {
        format!(
            "{}{}",
            suffix_,
            text.chars().skip(char_count - keep).collect::<String>(),
        )
    } else {
        format!("{}{}", text.chars().take(keep).collect::<String>(), suffix_)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_text_short_text_unchanged() {
        let text = "hello";
        assert_eq!(truncate_text(text, 10, None), "hello");
    }

    #[test]
    fn test_truncate_text_exact_limit_unchanged() {
        let text = "hello";
        assert_eq!(truncate_text(text, 5, None), "hello");
    }

    #[test]
    fn test_truncate_text_truncates_with_default_suffix() {
        let text = "hello world this is long";
        let result = truncate_text(text, 10, None);
        assert!(result.ends_with("..."));
        assert_eq!(result.chars().count(), 10);
        assert!(result.starts_with("hello w"));
    }

    #[test]
    fn test_truncate_text_custom_suffix() {
        let text = "hello world this is long";
        let result = truncate_text(text, 15, Some("[..]"));
        assert_eq!(result, "hello world[..]");
    }

    #[test]
    fn test_truncate_text_empty_input() {
        assert_eq!(truncate_text("", 5, None), "");
    }

    #[test]
    fn test_truncate_text_empty_input_with_limit_zero() {
        // Empty text with any limit returns the empty text unchanged.
        assert_eq!(truncate_text("", 0, None), "");
    }

    #[test]
    fn test_truncate_text_unicode_chars_counted_correctly() {
        // Each emoji is one char; limit is in chars, not bytes.
        let text = "😀😀😀😀😀";
        let result = truncate_text(text, 3, None);
        // keep = 3 - 3 = 0, so only the suffix remains.
        assert_eq!(result, "...");
    }

    #[test]
    fn test_truncate_text_left_short_text_unchanged() {
        assert_eq!(truncate_text_left("hello", 10, None), "hello");
    }

    #[test]
    fn test_truncate_text_left_exact_limit_unchanged() {
        assert_eq!(truncate_text_left("hello", 5, None), "hello");
    }

    #[test]
    fn test_truncate_text_left_truncates_left_side() {
        let text = "hello world this is long";
        let result = truncate_text_left(text, 10, None);
        assert!(result.starts_with("..."));
        assert_eq!(result.chars().count(), 10);
        assert!(result.ends_with("s long"));
    }

    #[test]
    fn test_truncate_text_left_custom_suffix() {
        let text = "hello world this is long";
        let result = truncate_text_left(text, 18, Some("[..]"));
        assert_eq!(result, "[..]d this is long");
    }

    #[test]
    fn test_truncate_text_left_empty_input() {
        assert_eq!(truncate_text_left("", 5, None), "");
    }

    #[test]
    fn test_truncate_text_left_unicode_chars_counted_correctly() {
        let text = "😀😀😀😀😀";
        let result = truncate_text_left(text, 3, None);
        // keep = 3 - 3 = 0, so only the suffix remains.
        assert_eq!(result, "...");
    }

    #[test]
    fn test_truncate_text_no_suffix_when_fits() {
        let text = "exactly10chars";
        // 14 chars > 13 limit, so it truncates with the custom suffix.
        let result = truncate_text(text, 13, Some("XYZ"));
        assert_eq!(result, "exactly10cXYZ");
    }

    #[test]
    fn test_truncate_text_suffix_longer_than_limit_returns_suffix() {
        // Regression test: previously `limit - suffix_.len()` underflowed in debug.
        let text = "hello world this is long";
        let result = truncate_text(text, 3, Some("VERY_LONG_SUFFIX"));
        assert_eq!(result, "VERY_LONG_SUFFIX");
    }

    #[test]
    fn test_truncate_text_left_suffix_longer_than_limit_returns_suffix() {
        let text = "hello world this is long";
        let result = truncate_text_left(text, 3, Some("VERY_LONG_SUFFIX"));
        assert_eq!(result, "VERY_LONG_SUFFIX");
    }

    #[test]
    fn test_truncate_text_default_suffix_at_limit_3() {
        let text = "hello world";
        // limit 3, default suffix "..." (len 3) → keep = 0, only suffix returned.
        let result = truncate_text(text, 3, None);
        assert_eq!(result, "...");
    }

    #[test]
    fn test_truncate_text_left_default_suffix_at_limit_3() {
        let text = "hello world";
        let result = truncate_text_left(text, 3, None);
        assert_eq!(result, "...");
    }
}
