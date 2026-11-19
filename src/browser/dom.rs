#![allow(clippy::write_with_newline)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write as _;

/// Element coordinates from DOM rect or quad calculation
#[derive(Debug, Clone, PartialEq)]
pub struct ElementRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl ElementRect {
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedElement {
    pub index: u32,
    pub tag: String,
    pub text: String,
    pub attributes: HashMap<String, String>,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserState {
    pub url: String,
    pub title: String,
    pub elements: Vec<EnhancedElement>,
    pub text: String,
}

impl BrowserState {
    pub fn format_for_prompt(&self) -> String {
        let mut out = String::new();
        let _ = write!(out, "Current URL: {}\n", self.url);
        let _ = write!(out, "Page Title: {}\n", self.title);

        out.push_str("\nInteractive Elements:\n");
        if self.elements.is_empty() {
            out.push_str("(No interactive elements found)\n");
        } else {
            for el in &self.elements {
                let mut attr_str = String::new();
                for (k, v) in &el.attributes {
                    if [
                        "type",
                        "name",
                        "id",
                        "placeholder",
                        "aria-label",
                        "href",
                        "role",
                    ]
                    .contains(&k.as_str())
                    {
                        let _ = write!(attr_str, " {k}=\"{v}\"");
                    }
                }
                let _ = write!(
                    out,
                    "[{}]{}{}>{}</{}>\n",
                    el.index, el.tag, attr_str, el.text, el.tag
                );
            }
        }

        out.push_str("\nPage Text Content (summary):\n");
        let truncated_text = if self.text.len() > 3000 {
            format!("{}...", &self.text[..3000])
        } else {
            self.text.clone()
        };
        out.push_str(&truncated_text);
        out.push('\n');

        out
    }
}

pub const EXTRACT_DOM_JS: &str = r#"
(() => {
    const isVisible = (elem) => {
        if (!elem) return false;
        const style = window.getComputedStyle(elem);
        if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return false;
        const rect = elem.getBoundingClientRect();
        return rect.width > 0 && rect.height > 0 && rect.bottom >= 0 && rect.top <= window.innerHeight;
    };

    const interactiveSelectors = [
        'a[href]', 'button', 'input', 'textarea', 'select',
        '[role="button"]', '[role="link"]', '[role="checkbox"]', '[role="menuitem"]',
        '[role="tab"]', '[role="combobox"]', '[role="switch"]', '[tabindex]:not([tabindex="-1"])'
    ];

    const elements = [];
    const matched = Array.from(document.querySelectorAll(interactiveSelectors.join(',')))
        .filter(isVisible);

    matched.slice(0, 100).forEach((el, i) => {
        const index = i + 1;
        const rect = el.getBoundingClientRect();
        const attrs = {};
        for (const attr of el.attributes) {
            attrs[attr.name] = attr.value;
        }

        let text = (el.innerText || el.textContent || el.getAttribute('aria-label') || el.getAttribute('placeholder') || el.getAttribute('value') || '').trim();
        if (text.length > 80) {
            text = text.substring(0, 77) + '...';
        }

        elements.push({
            index: index,
            tag: el.tagName.toLowerCase(),
            text: text,
            attributes: attrs,
            x: rect.left + rect.width / 2,
            y: rect.top + rect.height / 2
        });
    });

    let bodyText = document.body ? document.body.innerText : '';
    if (bodyText.length > 4000) {
        bodyText = bodyText.substring(0, 4000) + '...';
    }

    return {
        url: window.location.href,
        title: document.title,
        elements: elements,
        text: bodyText
    };
})()
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_element_rect_center() {
        let rect = ElementRect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let (cx, cy) = rect.center();
        assert!((cx - 60.0).abs() < f64::EPSILON);
        assert!((cy - 45.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_browser_state_format_with_elements() {
        let state = BrowserState {
            url: "https://example.com".into(),
            title: "Example".into(),
            elements: vec![EnhancedElement {
                index: 1,
                tag: "button".into(),
                text: "Click me".into(),
                attributes: HashMap::new(),
                x: 10.0,
                y: 20.0,
            }],
            text: "Page content".into(),
        };

        let formatted = state.format_for_prompt();
        assert!(formatted.contains("Current URL: https://example.com"));
        assert!(formatted.contains("Page Title: Example"));
        // Format is [index]<tag>text</tag>
        assert!(formatted.contains("[1]button>Click me</button"));
        assert!(formatted.contains("Page content"));
    }

    #[test]
    fn test_browser_state_format_empty_elements() {
        let state = BrowserState {
            url: String::new(),
            title: String::new(),
            elements: vec![],
            text: String::new(),
        };

        let formatted = state.format_for_prompt();
        assert!(formatted.contains("(No interactive elements found)"));
    }

    #[test]
    fn test_browser_state_format_truncates_long_text() {
        let long_text = "x".repeat(4000);
        let state = BrowserState {
            url: "test".into(),
            title: "test".into(),
            elements: vec![],
            text: long_text.clone(),
        };

        let formatted = state.format_for_prompt();
        assert!(formatted.ends_with("...\n"));
        // Should contain truncated content
        assert!(formatted.contains(&long_text[..3000]));
    }

    #[test]
    fn test_browser_state_format_with_attributes() {
        let mut attrs = HashMap::new();
        attrs.insert("type".into(), "text".into());
        attrs.insert("name".into(), "username".into());
        attrs.insert("class".into(), "input".into()); // should not appear

        let state = BrowserState {
            url: "test".into(),
            title: "test".into(),
            elements: vec![EnhancedElement {
                index: 1,
                tag: "input".into(),
                text: String::new(),
                attributes: attrs,
                x: 0.0,
                y: 0.0,
            }],
            text: String::new(),
        };

        let formatted = state.format_for_prompt();
        assert!(formatted.contains(r#"type="text""#));
        assert!(formatted.contains(r#"name="username""#));
        assert!(!formatted.contains("class"));
    }

    #[test]
    fn test_extract_dom_js_constant_exists() {
        assert_ne!(EXTRACT_DOM_JS, "");
        assert!(EXTRACT_DOM_JS.contains("getBoundingClientRect"));
        assert!(EXTRACT_DOM_JS.contains("interactiveSelectors"));
    }

    #[test]
    fn test_enhanced_element_structure() {
        let elem = EnhancedElement {
            index: 5,
            tag: "a".into(),
            text: "Link".into(),
            attributes: HashMap::from([("href".into(), "https://example.com".into())]),
            x: 100.0,
            y: 200.0,
        };
        assert_eq!(elem.index, 5);
        assert_eq!(elem.tag, "a");
        assert_eq!(
            elem.attributes.get("href"),
            Some(&"https://example.com".to_string())
        );
    }

    #[test]
    fn test_browser_state_clone() {
        let state = BrowserState {
            url: "https://example.com".into(),
            title: "Example".into(),
            elements: vec![],
            text: "Content".into(),
        };
        let cloned = state.clone();
        assert_eq!(state.url, cloned.url);
        assert_eq!(state.title, cloned.title);
        assert_eq!(state.text, cloned.text);
    }

    #[test]
    fn test_browser_state_format_with_multiple_elements() {
        let state = BrowserState {
            url: "https://multi.example.com".into(),
            title: "Multi".into(),
            elements: vec![
                EnhancedElement {
                    index: 1,
                    tag: "button".into(),
                    text: "First".into(),
                    attributes: HashMap::new(),
                    x: 0.0,
                    y: 0.0,
                },
                EnhancedElement {
                    index: 2,
                    tag: "a".into(),
                    text: "Link".into(),
                    attributes: HashMap::from([("href".into(), "#".into())]),
                    x: 0.0,
                    y: 0.0,
                },
            ],
            text: "Text".into(),
        };
        let formatted = state.format_for_prompt();
        assert!(formatted.contains("[1]button>First</button"));
        // Link with href attribute should appear in formatted output
        assert!(formatted.contains("[2]a"));
        assert!(formatted.contains("href=\"#\""));
        assert!(formatted.contains("Link"));
    }

    #[test]
    fn test_browser_state_format_text_truncation_at_boundary() {
        // Exactly 3000 chars should not be truncated
        let text_3000 = "x".repeat(3000);
        let state = BrowserState {
            url: "u".into(),
            title: "t".into(),
            elements: vec![],
            text: text_3000.clone(),
        };
        let formatted = state.format_for_prompt();
        assert!(!formatted.contains("..."));
        assert!(formatted.contains(&text_3000));

        // 3001 chars should be truncated
        let text_3001 = "x".repeat(3001);
        let state2 = BrowserState {
            url: "u".into(),
            title: "t".into(),
            elements: vec![],
            text: text_3001,
        };
        let formatted2 = state2.format_for_prompt();
        assert!(formatted2.contains("..."));
    }

    #[test]
    fn test_extract_dom_js_contains_key_features() {
        assert!(EXTRACT_DOM_JS.contains("getBoundingClientRect"));
        assert!(EXTRACT_DOM_JS.contains("getComputedStyle"));
        assert!(EXTRACT_DOM_JS.contains("querySelectorAll"));
        assert!(EXTRACT_DOM_JS.contains("window.location.href"));
        assert!(EXTRACT_DOM_JS.contains("document.title"));
    }

    #[test]
    fn test_element_rect_negative_coords() {
        let rect = ElementRect {
            x: -10.0,
            y: -5.0,
            width: 100.0,
            height: 50.0,
        };
        let (cx, cy) = rect.center();
        assert!((cx - 40.0).abs() < f64::EPSILON);
        assert!((cy - 20.0).abs() < f64::EPSILON);
    }
}
